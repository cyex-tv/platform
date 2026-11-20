//! What an account chose for itself: which notification kinds it gets and which pop up, and
//! whether its collection is public. See `20261208001301_announcements_prefs.sql`. No row means
//! the defaults (everything on, the collection private).

use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgConnection};
use uuid::Uuid;

use crate::error::DbResult;

#[derive(Clone, Debug, Default, PartialEq, Eq, FromRow)]
pub struct Prefs {
    /// Kinds that aren't created for this account.
    pub notify_off: Vec<String>,
    /// Kinds that are created but don't pop up.
    pub toast_off: Vec<String>,
    pub show_collection: bool,
}

pub async fn get(conn: &mut PgConnection, account_id: Uuid) -> DbResult<Prefs> {
    Ok(sqlx::query_as(
        "SELECT notify_off, toast_off, show_collection FROM account_prefs WHERE account_id = $1",
    )
    .bind(account_id)
    .fetch_optional(conn)
    .await?
    .unwrap_or_default())
}

/// The same as [`get`], but locks the row until the transaction ends (the row is created first
/// when there is none), so two changes to the same account's prefs take turns instead of the
/// later one overwriting the earlier one's merge.
pub async fn get_for_update(conn: &mut PgConnection, account_id: Uuid) -> DbResult<Prefs> {
    sqlx::query("INSERT INTO account_prefs (account_id) VALUES ($1) ON CONFLICT DO NOTHING")
        .bind(account_id)
        .execute(&mut *conn)
        .await?;
    Ok(sqlx::query_as(
        "SELECT notify_off, toast_off, show_collection FROM account_prefs
         WHERE account_id = $1 FOR UPDATE",
    )
    .bind(account_id)
    .fetch_one(conn)
    .await?)
}

pub async fn set_notifications(
    conn: &mut PgConnection,
    account_id: Uuid,
    notify_off: &[String],
    toast_off: &[String],
) -> DbResult<Prefs> {
    Ok(sqlx::query_as(
        "INSERT INTO account_prefs (account_id, notify_off, toast_off) VALUES ($1, $2, $3)
         ON CONFLICT (account_id) DO UPDATE
           SET notify_off = EXCLUDED.notify_off, toast_off = EXCLUDED.toast_off,
               updated_at = now()
         RETURNING notify_off, toast_off, show_collection",
    )
    .bind(account_id)
    .bind(notify_off)
    .bind(toast_off)
    .fetch_one(conn)
    .await?)
}

pub async fn set_show_collection(
    conn: &mut PgConnection,
    account_id: Uuid,
    show: bool,
) -> DbResult<Prefs> {
    Ok(sqlx::query_as(
        "INSERT INTO account_prefs (account_id, show_collection) VALUES ($1, $2)
         ON CONFLICT (account_id) DO UPDATE
           SET show_collection = EXCLUDED.show_collection, updated_at = now()
         RETURNING notify_off, toast_off, show_collection",
    )
    .bind(account_id)
    .bind(show)
    .fetch_one(conn)
    .await?)
}

/// One badge or paint an account holds now, as its public collection lists it: what it is, since
/// when, and how it came (`grant`, `purchase`, `event` or `code`). Never who gave it or why.
#[derive(Clone, Debug, PartialEq, Eq, FromRow)]
pub struct Owned {
    /// `badge` or `paint`.
    pub kind: String,
    pub item_id: Uuid,
    pub item_name: String,
    pub source: String,
    pub granted_at: DateTime<Utc>,
}

/// Every badge and paint an account holds now, newest first, at most `limit`.
pub async fn owned(conn: &mut PgConnection, account_id: Uuid, limit: i64) -> DbResult<Vec<Owned>> {
    Ok(sqlx::query_as(
        "SELECT kind, item_id, item_name, source, granted_at FROM (
             SELECT 'badge' AS kind, e.badge_id AS item_id, b.name AS item_name, e.source,
                    e.granted_at
             FROM badge_entitlements e JOIN badges b ON b.id = e.badge_id
             WHERE e.account_id = $1
             UNION ALL
             SELECT 'paint', e.paint_id, p.name, e.source, e.granted_at
             FROM paint_entitlements e JOIN paints p ON p.id = e.paint_id
             WHERE e.account_id = $1
         ) t
         ORDER BY granted_at DESC, item_id LIMIT $2",
    )
    .bind(account_id)
    .bind(limit)
    .fetch_all(conn)
    .await?)
}
