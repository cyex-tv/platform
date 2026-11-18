//! What the platform tells an account. See `20261118000006_moderation.sql`.

use chrono::{DateTime, Utc};
use serde_json::Value;
use sqlx::{FromRow, PgConnection};
use uuid::Uuid;

use crate::error::DbResult;
use crate::ids::new_id;

#[derive(Clone, Debug, PartialEq, FromRow)]
pub struct Notification {
    pub id: Uuid,
    pub account_id: Uuid,
    pub kind: String,
    pub payload: Value,
    pub created_at: DateTime<Utc>,
    pub read_at: Option<DateTime<Utc>>,
}

const NOTIFICATION: &str = "id, account_id, kind, payload, created_at, read_at";

pub async fn create(
    conn: &mut PgConnection,
    account_id: Uuid,
    kind: &str,
    payload: &Value,
) -> DbResult<Uuid> {
    Ok(sqlx::query_scalar(
        "INSERT INTO notifications (id, account_id, kind, payload) VALUES ($1, $2, $3, $4)
         RETURNING id",
    )
    .bind(new_id())
    .bind(account_id)
    .bind(kind)
    .bind(payload)
    .fetch_one(conn)
    .await?)
}

/// Newest first, the latest 100.
pub async fn list(
    conn: &mut PgConnection,
    account_id: Uuid,
    unread_only: bool,
) -> DbResult<Vec<Notification>> {
    Ok(sqlx::query_as(&format!(
        "SELECT {NOTIFICATION} FROM notifications
         WHERE account_id = $1 AND (NOT $2 OR read_at IS NULL)
         ORDER BY created_at DESC, id DESC LIMIT 100"
    ))
    .bind(account_id)
    .bind(unread_only)
    .fetch_all(conn)
    .await?)
}

/// Every notification of an account, newest first (for the data export).
pub async fn all_for(conn: &mut PgConnection, account_id: Uuid) -> DbResult<Vec<Notification>> {
    Ok(sqlx::query_as(&format!(
        "SELECT {NOTIFICATION} FROM notifications
         WHERE account_id = $1 ORDER BY created_at DESC, id DESC"
    ))
    .bind(account_id)
    .fetch_all(conn)
    .await?)
}

pub async fn count_unread(conn: &mut PgConnection, account_id: Uuid) -> DbResult<i64> {
    Ok(sqlx::query_scalar(
        "SELECT count(*) FROM notifications WHERE account_id = $1 AND read_at IS NULL",
    )
    .bind(account_id)
    .fetch_one(conn)
    .await?)
}

/// Marks some of an account's notifications read; IDs of other accounts' are ignored. Returns how
/// many changed.
pub async fn mark_read_ids(
    conn: &mut PgConnection,
    account_id: Uuid,
    ids: &[Uuid],
) -> DbResult<u64> {
    let done = sqlx::query(
        "UPDATE notifications SET read_at = now()
         WHERE account_id = $1 AND id = ANY($2) AND read_at IS NULL",
    )
    .bind(account_id)
    .bind(ids)
    .execute(conn)
    .await?;
    Ok(done.rows_affected())
}

/// Marks all of an account's notifications read. Returns how many changed.
pub async fn mark_read(conn: &mut PgConnection, account_id: Uuid) -> DbResult<u64> {
    let done = sqlx::query(
        "UPDATE notifications SET read_at = now() WHERE account_id = $1 AND read_at IS NULL",
    )
    .bind(account_id)
    .execute(conn)
    .await?;
    Ok(done.rows_affected())
}
