//! The audit log: every change to roles, grants and moderation state (ADR 0010). Write it in the
//! same transaction as the change. Rows are never changed afterwards.

use chrono::{DateTime, Utc};
use serde_json::Value;
use sqlx::{FromRow, PgConnection};
use uuid::Uuid;

use crate::error::DbResult;
use crate::ids::new_id;

#[derive(Clone, Debug, PartialEq, FromRow)]
pub struct Entry {
    pub id: Uuid,
    pub actor_account_id: Option<Uuid>,
    /// For example `roles.create` or `emotes.approve`.
    pub action: String,
    /// For example `role`, `emote`, `account`.
    pub target_kind: String,
    pub target_id: String,
    pub before_json: Option<Value>,
    pub after_json: Option<Value>,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Debug)]
pub struct NewEntry<'a> {
    /// `None` for changes the system makes on its own.
    pub actor: Option<Uuid>,
    pub action: &'a str,
    pub target_kind: &'a str,
    pub target_id: &'a str,
    pub before: Option<Value>,
    pub after: Option<Value>,
}

/// Narrows `list`. Empty fields match everything.
#[derive(Clone, Debug, Default)]
pub struct Filter<'a> {
    pub actor: Option<Uuid>,
    pub target_kind: Option<&'a str>,
    pub target_id: Option<&'a str>,
    /// Paging: only entries older than this one.
    pub before_id: Option<Uuid>,
}

const ENTRY: &str = "id, actor_account_id, action, target_kind, target_id, before_json, \
                     after_json, created_at";

pub async fn write(conn: &mut PgConnection, entry: &NewEntry<'_>) -> DbResult<Uuid> {
    Ok(sqlx::query_scalar(
        "INSERT INTO audit_log
             (id, actor_account_id, action, target_kind, target_id, before_json, after_json)
         VALUES ($1, $2, $3, $4, $5, $6, $7)
         RETURNING id",
    )
    .bind(new_id())
    .bind(entry.actor)
    .bind(entry.action)
    .bind(entry.target_kind)
    .bind(entry.target_id)
    .bind(&entry.before)
    .bind(&entry.after)
    .fetch_one(conn)
    .await?)
}

/// Entries newest first. IDs are ULIDs, so `before_id` pages back in time.
pub async fn list(
    conn: &mut PgConnection,
    filter: &Filter<'_>,
    limit: i64,
) -> DbResult<Vec<Entry>> {
    Ok(sqlx::query_as(&format!(
        "SELECT {ENTRY} FROM audit_log
         WHERE ($1::uuid IS NULL OR actor_account_id = $1)
           AND ($2::text IS NULL OR target_kind = $2)
           AND ($3::text IS NULL OR target_id = $3)
           AND ($4::uuid IS NULL OR id < $4)
         ORDER BY id DESC
         LIMIT $5"
    ))
    .bind(filter.actor)
    .bind(filter.target_kind)
    .bind(filter.target_id)
    .bind(filter.before_id)
    .bind(limit)
    .fetch_all(conn)
    .await?)
}
