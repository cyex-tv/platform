//! The cosmetics shop (ADR 0016): items staff sell, checkouts through cyex.tv's own Paddle
//! account, fulfilment from Paddle's signed webhook only, refunds, and orders.
//!
//! - `shop.manage`: create, change and delete items.
//! - `shop.orders`: see every order, refund.
//! - Anyone signed in buys for themselves or as a gift (recipient by Twitch or Kick login). An
//!   account that owns every cosmetic in an item can only gift it.
//!
//! A purchase makes entitlements with source `purchase` and the order's ID, and records every
//! cosmetic the order covers (also those owned already) in `shop_order_entitlements`. A refund or
//! chargeback takes back what nothing else covers: no other paid order, no redeemed code, no
//! staff grant (and a selection of it is cleared, which publishes `user.cosmetics_updated`).
//! Staff writes, fulfilments, refunds and payments that give nothing are audited; recipients are
//! notified of what they get and lose.

use std::collections::{HashMap, HashSet};

use axum::http::StatusCode;
use chrono::{DateTime, Utc};
use cyex_tv_auth::Caller;
use cyex_tv_db::cosmetics as cdb;
use cyex_tv_db::shop::{self as db, ItemFields, ItemRow, OrderFilter, OrderRow};
use cyex_tv_db::{DbError, accounts, ids, reads};
use cyex_tv_shared::error::{ApiError, ApiResult};
use cyex_tv_shared::model::{
    AccountSummary, Badge, Checkout, CosmeticKind, Order, OrderPage, OrderStatus, Paint, Recipient,
    ShopItem, ShopItemInput,
};
use cyex_tv_shared::{permissions, text};
use serde_json::{Value, json};
use sqlx::{Connection, PgConnection};
use uuid::Uuid;

use crate::catalog::{db_error, holds, not_found, parse_id, require_any};
use crate::errors::{self, Audit, bad, conflict, record_audit, sqlx_error};
use crate::limits::AccountLimit;
use crate::paddle::{self, PaddleError, SignatureError};
use crate::state::AppState;
use crate::views::time;

pub const NAME_MAX: usize = 64;
pub const DESCRIPTION_MAX: usize = 500;
pub const PRICE_MAX: u32 = 100_000;
pub const STOCK_MAX: u32 = 1_000_000;
pub const BUNDLE_MAX: usize = 10;
pub const REASON_MAX: usize = 500;

// Errors

/// Staff shop and code writes share the cosmetics writes limit (300 an hour per account).
pub(crate) fn staff_rate_limited(state: &AppState, caller: &Caller) -> ApiResult<()> {
    limited(state, AccountLimit::Cosmetics, caller)
}

/// One use of the per-account `limit`, at what the caller's roles allow.
pub(crate) fn limited(state: &AppState, limit: AccountLimit, caller: &Caller) -> ApiResult<()> {
    state
        .limits
        .take(
            limit,
            &ids::to_text(caller.account_id),
            Some(caller.account_id),
        )
        .map_err(|wait| {
            crate::errors::rate_limited_in("That's a lot of tries. Try again in a while.", wait)
        })
}

fn checkout_unavailable() -> ApiError {
    ApiError::new(
        StatusCode::SERVICE_UNAVAILABLE,
        "checkout_unavailable",
        "Buying isn't available yet.",
    )
}

fn provider_error(err: &PaddleError) -> ApiError {
    tracing::error!(error = %err, "paddle call failed");
    ApiError::new(
        StatusCode::BAD_GATEWAY,
        "payment_provider_error",
        "The payment provider didn't answer as expected. Try again in a moment.",
    )
}

// Checks on input (pure, unit-tested below)

pub fn item_name(raw: &str) -> ApiResult<String> {
    let name = raw.trim();
    let len = name.chars().count();
    if len == 0 || len > NAME_MAX || text::has_unsafe(name, false) {
        return Err(bad("bad_name", "Names are 1 to 64 characters."));
    }
    Ok(name.to_owned())
}

pub fn description(raw: Option<&str>) -> ApiResult<String> {
    let d = raw.unwrap_or("").trim();
    if d.chars().count() > DESCRIPTION_MAX || text::has_unsafe(d, true) {
        return Err(bad(
            "bad_description",
            "Descriptions are up to 500 characters.",
        ));
    }
    Ok(d.to_owned())
}

pub fn currency(raw: Option<&str>) -> ApiResult<String> {
    let c = raw
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("EUR");
    if c.len() != 3 || !c.bytes().all(|b| b.is_ascii_alphabetic()) {
        return Err(bad(
            "bad_currency",
            "Currencies are three letters, like EUR.",
        ));
    }
    Ok(c.to_ascii_uppercase())
}

pub fn price_id(raw: Option<&str>) -> ApiResult<Option<String>> {
    let Some(p) = raw.map(str::trim).filter(|s| !s.is_empty()) else {
        return Ok(None);
    };
    let ok = p.strip_prefix("pri_").is_some_and(|rest| {
        !rest.is_empty()
            && rest.len() <= 64
            && rest
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
    });
    if !ok {
        return Err(bad(
            "bad_price_id",
            "Paddle price IDs look like pri_01h…, in lower case.",
        ));
    }
    Ok(Some(p.to_owned()))
}

pub fn timestamp(raw: Option<&str>) -> ApiResult<Option<DateTime<Utc>>> {
    raw.map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| {
            DateTime::parse_from_rfc3339(s)
                .map(|t| t.with_timezone(&Utc))
                .map_err(|_| bad("bad_time", "Times are RFC 3339, like 2026-12-01T00:00:00Z."))
        })
        .transpose()
}

/// Badge and paint IDs: each parsed, each once, at least one and at most 10 in all.
pub fn bundle(
    badges: &[String],
    paints: &[String],
    allow_empty: bool,
) -> ApiResult<Vec<(CosmeticKind, Uuid)>> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for (kind, list) in [(CosmeticKind::Badge, badges), (CosmeticKind::Paint, paints)] {
        for raw in list {
            let id = ids::from_text(raw.trim()).ok_or_else(|| {
                bad(
                    "bad_cosmetics",
                    "One of the badges or paints doesn't exist.",
                )
            })?;
            if seen.insert((kind, id)) {
                out.push((kind, id));
            }
        }
    }
    if (out.is_empty() && !allow_empty) || out.len() > BUNDLE_MAX {
        return Err(bad(
            "bad_cosmetics",
            "An item holds 1 to 10 badges and paints.",
        ));
    }
    Ok(out)
}

/// An item's input, checked: everything but whether its cosmetics exist.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CheckedItem {
    pub name: String,
    pub description: String,
    pub price_cents: i32,
    pub currency: String,
    pub paddle_price_id: Option<String>,
    pub active_from: Option<DateTime<Utc>>,
    pub active_to: Option<DateTime<Utc>>,
    pub stock_limit: Option<i32>,
    pub visible: bool,
    pub sort_order: i32,
    pub bundle: Vec<(CosmeticKind, Uuid)>,
}

pub fn item_input(input: &ShopItemInput) -> ApiResult<CheckedItem> {
    let name = item_name(&input.name)?;
    let description = description(input.description.as_deref())?;
    if input.price_cents > PRICE_MAX {
        return Err(bad("bad_price", "Prices are 0 to 100000 cents."));
    }
    let active_from = timestamp(input.active_from.as_deref())?;
    let active_to = timestamp(input.active_to.as_deref())?;
    if let (Some(from), Some(to)) = (active_from, active_to)
        && to <= from
    {
        return Err(bad("bad_time", "The sale has to end after it starts."));
    }
    let stock_limit = match input.stock_limit {
        None => None,
        Some(n) if (1..=STOCK_MAX).contains(&n) => Some(i32::try_from(n).unwrap_or(i32::MAX)),
        Some(_) => {
            return Err(bad(
                "bad_stock",
                "Stock is 1 to 1000000, or none for no limit.",
            ));
        }
    };
    Ok(CheckedItem {
        name,
        description,
        price_cents: i32::try_from(input.price_cents).unwrap_or(i32::MAX),
        currency: currency(input.currency.as_deref())?,
        paddle_price_id: price_id(input.paddle_price_id.as_deref())?,
        active_from,
        active_to,
        stock_limit,
        visible: input.visible.unwrap_or(false),
        sort_order: input.sort_order.unwrap_or(0),
        bundle: bundle(&input.badge_ids, &input.paint_ids, false)?,
    })
}

pub fn refund_reason(raw: &str) -> ApiResult<String> {
    let r = raw.trim();
    let len = r.chars().count();
    if len == 0 || len > REASON_MAX || text::has_unsafe(r, true) {
        return Err(bad("bad_reason", "Give a reason of 1 to 500 characters."));
    }
    Ok(r.to_owned())
}

/// Page and size, clamped.
pub fn paging(page: Option<u32>, per_page: Option<u32>) -> (i64, i64) {
    let p = crate::paging::Paged::Orders.paging(page, per_page);
    (p.limit, p.offset)
}

/// Every cosmetic of `bundle` exists (`bad_cosmetics` otherwise).
pub(crate) async fn check_exist(
    conn: &mut PgConnection,
    bundle: &[(CosmeticKind, Uuid)],
) -> ApiResult<()> {
    let of = |k: CosmeticKind| -> Vec<Uuid> {
        bundle
            .iter()
            .filter(|(kind, _)| *kind == k)
            .map(|(_, id)| *id)
            .collect()
    };
    let (badges, paints) = (of(CosmeticKind::Badge), of(CosmeticKind::Paint));
    let found_b = cdb::badges(conn, Some(&badges))
        .await
        .map_err(db_error)?
        .len();
    let found_p = cdb::paints(conn, Some(&paints))
        .await
        .map_err(db_error)?
        .len();
    if found_b != badges.len() || found_p != paints.len() {
        return Err(bad(
            "bad_cosmetics",
            "One of the badges or paints doesn't exist.",
        ));
    }
    Ok(())
}

// Views

pub(crate) struct Cosmetics {
    badges: HashMap<Uuid, Badge>,
    paints: HashMap<Uuid, Paint>,
}

impl Cosmetics {
    /// Every badge and paint the bundles name, built once.
    pub(crate) async fn load<'a>(
        state: &AppState,
        conn: &mut PgConnection,
        bundles: impl Iterator<Item = &'a (CosmeticKind, Uuid)>,
    ) -> ApiResult<Self> {
        let (mut b, mut p) = (Vec::new(), Vec::new());
        for (kind, id) in bundles {
            match kind {
                CosmeticKind::Badge => b.push(*id),
                CosmeticKind::Paint => p.push(*id),
            }
        }
        let (badges, paints) =
            crate::cosmetics::build(conn, &state.cdn_url, Some(&b), Some(&p)).await?;
        let key = |id: &str| ids::from_text(id).unwrap_or_default();
        Ok(Self {
            badges: badges.into_iter().map(|x| (key(&x.id), x)).collect(),
            paints: paints.into_iter().map(|x| (key(&x.id), x)).collect(),
        })
    }

    pub(crate) fn of(&self, bundle: &[(CosmeticKind, Uuid)]) -> (Vec<Badge>, Vec<Paint>) {
        let mut badges = Vec::new();
        let mut paints = Vec::new();
        for (kind, id) in bundle {
            match kind {
                CosmeticKind::Badge => badges.extend(self.badges.get(id).cloned()),
                CosmeticKind::Paint => paints.extend(self.paints.get(id).cloned()),
            }
        }
        (badges, paints)
    }
}

fn opt_time(t: Option<DateTime<Utc>>) -> Option<String> {
    t.map(time)
}

/// Items as the API shows them, for `viewer` (whose `owned` flags they get). Paddle price IDs
/// only for `manager` (a holder of `shop.manage`).
async fn item_views(
    state: &AppState,
    conn: &mut PgConnection,
    rows: &[ItemRow],
    viewer: Option<Uuid>,
    manager: bool,
) -> ApiResult<Vec<ShopItem>> {
    let item_ids: Vec<Uuid> = rows.iter().map(|r| r.id).collect();
    let bundles = db::item_bundles(conn, &item_ids).await.map_err(db_error)?;
    let taken = db::taken(conn, &item_ids).await.map_err(db_error)?;
    let cosmetics = Cosmetics::load(state, conn, bundles.values().flatten()).await?;
    let mut owned: HashSet<(CosmeticKind, Uuid)> = HashSet::new();
    if let Some(account) = viewer {
        for kind in [CosmeticKind::Badge, CosmeticKind::Paint] {
            for id in cdb::owned(conn, kind, account).await.map_err(db_error)? {
                owned.insert((kind, id));
            }
        }
    }
    let now = Utc::now();
    let empty = Vec::new();
    Ok(rows
        .iter()
        .map(|row| {
            let bundle = bundles.get(&row.id).unwrap_or(&empty);
            let (badges, paints) = cosmetics.of(bundle);
            let stock_left = row.stock_limit.map(|limit| {
                let left = i64::from(limit) - taken.get(&row.id).copied().unwrap_or(0);
                u32::try_from(left.max(0)).unwrap_or(0)
            });
            ShopItem {
                id: ids::to_text(row.id),
                name: row.name.clone(),
                description: row.description.clone(),
                price_cents: u32::try_from(row.price_cents).unwrap_or(0),
                currency: row.currency.clone(),
                paddle_price_id: row.paddle_price_id.clone().filter(|_| manager),
                available: row.on_sale(now) && !bundle.is_empty() && stock_left != Some(0),
                owned: viewer.is_some()
                    && !bundle.is_empty()
                    && bundle.iter().all(|c| owned.contains(c)),
                badges,
                paints,
                active_from: opt_time(row.active_from),
                active_to: opt_time(row.active_to),
                stock_limit: row.stock_limit.and_then(|n| u32::try_from(n).ok()),
                stock_left,
                visible: row.visible,
                sort_order: row.sort_order,
                deleted: row.deleted_at.is_some(),
                created_at: time(row.created_at),
                updated_at: time(row.updated_at),
            }
        })
        .collect())
}

async fn one_item(
    state: &AppState,
    conn: &mut PgConnection,
    row: &ItemRow,
    viewer: Option<Uuid>,
    manager: bool,
) -> ApiResult<ShopItem> {
    item_views(state, conn, std::slice::from_ref(row), viewer, manager)
        .await?
        .pop()
        .ok_or_else(ApiError::internal)
}

fn summary(map: &HashMap<Uuid, reads::Summary>, id: Uuid) -> AccountSummary {
    match map.get(&id) {
        Some(s) => AccountSummary {
            id: ids::to_text(s.account_id),
            display_name: s.display_name.clone(),
            avatar: s.avatar.clone(),
        },
        None => AccountSummary {
            id: ids::to_text(id),
            display_name: String::new(),
            avatar: None,
        },
    }
}

/// Who is looking at an order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Viewer {
    /// A holder of `shop.orders`: everything, staff notes included.
    Staff,
    /// A signed-in account: the buyer sees what they paid; a gift's recipient sees no money
    /// fields and no transaction ID.
    Account(Uuid),
}

impl Viewer {
    fn staff(self) -> bool {
        self == Self::Staff
    }

    /// Only the recipient of this order, not its buyer (nor staff).
    fn recipient_only(self, order: &OrderRow) -> bool {
        match self {
            Self::Staff => false,
            Self::Account(a) => order.account_id != Some(a),
        }
    }
}

/// Orders as the API shows them. Refund reasons are staff notes: only staff (holders of
/// `shop.orders`) see them. A gift's recipient doesn't see what the buyer paid: `amountCents` is
/// 0 and the paid total, its currency and the transaction ID are null.
async fn order_views(
    conn: &mut PgConnection,
    rows: &[OrderRow],
    viewer: Viewer,
) -> ApiResult<Vec<Order>> {
    let staff = viewer.staff();
    let people: Vec<Uuid> = rows
        .iter()
        .flat_map(|r| r.account_id.into_iter().chain(r.recipient_id))
        .collect();
    let summaries = reads::summaries(conn, &people).await.map_err(db_error)?;
    Ok(rows
        .iter()
        .map(|r| {
            let money = !viewer.recipient_only(r);
            Order {
                id: ids::to_text(r.id),
                buyer: r.account_id.map(|a| summary(&summaries, a)),
                recipient: r.recipient_id.map(|a| summary(&summaries, a)),
                gift: r.is_gift,
                item_id: ids::to_text(r.item_id),
                item_name: r.item_name.clone(),
                status: order_status(&r.status),
                amount_cents: if money {
                    u32::try_from(r.amount_cents).unwrap_or(0)
                } else {
                    0
                },
                currency: r.currency.clone(),
                paid_total_cents: r
                    .paid_total_cents
                    .and_then(|v| u64::try_from(v).ok())
                    .filter(|_| money),
                paid_currency: r.paid_currency.clone().filter(|_| money),
                provider: r.provider.clone(),
                provider_transaction_id: r.provider_transaction_id.clone().filter(|_| money),
                refund_reason: r.refund_reason.clone().filter(|_| staff),
                created_at: time(r.created_at),
                paid_at: opt_time(r.paid_at),
                refunded_at: opt_time(r.refunded_at),
                failed_at: opt_time(r.failed_at),
                refund_rejected_at: opt_time(r.refund_rejected_at).filter(|_| staff),
            }
        })
        .collect())
}

/// An order's status as the API shows it: a refund being sent to Paddle is still paid.
pub fn order_status(raw: &str) -> OrderStatus {
    match raw {
        "refund_requested" => OrderStatus::Paid,
        other => OrderStatus::parse(other).unwrap_or(OrderStatus::Pending),
    }
}

async fn one_order(conn: &mut PgConnection, row: &OrderRow, viewer: Viewer) -> ApiResult<Order> {
    order_views(conn, std::slice::from_ref(row), viewer)
        .await?
        .pop()
        .ok_or_else(ApiError::internal)
}

fn order_json(order: &OrderRow) -> Value {
    json!({
        "status": order.status,
        "item_id": ids::to_text(order.item_id),
        "account_id": order.account_id.map(ids::to_text),
        "recipient_id": order.recipient_id.map(ids::to_text),
        "provider_transaction_id": order.provider_transaction_id,
    })
}

// Public reads

/// Items on sale now; with `all`, every item that isn't deleted (needs `shop.manage`).
pub async fn items(
    state: &AppState,
    caller: Option<&Caller>,
    all: bool,
) -> ApiResult<Vec<ShopItem>> {
    let mut conn = errors::conn(state).await?;
    if all {
        let caller = caller.ok_or_else(|| {
            ApiError::new(
                StatusCode::UNAUTHORIZED,
                "signed_out",
                "Sign in to use this.",
            )
        })?;
        require_any(&mut conn, caller, &[permissions::SHOP_MANAGE]).await?;
    }
    let manager = all || is_manager(&mut conn, caller).await?;
    let rows = db::items(&mut conn, (!all).then(Utc::now))
        .await
        .map_err(db_error)?;
    item_views(
        state,
        &mut conn,
        &rows,
        caller.map(|c| c.account_id),
        manager,
    )
    .await
}

async fn is_manager(conn: &mut PgConnection, caller: Option<&Caller>) -> ApiResult<bool> {
    match caller {
        Some(c) => holds(conn, c, permissions::SHOP_MANAGE).await,
        None => Ok(false),
    }
}

/// One item: on sale for everyone, any (deleted ones too) for `shop.manage`.
pub async fn item(
    state: &AppState,
    caller: Option<&Caller>,
    id: &str,
) -> ApiResult<Option<ShopItem>> {
    let Some(id) = ids::from_text(id.trim()) else {
        return Ok(None);
    };
    let mut conn = errors::conn(state).await?;
    let Some(row) = db::get_item(&mut conn, id).await.map_err(db_error)? else {
        return Ok(None);
    };
    let manager = is_manager(&mut conn, caller).await?;
    if !row.on_sale(Utc::now()) && !manager {
        return Ok(None);
    }
    one_item(
        state,
        &mut conn,
        &row,
        caller.map(|c| c.account_id),
        manager,
    )
    .await
    .map(Some)
}

// Staff: items

async fn item_write(
    state: &AppState,
    caller: &Caller,
    id: Option<Uuid>,
    input: &ShopItemInput,
) -> ApiResult<ShopItem> {
    let checked = item_input(input)?;
    let mut conn = errors::conn(state).await?;
    require_any(&mut conn, caller, &[permissions::SHOP_MANAGE]).await?;
    staff_rate_limited(state, caller)?;
    let mut tx = conn.begin().await.map_err(sqlx_error)?;
    check_exist(&mut tx, &checked.bundle).await?;
    let before = match id {
        Some(id) => {
            let row = db::get_item(&mut tx, id)
                .await
                .map_err(db_error)?
                .filter(|r| r.deleted_at.is_none())
                .ok_or_else(not_found)?;
            Some(one_item(state, &mut tx, &row, None, true).await?)
        }
        None => None,
    };
    let fields = ItemFields {
        name: &checked.name,
        description: &checked.description,
        price_cents: checked.price_cents,
        currency: &checked.currency,
        paddle_price_id: checked.paddle_price_id.as_deref(),
        active_from: checked.active_from,
        active_to: checked.active_to,
        stock_limit: checked.stock_limit,
        visible: checked.visible,
        sort_order: checked.sort_order,
    };
    let row = match id {
        Some(id) => db::replace_item(&mut tx, id, &fields, &checked.bundle).await,
        None => db::create_item(&mut tx, &fields, &checked.bundle, Some(caller.account_id)).await,
    }
    .map_err(|err| match err {
        DbError::NotFound if id.is_none() => bad(
            "bad_cosmetics",
            "One of the badges or paints doesn't exist.",
        ),
        other => db_error(other),
    })?;
    let item = one_item(state, &mut tx, &row, None, true).await?;
    // Putting a cosmetic on sale releases it: chat clients are told (only when it is on sale now).
    for (kind, cosmetic) in &checked.bundle {
        crate::cosmetics::announce_release(&mut tx, &state.cdn_url, *kind, *cosmetic, false)
            .await?;
    }
    let to_json = |v: &ShopItem| serde_json::to_value(v).unwrap_or(Value::Null);
    record_audit(
        &mut tx,
        Some(caller.account_id),
        Audit::new(
            if before.is_some() {
                "shop_item.updated"
            } else {
                "shop_item.created"
            },
            "shop_item",
            &ids::to_text(row.id),
        )
        .change(before.as_ref().map(to_json), Some(to_json(&item))),
    )
    .await?;
    tx.commit().await.map_err(sqlx_error)?;
    Ok(item)
}

pub async fn create_item(
    state: &AppState,
    caller: &Caller,
    input: &ShopItemInput,
) -> ApiResult<ShopItem> {
    item_write(state, caller, None, input).await
}

/// Replaces every field of an item. Orders already made keep their price and name.
pub async fn update_item(
    state: &AppState,
    caller: &Caller,
    id: &str,
    input: &ShopItemInput,
) -> ApiResult<ShopItem> {
    let id = parse_id(id)?;
    item_write(state, caller, Some(id), input).await
}

/// Deletes an item: it is never listed or sold again. Its orders and code batches stay.
pub async fn delete_item(state: &AppState, caller: &Caller, id: &str) -> ApiResult<bool> {
    let id = parse_id(id)?;
    let mut conn = errors::conn(state).await?;
    require_any(&mut conn, caller, &[permissions::SHOP_MANAGE]).await?;
    staff_rate_limited(state, caller)?;
    let mut tx = conn.begin().await.map_err(sqlx_error)?;
    if !db::delete_item(&mut tx, id).await.map_err(db_error)? {
        return Ok(false);
    }
    record_audit(
        &mut tx,
        Some(caller.account_id),
        Audit::new("shop_item.deleted", "shop_item", &ids::to_text(id)).change(None, None),
    )
    .await?;
    tx.commit().await.map_err(sqlx_error)?;
    Ok(true)
}

// Checkout

/// The account a gift goes to: a live account with this login on that platform.
async fn recipient_of(conn: &mut PgConnection, who: &Recipient) -> ApiResult<Uuid> {
    let login = who.login.trim().trim_start_matches('@');
    let missing = || {
        ApiError::new(
            StatusCode::NOT_FOUND,
            "recipient_not_found",
            "No cyex.tv account has that login.",
        )
    };
    if login.is_empty() || login.len() > 64 {
        return Err(missing());
    }
    let identity = accounts::find_by_login(conn, who.platform, login)
        .await
        .map_err(db_error)?
        .ok_or_else(missing)?;
    let live = accounts::get(conn, identity.account_id)
        .await
        .map_err(db_error)?
        .is_some_and(|a| a.deleted_at.is_none() && a.blocked_at.is_none());
    if !live {
        return Err(missing());
    }
    Ok(identity.account_id)
}

fn item_unavailable() -> ApiError {
    conflict("item_unavailable", "That item isn't on sale.")
}

/// Whether the item, as read under its lock, can be bought at `now`: on sale, with cosmetics,
/// and with a Paddle price. Gives that price.
pub fn sellable(
    item: &ItemRow,
    bundle: &[(CosmeticKind, Uuid)],
    now: DateTime<Utc>,
) -> ApiResult<String> {
    if !item.on_sale(now) || bundle.is_empty() {
        return Err(item_unavailable());
    }
    item.paddle_price_id
        .clone()
        .ok_or_else(checkout_unavailable)
}

/// How long a checkout that hasn't got its Paddle transaction yet is left to finish.
pub const STARTING_SECS: i64 = 120;

/// What to do with the buyer's pending order for the same item and recipient.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reuse {
    /// Its transaction is still open: hand it out again.
    Again,
    /// Another request is making its transaction right now.
    Busy,
    /// Fail it (`replaced`) and start a new one: its hold ran out, or the price changed.
    Replace,
}

/// Whether two bundles hold the same cosmetics, in any order.
fn same_bundle(a: &[(CosmeticKind, Uuid)], b: &[(CosmeticKind, Uuid)]) -> bool {
    let a: HashSet<_> = a.iter().collect();
    let b: HashSet<_> = b.iter().collect();
    a == b
}

/// `bundle` is the item's cosmetics now, read under its lock. An order that sells other
/// cosmetics (or was made before orders kept theirs) is replaced, like one with another price.
pub fn reuse_of(
    open: &OrderRow,
    item: &ItemRow,
    bundle: &[(CosmeticKind, Uuid)],
    now: DateTime<Utc>,
) -> Reuse {
    let age = now - open.created_at;
    if open.provider_transaction_id.is_none() {
        return if age < chrono::Duration::seconds(STARTING_SECS) {
            Reuse::Busy
        } else {
            Reuse::Replace
        };
    }
    let same_price = open.paddle_price_id == item.paddle_price_id
        && open.amount_cents == item.price_cents
        && open.currency == item.currency;
    let same_cosmetics = open.bundle().is_some_and(|b| same_bundle(&b, bundle));
    if same_price && same_cosmetics && age < chrono::Duration::minutes(db::HOLD_MINUTES) {
        Reuse::Again
    } else {
        Reuse::Replace
    }
}

async fn checkout_of(
    state: &AppState,
    conn: &mut PgConnection,
    order: &OrderRow,
) -> ApiResult<Checkout> {
    let paddle = state.paddle.as_ref().ok_or_else(checkout_unavailable)?;
    Ok(Checkout {
        order: one_order(
            conn,
            order,
            Viewer::Account(order.account_id.unwrap_or_default()),
        )
        .await?,
        transaction_id: order
            .provider_transaction_id
            .clone()
            .ok_or_else(ApiError::internal)?,
        checkout_url: order.checkout_url.clone(),
        client_token: paddle.cfg.client_token.clone(),
        environment: paddle.cfg.env.as_str().to_owned(),
    })
}

/// A checkout that didn't get going: the order is failed, and the audit row says why (with
/// Paddle's transaction ID when Paddle made one, so staff can find the payment).
async fn checkout_failed(
    conn: &mut PgConnection,
    order: &OrderRow,
    reason: &str,
    transaction_id: Option<&str>,
    error: &str,
) -> ApiResult<()> {
    db::mark_failed(conn, order.id, reason)
        .await
        .map_err(db_error)?;
    record_audit(
        conn,
        None,
        Audit::new(
            "shop.checkout_failed",
            "shop_order",
            &ids::to_text(order.id),
        )
        .change(
            Some(order_json(order)),
            Some(json!({
                "status": "failed",
                "reason": reason,
                "provider_transaction_id": transaction_id,
                "error": error,
            })),
        ),
    )
    .await
}

/// Starts buying an item, for the caller or as a gift. Makes a pending order and a Paddle
/// transaction; the website opens Paddle's checkout for it. The order is paid only when Paddle's
/// webhook says so. A buyer has at most one pending order per item and recipient: asking again
/// within the hold hands out the same transaction.
pub async fn start_checkout(
    state: &AppState,
    caller: &Caller,
    item_id: &str,
    recipient: Option<&Recipient>,
) -> ApiResult<Checkout> {
    let item_id = parse_id(item_id)?;
    limited(state, AccountLimit::Checkout, caller)?;
    let mut conn = errors::conn(state).await?;
    let item = db::get_item(&mut conn, item_id)
        .await
        .map_err(db_error)?
        .ok_or_else(not_found)?;
    if !item.on_sale(Utc::now()) {
        return Err(item_unavailable());
    }
    let paddle = state
        .paddle
        .clone()
        .filter(|p| p.checkout_ready())
        .ok_or_else(checkout_unavailable)?;
    if item.paddle_price_id.is_none() {
        return Err(checkout_unavailable());
    }
    let recipient_id = match recipient {
        Some(who) => recipient_of(&mut conn, who).await?,
        None => caller.account_id,
    };

    // Everything that decides the sale is read again under the item's lock: staff may have
    // hidden it, changed its price or its cosmetics since the read above.
    let mut tx = conn.begin().await.map_err(sqlx_error)?;
    let locked = db::lock_item(&mut tx, item.id)
        .await
        .map_err(db_error)?
        .ok_or_else(not_found)?;
    let bundle = db::item_bundles(&mut tx, &[locked.id])
        .await
        .map_err(db_error)?
        .remove(&locked.id)
        .unwrap_or_default();
    let price_id = sellable(&locked, &bundle, Utc::now())?;
    let owned = db::owned_of(&mut tx, recipient_id, &bundle)
        .await
        .map_err(db_error)?;
    if owned.len() == bundle.len() {
        return Err(conflict(
            "already_owned",
            if recipient_id == caller.account_id {
                "You own everything in it already. You can still give it to someone."
            } else {
                "They own everything in it already."
            },
        ));
    }
    let mut replaced: Option<OrderRow> = None;
    if let Some(open) = db::lock_pending(&mut tx, caller.account_id, locked.id, recipient_id)
        .await
        .map_err(db_error)?
    {
        match reuse_of(&open, &locked, &bundle, Utc::now()) {
            Reuse::Again => {
                let out = checkout_of(state, &mut tx, &open).await?;
                tx.commit().await.map_err(sqlx_error)?;
                return Ok(out);
            }
            Reuse::Busy => return Err(db_error(DbError::PendingOrder)),
            Reuse::Replace => {
                db::mark_failed(&mut tx, open.id, db::failed::REPLACED)
                    .await
                    .map_err(db_error)?;
                record_audit(
                    &mut tx,
                    Some(caller.account_id),
                    Audit::new("shop.order_replaced", "shop_order", &ids::to_text(open.id)).change(
                        Some(order_json(&open)),
                        Some(json!({ "status": "failed", "reason": db::failed::REPLACED })),
                    ),
                )
                .await?;
                replaced = Some(open);
            }
        }
    }
    if let Some(limit) = locked.stock_limit {
        let taken = db::taken(&mut tx, &[locked.id])
            .await
            .map_err(db_error)?
            .get(&locked.id)
            .copied()
            .unwrap_or(0);
        if taken >= i64::from(limit) {
            return Err(conflict("sold_out", "That item is sold out."));
        }
    }
    let order = db::create_order(&mut tx, caller.account_id, recipient_id, &locked, &bundle)
        .await
        .map_err(db_error)?;
    tx.commit().await.map_err(sqlx_error)?;

    // The replaced checkout can't be paid any more (best effort; the worker's sweep tries any
    // left over).
    if let Some(old) = replaced {
        cancel_transaction(&paddle, &mut conn, &old).await?;
    }

    // Paddle, without holding the lock.
    let order_text = ids::to_text(order.id);
    let transaction = match paddle.create_transaction(&price_id, &order_text).await {
        Ok(t) => t,
        Err(err) => {
            checkout_failed(
                &mut conn,
                &order,
                db::failed::PROVIDER_ERROR,
                None,
                &err.to_string(),
            )
            .await?;
            return Err(provider_error(&err));
        }
    };
    let currency = transaction.currency_code.as_deref();
    let url = transaction.checkout_url.as_deref();
    let saved = match db::set_transaction(&mut conn, order.id, &transaction.id, currency, url).await
    {
        Ok(row) => Ok(row),
        Err(first) => {
            tracing::warn!(error = %first, txn = %transaction.id, "saving the paddle transaction failed; trying again");
            drop(conn);
            conn = errors::conn(state).await?;
            db::set_transaction(&mut conn, order.id, &transaction.id, currency, url).await
        }
    };
    let order = match saved {
        Ok(row) => row,
        Err(err) => {
            // Paddle has a transaction we couldn't tie to the order. It is never fulfilled
            // automatically; the audit row (and this log line) name it for staff.
            tracing::error!(error = %err, txn = %transaction.id, order = %order_text, "paddle transaction not saved on its order");
            let mut fresh = errors::conn(state).await?;
            checkout_failed(
                &mut fresh,
                &order,
                db::failed::TRANSACTION_NOT_SAVED,
                Some(&transaction.id),
                &err.to_string(),
            )
            .await?;
            return Err(ApiError::new(
                StatusCode::SERVICE_UNAVAILABLE,
                "checkout_failed",
                "The checkout couldn't be started. Nothing was charged; try again in a moment.",
            ));
        }
    };
    checkout_of(state, &mut conn, &order).await
}

/// Cancels the Paddle transaction of a replaced or expired order, so its old checkout can't be
/// paid. When Paddle says no (it was paid meanwhile, or Paddle is down), the order is flagged and
/// audited (`shop.transaction_cancel_failed`), and a late payment is still taken.
pub async fn cancel_transaction(
    paddle: &paddle::Paddle,
    conn: &mut PgConnection,
    order: &OrderRow,
) -> ApiResult<()> {
    let Some(txn) = order.provider_transaction_id.as_deref() else {
        return Ok(());
    };
    match paddle.cancel_transaction(txn).await {
        Ok(()) => {
            db::mark_canceled(conn, order.id).await.map_err(db_error)?;
        }
        Err(err) => {
            tracing::warn!(error = %err, %txn, order = %ids::to_text(order.id), "cancelling an old paddle transaction failed; a late payment is still taken");
            if db::mark_cancel_failed(conn, order.id)
                .await
                .map_err(db_error)?
            {
                record_audit(
                    conn,
                    None,
                    Audit::new(
                        "shop.transaction_cancel_failed",
                        "shop_order",
                        &ids::to_text(order.id),
                    )
                    .change(
                        Some(order_json(order)),
                        Some(json!({
                            "provider_transaction_id": txn,
                            "reason": order.failed_reason,
                            "error": err.to_string(),
                        })),
                    ),
                )
                .await?;
            }
        }
    }
    Ok(())
}

// Orders

/// Orders the caller bought, and gifts they got once paid, newest first.
pub async fn my_orders(
    state: &AppState,
    caller: &Caller,
    page: Option<u32>,
    per_page: Option<u32>,
) -> ApiResult<OrderPage> {
    let (limit, offset) = paging(page, per_page);
    let mut conn = errors::conn(state).await?;
    let filter = OrderFilter {
        party: Some(caller.account_id),
        ..OrderFilter::default()
    };
    let (rows, total) = db::orders(&mut conn, &filter, limit, offset)
        .await
        .map_err(db_error)?;
    Ok(OrderPage {
        items: order_views(&mut conn, &rows, Viewer::Account(caller.account_id)).await?,
        total: u64::try_from(total).unwrap_or(0),
    })
}

/// One order, for its buyer, its recipient (once paid), or `shop.orders`. Poll it after a
/// checkout.
pub async fn order(state: &AppState, caller: &Caller, id: &str) -> ApiResult<Option<Order>> {
    let Some(id) = ids::from_text(id.trim()) else {
        return Ok(None);
    };
    let mut conn = errors::conn(state).await?;
    let Some(row) = db::get_order(&mut conn, id).await.map_err(db_error)? else {
        return Ok(None);
    };
    let party = row.account_id == Some(caller.account_id)
        || (row.recipient_id == Some(caller.account_id) && row.shown_to_recipient());
    let staff = holds(&mut conn, caller, permissions::SHOP_ORDERS).await?;
    if !party && !staff {
        return Ok(None);
    }
    let viewer = if staff {
        Viewer::Staff
    } else {
        Viewer::Account(caller.account_id)
    };
    one_order(&mut conn, &row, viewer).await.map(Some)
}

/// Every order, newest first, narrowed by status, account (bought or received) and item. Needs
/// `shop.orders`.
#[allow(clippy::too_many_arguments)]
pub async fn orders(
    state: &AppState,
    caller: &Caller,
    status: Option<OrderStatus>,
    account_id: Option<&str>,
    item_id: Option<&str>,
    page: Option<u32>,
    per_page: Option<u32>,
) -> ApiResult<OrderPage> {
    let filter_id = |raw: Option<&str>| {
        raw.map(str::trim)
            .filter(|s| !s.is_empty())
            .map(parse_id)
            .transpose()
    };
    let account = filter_id(account_id)?;
    let item = filter_id(item_id)?;
    let (limit, offset) = paging(page, per_page);
    let mut conn = errors::conn(state).await?;
    // `billing.view` reads one account's orders (the staff account page); every order, and
    // refunds, need `shop.orders`.
    if account.is_none() || !holds(&mut conn, caller, permissions::BILLING_VIEW).await? {
        require_any(&mut conn, caller, &[permissions::SHOP_ORDERS]).await?;
    }
    let filter = OrderFilter {
        account,
        status: status.map(OrderStatus::as_str),
        item,
        party: None,
    };
    let (rows, total) = db::orders(&mut conn, &filter, limit, offset)
        .await
        .map_err(db_error)?;
    Ok(OrderPage {
        items: order_views(&mut conn, &rows, Viewer::Staff).await?,
        total: u64::try_from(total).unwrap_or(0),
    })
}

/// Tells `account` about cosmetics it got (`cosmetic.granted`) or lost (`cosmetic.revoked`),
/// one notification each, with the payload `grantCosmetic` and `revokeCosmetic` use plus
/// `source` (`purchase` or `code`) and, for orders, `order_id`.
pub(crate) async fn notify_cosmetics(
    conn: &mut PgConnection,
    account: Uuid,
    kind: &str,
    cosmetics: &[(CosmeticKind, Uuid)],
    source: &str,
    reason: &str,
    order: Option<Uuid>,
) -> ApiResult<()> {
    for (k, id) in cosmetics {
        let name = crate::cosmetics::cosmetic_display_name(conn, *k, *id).await?;
        let mut payload = json!({
            "kind": k.as_str(),
            "cosmetic_id": ids::to_text(*id),
            "name": name,
            "reason": reason,
            "source": source,
        });
        if let Some(order) = order {
            payload["order_id"] = json!(ids::to_text(order));
        }
        crate::account::notify(conn, account, kind, &payload).await?;
    }
    Ok(())
}

/// Takes back what an order covers (what nothing else covers) and marks it refunded. A
/// selection of a taken-back cosmetic is cleared and published, and the recipient is told.
async fn reverse(
    conn: &mut PgConnection,
    order: &OrderRow,
    actor: Option<Uuid>,
    reason: Option<&str>,
    adjustment_id: Option<&str>,
    via: &str,
) -> ApiResult<OrderRow> {
    let recipient = order.recipient_id;
    let before = match recipient {
        Some(r) => Some(cdb::selection(conn, r).await.map_err(db_error)?),
        None => None,
    };
    let revoked = db::revoke_order(conn, order.id).await.map_err(db_error)?;
    if let (Some(r), Some(before)) = (recipient, before) {
        let after = cdb::selection(conn, r).await.map_err(db_error)?;
        if after != before {
            crate::cosmetics::publish_selection(conn, r, after).await?;
        }
    }
    let row = db::mark_refunded(conn, order.id, reason, adjustment_id)
        .await
        .map_err(db_error)?;
    record_audit(
        conn,
        actor,
        Audit::new("shop.order_refunded", "shop_order", &ids::to_text(order.id)).change(
            Some(order_json(order)),
            Some(json!({
                "status": "refunded",
                "reason": reason,
                "via": via,
                "provider_adjustment_id": adjustment_id,
                "revoked": revoked
                    .iter()
                    .map(|(k, _, c)| json!({ "kind": k.as_str(), "cosmetic_id": ids::to_text(*c) }))
                    .collect::<Vec<_>>(),
            })),
        ),
    )
    .await?;
    // The staff reason is a staff note; the recipient only hears that the order was refunded.
    let mut by_account: HashMap<Uuid, Vec<(CosmeticKind, Uuid)>> = HashMap::new();
    for (k, a, c) in revoked {
        by_account.entry(a).or_default().push((k, c));
    }
    let told = if via == "paddle_webhook" && reason == Some("Paddle chargeback") {
        "Payment charged back"
    } else {
        "Order refunded"
    };
    for (account, list) in by_account {
        notify_cosmetics(
            conn,
            account,
            "cosmetic.revoked",
            &list,
            "purchase",
            told,
            Some(order.id),
        )
        .await?;
    }
    Ok(row)
}

/// Paddle codes that mean the transaction can't be refunded (again): refunded already, a refund
/// waiting for approval, or nothing left to refund.
pub fn already_refunded(code: &str) -> bool {
    code.contains("already")
        || code.contains("pending_refund")
        || matches!(
            code,
            "adjustment_amount_above_remaining_allowed"
                | "adjustment_transaction_item_over_adjustment"
                | "adjustment_total_amount_above_remaining_allowed"
        )
}

fn refund_error(err: &PaddleError) -> ApiError {
    tracing::error!(error = %err, "paddle refund failed");
    match err {
        PaddleError::Api { code, .. } if already_refunded(code) => conflict(
            "already_refunded",
            "Paddle says this payment was refunded already, or a refund of it is waiting for approval. Nothing was changed here; Paddle's update will change the order.",
        ),
        _ => ApiError::new(
            StatusCode::BAD_GATEWAY,
            "refund_failed",
            "Paddle didn't take the refund. Nothing was changed.",
        ),
    }
}

/// A staff refund left as requested this long (the api stopped between asking Paddle and
/// finishing) may be tried again. Paddle answers a duplicate as `already_refunded`.
pub const REFUND_RETRY_MINUTES: i64 = 10;

/// Refunds a paid order (needs `shop.orders`). The refund is recorded as requested first, then
/// Paddle is asked for a full refund (when Paddle is set up and the order has a transaction)
/// without holding the order's lock, then the cosmetics are taken back. When Paddle says no, the
/// order is paid again.
pub async fn refund_order(
    state: &AppState,
    caller: &Caller,
    id: &str,
    reason: &str,
) -> ApiResult<Order> {
    let id = parse_id(id)?;
    let why = refund_reason(reason)?;
    let mut conn = errors::conn(state).await?;
    require_any(&mut conn, caller, &[permissions::SHOP_ORDERS]).await?;
    staff_rate_limited(state, caller)?;

    let mut tx = conn.begin().await.map_err(sqlx_error)?;
    let order = db::lock_order(&mut tx, id)
        .await
        .map_err(db_error)?
        .ok_or_else(not_found)?;
    let stuck = order
        .refund_requested_at
        .is_some_and(|t| Utc::now() - t >= chrono::Duration::minutes(REFUND_RETRY_MINUTES));
    match order.status.as_str() {
        "paid" => {}
        "refund_requested" if stuck => {}
        "refund_requested" => {
            return Err(conflict(
                "refund_in_progress",
                "A refund of this order is under way already.",
            ));
        }
        _ => {
            return Err(conflict(
                "not_refundable",
                "Only paid orders can be refunded.",
            ));
        }
    }
    // Never take the cosmetics back without refunding the payment.
    let paddle = state.paddle.clone().filter(|p| p.checkout_ready());
    if paddle.is_none() && order.provider_transaction_id.is_some() {
        return Err(ApiError::new(
            StatusCode::SERVICE_UNAVAILABLE,
            "refund_unavailable",
            "Refunds through Paddle aren't available right now. Nothing was changed.",
        ));
    }
    db::mark_refund_requested(&mut tx, id, &why)
        .await
        .map_err(db_error)?;
    record_audit(
        &mut tx,
        Some(caller.account_id),
        Audit::new("shop.refund_requested", "shop_order", &ids::to_text(id)).change(
            Some(order_json(&order)),
            Some(json!({ "status": "refund_requested", "reason": why, "retry": stuck })),
        ),
    )
    .await?;
    tx.commit().await.map_err(sqlx_error)?;

    let adjustment = match (paddle, order.provider_transaction_id.as_deref()) {
        (Some(paddle), Some(txn)) => match paddle.refund(txn, &why).await {
            Ok(adjustment) => Some(adjustment),
            Err(err) => {
                let mut tx = conn.begin().await.map_err(sqlx_error)?;
                if let Some(now) = db::lock_order(&mut tx, id).await.map_err(db_error)?
                    && now.status == "refund_requested"
                {
                    db::cancel_refund_request(&mut tx, id)
                        .await
                        .map_err(db_error)?;
                }
                record_audit(
                    &mut tx,
                    Some(caller.account_id),
                    Audit::new("shop.refund_failed", "shop_order", &ids::to_text(id))
                        .change(None, Some(json!({ "error": err.to_string() }))),
                )
                .await?;
                tx.commit().await.map_err(sqlx_error)?;
                return Err(refund_error(&err));
            }
        },
        _ => None,
    };
    let via = if adjustment.is_some() {
        "staff_paddle"
    } else {
        "staff_local"
    };

    let mut tx = conn.begin().await.map_err(sqlx_error)?;
    let current = db::lock_order(&mut tx, id)
        .await
        .map_err(db_error)?
        .ok_or_else(not_found)?;
    let row = if current.status == "refund_requested" {
        reverse(
            &mut tx,
            &current,
            Some(caller.account_id),
            Some(&why),
            adjustment.as_deref(),
            via,
        )
        .await?
    } else {
        // Paddle's webhook finished it while Paddle was being asked.
        match adjustment.as_deref() {
            Some(a) if current.provider_adjustment_id.is_none() => {
                db::set_adjustment(&mut tx, id, a).await.map_err(db_error)?
            }
            _ => current,
        }
    };
    let out = one_order(&mut tx, &row, Viewer::Staff).await?;
    tx.commit().await.map_err(sqlx_error)?;
    Ok(out)
}

// The Paddle webhook

/// What the webhook answers. Every handled or ignored event is 200, so Paddle stops retrying;
/// signature problems are 401, and errors on our side 500 (Paddle retries those).
pub async fn webhook(state: &AppState, signature: Option<&str>, body: &[u8]) -> ApiResult<()> {
    let Some(secret) = state
        .paddle
        .as_ref()
        .and_then(|p| p.cfg.webhook_secret.clone())
    else {
        return Err(ApiError::unavailable("Paddle isn't set up."));
    };
    paddle::verify_signature(&secret, signature, body, Utc::now().timestamp()).map_err(|err| {
        tracing::warn!(?err, "paddle webhook refused");
        let (code, message) = match err {
            SignatureError::Stale => ("stale_signature", "The signature is too old."),
            SignatureError::Malformed | SignatureError::Mismatch => {
                ("bad_signature", "The signature doesn't match.")
            }
        };
        ApiError::new(StatusCode::UNAUTHORIZED, code, message)
    })?;
    let note: paddle::Notification = serde_json::from_slice(body)
        .map_err(|_| bad("bad_payload", "Not a Paddle notification."))?;
    if note.event_id.is_empty() || note.event_id.len() > 128 {
        return Err(bad("bad_payload", "Not a Paddle notification."));
    }
    let mut conn = errors::conn(state).await?;
    let mut tx = conn.begin().await.map_err(sqlx_error)?;
    if !db::record_event(&mut tx, &note.event_id, &note.event_type)
        .await
        .map_err(db_error)?
    {
        tracing::info!(event = %note.event_id, "paddle event seen before; ignored");
        return Ok(());
    }
    match note.event_type.as_str() {
        "transaction.completed" => fulfil(&mut tx, &note.data).await?,
        "transaction.canceled" => cancel(&mut tx, &note.data).await?,
        "adjustment.created" | "adjustment.updated" => adjust(&mut tx, &note.data).await?,
        other => tracing::debug!(event_type = other, "paddle event not used"),
    }
    tx.commit().await.map_err(sqlx_error)?;
    Ok(())
}

/// The order a transaction belongs to, locked. Found only by the transaction ID stored when the
/// checkout made it, never by `custom_data` alone (anyone can make a Paddle transaction with the
/// public client token and tag it with any order). Our order ID in `custom_data`, when present,
/// has to agree.
async fn order_of(conn: &mut PgConnection, data: &Value) -> ApiResult<Option<OrderRow>> {
    let Some(txn) = data["id"].as_str() else {
        return Ok(None);
    };
    let tagged = data["custom_data"]["cyex_tv_order_id"].as_str();
    let Some(order) = db::lock_order_by_transaction(conn, txn)
        .await
        .map_err(db_error)?
    else {
        if let Some(tagged) = tagged {
            tracing::warn!(%txn, order = tagged, "paddle transaction isn't stored on any order; ignored, check it by hand");
        } else {
            tracing::info!(%txn, "paddle transaction isn't one of ours; ignored");
        }
        return Ok(None);
    };
    if tagged.is_some_and(|t| ids::from_text(t) != Some(order.id)) {
        tracing::warn!(%txn, "paddle transaction names another order; ignored");
        return Ok(None);
    }
    Ok(Some(order))
}

/// Whether a completed transaction is the one the checkout made: one item, the order's Paddle
/// price, quantity 1, in the currency Paddle gave it. The total isn't compared: Paddle adds the
/// buyer's tax, which depends on where they are.
pub fn matches_order(order: &OrderRow, data: &Value) -> Result<(), &'static str> {
    let items = data["items"].as_array().ok_or("no items")?;
    let [item] = items.as_slice() else {
        return Err("not exactly one item");
    };
    let expected = order
        .paddle_price_id
        .as_deref()
        .ok_or("order has no price")?;
    if item["price"]["id"].as_str() != Some(expected) {
        return Err("another price");
    }
    if item["quantity"].as_u64() != Some(1) {
        return Err("quantity isn't 1");
    }
    let currency = order
        .provider_currency
        .as_deref()
        .unwrap_or(&order.currency);
    if data["currency_code"].as_str() != Some(currency) {
        return Err("another currency");
    }
    Ok(())
}

/// A payment that gave nothing, for staff to look at (and usually refund): the audit row
/// `shop.order_paid_unfulfilled` with the transaction, the order's status and why.
async fn paid_unfulfilled(
    conn: &mut PgConnection,
    order: &OrderRow,
    transaction: &Value,
    status: &str,
    reason: &str,
) -> ApiResult<()> {
    tracing::warn!(order = %ids::to_text(order.id), reason, "paddle payment gave nothing; check it by hand");
    record_audit(
        conn,
        None,
        Audit::new(
            "shop.order_paid_unfulfilled",
            "shop_order",
            &ids::to_text(order.id),
        )
        .change(
            Some(order_json(order)),
            Some(json!({
                "provider_transaction_id": transaction,
                "status": status,
                "reason": reason,
            })),
        ),
    )
    .await
}

/// `transaction.completed`: the recipient gets every cosmetic of the item they don't own yet,
/// and the order covers all of them. Idempotent: an order that was paid already stays as it is.
/// A payment that can't give anything (the order was cancelled, the recipient is gone, the
/// transaction wasn't saved on its order) is paid or left as it is, and audited for staff.
async fn fulfil(conn: &mut PgConnection, data: &Value) -> ApiResult<()> {
    let Some(order) = order_of(conn, data).await? else {
        return unsaved_transaction(conn, data).await;
    };
    if !order.takes_payment() {
        // Paid before (a replay under a new event) needs nothing. A payment for an order that
        // was never paid and can't take it (cancelled at Paddle, or failed before its checkout
        // opened) gives nothing: staff refund it.
        if order.paid_at.is_none() {
            let why = format!(
                "order_{}{}",
                order.status,
                order
                    .failed_reason
                    .as_deref()
                    .map(|r| format!("_{r}"))
                    .unwrap_or_default()
            );
            return paid_unfulfilled(conn, &order, &data["id"], &order.status, &why).await;
        }
        return Ok(());
    }
    if let Err(why) = matches_order(&order, data) {
        tracing::warn!(order = %ids::to_text(order.id), why, "paddle transaction doesn't match its order; not fulfilled");
        return record_audit(conn, None, Audit::new("shop.order_mismatch", "shop_order", &ids::to_text(order.id)).change(Some(order_json(&order)), Some(json!({ "why": why, "transaction": data["id"], "items": data["items"], "currency_code": data["currency_code"] }))))
        .await;
    }
    // What was sold at checkout; orders from before the snapshot get the item's cosmetics now.
    let sold_bundle = match order.bundle() {
        Some(b) => b,
        None => db::item_bundles(conn, &[order.item_id])
            .await
            .map_err(db_error)?
            .remove(&order.item_id)
            .unwrap_or_default(),
    };
    // Staff may have deleted a cosmetic since; what is left is granted.
    let bundle = existing(conn, &sold_bundle).await?;
    if bundle.is_empty() {
        let total = data["details"]["totals"]["grand_total"]
            .as_str()
            .and_then(|s| s.parse::<i64>().ok());
        let row = db::mark_paid(conn, order.id, total, data["currency_code"].as_str())
            .await
            .map_err(db_error)?;
        return paid_unfulfilled(conn, &row, &data["id"], "paid", "item_empty").await;
    }
    // Never to a deleted account.
    let recipient = match order.recipient_id {
        Some(r) => accounts::get(conn, r)
            .await
            .map_err(db_error)?
            .filter(|a| a.deleted_at.is_none())
            .map(|a| a.id),
        None => None,
    };
    let mut granted = Vec::new();
    if let Some(recipient) = recipient {
        let reason = format!("Order {}", ids::to_text(order.id));
        for (kind, cosmetic) in &bundle {
            db::link_order(conn, order.id, recipient, *kind, *cosmetic)
                .await
                .map_err(db_error)?;
            if db::grant_from(
                conn,
                *kind,
                recipient,
                *cosmetic,
                "purchase",
                &reason,
                Some(order.id),
                None,
            )
            .await
            .map_err(db_error)?
            {
                granted.push((*kind, *cosmetic));
            }
        }
    }
    let total = data["details"]["totals"]["grand_total"]
        .as_str()
        .and_then(|s| s.parse::<i64>().ok());
    let currency = data["currency_code"].as_str();
    let row = db::mark_paid(conn, order.id, total, currency)
        .await
        .map_err(db_error)?;
    record_audit(
        conn,
        None,
        Audit::new("shop.order_paid", "shop_order", &ids::to_text(order.id)).change(
            Some(order_json(&order)),
            Some(json!({
                "status": row.status,
                "paid_total_cents": total,
                "paid_currency": currency,
                "granted": granted
                    .iter()
                    .map(|(k, c)| json!({ "kind": k.as_str(), "cosmetic_id": ids::to_text(*c) }))
                    .collect::<Vec<_>>(),
            })),
        ),
    )
    .await?;
    match recipient {
        None => {
            let why = if order.recipient_id.is_none() {
                "recipient_gone"
            } else {
                "recipient_deleted"
            };
            paid_unfulfilled(conn, &row, &data["id"], "paid", why).await?;
        }
        Some(_) if granted.is_empty() => {
            // Everything was owned already, say through two checkouts paid at the same time.
            record_audit(
                conn,
                None,
                Audit::new(
                    "shop.order_paid_no_grant",
                    "shop_order",
                    &ids::to_text(order.id),
                )
                .change(
                    None,
                    Some(json!({
                        "provider_transaction_id": data["id"],
                        "reason": "owned_already",
                    })),
                ),
            )
            .await?;
        }
        Some(recipient) => {
            let told = if order.account_id == Some(recipient) {
                format!("Bought {}", order.item_name)
            } else {
                format!("A gift: {}", order.item_name)
            };
            notify_cosmetics(
                conn,
                recipient,
                "cosmetic.granted",
                &granted,
                "purchase",
                &told,
                Some(order.id),
            )
            .await?;
        }
    }
    // A payment after its hold ran out is still taken, even when that sells one too many.
    if let Some(item) = db::get_item(conn, order.item_id).await.map_err(db_error)?
        && let Some(limit) = item.stock_limit
    {
        let sold = db::sold(conn, item.id).await.map_err(db_error)?;
        if sold > i64::from(limit) {
            record_audit(
                conn,
                None,
                Audit::new("shop.order_oversold", "shop_order", &ids::to_text(order.id)).change(
                    None,
                    Some(json!({
                        "item_id": ids::to_text(item.id),
                        "stock_limit": limit,
                        "sold": sold,
                    })),
                ),
            )
            .await?;
        }
    }
    Ok(())
}

/// Of `bundle`, the cosmetics that still exist, in order.
async fn existing(
    conn: &mut PgConnection,
    bundle: &[(CosmeticKind, Uuid)],
) -> ApiResult<Vec<(CosmeticKind, Uuid)>> {
    let (badges, paints) = db::split_bundle(bundle);
    let found_b: HashSet<Uuid> = cdb::badges(conn, Some(&badges))
        .await
        .map_err(db_error)?
        .into_iter()
        .map(|b| b.id)
        .collect();
    let found_p: HashSet<Uuid> = cdb::paints(conn, Some(&paints))
        .await
        .map_err(db_error)?
        .into_iter()
        .map(|p| p.id)
        .collect();
    Ok(bundle
        .iter()
        .filter(|(k, id)| match k {
            CosmeticKind::Badge => found_b.contains(id),
            CosmeticKind::Paint => found_p.contains(id),
        })
        .copied()
        .collect())
}

/// A completed transaction no order stores, tagged with one of our orders that has no
/// transaction: the checkout's save failed after Paddle made it. Never fulfilled from
/// `custom_data` (anyone can tag a transaction); audited so staff can match it by hand.
async fn unsaved_transaction(conn: &mut PgConnection, data: &Value) -> ApiResult<()> {
    let Some(tagged) = data["custom_data"]["cyex_tv_order_id"]
        .as_str()
        .and_then(ids::from_text)
    else {
        return Ok(());
    };
    let Some(order) = db::get_order(conn, tagged).await.map_err(db_error)? else {
        return Ok(());
    };
    if order.provider_transaction_id.is_some() {
        return Ok(());
    }
    paid_unfulfilled(
        conn,
        &order,
        &data["id"],
        &order.status,
        db::failed::TRANSACTION_NOT_SAVED,
    )
    .await
}

/// `transaction.canceled`: a pending order failed.
async fn cancel(conn: &mut PgConnection, data: &Value) -> ApiResult<()> {
    let Some(order) = order_of(conn, data).await? else {
        return Ok(());
    };
    if order.status != "pending" {
        return Ok(());
    }
    db::mark_failed(conn, order.id, db::failed::CANCELED)
        .await
        .map_err(db_error)?;
    record_audit(
        conn,
        None,
        Audit::new("shop.order_failed", "shop_order", &ids::to_text(order.id)).change(
            Some(order_json(&order)),
            Some(json!({ "status": "failed", "reason": db::failed::CANCELED })),
        ),
    )
    .await
}

/// `adjustment.*`: an approved full refund or a chargeback takes the cosmetics back. A partial
/// refund is recorded in the audit log only. One that arrives before its order is paid (while
/// the order can still take the payment) is answered 503, so Paddle sends it again later instead of it being dropped. A rejected refund
/// of an order whose cosmetics were taken back already (a staff refund) flags the order for
/// staff; the cosmetics stay taken back.
async fn adjust(conn: &mut PgConnection, data: &Value) -> ApiResult<()> {
    let action = data["action"].as_str().unwrap_or("");
    let status = data["status"].as_str().unwrap_or("");
    if !matches!(action, "refund" | "chargeback") || !matches!(status, "approved" | "rejected") {
        return Ok(());
    }
    let Some(txn) = data["transaction_id"].as_str() else {
        return Ok(());
    };
    let Some(order) = db::lock_order_by_transaction(conn, txn)
        .await
        .map_err(db_error)?
    else {
        return Ok(());
    };
    if status == "rejected" {
        let flagged = matches!(order.status.as_str(), "refunded" | "refund_requested");
        if flagged {
            db::mark_refund_rejected(conn, order.id, data["id"].as_str())
                .await
                .map_err(db_error)?;
        }
        return record_audit(
            conn,
            None,
            Audit::new(
                "shop.refund_rejected",
                "shop_order",
                &ids::to_text(order.id),
            )
            .change(
                Some(order_json(&order)),
                Some(json!({
                    "provider_adjustment_id": data["id"],
                    "action": action,
                    "flagged": flagged,
                })),
            ),
        )
        .await;
    }
    match order.status.as_str() {
        "paid" | "refund_requested" => {}
        // Pending, or replaced/expired with a transaction that wasn't cancelled: its payment
        // may still land, so Paddle sends this again later.
        _ if order.takes_payment() => {
            return Err(ApiError::new(
                StatusCode::SERVICE_UNAVAILABLE,
                "order_not_ready",
                "That order isn't paid yet. Send the adjustment again later.",
            ));
        }
        _ => return Ok(()),
    }
    let full = action == "chargeback" || data["type"].as_str() == Some("full");
    if !full {
        return record_audit(
            conn,
            None,
            Audit::new(
                "shop.order_partial_refund",
                "shop_order",
                &ids::to_text(order.id),
            )
            .change(
                Some(order_json(&order)),
                Some(json!({ "provider_adjustment_id": data["id"], "totals": data["totals"] })),
            ),
        )
        .await;
    }
    // A staff refund under way keeps its reason.
    let reason = match (order.status.as_str(), order.refund_reason.clone()) {
        ("refund_requested", Some(r)) => r,
        _ => format!("Paddle {action}"),
    };
    reverse(
        conn,
        &order,
        None,
        Some(&reason),
        data["id"].as_str(),
        "paddle_webhook",
    )
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input() -> ShopItemInput {
        ShopItemInput {
            name: "Starter pack".into(),
            description: Some("A badge\nand a paint".into()),
            price_cents: 499,
            currency: None,
            paddle_price_id: Some("pri_01h1vjg3sqjj1y9tvazkdqe5vt".into()),
            badge_ids: vec![ids::to_text(ids::new_id())],
            paint_ids: vec![ids::to_text(ids::new_id())],
            active_from: None,
            active_to: None,
            stock_limit: None,
            visible: Some(true),
            sort_order: None,
        }
    }

    #[test]
    fn items_are_checked() {
        let ok = item_input(&input()).unwrap();
        assert_eq!(
            (ok.currency.as_str(), ok.bundle.len(), ok.visible),
            ("EUR", 2, true)
        );
        let mut i = input();
        i.name = "x".repeat(65);
        assert_eq!(item_input(&i).unwrap_err().code, "bad_name");
        let mut i = input();
        i.price_cents = 100_001;
        assert_eq!(item_input(&i).unwrap_err().code, "bad_price");
        let mut i = input();
        i.currency = Some("EURO".into());
        assert_eq!(item_input(&i).unwrap_err().code, "bad_currency");
        let mut i = input();
        i.currency = Some("usd".into());
        assert_eq!(item_input(&i).unwrap().currency, "USD");
        for bad_id in ["pri_", "price_1", "pri_ABC", "pri_a-b"] {
            let mut i = input();
            i.paddle_price_id = Some(bad_id.into());
            assert_eq!(item_input(&i).unwrap_err().code, "bad_price_id", "{bad_id}");
        }
        let mut i = input();
        i.paddle_price_id = Some(String::new());
        assert_eq!(item_input(&i).unwrap().paddle_price_id, None);
        let mut i = input();
        i.active_from = Some("2026-12-02T00:00:00Z".into());
        i.active_to = Some("2026-12-01T00:00:00Z".into());
        assert_eq!(item_input(&i).unwrap_err().code, "bad_time");
        i.active_to = Some("next week".into());
        assert_eq!(item_input(&i).unwrap_err().code, "bad_time");
        let mut i = input();
        i.stock_limit = Some(0);
        assert_eq!(item_input(&i).unwrap_err().code, "bad_stock");
        let mut i = input();
        i.badge_ids.clear();
        i.paint_ids.clear();
        assert_eq!(item_input(&i).unwrap_err().code, "bad_cosmetics");
        i.badge_ids = (0..11).map(|_| ids::to_text(ids::new_id())).collect();
        assert_eq!(item_input(&i).unwrap_err().code, "bad_cosmetics");
        i.badge_ids = vec!["nope".into()];
        assert_eq!(item_input(&i).unwrap_err().code, "bad_cosmetics");
        // The same one twice counts once.
        let id = ids::to_text(ids::new_id());
        i.badge_ids = vec![id.clone(), id];
        assert_eq!(item_input(&i).unwrap().bundle.len(), 1);
        let mut i = input();
        i.description = Some("a\u{202E}b".into());
        assert_eq!(item_input(&i).unwrap_err().code, "bad_description");
    }

    fn order_row() -> OrderRow {
        let now = Utc::now();
        OrderRow {
            id: ids::new_id(),
            account_id: None,
            recipient_id: None,
            item_id: ids::new_id(),
            item_name: "x".into(),
            status: "pending".into(),
            amount_cents: 499,
            currency: "EUR".into(),
            provider: "paddle".into(),
            provider_transaction_id: Some("txn_1".into()),
            paddle_price_id: Some("pri_good".into()),
            provider_currency: Some("EUR".into()),
            paid_total_cents: None,
            paid_currency: None,
            provider_adjustment_id: None,
            refund_reason: None,
            created_at: now,
            updated_at: now,
            paid_at: None,
            refunded_at: None,
            failed_at: None,
            checkout_url: None,
            failed_reason: None,
            refund_requested_at: None,
            refund_rejected_at: None,
            badge_ids: Some(vec![BADGE]),
            paint_ids: Some(vec![PAINT]),
            is_gift: false,
            provider_canceled_at: None,
            cancel_failed_at: None,
        }
    }

    const BADGE: Uuid = Uuid::from_u128(1);
    const PAINT: Uuid = Uuid::from_u128(2);
    const SOLD: [(CosmeticKind, Uuid); 2] =
        [(CosmeticKind::Paint, PAINT), (CosmeticKind::Badge, BADGE)];

    fn item_row() -> ItemRow {
        let now = Utc::now();
        ItemRow {
            id: ids::new_id(),
            name: "x".into(),
            description: String::new(),
            price_cents: 499,
            currency: "EUR".into(),
            paddle_price_id: Some("pri_good".into()),
            active_from: None,
            active_to: None,
            stock_limit: None,
            visible: true,
            sort_order: 0,
            created_by: None,
            created_at: now,
            updated_at: now,
            deleted_at: None,
        }
    }

    #[test]
    fn the_locked_item_decides_the_sale() {
        let now = Utc::now();
        let bundle = [(CosmeticKind::Badge, ids::new_id())];
        assert_eq!(sellable(&item_row(), &bundle, now).unwrap(), "pri_good");
        // Hidden, deleted, out of its window, emptied or without a price after the first read.
        let mut hidden = item_row();
        hidden.visible = false;
        assert_eq!(
            sellable(&hidden, &bundle, now).unwrap_err().code,
            "item_unavailable"
        );
        let mut gone = item_row();
        gone.deleted_at = Some(now);
        assert_eq!(
            sellable(&gone, &bundle, now).unwrap_err().code,
            "item_unavailable"
        );
        let mut over = item_row();
        over.active_to = Some(now);
        assert_eq!(
            sellable(&over, &bundle, now).unwrap_err().code,
            "item_unavailable"
        );
        assert_eq!(
            sellable(&item_row(), &[], now).unwrap_err().code,
            "item_unavailable"
        );
        let mut unpriced = item_row();
        unpriced.paddle_price_id = None;
        assert_eq!(
            sellable(&unpriced, &bundle, now).unwrap_err().code,
            "checkout_unavailable"
        );
        // The locked row's price is the one used.
        let mut repriced = item_row();
        repriced.paddle_price_id = Some("pri_new".into());
        assert_eq!(sellable(&repriced, &bundle, now).unwrap(), "pri_new");
    }

    #[test]
    fn a_pending_order_is_handed_out_again_only_while_it_holds() {
        let now = Utc::now();
        let item = item_row();
        let open = order_row();
        assert_eq!(reuse_of(&open, &item, &SOLD, now), Reuse::Again);
        let mut starting = order_row();
        starting.provider_transaction_id = None;
        assert_eq!(reuse_of(&starting, &item, &SOLD, now), Reuse::Busy);
        starting.created_at = now - chrono::Duration::minutes(5);
        assert_eq!(reuse_of(&starting, &item, &SOLD, now), Reuse::Replace);
        let mut old = order_row();
        old.created_at = now - chrono::Duration::minutes(db::HOLD_MINUTES + 1);
        assert_eq!(reuse_of(&old, &item, &SOLD, now), Reuse::Replace);
        let mut repriced = item_row();
        repriced.price_cents = 599;
        assert_eq!(reuse_of(&open, &repriced, &SOLD, now), Reuse::Replace);
        let mut other_price = item_row();
        other_price.paddle_price_id = Some("pri_other".into());
        assert_eq!(reuse_of(&open, &other_price, &SOLD, now), Reuse::Replace);
    }

    #[test]
    fn a_pending_order_is_replaced_when_the_bundle_changed() {
        let now = Utc::now();
        let item = item_row();
        let open = order_row();
        // Fewer, more or other cosmetics than were sold: a new order.
        let fewer = [(CosmeticKind::Badge, BADGE)];
        assert_eq!(reuse_of(&open, &item, &fewer, now), Reuse::Replace);
        let more = [
            (CosmeticKind::Badge, BADGE),
            (CosmeticKind::Paint, PAINT),
            (CosmeticKind::Paint, Uuid::from_u128(3)),
        ];
        assert_eq!(reuse_of(&open, &item, &more, now), Reuse::Replace);
        // An order from before the snapshot is replaced too.
        let mut legacy = order_row();
        legacy.badge_ids = None;
        legacy.paint_ids = None;
        assert_eq!(legacy.bundle(), None);
        assert_eq!(reuse_of(&legacy, &item, &SOLD, now), Reuse::Replace);
        assert_eq!(
            open.bundle(),
            Some(vec![
                (CosmeticKind::Badge, BADGE),
                (CosmeticKind::Paint, PAINT)
            ])
        );
    }

    #[test]
    fn a_cancelled_transaction_takes_no_late_payment() {
        let mut o = order_row();
        o.status = "failed".into();
        o.failed_reason = Some(db::failed::REPLACED.into());
        assert!(o.takes_payment());
        o.cancel_failed_at = Some(Utc::now());
        assert!(o.takes_payment(), "the cancel failed: still taken");
        o.provider_canceled_at = Some(Utc::now());
        assert!(!o.takes_payment());
    }

    #[test]
    fn a_gift_recipient_sees_no_money() {
        let buyer = Uuid::from_u128(10);
        let recipient = Uuid::from_u128(11);
        let mut o = order_row();
        o.account_id = Some(buyer);
        o.recipient_id = Some(recipient);
        assert!(!Viewer::Account(buyer).recipient_only(&o));
        assert!(Viewer::Account(recipient).recipient_only(&o));
        assert!(!Viewer::Staff.recipient_only(&o));
        for (status, shown) in [
            ("pending", false),
            ("failed", false),
            ("paid", true),
            ("refund_requested", true),
            ("refunded", true),
        ] {
            o.status = status.into();
            assert_eq!(o.shown_to_recipient(), shown, "{status}");
        }
    }

    #[test]
    fn late_payments_are_taken_only_for_orders_we_failed() {
        let mut o = order_row();
        assert!(o.takes_payment());
        o.status = "failed".into();
        for (reason, takes) in [
            (Some(db::failed::REPLACED), true),
            (Some(db::failed::EXPIRED), true),
            (Some(db::failed::CANCELED), false),
            (Some(db::failed::PROVIDER_ERROR), false),
            (Some(db::failed::TRANSACTION_NOT_SAVED), false),
            (None, false),
        ] {
            o.failed_reason = reason.map(str::to_owned);
            assert_eq!(o.takes_payment(), takes, "{reason:?}");
        }
        for status in ["paid", "refund_requested", "refunded"] {
            o.status = status.into();
            assert!(!o.takes_payment(), "{status}");
        }
    }

    #[test]
    fn statuses_and_refund_errors() {
        assert_eq!(order_status("refund_requested"), OrderStatus::Paid);
        assert_eq!(order_status("refunded"), OrderStatus::Refunded);
        assert!(already_refunded(
            "adjustment_amount_above_remaining_allowed"
        ));
        assert!(already_refunded("transaction_already_refunded"));
        assert!(!already_refunded("bad_request"));
        let err = refund_error(&PaddleError::Api {
            status: 400,
            code: "adjustment_amount_above_remaining_allowed".into(),
        });
        assert_eq!(err.code, "already_refunded");
        let err = refund_error(&PaddleError::Shape);
        assert_eq!(err.code, "refund_failed");
    }

    #[test]
    fn completions_must_match_their_order() {
        let order = order_row();
        let data =
            |items: Value, currency: &str| json!({ "items": items, "currency_code": currency });
        let one = |price: &str, qty: u64| json!([{ "price": { "id": price }, "quantity": qty }]);
        assert_eq!(
            matches_order(&order, &data(one("pri_good", 1), "EUR")),
            Ok(())
        );
        assert!(matches_order(&order, &data(one("pri_cheap", 1), "EUR")).is_err());
        assert!(matches_order(&order, &data(one("pri_good", 2), "EUR")).is_err());
        assert!(matches_order(&order, &data(one("pri_good", 1), "USD")).is_err());
        let two = json!([{ "price": { "id": "pri_good" }, "quantity": 1 }, { "price": { "id": "pri_x" }, "quantity": 1 }]);
        assert!(matches_order(&order, &data(two, "EUR")).is_err());
        assert!(matches_order(&order, &json!({})).is_err());
    }

    #[test]
    fn paging_is_clamped() {
        assert_eq!(paging(None, None), (50, 0));
        assert_eq!(paging(Some(3), Some(10)), (10, 20));
        assert_eq!(paging(Some(0), Some(1000)), (100, 0));
    }

    #[test]
    fn refund_reasons() {
        assert!(refund_reason("asked by the buyer").is_ok());
        assert_eq!(refund_reason(" ").unwrap_err().code, "bad_reason");
    }
}
