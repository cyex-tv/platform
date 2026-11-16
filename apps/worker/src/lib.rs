//! The worker: runs queued jobs (PLAN.md Phase 3) and the periodic clean-up.
//!
//! - Jobs are claimed with `FOR UPDATE SKIP LOCKED` and leased; a worker that dies loses its
//!   lease, and the job runs again elsewhere.
//! - A failed attempt is retried with exponential backoff (10 s, 20 s, 40 s, ... up to an hour),
//!   until the job's `max_attempts`. A handler can also fail a job for good.
//! - Each job runs under a time limit. Memory limits come from the container (Phase 9).
//! - New jobs `NOTIFY cyex_tv_jobs`, so an idle worker starts at once; it also polls, in case a
//!   notification is missed.

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;
use cyex_tv_db::jobs::{self, Job};
use cyex_tv_db::{oauth_states, outbox, sessions};
use cyex_tv_storage::Store;
use sqlx::PgPool;
use sqlx::postgres::PgListener;

/// How a job failed.
#[derive(Debug, thiserror::Error)]
pub enum JobError {
    /// Worth another try (a network error, a busy store).
    #[error("{0}")]
    Retry(String),
    /// Won't work however often it runs (a corrupt file, a missing row).
    #[error("{0}")]
    Permanent(String),
}

/// What handlers can reach.
#[derive(Clone)]
pub struct Ctx {
    pub db: PgPool,
    /// `None` when storage isn't configured; handlers that need it fail for good.
    pub store: Option<Store>,
}

pub type JobFuture = Pin<Box<dyn Future<Output = Result<(), JobError>> + Send>>;
pub type Handler = Arc<dyn Fn(Ctx, Job) -> JobFuture + Send + Sync>;

/// Handlers by job kind.
#[derive(Clone, Default)]
pub struct Handlers(HashMap<String, Handler>);

impl Handlers {
    pub fn on<F, Fut>(mut self, kind: &str, handler: F) -> Self
    where
        F: Fn(Ctx, Job) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<(), JobError>> + Send + 'static,
    {
        self.0.insert(
            kind.to_owned(),
            Arc::new(move |ctx, job| Box::pin(handler(ctx, job)) as JobFuture),
        );
        self
    }

    pub fn kinds(&self) -> Vec<String> {
        let mut kinds: Vec<String> = self.0.keys().cloned().collect();
        kinds.sort();
        kinds
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Limits {
    /// How long a claim lasts before another worker may take the job.
    pub lease: Duration,
    /// How long one attempt may run. Keep it below `lease`.
    pub timeout: Duration,
    /// How often an idle worker looks for jobs without a notification.
    pub poll: Duration,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            lease: Duration::from_secs(300),
            timeout: Duration::from_secs(120),
            poll: Duration::from_secs(10),
        }
    }
}

/// The wait before attempt `attempts + 1`: 10 s doubling, at most an hour.
pub fn backoff(attempts: i32) -> Duration {
    let exp = u32::try_from(attempts.saturating_sub(1))
        .unwrap_or(0)
        .min(9);
    Duration::from_secs((10u64 << exp).min(3600))
}

/// What happened to one job.
#[derive(Debug, PartialEq, Eq)]
pub enum Outcome {
    Done,
    /// Will run again.
    Retrying,
    Failed,
}

fn chrono(d: Duration) -> chrono::Duration {
    chrono::Duration::from_std(d).unwrap_or(chrono::Duration::hours(1))
}

/// Claims and runs one job. `None` when nothing is due.
pub async fn run_one(
    ctx: &Ctx,
    handlers: &Handlers,
    limits: Limits,
) -> Result<Option<(i64, Outcome)>, cyex_tv_db::DbError> {
    let kinds = handlers.kinds();
    if kinds.is_empty() {
        return Ok(None);
    }
    let mut conn = ctx.db.acquire().await?;
    let Some(job) = jobs::claim(&mut conn, &kinds, chrono(limits.lease)).await? else {
        return Ok(None);
    };
    drop(conn);
    let id = job.id;
    let attempts = job.attempts;
    let kind = job.kind.clone();
    let Some(handler) = handlers.0.get(&job.kind).cloned() else {
        return Ok(None);
    };

    let started = std::time::Instant::now();
    let result = match tokio::time::timeout(limits.timeout, handler(ctx.clone(), job)).await {
        Ok(result) => result,
        Err(_) => Err(JobError::Retry(format!(
            "timed out after {} s",
            limits.timeout.as_secs()
        ))),
    };
    let mut conn = ctx.db.acquire().await?;
    let outcome = match result {
        Ok(()) => {
            jobs::complete(&mut conn, id).await?;
            Outcome::Done
        }
        Err(err) => {
            let permanent = matches!(err, JobError::Permanent(_));
            let state = jobs::fail(
                &mut conn,
                id,
                &err.to_string(),
                chrono(backoff(attempts)),
                permanent,
            )
            .await?;
            tracing::warn!(job = id, %kind, attempts, error = %err, %state, "job failed");
            if state == "failed" {
                Outcome::Failed
            } else {
                Outcome::Retrying
            }
        }
    };
    tracing::info!(job = id, %kind, ?outcome, ms = started.elapsed().as_millis() as u64, "job finished");
    Ok(Some((id, outcome)))
}

/// Removes what has expired: sessions, sign-ins never finished, outbox events past the replay
/// window (48 h, ADR 0005), and finished jobs after a week.
pub async fn clean_up(db: &PgPool) -> Result<(), cyex_tv_db::DbError> {
    let mut conn = db.acquire().await?;
    let sessions = sessions::delete_expired(&mut conn).await?;
    let states = oauth_states::delete_expired(&mut conn).await?;
    let events =
        outbox::delete_older_than(&mut conn, Utc::now() - chrono::Duration::hours(48)).await?;
    let done = jobs::delete_done_before(&mut conn, Utc::now() - chrono::Duration::days(7)).await?;
    tracing::info!(sessions, states, events, done, "cleaned up");
    Ok(())
}

/// Runs jobs until `shutdown` resolves, and cleans up every `clean_every`.
pub async fn run(
    ctx: Ctx,
    handlers: Handlers,
    limits: Limits,
    clean_every: Duration,
    shutdown: impl Future<Output = ()>,
) -> Result<(), cyex_tv_db::DbError> {
    let mut listener = PgListener::connect_with(&ctx.db).await?;
    listener.listen(jobs::NOTIFY_CHANNEL).await?;
    let mut cleaner = tokio::time::interval(clean_every);
    tokio::pin!(shutdown);
    loop {
        // Drain what's due.
        loop {
            match run_one(&ctx, &handlers, limits).await {
                Ok(Some(_)) => continue,
                Ok(None) => break,
                Err(err) => {
                    tracing::error!(error = %err, "job loop error");
                    break;
                }
            }
        }
        tokio::select! {
            () = &mut shutdown => return Ok(()),
            _ = cleaner.tick() => {
                if let Err(err) = clean_up(&ctx.db).await {
                    tracing::error!(error = %err, "clean-up failed");
                }
            }
            note = listener.recv() => {
                if let Err(err) = note {
                    tracing::warn!(error = %err, "job listener dropped; it reconnects on the next recv");
                }
            }
            () = tokio::time::sleep(limits.poll) => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backoff_doubles_up_to_an_hour() {
        assert_eq!(backoff(0), Duration::from_secs(10));
        assert_eq!(backoff(1), Duration::from_secs(10));
        assert_eq!(backoff(2), Duration::from_secs(20));
        assert_eq!(backoff(4), Duration::from_secs(80));
        assert_eq!(backoff(9), Duration::from_secs(2560));
        assert_eq!(backoff(10), Duration::from_secs(3600));
        assert_eq!(backoff(1000), Duration::from_secs(3600));
    }

    #[test]
    fn handlers_list_their_kinds() {
        let h = Handlers::default()
            .on("b", |_, _| async { Ok(()) })
            .on("a", |_, _| async { Ok(()) });
        assert_eq!(h.kinds(), ["a", "b"]);
    }
}
