//! Event WebSocket sessions. See `20261118000004_event_sessions.sql`.

use chrono::{DateTime, Duration, Utc};
use sqlx::PgConnection;
use uuid::Uuid;

use crate::error::DbResult;

/// Saves the session's topics (creating it if new).
pub async fn save(conn: &mut PgConnection, id: Uuid, topics: &[String]) -> DbResult<()> {
    sqlx::query(
        "INSERT INTO event_sessions (id, topics) VALUES ($1, $2)
         ON CONFLICT (id) DO UPDATE SET topics = EXCLUDED.topics, updated_at = now()",
    )
    .bind(id)
    .bind(topics)
    .execute(conn)
    .await?;
    Ok(())
}

/// A session's topics, if it was used within `max_age`.
pub async fn topics(
    conn: &mut PgConnection,
    id: Uuid,
    max_age: Duration,
) -> DbResult<Option<Vec<String>>> {
    Ok(sqlx::query_scalar(
        "SELECT topics FROM event_sessions WHERE id = $1 AND updated_at > now() - $2",
    )
    .bind(id)
    .bind(max_age)
    .fetch_optional(conn)
    .await?)
}

/// Marks a session as still in use, so it stays resumable.
pub async fn touch(conn: &mut PgConnection, id: Uuid) -> DbResult<()> {
    sqlx::query("UPDATE event_sessions SET updated_at = now() WHERE id = $1")
        .bind(id)
        .execute(conn)
        .await?;
    Ok(())
}

pub async fn delete_older_than(conn: &mut PgConnection, cutoff: DateTime<Utc>) -> DbResult<u64> {
    let done = sqlx::query("DELETE FROM event_sessions WHERE updated_at < $1")
        .bind(cutoff)
        .execute(conn)
        .await?;
    Ok(done.rows_affected())
}
