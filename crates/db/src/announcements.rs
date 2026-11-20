//! The site announcement: one line staff put under the header for everyone. See
//! `20261208001301_announcements_prefs.sql`.

use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgConnection};
use uuid::Uuid;

use crate::error::DbResult;
use crate::ids::new_id;

#[derive(Clone, Debug, PartialEq, Eq, FromRow)]
pub struct Announcement {
    pub id: Uuid,
    /// `info`, `maintenance` or `campaign`.
    pub kind: String,
    pub body: String,
    pub link_url: Option<String>,
    pub link_label: Option<String>,
    pub emote_id: Option<Uuid>,
    pub badge_id: Option<Uuid>,
    pub starts_at: Option<DateTime<Utc>>,
    pub ends_at: Option<DateTime<Utc>>,
    pub ended_at: Option<DateTime<Utc>>,
    pub created_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// What staff write: everything but the times the database keeps.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Fields {
    pub kind: String,
    pub body: String,
    pub link_url: Option<String>,
    pub link_label: Option<String>,
    pub emote_id: Option<Uuid>,
    pub badge_id: Option<Uuid>,
    pub starts_at: Option<DateTime<Utc>>,
    pub ends_at: Option<DateTime<Utc>>,
}

const COLUMNS: &str = "id, kind, body, link_url, link_label, emote_id, badge_id, starts_at, \
                       ends_at, ended_at, created_by, created_at, updated_at";

/// The one shown at `at`: not ended, started (or without a start) and not over; the latest start
/// wins (one without a start counts from when it was made).
pub async fn live_at(conn: &mut PgConnection, at: DateTime<Utc>) -> DbResult<Option<Announcement>> {
    Ok(sqlx::query_as(&format!(
        "SELECT {COLUMNS} FROM announcements
         WHERE ended_at IS NULL
           AND (starts_at IS NULL OR starts_at <= $1)
           AND (ends_at IS NULL OR ends_at > $1)
         ORDER BY coalesce(starts_at, created_at) DESC, created_at DESC, id DESC
         LIMIT 1"
    ))
    .bind(at)
    .fetch_optional(conn)
    .await?)
}

/// The newest first, at most `limit`, ended ones too.
pub async fn list(conn: &mut PgConnection, limit: i64) -> DbResult<Vec<Announcement>> {
    Ok(sqlx::query_as(&format!(
        "SELECT {COLUMNS} FROM announcements ORDER BY created_at DESC, id DESC LIMIT $1"
    ))
    .bind(limit)
    .fetch_all(conn)
    .await?)
}

/// Locks the row for a change.
pub async fn lock(conn: &mut PgConnection, id: Uuid) -> DbResult<Option<Announcement>> {
    Ok(sqlx::query_as(&format!(
        "SELECT {COLUMNS} FROM announcements WHERE id = $1 FOR UPDATE"
    ))
    .bind(id)
    .fetch_optional(conn)
    .await?)
}

pub async fn create(conn: &mut PgConnection, by: Uuid, f: &Fields) -> DbResult<Announcement> {
    Ok(sqlx::query_as(&format!(
        "INSERT INTO announcements
             (id, kind, body, link_url, link_label, emote_id, badge_id, starts_at, ends_at,
              created_by)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
         RETURNING {COLUMNS}"
    ))
    .bind(new_id())
    .bind(&f.kind)
    .bind(&f.body)
    .bind(&f.link_url)
    .bind(&f.link_label)
    .bind(f.emote_id)
    .bind(f.badge_id)
    .bind(f.starts_at)
    .bind(f.ends_at)
    .bind(by)
    .fetch_one(conn)
    .await?)
}

pub async fn update(conn: &mut PgConnection, id: Uuid, f: &Fields) -> DbResult<Announcement> {
    Ok(sqlx::query_as(&format!(
        "UPDATE announcements
         SET kind = $2, body = $3, link_url = $4, link_label = $5, emote_id = $6, badge_id = $7,
             starts_at = $8, ends_at = $9, updated_at = now()
         WHERE id = $1
         RETURNING {COLUMNS}"
    ))
    .bind(id)
    .bind(&f.kind)
    .bind(&f.body)
    .bind(&f.link_url)
    .bind(&f.link_label)
    .bind(f.emote_id)
    .bind(f.badge_id)
    .bind(f.starts_at)
    .bind(f.ends_at)
    .fetch_one(conn)
    .await?)
}

/// Ends it now. `None` when it was already ended.
pub async fn end(conn: &mut PgConnection, id: Uuid) -> DbResult<Option<Announcement>> {
    Ok(sqlx::query_as(&format!(
        "UPDATE announcements SET ended_at = now(), updated_at = now()
         WHERE id = $1 AND ended_at IS NULL
         RETURNING {COLUMNS}"
    ))
    .bind(id)
    .fetch_optional(conn)
    .await?)
}

/// Whether a badge with this ID exists.
pub async fn badge_exists(conn: &mut PgConnection, id: Uuid) -> DbResult<bool> {
    Ok(
        sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM badges WHERE id = $1)")
            .bind(id)
            .fetch_one(conn)
            .await?,
    )
}
