//! Emote reports. See `20261118000006_moderation.sql`.

use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgConnection};
use uuid::Uuid;

use crate::error::{DbError, DbResult, unique_violation};
use crate::ids::new_id;

#[derive(Clone, Debug, PartialEq, Eq, FromRow)]
pub struct Report {
    pub id: Uuid,
    pub emote_id: Uuid,
    pub reporter_id: Option<Uuid>,
    pub reason: String,
    /// `open`, `dismissed` or `actioned`.
    pub state: String,
    pub resolved_by: Option<Uuid>,
    pub resolution: Option<String>,
    pub created_at: DateTime<Utc>,
    pub resolved_at: Option<DateTime<Utc>>,
}

const REPORT: &str =
    "id, emote_id, reporter_id, reason, state, resolved_by, resolution, created_at, resolved_at";

/// Files a report. `AlreadyInSet` stands for "this reporter has an open report on it".
pub async fn create(
    conn: &mut PgConnection,
    emote_id: Uuid,
    reporter_id: Uuid,
    reason: &str,
) -> DbResult<Report> {
    sqlx::query_as(&format!(
        "INSERT INTO reports (id, emote_id, reporter_id, reason) VALUES ($1, $2, $3, $4)
         RETURNING {REPORT}"
    ))
    .bind(new_id())
    .bind(emote_id)
    .bind(reporter_id)
    .bind(reason)
    .fetch_one(conn)
    .await
    .map_err(|err| match unique_violation(&err) {
        Some("reports_one_open") => DbError::AlreadyInSet,
        _ => err.into(),
    })
}

pub async fn get(conn: &mut PgConnection, id: Uuid) -> DbResult<Option<Report>> {
    Ok(
        sqlx::query_as(&format!("SELECT {REPORT} FROM reports WHERE id = $1"))
            .bind(id)
            .fetch_optional(conn)
            .await?,
    )
}

/// Open reports, oldest first, and how many there are.
pub async fn open(
    conn: &mut PgConnection,
    limit: i64,
    offset: i64,
) -> DbResult<(Vec<Report>, i64)> {
    let items = sqlx::query_as(&format!(
        "SELECT {REPORT} FROM reports WHERE state = 'open'
         ORDER BY created_at, id LIMIT $1 OFFSET $2"
    ))
    .bind(limit)
    .bind(offset)
    .fetch_all(&mut *conn)
    .await?;
    let total = sqlx::query_scalar("SELECT count(*) FROM reports WHERE state = 'open'")
        .fetch_one(conn)
        .await?;
    Ok((items, total))
}

/// Closes an open report. `None` if it wasn't open.
pub async fn resolve(
    conn: &mut PgConnection,
    id: Uuid,
    state: &str,
    by: Uuid,
    resolution: Option<&str>,
) -> DbResult<Option<Report>> {
    Ok(sqlx::query_as(&format!(
        "UPDATE reports SET state = $2, resolved_by = $3, resolution = $4, resolved_at = now()
         WHERE id = $1 AND state = 'open'
         RETURNING {REPORT}"
    ))
    .bind(id)
    .bind(state)
    .bind(by)
    .bind(resolution)
    .fetch_optional(conn)
    .await?)
}

/// Closes every open report on `emote_ids` as actioned (after a takedown). Returns how many.
pub async fn action_all(
    conn: &mut PgConnection,
    emote_ids: &[Uuid],
    by: Uuid,
    resolution: &str,
) -> DbResult<u64> {
    let done = sqlx::query(
        "UPDATE reports
         SET state = 'actioned', resolved_by = $2, resolution = $3, resolved_at = now()
         WHERE emote_id = ANY($1) AND state = 'open'",
    )
    .bind(emote_ids)
    .bind(by)
    .bind(resolution)
    .execute(conn)
    .await?;
    Ok(done.rows_affected())
}

/// Reports filed by `reporter` since `since`, for the per-account limit.
pub async fn count_by_since(
    conn: &mut PgConnection,
    reporter: Uuid,
    since: DateTime<Utc>,
) -> DbResult<i64> {
    Ok(sqlx::query_scalar(
        "SELECT count(*) FROM reports WHERE reporter_id = $1 AND created_at >= $2",
    )
    .bind(reporter)
    .bind(since)
    .fetch_one(conn)
    .await?)
}
