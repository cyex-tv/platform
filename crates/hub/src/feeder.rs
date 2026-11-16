//! Reads new outbox events and publishes them to a [`Hub`].
//!
//! Event IDs come from a sequence, and a transaction that took a lower ID can commit after a
//! higher one is already visible (ADR 0005, `crates/db/src/outbox.rs`). Reading "everything after
//! the last ID" would skip it. So the feeder keeps a watermark: every ID at or below it has been
//! seen. Events above it are delivered as they appear and remembered; a missing ID holds the
//! watermark back for up to [`GAP_WAIT`], after which it is taken as rolled back (sequences
//! skip IDs on rollback) and passed.

use std::collections::BTreeSet;
use std::future::Future;
use std::sync::Arc;
use std::time::{Duration, Instant};

use cyex_tv_db::{DbError, outbox};
use sqlx::PgPool;
use sqlx::postgres::PgListener;

use crate::Hub;

/// How often the outbox is read without a notification.
pub const POLL: Duration = Duration::from_secs(1);
/// How long a missing event ID may hold the watermark back.
pub const GAP_WAIT: Duration = Duration::from_secs(5);
/// Events read per query.
pub const BATCH: i64 = 1000;

pub struct Feeder {
    watermark: i64,
    seen: BTreeSet<i64>,
    gap_since: Option<Instant>,
    gap_wait: Duration,
}

impl Feeder {
    /// Starts after `watermark`: older events are for resume, not for live delivery.
    pub fn new(watermark: i64) -> Self {
        Self {
            watermark,
            seen: BTreeSet::new(),
            gap_since: None,
            gap_wait: GAP_WAIT,
        }
    }

    /// For tests: a shorter wait before a missing ID is passed.
    pub fn with_gap_wait(mut self, wait: Duration) -> Self {
        self.gap_wait = wait;
        self
    }

    pub fn watermark(&self) -> i64 {
        self.watermark
    }

    /// Reads once and publishes what's new. Returns how many events were published, and whether
    /// the batch was full (so there may be more).
    pub async fn tick(&mut self, db: &PgPool, hub: &Hub) -> Result<(usize, bool), DbError> {
        let mut conn = db.acquire().await?;
        let events = outbox::after(&mut conn, self.watermark, BATCH).await?;
        drop(conn);
        let full = i64::try_from(events.len()).unwrap_or(i64::MAX) >= BATCH;
        let mut published = 0;
        for event in events {
            if self.seen.insert(event.id) {
                hub.publish(Arc::new(event));
                published += 1;
            }
        }
        self.advance();
        Ok((published, full))
    }

    fn advance(&mut self) {
        loop {
            while self.seen.remove(&(self.watermark + 1)) {
                self.watermark += 1;
            }
            let Some(&next) = self.seen.first() else {
                self.gap_since = None;
                return;
            };
            // `next` is above a missing ID. Wait for it a while, then pass it.
            let since = *self.gap_since.get_or_insert_with(Instant::now);
            if since.elapsed() < self.gap_wait {
                return;
            }
            tracing::debug!(
                from = self.watermark + 1,
                to = next - 1,
                "passing missing event IDs"
            );
            self.watermark = next - 1;
            self.gap_since = None;
        }
    }
}

/// Runs until `shutdown`: wakes on `NOTIFY cyex_tv_events` or every [`POLL`].
pub async fn run(db: PgPool, hub: Hub, shutdown: impl Future<Output = ()>) -> Result<(), DbError> {
    let mut conn = db.acquire().await?;
    let start = outbox::latest_id(&mut conn).await?;
    drop(conn);
    let mut feeder = Feeder::new(start);
    let mut listener = PgListener::connect_with(&db).await?;
    listener.listen(outbox::NOTIFY_CHANNEL).await?;
    tokio::pin!(shutdown);
    tracing::info!(from = start, "event feeder started");
    loop {
        loop {
            match feeder.tick(&db, &hub).await {
                Ok((_, true)) => continue,
                Ok((_, false)) => break,
                Err(err) => {
                    tracing::warn!(error = %err, "event feeder read failed");
                    break;
                }
            }
        }
        tokio::select! {
            () = &mut shutdown => return Ok(()),
            note = listener.recv() => {
                if let Err(err) = note {
                    tracing::warn!(error = %err, "event listener dropped; it reconnects on the next recv");
                    tokio::time::sleep(POLL).await;
                }
            }
            () = tokio::time::sleep(POLL) => {}
        }
    }
}
