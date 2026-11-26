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
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use chrono::Utc;
use cyex_tv_db::jobs::{self, Job};
use cyex_tv_db::{
    audit, channel_counts, event_sessions, idempotency, ids, oauth_states, outbox, privacy, search,
    sessions, shop,
};
use cyex_tv_shared::metrics::{self, Labels, Registry};
use cyex_tv_storage::Store;
use sqlx::PgPool;
use sqlx::postgres::PgListener;

pub mod cosmetics;
pub mod purge;
pub mod transcode;

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

/// Sets a flag when dropped. A handler holds one across its blocking encode: when the time limit
/// drops the handler's future, the flag tells the encoding thread to stop, so retries don't pile
/// up threads.
#[derive(Default)]
pub struct CancelOnDrop(Arc<AtomicBool>);

impl CancelOnDrop {
    /// The flag the blocking work checks.
    pub fn flag(&self) -> Arc<AtomicBool> {
        self.0.clone()
    }
}

impl Drop for CancelOnDrop {
    fn drop(&mut self) {
        self.0.store(true, Ordering::Relaxed);
    }
}

/// How a failed media step ends its job: a stopped or failed encoder is tried again, anything
/// else is the file's fault.
pub fn media_failure(err: cyex_tv_media::MediaError) -> JobError {
    use cyex_tv_media::MediaError;
    match err {
        MediaError::Encode(message) => JobError::Retry(message),
        MediaError::Cancelled => JobError::Retry("cancelled".into()),
        other => JobError::Permanent(other.to_string()),
    }
}

/// What handlers can reach.
#[derive(Clone)]
pub struct Ctx {
    pub db: PgPool,
    /// `None` when storage isn't configured; handlers that need it wait for it.
    pub store: Option<Store>,
    /// Cloudflare cache purging for takedowns; `None` without a token.
    pub purge: Option<purge::Purge>,
    /// cyex.tv's Paddle account, for cancelling stale checkouts; `None` (or no API key) skips
    /// that.
    pub paddle: Option<Arc<cyex_tv_paddle::Paddle>>,
}

pub type JobFuture = Pin<Box<dyn Future<Output = Result<(), JobError>> + Send>>;
pub type Handler = Arc<dyn Fn(Ctx, Job) -> JobFuture + Send + Sync>;

/// Handlers by job kind, and where their outcomes are counted.
#[derive(Clone, Default)]
pub struct Handlers {
    map: HashMap<String, Handler>,
    metrics: Option<Registry>,
}

impl Handlers {
    pub fn on<F, Fut>(mut self, kind: &str, handler: F) -> Self
    where
        F: Fn(Ctx, Job) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<(), JobError>> + Send + 'static,
    {
        self.map.insert(
            kind.to_owned(),
            Arc::new(move |ctx, job| Box::pin(handler(ctx, job)) as JobFuture),
        );
        self
    }

    /// Counts finished attempts (`cyex_tv_jobs_finished_total`) and their duration.
    pub fn with_metrics(mut self, registry: Registry) -> Self {
        self.metrics = Some(registry);
        self
    }

    pub fn kinds(&self) -> Vec<String> {
        let mut kinds: Vec<String> = self.map.keys().cloned().collect();
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

impl Outcome {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Done => "done",
            Self::Retrying => "retrying",
            Self::Failed => "failed",
        }
    }
}

/// The queue's gauges, read at scrape time: jobs not done, by kind and state. Alert when `due`
/// stays high (PLAN.md Phase 9).
pub async fn queue_metrics(db: &PgPool) -> String {
    let mut out = String::new();
    let mut conn = match db.acquire().await {
        Ok(conn) => conn,
        Err(err) => {
            tracing::warn!(error = %err, "metrics: no database connection");
            metrics::gauge(
                &mut out,
                "cyex_tv_metrics_db_up",
                "Whether the scrape reached the database.",
                &[(vec![], 0.0)],
            );
            return out;
        }
    };
    metrics::gauge(
        &mut out,
        "cyex_tv_metrics_db_up",
        "Whether the scrape reached the database.",
        &[(vec![], 1.0)],
    );
    match jobs::counts(&mut conn).await {
        Ok(rows) => {
            let samples: Vec<(Labels, f64)> = rows
                .into_iter()
                .map(|(kind, state, n)| (vec![("kind", kind), ("state", state)], n as f64))
                .collect();
            metrics::gauge(
                &mut out,
                "cyex_tv_jobs",
                "Jobs not done, by kind and state (due, waiting, running, failed).",
                &samples,
            );
        }
        Err(err) => tracing::warn!(error = %err, "metrics: job counts failed"),
    }
    out
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
    let Some(handler) = handlers.map.get(&job.kind).cloned() else {
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
    let elapsed = started.elapsed();
    if let Some(registry) = &handlers.metrics {
        let labels: Labels = vec![
            ("kind", kind.clone()),
            ("outcome", outcome.label().to_owned()),
        ];
        registry.inc(
            "cyex_tv_jobs_finished_total",
            "Job attempts finished, by kind and outcome (done, retrying, failed).",
            &labels,
        );
        registry.observe(
            "cyex_tv_job_duration_seconds",
            "Time one job attempt took, by kind and outcome.",
            &labels,
            elapsed.as_secs_f64(),
        );
    }
    tracing::info!(job = id, %kind, ?outcome, ms = elapsed.as_millis() as u64, "job finished");
    Ok(Some((id, outcome)))
}

/// Fails the jobs whose lease ran out on their last attempt (the worker died or was killed
/// during it, so no handler reported). Without this they would sit as `running` for good.
/// Returns how many.
pub async fn sweep_exhausted(db: &PgPool) -> Result<u64, cyex_tv_db::DbError> {
    let mut conn = db.acquire().await?;
    let swept = jobs::sweep_exhausted(&mut conn).await?;
    for (id, kind) in &swept {
        tracing::warn!(job = id, %kind, "job failed: lease expired after last attempt");
    }
    Ok(swept.len() as u64)
}

/// Shop orders left pending this long are failed (`expired`); a late payment is still taken.
pub const PENDING_ORDER_HOURS: i64 = 24;
/// Handled Paddle webhook events are forgotten after this many days.
pub const PADDLE_EVENT_DAYS: i64 = 90;

/// Fails shop orders pending for more than a day, each with an audit row
/// (`shop.order_expired`), in batches. Returns how many.
pub async fn expire_orders(db: &PgPool) -> Result<u64, cyex_tv_db::DbError> {
    let before = Utc::now() - chrono::Duration::hours(PENDING_ORDER_HOURS);
    let mut n = 0;
    loop {
        let mut tx = db.begin().await?;
        let expired = shop::expire_pending(&mut tx, before, 500).await?;
        for order in &expired {
            audit::write(
                &mut tx,
                &audit::NewEntry {
                    actor: None,
                    action: "shop.order_expired",
                    target_kind: "shop_order",
                    target_id: &ids::to_text(order.id),
                    before: Some(serde_json::json!({ "status": "pending" })),
                    after: Some(serde_json::json!({
                        "status": "failed",
                        "reason": shop::failed::EXPIRED,
                        "provider_transaction_id": order.provider_transaction_id,
                    })),
                },
            )
            .await?;
        }
        tx.commit().await?;
        n += expired.len() as u64;
        if expired.len() < 500 {
            return Ok(n);
        }
    }
}

/// A staff refund left as requested this long is put back to paid (see [`revert_refunds`]).
pub const REFUND_REQUEST_HOURS: i64 = 1;
/// Old checkouts cancelled at Paddle per clean-up.
pub const CANCEL_BATCH: i64 = 200;

/// Staff refunds stuck as requested for over an hour (the api stopped between asking Paddle
/// and finishing) are paid again, each with an audit row (`shop.refund_request_reverted`), so
/// staff can refund them again. A refund Paddle did take still arrives through its webhook.
pub async fn revert_refunds(db: &PgPool) -> Result<u64, cyex_tv_db::DbError> {
    let before = Utc::now() - chrono::Duration::hours(REFUND_REQUEST_HOURS);
    let mut n = 0;
    loop {
        let mut tx = db.begin().await?;
        let reverted = shop::revert_refund_requests(&mut tx, before, 500).await?;
        for order in &reverted {
            audit::write(
                &mut tx,
                &audit::NewEntry {
                    actor: None,
                    action: "shop.refund_request_reverted",
                    target_kind: "shop_order",
                    target_id: &ids::to_text(order.id),
                    before: Some(serde_json::json!({ "status": "refund_requested" })),
                    after: Some(serde_json::json!({
                        "status": "paid",
                        "provider_transaction_id": order.provider_transaction_id,
                    })),
                },
            )
            .await?;
        }
        tx.commit().await?;
        n += reverted.len() as u64;
        if reverted.len() < 500 {
            return Ok(n);
        }
    }
}

/// Cancels at Paddle the transactions of replaced and expired orders not tried yet, so their old
/// checkouts can't be paid. A cancel Paddle refuses flags the order (it is not tried again; a
/// late payment is still taken) with an audit row (`shop.transaction_cancel_failed`). Returns
/// how many were cancelled.
pub async fn cancel_stale_checkouts(
    db: &PgPool,
    paddle: &cyex_tv_paddle::Paddle,
) -> Result<u64, cyex_tv_db::DbError> {
    if !paddle.checkout_ready() {
        return Ok(0);
    }
    let mut conn = db.acquire().await?;
    let orders = shop::to_cancel(&mut conn, CANCEL_BATCH).await?;
    drop(conn);
    let mut n = 0;
    for order in orders {
        let Some(txn) = order.provider_transaction_id.as_deref() else {
            continue;
        };
        let result = paddle.cancel_transaction(txn).await;
        let mut tx = db.begin().await?;
        match result {
            Ok(()) => {
                if shop::mark_canceled(&mut tx, order.id).await? {
                    n += 1;
                }
            }
            Err(err) => {
                tracing::warn!(error = %err, %txn, "cancelling an old paddle transaction failed; a late payment is still taken");
                if shop::mark_cancel_failed(&mut tx, order.id).await? {
                    audit::write(
                        &mut tx,
                        &audit::NewEntry {
                            actor: None,
                            action: "shop.transaction_cancel_failed",
                            target_kind: "shop_order",
                            target_id: &ids::to_text(order.id),
                            before: None,
                            after: Some(serde_json::json!({
                                "provider_transaction_id": txn,
                                "reason": order.failed_reason,
                                "error": err.to_string(),
                            })),
                        },
                    )
                    .await?;
                }
            }
        }
        tx.commit().await?;
    }
    Ok(n)
}

/// Accounts erased per clean-up, at most. More wait for the next one.
pub const ERASE_BATCH: i64 = 50;

/// Erases the accounts whose 30-day undo window has passed (ADR 0019), each in its own
/// transaction with its events and audit row (`privacy::erase_account`); their files go through
/// `files.delete`. One that fails is logged and tried again at the next clean-up. Returns how many
/// were erased.
pub async fn erase_due_accounts(db: &PgPool) -> Result<u64, cyex_tv_db::DbError> {
    let mut conn = db.acquire().await?;
    let due = privacy::due_for_erase(&mut conn, ERASE_BATCH).await?;
    let mut erased = 0;
    for account_id in due {
        let mut tx = sqlx::Connection::begin(&mut *conn).await?;
        match privacy::erase_account(&mut tx, account_id).await {
            Ok(done) => {
                tx.commit().await?;
                erased += 1;
                tracing::info!(
                    account = %ids::to_text(account_id),
                    emotes_deleted = done.emotes_deleted,
                    files_deleted = done.files_deleted,
                    "account erased"
                );
            }
            Err(err) => {
                tx.rollback().await?;
                tracing::error!(account = %ids::to_text(account_id), error = %err, "erasing an account failed");
            }
        }
    }
    Ok(erased)
}

/// Removes what has expired: sessions, sign-ins never finished, outbox events past the replay
/// window (48 h, ADR 0005), event socket sessions after 24 h, finished jobs after a week, shop
/// orders pending for a day (failed, see [`expire_orders`]), Paddle webhook events after 90
/// days and idempotency keys after 24 h. Erases accounts deleted over 30 days ago
/// ([`erase_due_accounts`]). Puts staff refunds stuck for an hour back to paid
/// ([`revert_refunds`]). Also refreshes
/// the search ranking, the trending counts and every stored channel count. The worker's loop also runs [`cancel_stale_checkouts`] after it.
pub async fn clean_up(db: &PgPool) -> Result<(), cyex_tv_db::DbError> {
    let mut conn = db.acquire().await?;
    let sessions = sessions::delete_expired(&mut conn).await?;
    let states = oauth_states::delete_expired(&mut conn).await?;
    // Trending (ADR 0004): count channel adds from the outbox first, then it may be pruned. A
    // failure stops here, so nothing is pruned before it is counted.
    let trending = search::rollup_channel_adds(&mut conn).await?;
    let events =
        outbox::delete_older_than(&mut conn, Utc::now() - chrono::Duration::hours(48)).await?;
    let done = jobs::delete_done_before(&mut conn, Utc::now() - chrono::Duration::days(7)).await?;
    let sockets =
        event_sessions::delete_older_than(&mut conn, Utc::now() - chrono::Duration::hours(24))
            .await?;
    let paddle_events = shop::delete_events_before(
        &mut conn,
        Utc::now() - chrono::Duration::days(PADDLE_EVENT_DAYS),
    )
    .await?;
    let idempotency_keys = idempotency::delete_older_than(
        &mut conn,
        Utc::now() - chrono::Duration::hours(idempotency::KEEP_HOURS),
    )
    .await?;
    // Search ranking (ADR 0004): channels showing each emote.
    let ranked = search::refresh_popularity(&mut conn).await?;
    // Stored channel counts, all of them (the minutely refresh only does the queued ones).
    let counts_reconciled = channel_counts::reconcile_all(&mut conn).await?;
    drop(conn);
    let orders_expired = expire_orders(db).await?;
    let refunds_reverted = revert_refunds(db).await?;
    let accounts_erased = erase_due_accounts(db).await?;
    tracing::info!(
        sessions,
        states,
        trending,
        events,
        done,
        sockets,
        paddle_events,
        idempotency_keys,
        orders_expired,
        refunds_reverted,
        accounts_erased,
        ranked,
        counts_reconciled,
        "cleaned up"
    );
    Ok(())
}

/// How often the queued channel counts are recounted ([`refresh_channel_counts`]).
pub const CHANNEL_COUNTS_EVERY: Duration = Duration::from_secs(60);

/// Recounts the emotes queued since the last round (a set member added or removed, a channel's
/// active set switched, an account blocked or deleted). Returns how many counts changed.
pub async fn refresh_channel_counts(db: &PgPool) -> Result<u64, cyex_tv_db::DbError> {
    let mut conn = db.acquire().await?;
    channel_counts::refresh_queued(&mut conn).await
}

/// Runs jobs until `shutdown` resolves, cleans up every `clean_every` and recounts the queued
/// channel counts every [`CHANNEL_COUNTS_EVERY`].
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
    let mut counter = tokio::time::interval(CHANNEL_COUNTS_EVERY);
    counter.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
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
                match sweep_exhausted(&ctx.db).await {
                    Ok(0) => {}
                    Ok(n) => tracing::warn!(failed = n, "jobs out of attempts failed"),
                    Err(err) => tracing::error!(error = %err, "sweeping exhausted jobs failed"),
                }
                if let Err(err) = clean_up(&ctx.db).await {
                    tracing::error!(error = %err, "clean-up failed");
                }
                if let Some(paddle) = &ctx.paddle {
                    match cancel_stale_checkouts(&ctx.db, paddle).await {
                        Ok(n) => tracing::info!(cancelled = n, "old checkouts cancelled at paddle"),
                        Err(err) => tracing::error!(error = %err, "cancelling old checkouts failed"),
                    }
                }
            }
            _ = counter.tick() => {
                match refresh_channel_counts(&ctx.db).await {
                    Ok(0) => {}
                    Ok(n) => tracing::info!(changed = n, "channel counts refreshed"),
                    Err(err) => tracing::error!(error = %err, "refreshing channel counts failed"),
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

    #[tokio::test]
    async fn the_time_limit_sets_the_cancel_flag() {
        let cancel = CancelOnDrop::default();
        let flag = cancel.flag();
        let slow = async move {
            let _held = cancel;
            tokio::time::sleep(Duration::from_secs(5)).await;
        };
        assert!(
            tokio::time::timeout(Duration::from_millis(20), slow)
                .await
                .is_err()
        );
        assert!(flag.load(Ordering::Relaxed));
    }

    #[test]
    fn a_cancelled_encode_is_retried() {
        use cyex_tv_media::MediaError;
        assert!(matches!(
            media_failure(MediaError::Cancelled),
            JobError::Retry(_)
        ));
        assert!(matches!(
            media_failure(MediaError::Invalid("x".into())),
            JobError::Permanent(_)
        ));
    }

    #[test]
    fn handlers_list_their_kinds() {
        let h = Handlers::default()
            .on("b", |_, _| async { Ok(()) })
            .on("a", |_, _| async { Ok(()) });
        assert_eq!(h.kinds(), ["a", "b"]);
    }
}
