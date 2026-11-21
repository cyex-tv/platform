//! Stored channel counts (`emotes.channel_count`): how many channels show each emote, kept by
//! the worker. Triggers queue every emote whose count may have changed; [`refresh_queued`]
//! (every minute) recounts those, and [`reconcile_all`] (hourly) recounts everything, which also
//! covers timed blocks running out (no write happens then). Reads use
//! [`crate::reads::channel_counts`], at most about a minute behind.

use std::collections::HashMap;

use sqlx::{Connection, PgConnection};
use uuid::Uuid;

use crate::error::DbResult;

/// Queue rows read per refresh round.
pub const QUEUE_BATCH: i64 = 5_000;
/// Emotes recounted per statement in the full reconcile.
pub const RECONCILE_BATCH: i64 = 2_000;

/// The live count, per emote of `$1`: channels whose active set holds it, without channels of
/// deleted or blocked accounts. Emotes in no channel have no row.
const LIVE: &str = "SELECT m.emote_id, count(DISTINCT c.id)::integer AS n
    FROM emote_set_members m
    JOIN channels c ON c.active_set_id = m.set_id
    JOIN accounts a ON a.id = c.account_id AND a.deleted_at IS NULL
        AND (a.blocked_at IS NULL OR a.blocked_until <= now())
    WHERE m.emote_id = ANY($1)
    GROUP BY m.emote_id";

/// Counts channels live, as reads did before the stored count. The worker's source of truth.
pub async fn live(conn: &mut PgConnection, emote_ids: &[Uuid]) -> DbResult<HashMap<Uuid, u32>> {
    let rows: Vec<(Uuid, i32)> = sqlx::query_as(LIVE).bind(emote_ids).fetch_all(conn).await?;
    Ok(rows
        .into_iter()
        .map(|(id, n)| (id, u32::try_from(n).unwrap_or(0)))
        .collect())
}

/// The statement [`recount`] runs, with the emote IDs as `$1`. Public so the scale test can ask
/// Postgres for its plan.
#[doc(hidden)]
pub fn recount_sql() -> String {
    format!(
        "UPDATE emotes e SET channel_count = coalesce(n.n, 0)
         FROM unnest($1::uuid[]) AS b(id)
         LEFT JOIN ({LIVE}) n ON n.emote_id = b.id
         WHERE e.id = b.id AND e.channel_count <> coalesce(n.n, 0)"
    )
}

/// Recounts `emote_ids` and stores the counts that changed. Returns how many changed.
pub async fn recount(conn: &mut PgConnection, emote_ids: &[Uuid]) -> DbResult<u64> {
    if emote_ids.is_empty() {
        return Ok(0);
    }
    let done = sqlx::query(&recount_sql())
        .bind(emote_ids)
        .execute(conn)
        .await?;
    Ok(done.rows_affected())
}

/// Recounts the queued emotes, `QUEUE_BATCH` queue rows at a time, until the queue is empty.
/// Only the rows read are removed (after their counts are stored), so a change queued meanwhile
/// is counted in the next round. Returns how many counts changed.
pub async fn refresh_queued(conn: &mut PgConnection) -> DbResult<u64> {
    let mut changed = 0;
    loop {
        let rows: Vec<(i64, Uuid)> = sqlx::query_as(
            "SELECT id, emote_id FROM emote_channel_count_queue ORDER BY id LIMIT $1",
        )
        .bind(QUEUE_BATCH)
        .fetch_all(&mut *conn)
        .await?;
        if rows.is_empty() {
            return Ok(changed);
        }
        let mut emotes: Vec<Uuid> = rows.iter().map(|(_, e)| *e).collect();
        emotes.sort_unstable();
        emotes.dedup();
        let queue_ids: Vec<i64> = rows.iter().map(|(id, _)| *id).collect();
        // The counts are read after the queue rows, so every change those rows stand for is in
        // them; emotes in ID order, so two workers lock rows in the same order.
        let mut tx = conn.begin().await?;
        changed += recount(&mut tx, &emotes).await?;
        tx.commit().await?;
        sqlx::query("DELETE FROM emote_channel_count_queue WHERE id = ANY($1)")
            .bind(&queue_ids)
            .execute(&mut *conn)
            .await?;
        if (rows.len() as i64) < QUEUE_BATCH {
            return Ok(changed);
        }
    }
}

/// Recounts every emote, `RECONCILE_BATCH` at a time in ID order, each batch its own statement.
/// Returns how many counts changed.
pub async fn reconcile_all(conn: &mut PgConnection) -> DbResult<u64> {
    let mut changed = 0;
    let mut after = Uuid::nil();
    loop {
        let batch: Vec<Uuid> =
            sqlx::query_scalar("SELECT id FROM emotes WHERE id > $1 ORDER BY id LIMIT $2")
                .bind(after)
                .bind(RECONCILE_BATCH)
                .fetch_all(&mut *conn)
                .await?;
        let Some(last) = batch.last().copied() else {
            return Ok(changed);
        };
        changed += recount(conn, &batch).await?;
        if (batch.len() as i64) < RECONCILE_BATCH {
            return Ok(changed);
        }
        after = last;
    }
}

/// Queue rows waiting, for the worker's log and the tests.
pub async fn queued(conn: &mut PgConnection) -> DbResult<i64> {
    Ok(
        sqlx::query_scalar("SELECT count(*) FROM emote_channel_count_queue")
            .fetch_one(conn)
            .await?,
    )
}
