//! A short-lived cache, per api instance, of the public reads chat clients make (`/v1/sets/{id}`,
//! `/v1/sets/global`, `/v1/channels/{platform}/{id}`, `/v1/emotes/{id}`, `/v1/cosmetics`).
//!
//! When an editor changes a set, every chat showing it gets the event and refetches the set about
//! a second later: thousands of identical reads of the largest answer at once. Here the first one
//! builds the answer (its JSON and `ETag`) and the others wait for it and share it; until the
//! entry is `ttl` old, later reads are served from it too. A `304` costs no database read at all.
//!
//! Entries are dropped as soon as this instance's feeder sees an event about them (the set, its
//! channel, the cosmetics catalog; every entry for events that may touch any read, such as an
//! emote change). Changes clients aren't told about (a set renamed, made private or deleted)
//! publish the internal `cache.forget` event naming the entries, which every instance's feeder
//! hands here too; the instance that made the change drops them at once as well.
//! Only reads made without a caller are cached, so every viewer would get the same answer.

use std::collections::HashMap;
use std::future::Future;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

use cyex_tv_db::outbox::{self, Event};
use cyex_tv_shared::error::ApiResult;
use tokio::sync::OnceCell;
use uuid::Uuid;

/// The default `ttl` for the api binary (`CYEX_TV_READ_CACHE_MS`).
pub const DEFAULT_TTL: Duration = Duration::from_secs(5);
/// Entries kept at most; past this, expired ones go, then all of them.
pub const MAX_ENTRIES: usize = 4096;

/// A built answer: the JSON body and its strong `ETag`.
#[derive(Debug, PartialEq, Eq)]
pub struct Body {
    pub json: Vec<u8>,
    pub etag: String,
}

/// `None`: nothing there (the read answers 404).
pub type Cached = Option<Arc<Body>>;

struct Slot {
    at: Instant,
    cell: OnceCell<Cached>,
}

pub struct ReadCache {
    ttl: Duration,
    slots: Mutex<HashMap<String, Arc<Slot>>>,
}

/// The key of a set read; the same text as the set's event topic.
pub fn set_key(id: Uuid) -> String {
    outbox::set_topic(id)
}

pub const GLOBAL_SET_KEY: &str = "set:global";
pub const COSMETICS_KEY: &str = outbox::COSMETICS_TOPIC;
/// Not a read-cache entry: the live announcement (`site`), dropped by the same event.
pub const ANNOUNCEMENT_KEY: &str = "announcement";

pub fn emote_key(id: Uuid) -> String {
    format!("emote:{}", cyex_tv_db::ids::to_text(id))
}

impl ReadCache {
    /// A zero `ttl` turns the cache off: every read builds its answer.
    pub fn new(ttl: Duration) -> Self {
        Self {
            ttl,
            slots: Mutex::new(HashMap::new()),
        }
    }

    pub fn disabled() -> Self {
        Self::new(Duration::ZERO)
    }

    pub fn ttl(&self) -> Duration {
        self.ttl
    }

    fn slots(&self) -> MutexGuard<'_, HashMap<String, Arc<Slot>>> {
        self.slots.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// The answer for `key`: a fresh entry's, the one being built right now, or `load`'s. A
    /// failed `load` is not kept; the next read tries again.
    pub async fn get_or_load<F, Fut>(&self, key: &str, load: F) -> ApiResult<Cached>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = ApiResult<Cached>>,
    {
        if self.ttl.is_zero() {
            return load().await;
        }
        let slot = {
            let mut slots = self.slots();
            match slots.get(key) {
                Some(slot) if slot.at.elapsed() < self.ttl => slot.clone(),
                _ => {
                    if slots.len() >= MAX_ENTRIES {
                        let ttl = self.ttl;
                        slots.retain(|_, s| s.at.elapsed() < ttl);
                        if slots.len() >= MAX_ENTRIES {
                            slots.clear();
                        }
                    }
                    let slot = Arc::new(Slot {
                        at: Instant::now(),
                        cell: OnceCell::new(),
                    });
                    slots.insert(key.to_owned(), slot.clone());
                    slot
                }
            }
        };
        slot.cell.get_or_try_init(load).await.cloned()
    }

    /// Drops one entry. Reads already waiting on it still get it; new ones build afresh.
    pub fn forget(&self, key: &str) {
        self.slots().remove(key);
    }

    pub fn clear(&self) {
        self.slots().clear();
    }

    pub fn len(&self) -> usize {
        self.slots().len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Drops what `event` may have changed: the entries named by its topics and its set, or
    /// everything for an event that may touch any read.
    pub fn on_event(&self, event: &Event) {
        match event.event_type.as_str() {
            // Only the chatter's own picks, which aren't cached; a new set changes no read.
            "user.cosmetics_updated" | "set.created" => {}
            "set.emote_added"
            | "set.emote_removed"
            | "set.emote_renamed"
            | "channel.active_set_changed"
            | "personal.updated"
            | "cosmetic.created"
            | "cosmetic.updated"
            | "cosmetic.deleted" => {
                let mut slots = self.slots();
                for topic in &event.topics {
                    slots.remove(topic);
                }
                if let Some(set) = event.set_id {
                    slots.remove(&set_key(set));
                    slots.remove(GLOBAL_SET_KEY);
                }
            }
            outbox::CACHE_FORGET => {
                let mut slots = self.slots();
                for key in outbox::forget_keys(event) {
                    slots.remove(&key);
                }
            }
            // `emote.updated`, `resync`, and anything new.
            _ => self.clear(),
        }
    }
}

impl Default for ReadCache {
    fn default() -> Self {
        Self::disabled()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::*;

    fn body(text: &str) -> Cached {
        Some(Arc::new(Body {
            json: text.as_bytes().to_vec(),
            etag: format!("\"{text}\""),
        }))
    }

    fn event(kind: &str, topics: &[&str], set_id: Option<Uuid>) -> Event {
        Event {
            id: 1,
            event_type: kind.to_owned(),
            topics: topics.iter().map(|t| (*t).to_owned()).collect(),
            set_id,
            payload: serde_json::Value::Null,
            created_at: chrono::Utc::now(),
        }
    }

    #[tokio::test]
    async fn concurrent_reads_build_once() {
        let cache = Arc::new(ReadCache::new(Duration::from_secs(60)));
        let builds = Arc::new(AtomicUsize::new(0));
        let mut tasks = Vec::new();
        for _ in 0..50 {
            let cache = cache.clone();
            let builds = builds.clone();
            tasks.push(tokio::spawn(async move {
                cache
                    .get_or_load("set:a", || async {
                        builds.fetch_add(1, Ordering::SeqCst);
                        tokio::time::sleep(Duration::from_millis(20)).await;
                        Ok(body("a"))
                    })
                    .await
                    .unwrap()
            }));
        }
        for t in tasks {
            assert_eq!(t.await.unwrap(), body("a"));
        }
        assert_eq!(builds.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn entries_expire_and_failures_are_not_kept() {
        let cache = ReadCache::new(Duration::from_millis(500));
        let got = cache.get_or_load("k", || async { Ok(body("1")) }).await;
        assert_eq!(got.unwrap(), body("1"));
        let got = cache.get_or_load("k", || async { Ok(body("2")) }).await;
        assert_eq!(got.unwrap(), body("1"));
        tokio::time::sleep(Duration::from_millis(600)).await;
        let got = cache.get_or_load("k", || async { Ok(body("3")) }).await;
        assert_eq!(got.unwrap(), body("3"));

        let failed = cache
            .get_or_load("e", || async {
                Err(cyex_tv_shared::error::ApiError::internal())
            })
            .await;
        assert!(failed.is_err());
        let got = cache.get_or_load("e", || async { Ok(None) }).await;
        assert_eq!(got.unwrap(), None);
    }

    #[tokio::test]
    async fn a_zero_ttl_never_caches() {
        let cache = ReadCache::disabled();
        cache
            .get_or_load("k", || async { Ok(body("1")) })
            .await
            .unwrap();
        let got = cache.get_or_load("k", || async { Ok(body("2")) }).await;
        assert_eq!(got.unwrap(), body("2"));
        assert!(cache.is_empty());
    }

    #[tokio::test]
    async fn events_drop_what_they_change() {
        let cache = ReadCache::new(Duration::from_secs(60));
        let set = cyex_tv_db::ids::new_id();
        let other = cyex_tv_db::ids::new_id();
        for key in [
            set_key(set),
            set_key(other),
            GLOBAL_SET_KEY.to_owned(),
            "channel:twitch:1".to_owned(),
            COSMETICS_KEY.to_owned(),
        ] {
            cache
                .get_or_load(&key, || async { Ok(body("x")) })
                .await
                .unwrap();
        }
        assert_eq!(cache.len(), 5);
        cache.on_event(&event("user.cosmetics_updated", &["user:twitch:1"], None));
        assert_eq!(cache.len(), 5);
        cache.on_event(&event(
            "set.emote_added",
            &[&set_key(set), "channel:twitch:1"],
            Some(set),
        ));
        let mut left: Vec<String> = cache.slots().keys().cloned().collect();
        left.sort();
        let mut want = vec![set_key(other), COSMETICS_KEY.to_owned()];
        want.sort();
        assert_eq!(left, want);
        cache.on_event(&event("cosmetic.updated", &[COSMETICS_KEY], None));
        assert_eq!(cache.len(), 1);
        let mut forget = event(outbox::CACHE_FORGET, &[outbox::INTERNAL_TOPIC], None);
        forget.payload = serde_json::json!({ "keys": [set_key(other), "nope"] });
        cache.on_event(&forget);
        assert!(cache.is_empty());
        for key in [set_key(set), set_key(other)] {
            cache
                .get_or_load(&key, || async { Ok(body("x")) })
                .await
                .unwrap();
        }
        cache.on_event(&event("emote.updated", &["emote:x"], None));
        assert!(cache.is_empty());
    }

    #[tokio::test]
    async fn the_cache_stays_bounded() {
        let cache = ReadCache::new(Duration::from_secs(60));
        for i in 0..(MAX_ENTRIES + 10) {
            cache
                .get_or_load(&format!("k{i}"), || async { Ok(None) })
                .await
                .unwrap();
        }
        assert!(cache.len() <= MAX_ENTRIES);
    }
}
