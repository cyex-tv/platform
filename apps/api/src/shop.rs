//! The cosmetics shop (ADR 0016): items staff sell, checkouts through cyex.tv's own Paddle
//! account, fulfilment from Paddle's signed webhook only, refunds, and orders.
//!
//! - `shop.manage`: create, change and delete items.
//! - `shop.orders`: see every order, refund.
//! - Anyone signed in buys for themselves or as a gift (recipient by Twitch or Kick login). An
//!   account that owns every cosmetic in an item can only gift it.
//!
//! A purchase makes entitlements with source `purchase` and the order's ID; a refund or
//! chargeback takes exactly those back (and a selection of them is cleared, which publishes
//! `user.cosmetics_updated`). Staff writes, fulfilments and refunds are audited.

use std::collections::{HashMap, HashSet};

use axum::http::StatusCode;
use chrono::{DateTime, Utc};
use cyex_tv_auth::Caller;
use cyex_tv_db::cosmetics as cdb;
use cyex_tv_db::shop::{self as db, ItemFields, ItemRow, OrderFilter, OrderRow};
use cyex_tv_db::{DbError, accounts, audit, ids, reads};
use cyex_tv_shared::error::{ApiError, ApiResult};
use cyex_tv_shared::model::{
    AccountSummary, Badge, Checkout, CosmeticKind, Order, OrderPage, OrderStatus, Paint, Recipient,
    ShopItem, ShopItemInput,
};
use cyex_tv_shared::{permissions, text};
use serde_json::{Value, json};
use sqlx::{Connection, PgConnection};
use uuid::Uuid;

use crate::catalog::{db_error, forbidden, holds, not_found, parse_id};
use crate::paddle::{self, PaddleError, SignatureError};
use crate::state::AppState;
use crate::views::time;

pub const NAME_MAX: usize = 64;
pub const DESCRIPTION_MAX: usize = 500;
pub const PRICE_MAX: u32 = 100_000;
pub const STOCK_MAX: u32 = 1_000_000;
pub const BUNDLE_MAX: usize = 10;
pub const REASON_MAX: usize = 500;
pub const ORDERS_PAGE: u32 = 50;
pub const ORDERS_PAGE_MAX: u32 = 100;

// Errors

pub(crate) fn bad(code: &'static str, message: impl Into<String>) -> ApiError {
    ApiError::new(StatusCode::BAD_REQUEST, code, message)
}

pub(crate) fn conflict(code: &'static str, message: &str) -> ApiError {
    ApiError::new(StatusCode::CONFLICT, code, message)
}

pub(crate) fn sqlx_error(err: sqlx::Error) -> ApiError {
    db_error(DbError::Sqlx(err))
}

pub(crate) async fn acquire(
    state: &AppState,
) -> ApiResult<sqlx::pool::PoolConnection<sqlx::Postgres>> {
    state.db.acquire().await.map_err(sqlx_error)
}

pub(crate) async fn require(
    conn: &mut PgConnection,
    caller: &Caller,
    permission: &str,
) -> ApiResult<()> {
    if holds(conn, caller, permission).await? {
        Ok(())
    } else {
        Err(forbidden(&format!("This needs {permission}.")))
    }
}

/// Staff shop and code writes share the cosmetics writes limit (300 an hour per account).
pub(crate) fn staff_rate_limited(state: &AppState, caller: &Caller) -> ApiResult<()> {
    limited(&state.limits.cosmetics, caller)
}

pub(crate) fn limited(
    limiter: &cyex_tv_shared::ratelimit::Limiter,
    caller: &Caller,
) -> ApiResult<()> {
    limiter
        .check(&ids::to_text(caller.account_id))
        .map_err(|_| {
            ApiError::new(
                StatusCode::TOO_MANY_REQUESTS,
                "rate_limited",
                "That's a lot of tries. Try again in a while.",
            )
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

pub(crate) async fn write_audit(
    conn: &mut PgConnection,
    actor: Option<Uuid>,
    action: &str,
    target_kind: &str,
    target_id: Uuid,
    before: Option<Value>,
    after: Option<Value>,
) -> ApiResult<()> {
    audit::write(
        conn,
        &audit::NewEntry {
            actor,
            action,
            target_kind,
            target_id: &ids::to_text(target_id),
            before,
            after,
        },
    )
    .await
    .map_err(db_error)?;
    Ok(())
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
    let per = per_page.unwrap_or(ORDERS_PAGE).clamp(1, ORDERS_PAGE_MAX);
    let page = page.unwrap_or(1).max(1);
    (i64::from(per), i64::from(page - 1) * i64::from(per))
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

/// Items as the API shows them, for `viewer` (whose `owned` flags they get).
async fn item_views(
    state: &AppState,
    conn: &mut PgConnection,
    rows: &[ItemRow],
    viewer: Option<Uuid>,
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
                paddle_price_id: row.paddle_price_id.clone(),
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
) -> ApiResult<ShopItem> {
    item_views(state, conn, std::slice::from_ref(row), viewer)
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

/// Orders as the API shows them. Refund reasons are staff notes: only `staff` (holders of
/// `shop.orders`) see them.
async fn order_views(
    conn: &mut PgConnection,
    rows: &[OrderRow],
    staff: bool,
) -> ApiResult<Vec<Order>> {
    let people: Vec<Uuid> = rows
        .iter()
        .flat_map(|r| r.account_id.into_iter().chain(r.recipient_id))
        .collect();
    let summaries = reads::summaries(conn, &people).await.map_err(db_error)?;
    Ok(rows
        .iter()
        .map(|r| Order {
            id: ids::to_text(r.id),
            buyer: r.account_id.map(|a| summary(&summaries, a)),
            recipient: r.recipient_id.map(|a| summary(&summaries, a)),
            gift: r.account_id != r.recipient_id,
            item_id: ids::to_text(r.item_id),
            item_name: r.item_name.clone(),
            status: OrderStatus::parse(&r.status).unwrap_or(OrderStatus::Pending),
            amount_cents: u32::try_from(r.amount_cents).unwrap_or(0),
            currency: r.currency.clone(),
            paid_total_cents: r.paid_total_cents.and_then(|v| u64::try_from(v).ok()),
            paid_currency: r.paid_currency.clone(),
            provider: r.provider.clone(),
            provider_transaction_id: r.provider_transaction_id.clone(),
            refund_reason: r.refund_reason.clone().filter(|_| staff),
            created_at: time(r.created_at),
            paid_at: opt_time(r.paid_at),
            refunded_at: opt_time(r.refunded_at),
            failed_at: opt_time(r.failed_at),
        })
        .collect())
}

async fn one_order(conn: &mut PgConnection, row: &OrderRow, staff: bool) -> ApiResult<Order> {
    order_views(conn, std::slice::from_ref(row), staff)
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
    let mut conn = acquire(state).await?;
    if all {
        let caller = caller.ok_or_else(|| {
            ApiError::new(
                StatusCode::UNAUTHORIZED,
                "signed_out",
                "Sign in to use this.",
            )
        })?;
        require(&mut conn, caller, permissions::SHOP_MANAGE).await?;
    }
    let rows = db::items(&mut conn, (!all).then(Utc::now))
        .await
        .map_err(db_error)?;
    item_views(state, &mut conn, &rows, caller.map(|c| c.account_id)).await
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
    let mut conn = acquire(state).await?;
    let Some(row) = db::get_item(&mut conn, id).await.map_err(db_error)? else {
        return Ok(None);
    };
    if !row.on_sale(Utc::now()) {
        let staff = match caller {
            Some(c) => holds(&mut conn, c, permissions::SHOP_MANAGE).await?,
            None => false,
        };
        if !staff {
            return Ok(None);
        }
    }
    one_item(state, &mut conn, &row, caller.map(|c| c.account_id))
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
    let mut conn = acquire(state).await?;
    require(&mut conn, caller, permissions::SHOP_MANAGE).await?;
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
            Some(one_item(state, &mut tx, &row, None).await?)
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
    let item = one_item(state, &mut tx, &row, None).await?;
    let to_json = |v: &ShopItem| serde_json::to_value(v).unwrap_or(Value::Null);
    write_audit(
        &mut tx,
        Some(caller.account_id),
        if before.is_some() {
            "shop_item.updated"
        } else {
            "shop_item.created"
        },
        "shop_item",
        row.id,
        before.as_ref().map(to_json),
        Some(to_json(&item)),
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
    let mut conn = acquire(state).await?;
    require(&mut conn, caller, permissions::SHOP_MANAGE).await?;
    staff_rate_limited(state, caller)?;
    let mut tx = conn.begin().await.map_err(sqlx_error)?;
    if !db::delete_item(&mut tx, id).await.map_err(db_error)? {
        return Ok(false);
    }
    write_audit(
        &mut tx,
        Some(caller.account_id),
        "shop_item.deleted",
        "shop_item",
        id,
        None,
        None,
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

/// Starts buying an item, for the caller or as a gift. Makes a pending order and a Paddle
/// transaction; the website opens Paddle's checkout for it. The order is paid only when Paddle's
/// webhook says so.
pub async fn start_checkout(
    state: &AppState,
    caller: &Caller,
    item_id: &str,
    recipient: Option<&Recipient>,
) -> ApiResult<Checkout> {
    let item_id = parse_id(item_id)?;
    limited(&state.limits.checkout, caller)?;
    let mut conn = acquire(state).await?;
    let item = db::get_item(&mut conn, item_id)
        .await
        .map_err(db_error)?
        .ok_or_else(not_found)?;
    let unavailable = || conflict("item_unavailable", "That item isn't on sale.");
    if !item.on_sale(Utc::now()) {
        return Err(unavailable());
    }
    let paddle = state
        .paddle
        .clone()
        .filter(|p| p.checkout_ready())
        .ok_or_else(checkout_unavailable)?;
    let price_id = item
        .paddle_price_id
        .clone()
        .ok_or_else(checkout_unavailable)?;
    let recipient_id = match recipient {
        Some(who) => recipient_of(&mut conn, who).await?,
        None => caller.account_id,
    };
    let bundle = db::item_bundles(&mut conn, &[item.id])
        .await
        .map_err(db_error)?
        .remove(&item.id)
        .unwrap_or_default();
    if bundle.is_empty() {
        return Err(unavailable());
    }
    let owned = db::owned_of(&mut conn, recipient_id, &bundle)
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

    // Reserve a copy under the item's lock, then talk to Paddle without holding it.
    let mut tx = conn.begin().await.map_err(sqlx_error)?;
    let locked = db::lock_item(&mut tx, item.id)
        .await
        .map_err(db_error)?
        .ok_or_else(not_found)?;
    if let Some(limit) = locked.stock_limit {
        let taken = db::taken(&mut tx, &[item.id])
            .await
            .map_err(db_error)?
            .get(&item.id)
            .copied()
            .unwrap_or(0);
        if taken >= i64::from(limit) {
            return Err(conflict("sold_out", "That item is sold out."));
        }
    }
    let order = db::create_order(&mut tx, caller.account_id, recipient_id, &locked)
        .await
        .map_err(db_error)?;
    tx.commit().await.map_err(sqlx_error)?;

    let order_text = ids::to_text(order.id);
    let transaction = match paddle.create_transaction(&price_id, &order_text).await {
        Ok(t) => t,
        Err(err) => {
            db::mark_failed(&mut conn, order.id)
                .await
                .map_err(db_error)?;
            return Err(provider_error(&err));
        }
    };
    let order = db::set_transaction(
        &mut conn,
        order.id,
        &transaction.id,
        transaction.currency_code.as_deref(),
    )
    .await
    .map_err(db_error)?;
    Ok(Checkout {
        order: one_order(&mut conn, &order, false).await?,
        transaction_id: transaction.id,
        checkout_url: transaction.checkout_url,
        client_token: paddle.cfg.client_token.clone(),
        environment: paddle.cfg.env.as_str().to_owned(),
    })
}

// Orders

/// Orders the caller bought or received, newest first.
pub async fn my_orders(
    state: &AppState,
    caller: &Caller,
    page: Option<u32>,
    per_page: Option<u32>,
) -> ApiResult<OrderPage> {
    let (limit, offset) = paging(page, per_page);
    let mut conn = acquire(state).await?;
    let filter = OrderFilter {
        account: Some(caller.account_id),
        ..OrderFilter::default()
    };
    let (rows, total) = db::orders(&mut conn, &filter, limit, offset)
        .await
        .map_err(db_error)?;
    Ok(OrderPage {
        items: order_views(&mut conn, &rows, false).await?,
        total: u64::try_from(total).unwrap_or(0),
    })
}

/// One order, for its buyer, its recipient, or `shop.orders`. Poll it after a checkout.
pub async fn order(state: &AppState, caller: &Caller, id: &str) -> ApiResult<Option<Order>> {
    let Some(id) = ids::from_text(id.trim()) else {
        return Ok(None);
    };
    let mut conn = acquire(state).await?;
    let Some(row) = db::get_order(&mut conn, id).await.map_err(db_error)? else {
        return Ok(None);
    };
    let party =
        row.account_id == Some(caller.account_id) || row.recipient_id == Some(caller.account_id);
    let staff = holds(&mut conn, caller, permissions::SHOP_ORDERS).await?;
    if !party && !staff {
        return Ok(None);
    }
    one_order(&mut conn, &row, staff).await.map(Some)
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
    let mut conn = acquire(state).await?;
    require(&mut conn, caller, permissions::SHOP_ORDERS).await?;
    let filter = OrderFilter {
        account,
        status: status.map(OrderStatus::as_str),
        item,
    };
    let (rows, total) = db::orders(&mut conn, &filter, limit, offset)
        .await
        .map_err(db_error)?;
    Ok(OrderPage {
        items: order_views(&mut conn, &rows, true).await?,
        total: u64::try_from(total).unwrap_or(0),
    })
}

/// Takes back what an order gave and marks it refunded. A selection of a taken-back cosmetic is
/// cleared and published.
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
    write_audit(
        conn,
        actor,
        "shop.order_refunded",
        "shop_order",
        order.id,
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
    )
    .await?;
    Ok(row)
}

/// Refunds a paid order (needs `shop.orders`): asks Paddle for a full refund when Paddle is set
/// up and the order has a transaction, then takes the cosmetics back at once.
pub async fn refund_order(
    state: &AppState,
    caller: &Caller,
    id: &str,
    reason: &str,
) -> ApiResult<Order> {
    let id = parse_id(id)?;
    let why = refund_reason(reason)?;
    let mut conn = acquire(state).await?;
    require(&mut conn, caller, permissions::SHOP_ORDERS).await?;
    staff_rate_limited(state, caller)?;
    let mut tx = conn.begin().await.map_err(sqlx_error)?;
    let order = db::lock_order(&mut tx, id)
        .await
        .map_err(db_error)?
        .ok_or_else(not_found)?;
    if order.status != "paid" {
        return Err(conflict(
            "not_refundable",
            "Only paid orders can be refunded.",
        ));
    }
    let paddle = state.paddle.clone().filter(|p| p.checkout_ready());
    let adjustment = match (paddle, order.provider_transaction_id.as_deref()) {
        (Some(paddle), Some(txn)) => Some(paddle.refund(txn, &why).await.map_err(|err| {
            tracing::error!(error = %err, "paddle refund failed");
            ApiError::new(
                StatusCode::BAD_GATEWAY,
                "refund_failed",
                "Paddle didn't take the refund. Nothing was changed.",
            )
        })?),
        _ => None,
    };
    let via = if adjustment.is_some() {
        "staff_paddle"
    } else {
        "staff_local"
    };
    let row = reverse(
        &mut tx,
        &order,
        Some(caller.account_id),
        Some(&why),
        adjustment.as_deref(),
        via,
    )
    .await?;
    let out = one_order(&mut tx, &row, true).await?;
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
    let mut conn = acquire(state).await?;
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

/// `transaction.completed`: the recipient gets every cosmetic of the item they don't own yet.
/// Idempotent: an order that is paid or refunded already stays as it is.
async fn fulfil(conn: &mut PgConnection, data: &Value) -> ApiResult<()> {
    let Some(order) = order_of(conn, data).await? else {
        return Ok(());
    };
    // Only a pending order is fulfilled: paid and refunded ones are done, failed ones were
    // cancelled at Paddle or never got a transaction.
    if order.status != "pending" {
        return Ok(());
    }
    if let Err(why) = matches_order(&order, data) {
        tracing::warn!(order = %ids::to_text(order.id), why, "paddle transaction doesn't match its order; not fulfilled");
        return write_audit(
            conn,
            None,
            "shop.order_mismatch",
            "shop_order",
            order.id,
            Some(order_json(&order)),
            Some(json!({ "why": why, "transaction": data["id"], "items": data["items"], "currency_code": data["currency_code"] })),
        )
        .await;
    }
    let bundle = db::item_bundles(conn, &[order.item_id])
        .await
        .map_err(db_error)?
        .remove(&order.item_id)
        .unwrap_or_default();
    let mut granted = Vec::new();
    if let Some(recipient) = order.recipient_id {
        let reason = format!("Order {}", ids::to_text(order.id));
        for (kind, cosmetic) in &bundle {
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
                granted
                    .push(json!({ "kind": kind.as_str(), "cosmetic_id": ids::to_text(*cosmetic) }));
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
    write_audit(
        conn,
        None,
        "shop.order_paid",
        "shop_order",
        order.id,
        Some(order_json(&order)),
        Some(json!({
            "status": row.status,
            "paid_total_cents": total,
            "paid_currency": currency,
            "granted": granted,
        })),
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
    db::mark_failed(conn, order.id).await.map_err(db_error)?;
    write_audit(
        conn,
        None,
        "shop.order_failed",
        "shop_order",
        order.id,
        Some(order_json(&order)),
        Some(json!({ "status": "failed" })),
    )
    .await
}

/// `adjustment.*`: an approved full refund or a chargeback takes the cosmetics back. A partial
/// refund is recorded in the audit log only. One that arrives before its order is paid is
/// answered 503, so Paddle sends it again later instead of it being dropped.
async fn adjust(conn: &mut PgConnection, data: &Value) -> ApiResult<()> {
    let action = data["action"].as_str().unwrap_or("");
    let status = data["status"].as_str().unwrap_or("");
    if !matches!(action, "refund" | "chargeback") || status != "approved" {
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
    match order.status.as_str() {
        "paid" => {}
        "pending" => {
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
        return write_audit(
            conn,
            None,
            "shop.order_partial_refund",
            "shop_order",
            order.id,
            Some(order_json(&order)),
            Some(json!({ "provider_adjustment_id": data["id"], "totals": data["totals"] })),
        )
        .await;
    }
    let reason = format!("Paddle {action}");
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

    #[test]
    fn completions_must_match_their_order() {
        let now = Utc::now();
        let order = OrderRow {
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
        };
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
