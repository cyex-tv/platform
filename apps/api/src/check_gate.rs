//! How many uploads are checked (decoded) at the same time.
//!
//! Probing an image is CPU work on the blocking pool. Without a cap, a burst of uploads decodes
//! all at once and starves everything else on the machine. `CYEX_TV_UPLOAD_CHECKS` sets the
//! number of checks that run together (default: the number of CPUs, at least 1). A request that
//! finds them all busy waits its turn for up to [`WAIT`], then answers `503 busy` with
//! `Retry-After`; nothing has been stored by then, so the client just sends it again.

use std::sync::{Arc, OnceLock};
use std::time::Duration;

use axum::http::StatusCode;
use cyex_tv_shared::error::{ApiError, ApiResult};
use tokio::sync::Semaphore;

/// How long a request waits for a free check slot before it gets `busy`.
pub const WAIT: Duration = Duration::from_secs(10);

/// What `Retry-After` says on `busy`, in seconds.
const RETRY_AFTER_SECS: u64 = 5;

/// A fixed number of check slots.
#[derive(Clone)]
pub struct CheckGate {
    slots: Arc<Semaphore>,
    wait: Duration,
}

impl CheckGate {
    /// `slots` checks at once (at least 1); a request waits up to `wait` for one.
    pub fn new(slots: usize, wait: Duration) -> Self {
        Self {
            slots: Arc::new(Semaphore::new(slots.max(1))),
            wait,
        }
    }

    /// Runs `check` on the blocking pool once a slot is free. The slot is held until the check
    /// itself ends, even if the request is dropped meanwhile, so the cap counts real work.
    pub async fn run<T, F>(&self, check: F) -> ApiResult<T>
    where
        T: Send + 'static,
        F: FnOnce() -> T + Send + 'static,
    {
        let waiting = tokio::time::timeout(self.wait, self.slots.clone().acquire_owned());
        let Ok(Ok(permit)) = waiting.await else {
            return Err(busy());
        };
        tokio::task::spawn_blocking(move || {
            let out = check();
            drop(permit);
            out
        })
        .await
        .map_err(|_| ApiError::internal())
    }
}

fn busy() -> ApiError {
    ApiError::new(
        StatusCode::SERVICE_UNAVAILABLE,
        "busy",
        "A lot of uploads are being checked right now. Try again in a few seconds.",
    )
    .retry_after(RETRY_AFTER_SECS)
}

/// The slot count from `CYEX_TV_UPLOAD_CHECKS`; unset, zero or junk means the number of CPUs.
fn configured_slots(raw: Option<&str>) -> usize {
    raw.and_then(|v| v.trim().parse::<usize>().ok())
        .filter(|n| *n > 0)
        .unwrap_or_else(|| std::thread::available_parallelism().map_or(1, usize::from))
}

/// The process-wide gate every upload check goes through.
pub fn gate() -> &'static CheckGate {
    static GATE: OnceLock<CheckGate> = OnceLock::new();
    GATE.get_or_init(|| {
        let raw = std::env::var("CYEX_TV_UPLOAD_CHECKS").ok();
        CheckGate::new(configured_slots(raw.as_deref()), WAIT)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[tokio::test]
    async fn runs_at_most_the_slot_count_at_once() {
        let gate = CheckGate::new(2, Duration::from_secs(10));
        let now = Arc::new(AtomicUsize::new(0));
        let peak = Arc::new(AtomicUsize::new(0));
        let mut jobs = Vec::new();
        for _ in 0..6 {
            let (gate, now, peak) = (gate.clone(), now.clone(), peak.clone());
            jobs.push(tokio::spawn(async move {
                gate.run(move || {
                    let n = now.fetch_add(1, Ordering::SeqCst) + 1;
                    peak.fetch_max(n, Ordering::SeqCst);
                    std::thread::sleep(Duration::from_millis(40));
                    now.fetch_sub(1, Ordering::SeqCst);
                })
                .await
            }));
        }
        for job in jobs {
            job.await.unwrap().unwrap();
        }
        assert_eq!(peak.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn a_request_that_waits_too_long_gets_busy_with_retry_after() {
        let gate = CheckGate::new(1, Duration::from_millis(50));
        let slow = {
            let gate = gate.clone();
            tokio::spawn(async move {
                gate.run(|| std::thread::sleep(Duration::from_millis(400)))
                    .await
            })
        };
        tokio::time::sleep(Duration::from_millis(50)).await;
        let err = gate.run(|| ()).await.unwrap_err();
        assert_eq!(err.status, StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(err.code, "busy");
        assert_eq!(err.retry_after, Some(RETRY_AFTER_SECS));
        slow.await.unwrap().unwrap();
        // The slot comes back once the slow check ends.
        gate.run(|| ()).await.unwrap();
    }

    #[test]
    fn slot_count_falls_back_to_the_cpus() {
        assert_eq!(configured_slots(Some("3")), 3);
        let cpus = std::thread::available_parallelism().map_or(1, usize::from);
        for raw in [None, Some("0"), Some("many"), Some("")] {
            assert_eq!(configured_slots(raw), cpus);
        }
    }
}
