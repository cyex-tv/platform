//! The worker's job queue. See the migration `20261118000002_jobs.sql`.

use chrono::{DateTime, Duration, Utc};
use serde_json::Value;
use sqlx::{FromRow, PgConnection};

use crate::error::DbResult;

/// The `LISTEN` channel a new job notifies.
pub const NOTIFY_CHANNEL: &str = "cyex_tv_jobs";

#[derive(Clone, Debug, PartialEq, FromRow)]
pub struct Job {
    pub id: i64,
    pub kind: String,
    pub payload: Value,
    pub state: String,
    pub attempts: i32,
    pub max_attempts: i32,
    pub run_after: DateTime<Utc>,
    pub locked_until: Option<DateTime<Utc>>,
    pub last_error: Option<String>,
    pub created_at: DateTime<Utc>,
}

const JOB: &str = "id, kind, payload, state, attempts, max_attempts, run_after, locked_until, last_error, created_at";

/// Queues a job. Returns its ID.
pub async fn enqueue(
    conn: &mut PgConnection,
    kind: &str,
    payload: &Value,
    max_attempts: i32,
) -> DbResult<i64> {
    Ok(sqlx::query_scalar(
        "INSERT INTO jobs (kind, payload, max_attempts) VALUES ($1, $2, $3) RETURNING id",
    )
    .bind(kind)
    .bind(payload)
    .bind(max_attempts)
    .fetch_one(conn)
    .await?)
}

/// Claims the oldest due job of one of `kinds` and leases it for `lease`. Several workers can
/// call this at once; each gets a different job or none.
pub async fn claim(
    conn: &mut PgConnection,
    kinds: &[String],
    lease: Duration,
) -> DbResult<Option<Job>> {
    Ok(sqlx::query_as(&format!(
        "UPDATE jobs SET state = 'running', attempts = attempts + 1,
                         locked_until = now() + $2, updated_at = now()
         WHERE id = (
             SELECT id FROM jobs
             WHERE kind = ANY($1)
               AND ((state = 'queued' AND run_after <= now())
                    OR (state = 'running' AND locked_until < now()
                        AND attempts < max_attempts))
             ORDER BY run_after, id
             LIMIT 1
             FOR UPDATE SKIP LOCKED
         )
         RETURNING {JOB}"
    ))
    .bind(kinds)
    .bind(lease)
    .fetch_optional(conn)
    .await?)
}

/// Fails the jobs whose lease expired with no attempts left (the worker died during the last
/// one), the same final state [`fail`] gives. Returns their IDs and kinds.
pub async fn sweep_exhausted(conn: &mut PgConnection) -> DbResult<Vec<(i64, String)>> {
    Ok(sqlx::query_as(
        "UPDATE jobs
         SET state = 'failed', locked_until = NULL,
             last_error = 'lease expired after last attempt', updated_at = now()
         WHERE state = 'running' AND locked_until < now() AND attempts >= max_attempts
         RETURNING id, kind",
    )
    .fetch_all(conn)
    .await?)
}

/// Marks a claimed job done.
pub async fn complete(conn: &mut PgConnection, id: i64) -> DbResult<()> {
    sqlx::query(
        "UPDATE jobs SET state = 'done', locked_until = NULL, last_error = NULL, updated_at = now()
         WHERE id = $1",
    )
    .bind(id)
    .execute(conn)
    .await?;
    Ok(())
}

/// Records a failed attempt. The job runs again after `retry_in`, or is `failed` for good once
/// it has used its attempts (or when `permanent`). Returns the new state.
pub async fn fail(
    conn: &mut PgConnection,
    id: i64,
    error: &str,
    retry_in: Duration,
    permanent: bool,
) -> DbResult<String> {
    let error: String = error.chars().take(2000).collect();
    Ok(sqlx::query_scalar(
        "UPDATE jobs
         SET state = CASE WHEN $4 OR attempts >= max_attempts THEN 'failed' ELSE 'queued' END,
             run_after = now() + $3, locked_until = NULL, last_error = $2, updated_at = now()
         WHERE id = $1
         RETURNING state",
    )
    .bind(id)
    .bind(error)
    .bind(retry_in)
    .bind(permanent)
    .fetch_one(conn)
    .await?)
}

pub async fn get(conn: &mut PgConnection, id: i64) -> DbResult<Option<Job>> {
    Ok(
        sqlx::query_as(&format!("SELECT {JOB} FROM jobs WHERE id = $1"))
            .bind(id)
            .fetch_optional(conn)
            .await?,
    )
}

/// Removes finished jobs older than `cutoff`. Failed ones are kept for a look.
pub async fn delete_done_before(conn: &mut PgConnection, cutoff: DateTime<Utc>) -> DbResult<u64> {
    let done = sqlx::query("DELETE FROM jobs WHERE state = 'done' AND updated_at < $1")
        .bind(cutoff)
        .execute(conn)
        .await?;
    Ok(done.rows_affected())
}

/// Jobs not yet done, by kind and state, for metrics. A queued job is `due` once its
/// `run_after` has passed and `waiting` before that; `running` and `failed` are as stored.
pub async fn counts(conn: &mut PgConnection) -> DbResult<Vec<(String, String, i64)>> {
    let rows = sqlx::query_as(
        "SELECT kind,
                CASE WHEN state = 'queued' AND run_after <= now() THEN 'due'
                     WHEN state = 'queued' THEN 'waiting'
                     ELSE state END AS state,
                count(*)
           FROM jobs
          WHERE state <> 'done'
          GROUP BY 1, 2
          ORDER BY 1, 2",
    )
    .fetch_all(conn)
    .await?;
    Ok(rows)
}
