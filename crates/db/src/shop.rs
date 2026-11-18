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

/// Copies taken per item: paid orders, and pending ones from the last hour.
pub async fn taken(conn: &mut PgConnection, items: &[Uuid]) -> DbResult<HashMap<Uuid, i64>> {
    let rows: Vec<(Uuid, i64)> = sqlx::query_as(
        "SELECT item_id, count(*) FROM shop_orders
         WHERE item_id = ANY($1)
           AND (status = 'paid' OR (status = 'pending' AND created_at > now() - interval '1 hour'))
         GROUP BY item_id",
    )
    .bind(items)
    .fetch_all(conn)
    .await?;
    Ok(rows.into_iter().collect())
}

// Entitlements made by the shop and by codes

/// Gives `account` the cosmetic from an order or a code. False when they owned it already.
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

/// Takes back every entitlement an order made, and returns them. Selections of them are cleared
/// by the foreign keys.
pub async fn revoke_order(
    conn: &mut PgConnection,
    order: Uuid,
) -> DbResult<Vec<(CosmeticKind, Uuid, Uuid)>> {
    let mut out = Vec::new();
    for kind in [CosmeticKind::Badge, CosmeticKind::Paint] {
        let (table, column) = entitlement_table(kind);
        let rows: Vec<(Uuid, Uuid)> = sqlx::query_as(&format!(
            "DELETE FROM {table} WHERE order_id = $1 RETURNING account_id, {column}"
        ))
        .bind(order)
        .fetch_all(&mut *conn)
        .await?;
        out.extend(rows.into_iter().map(|(a, c)| (kind, a, c)));
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
    /// `pending`, `paid`, `refunded` or `failed`.
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
}

const ORDER: &str = "id, account_id, recipient_id, item_id, item_name, status, amount_cents, \
                     currency, provider, provider_transaction_id, paddle_price_id, \
                     provider_currency, paid_total_cents, \
                     paid_currency, provider_adjustment_id, refund_reason, created_at, \
                     updated_at, paid_at, refunded_at, failed_at";

/// A pending order for `item`, at its current price.
pub async fn create_order(
    conn: &mut PgConnection,
    buyer: Uuid,
    recipient: Uuid,
    item: &ItemRow,
) -> DbResult<OrderRow> {
    Ok(sqlx::query_as(&format!(
        "INSERT INTO shop_orders (id, account_id, recipient_id, item_id, item_name, amount_cents, currency,
                                  paddle_price_id)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
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
    .fetch_one(conn)
    .await?)
}

pub async fn set_transaction(
    conn: &mut PgConnection,
    id: Uuid,
    transaction_id: &str,
    currency: Option<&str>,
) -> DbResult<OrderRow> {
    sqlx::query_as(&format!(
        "UPDATE shop_orders SET provider_transaction_id = $2, provider_currency = $3,
                updated_at = now()
         WHERE id = $1 RETURNING {ORDER}"
    ))
    .bind(id)
    .bind(transaction_id)
    .bind(currency)
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

pub async fn mark_failed(conn: &mut PgConnection, id: Uuid) -> DbResult<OrderRow> {
    sqlx::query_as(&format!(
        "UPDATE shop_orders SET status = 'failed', failed_at = now(), updated_at = now()
         WHERE id = $1 RETURNING {ORDER}"
    ))
    .bind(id)
    .fetch_optional(conn)
    .await?
    .ok_or(DbError::NotFound)
}

/// Narrows `orders`. Empty fields match everything.
#[derive(Clone, Debug, Default)]
pub struct OrderFilter<'a> {
    /// Orders this account bought or received.
    pub account: Option<Uuid>,
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
                      AND ($2::text IS NULL OR status = $2)
                      AND ($3::uuid IS NULL OR item_id = $3)";
    let rows = sqlx::query_as(&format!(
        "SELECT {ORDER} FROM shop_orders WHERE {filter_sql} ORDER BY id DESC LIMIT $4 OFFSET $5"
    ))
    .bind(filter.account)
    .bind(filter.status)
    .bind(filter.item)
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
    .fetch_one(conn)
    .await?;
    Ok((rows, total))
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
