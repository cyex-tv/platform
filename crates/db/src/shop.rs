//! The cosmetics shop (ADR 0016): items, orders, Paddle webhook events, and the entitlements an
//! order makes. See the migration `20261201000001_shop.sql`. The api checks permissions and
//! input, and runs a checkout's or a fulfilment's statements in one transaction.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use cyex_tv_shared::model::CosmeticKind;
use sqlx::{FromRow, PgConnection};
use uuid::Uuid;

use crate::error::{DbError, DbResult};
use crate::ids::new_id;

/// The entitlement table and its cosmetic column for a kind. Constants only, never input.
pub(crate) fn entitlement_table(kind: CosmeticKind) -> (&'static str, &'static str) {
    match kind {
        CosmeticKind::Badge => ("badge_entitlements", "badge_id"),
        CosmeticKind::Paint => ("paint_entitlements", "paint_id"),
    }
}

// Items

#[derive(Clone, Debug, PartialEq, Eq, FromRow)]
pub struct ItemRow {
    pub id: Uuid,
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
    pub created_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}

impl ItemRow {
    /// Visible, not deleted, and inside its sale window at `now`.
    pub fn on_sale(&self, now: DateTime<Utc>) -> bool {
        self.visible
            && self.deleted_at.is_none()
            && self.active_from.is_none_or(|t| t <= now)
            && self.active_to.is_none_or(|t| t > now)
    }
}

/// An item's fields, checked by the api.
#[derive(Clone, Debug)]
pub struct ItemFields<'a> {
    pub name: &'a str,
    pub description: &'a str,
    pub price_cents: i32,
    pub currency: &'a str,
    pub paddle_price_id: Option<&'a str>,
    pub active_from: Option<DateTime<Utc>>,
    pub active_to: Option<DateTime<Utc>>,
    pub stock_limit: Option<i32>,
    pub visible: bool,
    pub sort_order: i32,
}

const ITEM: &str = "id, name, description, price_cents, currency, paddle_price_id, active_from, \
                    active_to, stock_limit, visible, sort_order, created_by, created_at, \
                    updated_at, deleted_at";

/// The cosmetics in an item or a code batch, in the order given.
pub type Bundle = [(CosmeticKind, Uuid)];

async fn set_bundle(
    conn: &mut PgConnection,
    table: &str,
    owner_column: &str,
    owner: Uuid,
    bundle: &Bundle,
) -> DbResult<()> {
    sqlx::query(&format!("DELETE FROM {table} WHERE {owner_column} = $1"))
        .bind(owner)
        .execute(&mut *conn)
        .await?;
    for (kind, id) in bundle {
        let (badge, paint) = match kind {
            CosmeticKind::Badge => (Some(*id), None),
            CosmeticKind::Paint => (None, Some(*id)),
        };
        sqlx::query(&format!(
            "INSERT INTO {table} ({owner_column}, badge_id, paint_id) VALUES ($1, $2, $3)
             ON CONFLICT DO NOTHING"
        ))
        .bind(owner)
        .bind(badge)
        .bind(paint)
        .execute(&mut *conn)
        .await
        .map_err(|err| match err.as_database_error() {
            Some(db) if db.is_foreign_key_violation() => DbError::NotFound,
            _ => err.into(),
        })?;
    }
    Ok(())
}

pub(crate) async fn bundles(
    conn: &mut PgConnection,
    table: &str,
    owner_column: &str,
    owners: &[Uuid],
) -> DbResult<HashMap<Uuid, Vec<(CosmeticKind, Uuid)>>> {
    let rows: Vec<(Uuid, Option<Uuid>, Option<Uuid>)> = sqlx::query_as(&format!(
        "SELECT {owner_column}, badge_id, paint_id FROM {table}
         WHERE {owner_column} = ANY($1)
         ORDER BY {owner_column}, badge_id IS NULL, badge_id, paint_id"
    ))
    .bind(owners)
    .fetch_all(conn)
    .await?;
    let mut out: HashMap<Uuid, Vec<(CosmeticKind, Uuid)>> = HashMap::new();
    for (owner, badge, paint) in rows {
        let entry = match (badge, paint) {
            (Some(b), _) => (CosmeticKind::Badge, b),
            (None, Some(p)) => (CosmeticKind::Paint, p),
            (None, None) => continue,
        };
        out.entry(owner).or_default().push(entry);
    }
    Ok(out)
}

/// Makes an item with its cosmetics. `NotFound` when a cosmetic doesn't exist.
pub async fn create_item(
    conn: &mut PgConnection,
    fields: &ItemFields<'_>,
    bundle: &Bundle,
    created_by: Option<Uuid>,
) -> DbResult<ItemRow> {
    let row: ItemRow = sqlx::query_as(&format!(
        "INSERT INTO shop_items (id, name, description, price_cents, currency, paddle_price_id,
                                 active_from, active_to, stock_limit, visible, sort_order, created_by)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
         RETURNING {ITEM}"
    ))
    .bind(new_id())
    .bind(fields.name)
    .bind(fields.description)
    .bind(fields.price_cents)
    .bind(fields.currency)
    .bind(fields.paddle_price_id)
    .bind(fields.active_from)
    .bind(fields.active_to)
    .bind(fields.stock_limit)
    .bind(fields.visible)
    .bind(fields.sort_order)
    .bind(created_by)
    .fetch_one(&mut *conn)
    .await?;
    set_bundle(conn, "shop_item_cosmetics", "item_id", row.id, bundle).await?;
    Ok(row)
}

/// Replaces every field and the cosmetics of an item that isn't deleted.
pub async fn replace_item(
    conn: &mut PgConnection,
    id: Uuid,
    fields: &ItemFields<'_>,
    bundle: &Bundle,
) -> DbResult<ItemRow> {
    let row: ItemRow = sqlx::query_as(&format!(
        "UPDATE shop_items SET name = $2, description = $3, price_cents = $4, currency = $5,
                paddle_price_id = $6, active_from = $7, active_to = $8, stock_limit = $9,
                visible = $10, sort_order = $11, updated_at = now()
         WHERE id = $1 AND deleted_at IS NULL
         RETURNING {ITEM}"
    ))
    .bind(id)
    .bind(fields.name)
    .bind(fields.description)
    .bind(fields.price_cents)
    .bind(fields.currency)
    .bind(fields.paddle_price_id)
    .bind(fields.active_from)
    .bind(fields.active_to)
    .bind(fields.stock_limit)
    .bind(fields.visible)
    .bind(fields.sort_order)
    .fetch_optional(&mut *conn)
    .await?
    .ok_or(DbError::NotFound)?;
    set_bundle(conn, "shop_item_cosmetics", "item_id", id, bundle).await?;
    Ok(row)
}

/// Marks an item deleted. False when there was none (or it was deleted already).
pub async fn delete_item(conn: &mut PgConnection, id: Uuid) -> DbResult<bool> {
    let done = sqlx::query(
        "UPDATE shop_items SET deleted_at = now(), visible = false, updated_at = now()
         WHERE id = $1 AND deleted_at IS NULL",
    )
    .bind(id)
    .execute(conn)
    .await?;
    Ok(done.rows_affected() > 0)
}

/// An item, deleted or not.
pub async fn get_item(conn: &mut PgConnection, id: Uuid) -> DbResult<Option<ItemRow>> {
    Ok(
        sqlx::query_as(&format!("SELECT {ITEM} FROM shop_items WHERE id = $1"))
            .bind(id)
            .fetch_optional(conn)
            .await?,
    )
}

/// Locks an item's row for a checkout, so two checkouts can't both take the last copy.
pub async fn lock_item(conn: &mut PgConnection, id: Uuid) -> DbResult<Option<ItemRow>> {
    Ok(sqlx::query_as(&format!(
        "SELECT {ITEM} FROM shop_items WHERE id = $1 FOR UPDATE"
    ))
    .bind(id)
    .fetch_optional(conn)
    .await?)
}

/// Items that aren't deleted, by sort order: every one, or only those on sale at `now`.
pub async fn items(
    conn: &mut PgConnection,
    on_sale_at: Option<DateTime<Utc>>,
) -> DbResult<Vec<ItemRow>> {
    Ok(sqlx::query_as(&format!(
        "SELECT {ITEM} FROM shop_items
         WHERE deleted_at IS NULL
           AND ($1::timestamptz IS NULL OR (visible
                AND (active_from IS NULL OR active_from <= $1)
                AND (active_to IS NULL OR active_to > $1)))
         ORDER BY sort_order, id"
    ))
    .bind(on_sale_at)
    .fetch_all(conn)
    .await?)
}

/// The cosmetics of each item.
pub async fn item_bundles(
    conn: &mut PgConnection,
    items: &[Uuid],
) -> DbResult<HashMap<Uuid, Vec<(CosmeticKind, Uuid)>>> {
    bundles(conn, "shop_item_cosmetics", "item_id", items).await
}

/// How long a pending checkout holds a copy of a limited item.
pub const HOLD_MINUTES: i64 = 60;

/// Copies taken per item: paid orders (and ones with a refund in flight), and pending ones from
/// the last hour.
pub async fn taken(conn: &mut PgConnection, items: &[Uuid]) -> DbResult<HashMap<Uuid, i64>> {
    let rows: Vec<(Uuid, i64)> = sqlx::query_as(
        "SELECT item_id, count(*) FROM shop_orders
         WHERE item_id = ANY($1)
           AND (status IN ('paid', 'refund_requested')
                OR (status = 'pending' AND created_at > now() - make_interval(mins => $2::int)))
         GROUP BY item_id",
    )
    .bind(items)
    .bind(HOLD_MINUTES as i32)
    .fetch_all(conn)
    .await?;
    Ok(rows.into_iter().collect())
}

/// Copies of an item sold: paid orders, and ones with a refund in flight.
pub async fn sold(conn: &mut PgConnection, item: Uuid) -> DbResult<i64> {
    Ok(sqlx::query_scalar(
        "SELECT count(*) FROM shop_orders
         WHERE item_id = $1 AND status IN ('paid', 'refund_requested')",
    )
    .bind(item)
    .fetch_one(conn)
    .await?)
}

// Entitlements made by the shop and by codes

/// Gives `account` the cosmetic from an order or a code. False when they owned it already.
///
/// An existing entitlement is locked first, so a refund taking it back at the same time either
/// finishes first (and this grants it again) or waits for this order to be paid (and then sees
/// it covering the cosmetic).
#[allow(clippy::too_many_arguments)]
pub async fn grant_from(
    conn: &mut PgConnection,
    kind: CosmeticKind,
    account: Uuid,
    cosmetic: Uuid,
    source: &str,
    reason: &str,
    order_id: Option<Uuid>,
    code_id: Option<Uuid>,
) -> DbResult<bool> {
    let (table, column) = entitlement_table(kind);
    let owned: Option<i32> = sqlx::query_scalar(&format!(
        "SELECT 1 FROM {table} WHERE account_id = $1 AND {column} = $2 FOR UPDATE"
    ))
    .bind(account)
    .bind(cosmetic)
    .fetch_optional(&mut *conn)
    .await?;
    if owned.is_some() {
        return Ok(false);
    }
    let done = sqlx::query(&format!(
        "INSERT INTO {table} (account_id, {column}, source, reason, order_id, code_id)
         VALUES ($1, $2, $3, $4, $5, $6)
         ON CONFLICT DO NOTHING"
    ))
    .bind(account)
    .bind(cosmetic)
    .bind(source)
    .bind(reason)
    .bind(order_id)
    .bind(code_id)
    .execute(conn)
    .await?;
    Ok(done.rows_affected() > 0)
}

/// Records that `order` covers a cosmetic of `account`, whether it made the entitlement or found
/// it owned already.
pub async fn link_order(
    conn: &mut PgConnection,
    order: Uuid,
    account: Uuid,
    kind: CosmeticKind,
    cosmetic: Uuid,
) -> DbResult<()> {
    sqlx::query(
        "INSERT INTO shop_order_entitlements (order_id, account_id, kind, cosmetic_id)
         VALUES ($1, $2, $3, $4) ON CONFLICT DO NOTHING",
    )
    .bind(order)
    .bind(account)
    .bind(kind.as_str())
    .bind(cosmetic)
    .execute(conn)
    .await?;
    Ok(())
}

/// A code `account` redeemed that gives the cosmetic (directly, or through its batch's item).
async fn covering_code(
    conn: &mut PgConnection,
    kind: CosmeticKind,
    account: Uuid,
    cosmetic: Uuid,
) -> DbResult<Option<Uuid>> {
    let (_, column) = entitlement_table(kind);
    Ok(sqlx::query_scalar(&format!(
        "SELECT r.code_id FROM code_redemptions r
           JOIN redeem_codes c ON c.id = r.code_id
           JOIN code_batches b ON b.id = c.batch_id
          WHERE r.account_id = $1
            AND (EXISTS (SELECT 1 FROM code_batch_cosmetics bc
                          WHERE bc.batch_id = b.id AND bc.{column} = $2)
                 OR EXISTS (SELECT 1 FROM shop_item_cosmetics ic
                             WHERE ic.item_id = b.item_id AND ic.{column} = $2))
          ORDER BY r.redeemed_at LIMIT 1"
    ))
    .bind(account)
    .bind(cosmetic)
    .fetch_optional(conn)
    .await?)
}

/// Takes back what an order covers, and returns what was taken. A cosmetic stays when its
/// entitlement didn't come from a purchase (a staff grant, an event, a code), when another paid
/// order covers it (the entitlement then names that order), or when a code the account redeemed
/// gives it (the entitlement then becomes the code's). Selections of what was taken are cleared
/// by the foreign keys. Entitlements are locked in a fixed order (badges, then paints, by ID).
pub async fn revoke_order(
    conn: &mut PgConnection,
    order: Uuid,
) -> DbResult<Vec<(CosmeticKind, Uuid, Uuid)>> {
    let links: Vec<(Uuid, String, Uuid)> = sqlx::query_as(
        "SELECT account_id, kind, cosmetic_id FROM shop_order_entitlements
          WHERE order_id = $1 ORDER BY kind, cosmetic_id",
    )
    .bind(order)
    .fetch_all(&mut *conn)
    .await?;
    let mut out = Vec::new();
    for (account, kind, cosmetic) in links {
        let kind = if kind == "badge" {
            CosmeticKind::Badge
        } else {
            CosmeticKind::Paint
        };
        let (table, column) = entitlement_table(kind);
        let source: Option<String> = sqlx::query_scalar(&format!(
            "SELECT source FROM {table} WHERE account_id = $1 AND {column} = $2 FOR UPDATE"
        ))
        .bind(account)
        .bind(cosmetic)
        .fetch_optional(&mut *conn)
        .await?;
        if source.as_deref() != Some("purchase") {
            continue;
        }
        let other: Option<Uuid> = sqlx::query_scalar(
            "SELECT l.order_id FROM shop_order_entitlements l
               JOIN shop_orders o ON o.id = l.order_id
              WHERE l.account_id = $1 AND l.kind = $2 AND l.cosmetic_id = $3
                AND l.order_id <> $4 AND o.status = 'paid'
              ORDER BY l.order_id LIMIT 1",
        )
        .bind(account)
        .bind(kind.as_str())
        .bind(cosmetic)
        .bind(order)
        .fetch_optional(&mut *conn)
        .await?;
        if let Some(other) = other {
            sqlx::query(&format!(
                "UPDATE {table} SET order_id = $3, reason = $4
                  WHERE account_id = $1 AND {column} = $2"
            ))
            .bind(account)
            .bind(cosmetic)
            .bind(other)
            .bind(format!("Order {}", crate::ids::to_text(other)))
            .execute(&mut *conn)
            .await?;
            continue;
        }
        if let Some(code) = covering_code(conn, kind, account, cosmetic).await? {
            sqlx::query(&format!(
                "UPDATE {table} SET source = 'code', order_id = NULL, code_id = $3
                  WHERE account_id = $1 AND {column} = $2"
            ))
            .bind(account)
            .bind(cosmetic)
            .bind(code)
            .execute(&mut *conn)
            .await?;
            continue;
        }
        sqlx::query(&format!(
            "DELETE FROM {table} WHERE account_id = $1 AND {column} = $2"
        ))
        .bind(account)
        .bind(cosmetic)
        .execute(&mut *conn)
        .await?;
        out.push((kind, account, cosmetic));
    }
    Ok(out)
}

/// Of `bundle`, the cosmetics `account` owns.
pub async fn owned_of(
    conn: &mut PgConnection,
    account: Uuid,
    bundle: &Bundle,
) -> DbResult<Vec<(CosmeticKind, Uuid)>> {
    let mut out = Vec::new();
    for kind in [CosmeticKind::Badge, CosmeticKind::Paint] {
        let ids: Vec<Uuid> = bundle
            .iter()
            .filter(|(k, _)| *k == kind)
            .map(|(_, id)| *id)
            .collect();
        if ids.is_empty() {
            continue;
        }
        let (table, column) = entitlement_table(kind);
        let owned: Vec<Uuid> = sqlx::query_scalar(&format!(
            "SELECT {column} FROM {table} WHERE account_id = $1 AND {column} = ANY($2)"
        ))
        .bind(account)
        .bind(&ids)
        .fetch_all(&mut *conn)
        .await?;
        out.extend(owned.into_iter().map(|id| (kind, id)));
    }
    Ok(out)
}

// Orders

#[derive(Clone, Debug, PartialEq, Eq, FromRow)]
pub struct OrderRow {
    pub id: Uuid,
    pub account_id: Option<Uuid>,
    pub recipient_id: Option<Uuid>,
    pub item_id: Uuid,
    pub item_name: String,
    /// `pending`, `paid`, `refund_requested`, `refunded` or `failed`.
    pub status: String,
    pub amount_cents: i32,
    pub currency: String,
    pub provider: String,
    pub provider_transaction_id: Option<String>,
    /// The item's Paddle price when the checkout started.
    pub paddle_price_id: Option<String>,
    /// The currency Paddle gave the transaction.
    pub provider_currency: Option<String>,
    pub paid_total_cents: Option<i64>,
    pub paid_currency: Option<String>,
    pub provider_adjustment_id: Option<String>,
    pub refund_reason: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub paid_at: Option<DateTime<Utc>>,
    pub refunded_at: Option<DateTime<Utc>>,
    pub failed_at: Option<DateTime<Utc>>,
    /// Paddle's hosted checkout link for the transaction.
    pub checkout_url: Option<String>,
    /// Why a failed order failed (see [`failed`]).
    pub failed_reason: Option<String>,
    pub refund_requested_at: Option<DateTime<Utc>>,
    /// Paddle rejected the refund after the cosmetics were taken back.
    pub refund_rejected_at: Option<DateTime<Utc>>,
    /// The item's badges and paints when the checkout started (`None` on older orders).
    pub badge_ids: Option<Vec<Uuid>>,
    pub paint_ids: Option<Vec<Uuid>>,
    /// Bought for another account, as decided at checkout.
    pub is_gift: bool,
    /// Its Paddle transaction was cancelled by us (a replaced or expired checkout).
    pub provider_canceled_at: Option<DateTime<Utc>>,
    /// Cancelling its Paddle transaction failed; a late payment is still taken.
    pub cancel_failed_at: Option<DateTime<Utc>>,
}

impl OrderRow {
    /// A completed payment is still taken for it: pending, or failed by us (replaced by a newer
    /// checkout, or expired) rather than cancelled at Paddle.
    /// Once its transaction is cancelled at Paddle, a failed order takes nothing.
    pub fn takes_payment(&self) -> bool {
        self.status == "pending"
            || (self.status == "failed"
                && self.provider_canceled_at.is_none()
                && matches!(
                    self.failed_reason.as_deref(),
                    Some(failed::REPLACED | failed::EXPIRED)
                ))
    }

    /// What the order sells, as snapshotted at checkout; `None` for orders made before the
    /// snapshot existed.
    pub fn bundle(&self) -> Option<Vec<(CosmeticKind, Uuid)>> {
        if self.badge_ids.is_none() && self.paint_ids.is_none() {
            return None;
        }
        let badges = self
            .badge_ids
            .iter()
            .flatten()
            .map(|id| (CosmeticKind::Badge, *id));
        let paints = self
            .paint_ids
            .iter()
            .flatten()
            .map(|id| (CosmeticKind::Paint, *id));
        Some(badges.chain(paints).collect())
    }

    /// Paid, or paid and refunded: what a gift's recipient may see of it.
    pub fn shown_to_recipient(&self) -> bool {
        matches!(
            self.status.as_str(),
            "paid" | "refund_requested" | "refunded"
        )
    }
}

/// Splits a bundle into its badge and paint IDs, in order.
pub fn split_bundle(bundle: &Bundle) -> (Vec<Uuid>, Vec<Uuid>) {
    let of = |k: CosmeticKind| -> Vec<Uuid> {
        bundle
            .iter()
            .filter(|(kind, _)| *kind == k)
            .map(|(_, id)| *id)
            .collect()
    };
    (of(CosmeticKind::Badge), of(CosmeticKind::Paint))
}

/// Why a failed order failed.
pub mod failed {
    /// Cancelled at Paddle.
    pub const CANCELED: &str = "canceled";
    /// Paddle didn't make the transaction.
    pub const PROVIDER_ERROR: &str = "provider_error";
    /// Paddle made the transaction, and saving its ID failed (the audit row holds it).
    pub const TRANSACTION_NOT_SAVED: &str = "transaction_not_saved";
    /// A newer checkout of the same buyer, item and recipient took its place.
    pub const REPLACED: &str = "replaced";
    /// Not paid within 24 hours.
    pub const EXPIRED: &str = "expired";
}

const ORDER: &str = "id, account_id, recipient_id, item_id, item_name, status, amount_cents, \
                     currency, provider, provider_transaction_id, paddle_price_id, \
                     provider_currency, paid_total_cents, \
                     paid_currency, provider_adjustment_id, refund_reason, created_at, \
                     updated_at, paid_at, refunded_at, failed_at, checkout_url, failed_reason, \
                     refund_requested_at, refund_rejected_at, badge_ids, paint_ids, is_gift, \
                     provider_canceled_at, cancel_failed_at";

/// A pending order for `item`, at its current price and with its current cosmetics (`bundle`,
/// read under the item's lock). `PendingOrder` when the buyer has a pending order for it (for the
/// same recipient) already.
pub async fn create_order(
    conn: &mut PgConnection,
    buyer: Uuid,
    recipient: Uuid,
    item: &ItemRow,
    bundle: &Bundle,
) -> DbResult<OrderRow> {
    let (badges, paints) = split_bundle(bundle);
    sqlx::query_as(&format!(
        "INSERT INTO shop_orders (id, account_id, recipient_id, item_id, item_name, amount_cents, currency,
                                  paddle_price_id, badge_ids, paint_ids, is_gift)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $2 <> $3)
         RETURNING {ORDER}"
    ))
    .bind(new_id())
    .bind(buyer)
    .bind(recipient)
    .bind(item.id)
    .bind(&item.name)
    .bind(item.price_cents)
    .bind(&item.currency)
    .bind(&item.paddle_price_id)
    .bind(&badges)
    .bind(&paints)
    .fetch_one(conn)
    .await
    .map_err(|err| match err.as_database_error() {
        Some(db) if db.is_unique_violation() => DbError::PendingOrder,
        _ => err.into(),
    })
}

/// The buyer's pending order for an item and recipient, locked.
pub async fn lock_pending(
    conn: &mut PgConnection,
    buyer: Uuid,
    item: Uuid,
    recipient: Uuid,
) -> DbResult<Option<OrderRow>> {
    Ok(sqlx::query_as(&format!(
        "SELECT {ORDER} FROM shop_orders
          WHERE account_id = $1 AND item_id = $2 AND recipient_id = $3 AND status = 'pending'
          FOR UPDATE"
    ))
    .bind(buyer)
    .bind(item)
    .bind(recipient)
    .fetch_optional(conn)
    .await?)
}

pub async fn set_transaction(
    conn: &mut PgConnection,
    id: Uuid,
    transaction_id: &str,
    currency: Option<&str>,
    checkout_url: Option<&str>,
) -> DbResult<OrderRow> {
    sqlx::query_as(&format!(
        "UPDATE shop_orders SET provider_transaction_id = $2, provider_currency = $3,
                checkout_url = $4, updated_at = now()
         WHERE id = $1 RETURNING {ORDER}"
    ))
    .bind(id)
    .bind(transaction_id)
    .bind(currency)
    .bind(checkout_url)
    .fetch_optional(conn)
    .await?
    .ok_or(DbError::NotFound)
}

pub async fn get_order(conn: &mut PgConnection, id: Uuid) -> DbResult<Option<OrderRow>> {
    Ok(
        sqlx::query_as(&format!("SELECT {ORDER} FROM shop_orders WHERE id = $1"))
            .bind(id)
            .fetch_optional(conn)
            .await?,
    )
}

/// Locks an order for a status change.
pub async fn lock_order(conn: &mut PgConnection, id: Uuid) -> DbResult<Option<OrderRow>> {
    Ok(sqlx::query_as(&format!(
        "SELECT {ORDER} FROM shop_orders WHERE id = $1 FOR UPDATE"
    ))
    .bind(id)
    .fetch_optional(conn)
    .await?)
}

/// Locks the order of a Paddle transaction.
pub async fn lock_order_by_transaction(
    conn: &mut PgConnection,
    transaction_id: &str,
) -> DbResult<Option<OrderRow>> {
    Ok(sqlx::query_as(&format!(
        "SELECT {ORDER} FROM shop_orders WHERE provider_transaction_id = $1 FOR UPDATE"
    ))
    .bind(transaction_id)
    .fetch_optional(conn)
    .await?)
}

pub async fn mark_paid(
    conn: &mut PgConnection,
    id: Uuid,
    paid_total_cents: Option<i64>,
    paid_currency: Option<&str>,
) -> DbResult<OrderRow> {
    sqlx::query_as(&format!(
        "UPDATE shop_orders SET status = 'paid', paid_at = now(), updated_at = now(),
                paid_total_cents = $2, paid_currency = $3
         WHERE id = $1 RETURNING {ORDER}"
    ))
    .bind(id)
    .bind(paid_total_cents)
    .bind(paid_currency)
    .fetch_optional(conn)
    .await?
    .ok_or(DbError::NotFound)
}

/// A staff refund is under way: Paddle is asked next, outside the order's lock.
pub async fn mark_refund_requested(
    conn: &mut PgConnection,
    id: Uuid,
    reason: &str,
) -> DbResult<OrderRow> {
    sqlx::query_as(&format!(
        "UPDATE shop_orders SET status = 'refund_requested', refund_requested_at = now(),
                refund_reason = $2, updated_at = now()
         WHERE id = $1 RETURNING {ORDER}"
    ))
    .bind(id)
    .bind(reason)
    .fetch_optional(conn)
    .await?
    .ok_or(DbError::NotFound)
}

/// Paddle didn't take a requested refund: the order is paid again.
pub async fn cancel_refund_request(conn: &mut PgConnection, id: Uuid) -> DbResult<OrderRow> {
    sqlx::query_as(&format!(
        "UPDATE shop_orders SET status = 'paid', refund_requested_at = NULL, refund_reason = NULL,
                updated_at = now()
         WHERE id = $1 RETURNING {ORDER}"
    ))
    .bind(id)
    .fetch_optional(conn)
    .await?
    .ok_or(DbError::NotFound)
}

/// Paddle rejected the refund of an order whose cosmetics were taken back already.
pub async fn mark_refund_rejected(
    conn: &mut PgConnection,
    id: Uuid,
    adjustment_id: Option<&str>,
) -> DbResult<OrderRow> {
    sqlx::query_as(&format!(
        "UPDATE shop_orders SET refund_rejected_at = now(),
                provider_adjustment_id = coalesce($2, provider_adjustment_id), updated_at = now()
         WHERE id = $1 RETURNING {ORDER}"
    ))
    .bind(id)
    .bind(adjustment_id)
    .fetch_optional(conn)
    .await?
    .ok_or(DbError::NotFound)
}

/// Stores the adjustment of a refund Paddle took, on an order its webhook refunded already.
pub async fn set_adjustment(
    conn: &mut PgConnection,
    id: Uuid,
    adjustment_id: &str,
) -> DbResult<OrderRow> {
    sqlx::query_as(&format!(
        "UPDATE shop_orders SET provider_adjustment_id = $2, updated_at = now()
         WHERE id = $1 RETURNING {ORDER}"
    ))
    .bind(id)
    .bind(adjustment_id)
    .fetch_optional(conn)
    .await?
    .ok_or(DbError::NotFound)
}

pub async fn mark_refunded(
    conn: &mut PgConnection,
    id: Uuid,
    reason: Option<&str>,
    adjustment_id: Option<&str>,
) -> DbResult<OrderRow> {
    sqlx::query_as(&format!(
        "UPDATE shop_orders SET status = 'refunded', refunded_at = now(), updated_at = now(),
                refund_reason = $2, provider_adjustment_id = coalesce($3, provider_adjustment_id)
         WHERE id = $1 RETURNING {ORDER}"
    ))
    .bind(id)
    .bind(reason)
    .bind(adjustment_id)
    .fetch_optional(conn)
    .await?
    .ok_or(DbError::NotFound)
}

/// Fails an order, saying why (see [`failed`]).
pub async fn mark_failed(conn: &mut PgConnection, id: Uuid, reason: &str) -> DbResult<OrderRow> {
    sqlx::query_as(&format!(
        "UPDATE shop_orders SET status = 'failed', failed_at = now(), failed_reason = $2,
                updated_at = now()
         WHERE id = $1 RETURNING {ORDER}"
    ))
    .bind(id)
    .bind(reason)
    .fetch_optional(conn)
    .await?
    .ok_or(DbError::NotFound)
}

/// Fails pending orders made before `before` (reason `expired`; a late payment is still taken),
/// at most `limit` at a time, and returns them.
pub async fn expire_pending(
    conn: &mut PgConnection,
    before: DateTime<Utc>,
    limit: i64,
) -> DbResult<Vec<OrderRow>> {
    Ok(sqlx::query_as(&format!(
        "UPDATE shop_orders SET status = 'failed', failed_at = now(), failed_reason = 'expired',
                updated_at = now()
          WHERE id IN (SELECT id FROM shop_orders
                        WHERE status = 'pending' AND created_at < $1
                        ORDER BY id LIMIT $2 FOR UPDATE SKIP LOCKED)
          RETURNING {ORDER}"
    ))
    .bind(before)
    .bind(limit)
    .fetch_all(conn)
    .await?)
}

/// The Paddle transaction of a replaced or expired order was cancelled.
pub async fn mark_canceled(conn: &mut PgConnection, id: Uuid) -> DbResult<bool> {
    let done = sqlx::query(
        "UPDATE shop_orders SET provider_canceled_at = now(), updated_at = now()
          WHERE id = $1 AND status = 'failed' AND provider_canceled_at IS NULL",
    )
    .bind(id)
    .execute(conn)
    .await?;
    Ok(done.rows_affected() > 0)
}

/// Cancelling the Paddle transaction of a replaced or expired order failed: it isn't tried
/// again, and a late payment is still taken.
pub async fn mark_cancel_failed(conn: &mut PgConnection, id: Uuid) -> DbResult<bool> {
    let done = sqlx::query(
        "UPDATE shop_orders SET cancel_failed_at = now(), updated_at = now()
          WHERE id = $1 AND provider_canceled_at IS NULL AND cancel_failed_at IS NULL",
    )
    .bind(id)
    .execute(conn)
    .await?;
    Ok(done.rows_affected() > 0)
}

/// Replaced or expired orders whose Paddle transaction hasn't been cancelled or tried yet,
/// oldest first.
pub async fn to_cancel(conn: &mut PgConnection, limit: i64) -> DbResult<Vec<OrderRow>> {
    Ok(sqlx::query_as(&format!(
        "SELECT {ORDER} FROM shop_orders
          WHERE status = 'failed' AND failed_reason IN ('replaced', 'expired')
            AND provider_transaction_id IS NOT NULL
            AND provider_canceled_at IS NULL AND cancel_failed_at IS NULL
          ORDER BY id LIMIT $1"
    ))
    .bind(limit)
    .fetch_all(conn)
    .await?)
}

/// Staff refunds stuck as requested since before `before` (the api stopped between asking
/// Paddle and finishing) are paid again, at most `limit` at a time, and returned. A refund Paddle
/// took still lands through its webhook.
pub async fn revert_refund_requests(
    conn: &mut PgConnection,
    before: DateTime<Utc>,
    limit: i64,
) -> DbResult<Vec<OrderRow>> {
    Ok(sqlx::query_as(&format!(
        "UPDATE shop_orders SET status = 'paid', refund_requested_at = NULL,
                refund_reason = NULL, updated_at = now()
          WHERE id IN (SELECT id FROM shop_orders
                        WHERE status = 'refund_requested' AND refund_requested_at < $1
                        ORDER BY id LIMIT $2 FOR UPDATE SKIP LOCKED)
          RETURNING {ORDER}"
    ))
    .bind(before)
    .bind(limit)
    .fetch_all(conn)
    .await?)
}

/// Narrows `orders`. Empty fields match everything.
#[derive(Clone, Debug, Default)]
pub struct OrderFilter<'a> {
    /// Orders this account bought or received (staff view).
    pub account: Option<Uuid>,
    /// Orders as this account may see them: every order it bought, and gifts to it once paid
    /// (paid, refund in flight, refunded).
    pub party: Option<Uuid>,
    /// `paid` also matches orders with a refund in flight.
    pub status: Option<&'a str>,
    pub item: Option<Uuid>,
}

/// Orders newest first, with the total.
pub async fn orders(
    conn: &mut PgConnection,
    filter: &OrderFilter<'_>,
    limit: i64,
    offset: i64,
) -> DbResult<(Vec<OrderRow>, i64)> {
    let filter_sql = "($1::uuid IS NULL OR account_id = $1 OR recipient_id = $1)
                      AND ($2::text IS NULL OR status = $2
                           OR ($2 = 'paid' AND status = 'refund_requested'))
                      AND ($3::uuid IS NULL OR item_id = $3)
                      AND ($4::uuid IS NULL OR account_id = $4
                           OR (recipient_id = $4
                               AND status IN ('paid', 'refund_requested', 'refunded')))";
    let rows = sqlx::query_as(&format!(
        "SELECT {ORDER} FROM shop_orders WHERE {filter_sql} ORDER BY id DESC LIMIT $5 OFFSET $6"
    ))
    .bind(filter.account)
    .bind(filter.status)
    .bind(filter.item)
    .bind(filter.party)
    .bind(limit)
    .bind(offset)
    .fetch_all(&mut *conn)
    .await?;
    let total = sqlx::query_scalar(&format!(
        "SELECT count(*) FROM shop_orders WHERE {filter_sql}"
    ))
    .bind(filter.account)
    .bind(filter.status)
    .bind(filter.item)
    .bind(filter.party)
    .fetch_one(conn)
    .await?;
    Ok((rows, total))
}

/// The buyers with the most orders that pass `status` and `item` (as [`orders`] reads them),
/// most first, then the newest order. Gifts count for the buyer.
pub async fn top_buyers(
    conn: &mut PgConnection,
    status: Option<&str>,
    item: Option<Uuid>,
    limit: i64,
) -> DbResult<Vec<crate::audit::PersonCount>> {
    Ok(sqlx::query_as(
        "SELECT o.account_id, count(*) AS count FROM shop_orders o
         JOIN accounts x ON x.id = o.account_id AND x.deleted_at IS NULL
         WHERE ($1::text IS NULL OR o.status = $1
                OR ($1 = 'paid' AND o.status = 'refund_requested'))
           AND ($2::uuid IS NULL OR o.item_id = $2)
         GROUP BY o.account_id
         ORDER BY count(*) DESC, max(o.created_at) DESC, o.account_id
         LIMIT $3",
    )
    .bind(status)
    .bind(item)
    .bind(limit)
    .fetch_all(conn)
    .await?)
}

// Paddle events

/// Records a webhook event as handled. False when it was handled before.
pub async fn record_event(
    conn: &mut PgConnection,
    event_id: &str,
    event_type: &str,
) -> DbResult<bool> {
    let done = sqlx::query(
        "INSERT INTO paddle_events (event_id, event_type) VALUES ($1, $2) ON CONFLICT DO NOTHING",
    )
    .bind(event_id)
    .bind(event_type)
    .execute(conn)
    .await?;
    Ok(done.rows_affected() > 0)
}

/// Forgets handled webhook events received before `before` (Paddle stops redelivering long
/// before then).
pub async fn delete_events_before(conn: &mut PgConnection, before: DateTime<Utc>) -> DbResult<u64> {
    let done = sqlx::query("DELETE FROM paddle_events WHERE received_at < $1")
        .bind(before)
        .execute(conn)
        .await?;
    Ok(done.rows_affected())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(visible: bool) -> ItemRow {
        let now = Utc::now();
        ItemRow {
            id: new_id(),
            name: "x".into(),
            description: String::new(),
            price_cents: 100,
            currency: "EUR".into(),
            paddle_price_id: None,
            active_from: None,
            active_to: None,
            stock_limit: None,
            visible,
            sort_order: 0,
            created_by: None,
            created_at: now,
            updated_at: now,
            deleted_at: None,
        }
    }

    #[test]
    fn sale_windows() {
        let now = Utc::now();
        let hour = chrono::Duration::hours(1);
        assert!(item(true).on_sale(now));
        assert!(!item(false).on_sale(now));
        let mut later = item(true);
        later.active_from = Some(now + hour);
        assert!(!later.on_sale(now));
        let mut over = item(true);
        over.active_to = Some(now);
        assert!(!over.on_sale(now));
        let mut gone = item(true);
        gone.deleted_at = Some(now);
        assert!(!gone.on_sale(now));
    }
}
