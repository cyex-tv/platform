//! Automatic persisted queries and GraphQL over `GET` (ADR 0035).
//!
//! A client sends `extensions.persistedQuery = { version: 1, sha256Hash }` (the SHA-256 of the
//! query text, lowercase hex). Without the text, a known hash runs the query stored for it; an
//! unknown one answers `PersistedQueryNotFound` (`persisted_query_not_found`), and the client sends
//! the text with the hash once, which stores it. The text is put back into the request before it
//! is parsed, so every other rule (depth, complexity, the root-field cap, Snowflake IDs) runs on
//! it as on any request. The store is per instance and bounded; a miss on another instance only
//! costs the client one more round trip.
//!
//! Only a query that parsed, passed validation and ran without errors is stored, so text that
//! merely hashes right (garbage, or a query that always fails) never takes a place. The store
//! drops the least recently used query first (a hit counts as a use), and one IP stores at most
//! [`MAX_STORES_PER_IP`] new queries a minute; past that its queries still run, unstored.
//!
//! Over `GET` only queries run: a document with a mutation or subscription is refused
//! (`post_required`, `405`), so a link or an image tag can never change anything.

use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

use async_graphql::extensions::{
    Extension, ExtensionContext, ExtensionFactory, NextExecute, NextParseQuery, NextPrepareRequest,
};
use async_graphql::parser::types::{ExecutableDocument, OperationType};
use async_graphql::{Pos, Request, Response, ServerResult, Variables};
use sha2::{Digest, Sha256};

use super::coded;

/// How many persisted queries one instance keeps (the least recently used go first).
pub const MAX_PERSISTED: usize = 5_000;
/// New queries one IP may store per minute. Generous for an app (it has a handful of queries);
/// it only stops one client from churning the whole store with throwaway text.
pub const MAX_STORES_PER_IP: u32 = 30;
const STORE_WINDOW: Duration = Duration::from_secs(60);
/// IPs whose store counts are kept before old windows are swept out.
const IP_SWEEP_AT: usize = 10_000;
/// The longest query text that is stored. Longer ones still run when sent in full.
pub const MAX_PERSISTED_BYTES: usize = 32 * 1024;

/// The message Apollo-style clients look for to resend the query text.
pub const NOT_FOUND_MESSAGE: &str = "PersistedQueryNotFound";

#[derive(Default)]
struct Store {
    /// Text and last use, by hash.
    texts: HashMap<String, (String, u64)>,
    /// Hashes by last use, oldest first.
    order: BTreeMap<u64, String>,
    /// A counter, bumped on every use.
    clock: u64,
    /// New stores per IP in the current window: when it started and how many.
    by_ip: HashMap<String, (Instant, u32)>,
}

impl Store {
    fn touch(&mut self, hash: &str) {
        self.clock += 1;
        let now = self.clock;
        if let Some((_, used)) = self.texts.get_mut(hash) {
            let before = std::mem::replace(used, now);
            if let Some(h) = self.order.remove(&before) {
                self.order.insert(now, h);
            }
        }
    }

    /// Whether `ip` may store one more query now; counts it if so.
    fn allow(&mut self, ip: &str, now: Instant) -> bool {
        if self.by_ip.len() >= IP_SWEEP_AT {
            self.by_ip
                .retain(|_, (since, _)| now.duration_since(*since) < STORE_WINDOW);
        }
        let entry = self.by_ip.entry(ip.to_owned()).or_insert((now, 0));
        if now.duration_since(entry.0) >= STORE_WINDOW {
            *entry = (now, 0);
        }
        if entry.1 >= MAX_STORES_PER_IP {
            return false;
        }
        entry.1 += 1;
        true
    }
}

/// The persisted queries of this instance, by hash.
#[derive(Clone, Default)]
pub struct PersistedStore(Arc<Mutex<Store>>);

impl PersistedStore {
    /// The text stored for `hash`; a hit counts as a use, so it is dropped last.
    pub fn get(&self, hash: &str) -> Option<String> {
        let mut store = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        let text = store.texts.get(hash).map(|(text, _)| text.clone())?;
        store.touch(hash);
        Some(text)
    }

    /// Whether `hash` is stored, without counting a use.
    pub fn contains(&self, hash: &str) -> bool {
        let store = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        store.texts.contains_key(hash)
    }

    /// Stores `text` for `hash`, dropping the least recently used past [`MAX_PERSISTED`], unless
    /// `ip` stored [`MAX_STORES_PER_IP`] queries this minute already. Returns whether it is
    /// stored now.
    pub fn put(&self, hash: String, text: String, ip: &str) -> bool {
        let mut store = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        if store.texts.contains_key(&hash) {
            store.touch(&hash);
            return true;
        }
        if !store.allow(ip, Instant::now()) {
            return false;
        }
        while store.texts.len() >= MAX_PERSISTED {
            match store.order.pop_first() {
                Some((_, old)) => {
                    store.texts.remove(&old);
                }
                None => break,
            }
        }
        store.clock += 1;
        let now = store.clock;
        store.order.insert(now, hash.clone());
        store.texts.insert(hash, (text, now));
        true
    }
}

/// The SHA-256 of a query text, as persisted queries name it.
pub fn hash_of(text: &str) -> String {
    Sha256::digest(text.as_bytes())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn is_hash(text: &str) -> bool {
    text.len() == 64 && text.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

/// The schema extension: fills in a persisted query's text, or stores a new one.
#[derive(Default)]
pub struct PersistedQueries(pub PersistedStore);

impl ExtensionFactory for PersistedQueries {
    fn create(&self) -> Arc<dyn Extension> {
        Arc::new(PersistedRequest {
            store: self.0.clone(),
            pending: Mutex::default(),
        })
    }
}

/// One request's view of the store (the factory makes one per request).
struct PersistedRequest {
    store: PersistedStore,
    /// A new hash and text sent with this request, stored once it has run without errors.
    pending: Mutex<Option<(String, String)>>,
}

#[async_trait::async_trait]
impl Extension for PersistedRequest {
    async fn prepare_request(
        &self,
        ctx: &ExtensionContext<'_>,
        mut request: Request,
        next: NextPrepareRequest<'_>,
    ) -> ServerResult<Request> {
        if let Some(value) = request.extensions.remove("persistedQuery") {
            let json = value.into_json().unwrap_or_default();
            let version = json.get("version").and_then(serde_json::Value::as_i64);
            let hash = json
                .get("sha256Hash")
                .and_then(serde_json::Value::as_str)
                .filter(|h| is_hash(h))
                .map(str::to_owned);
            let (Some(1), Some(hash)) = (version, hash) else {
                return Err(coded(
                    "bad_persisted_query",
                    "persistedQuery needs version 1 and a sha256Hash (64 lowercase hex digits).",
                )
                .into_server_error(Pos::default()));
            };
            if request.query.is_empty() {
                match self.store.get(&hash) {
                    Some(text) => request.query = text,
                    None => {
                        return Err(coded("persisted_query_not_found", NOT_FOUND_MESSAGE)
                            .into_server_error(Pos::default()));
                    }
                }
            } else if hash_of(&request.query) != hash {
                return Err(coded(
                    "persisted_query_mismatch",
                    "sha256Hash isn't the SHA-256 of the query sent with it.",
                )
                .into_server_error(Pos::default()));
            } else if request.query.len() <= MAX_PERSISTED_BYTES && !self.store.contains(&hash) {
                // Not yet: only once it has parsed, validated and run cleanly (`execute`).
                *self.pending.lock().unwrap_or_else(PoisonError::into_inner) =
                    Some((hash, request.query.clone()));
            }
        }
        next.run(ctx, request).await
    }

    /// Runs only for a request that parsed and passed validation; stores its pending query when
    /// it also ran without errors.
    async fn execute(
        &self,
        ctx: &ExtensionContext<'_>,
        operation_name: Option<&str>,
        next: NextExecute<'_>,
    ) -> Response {
        let response = next.run(ctx, operation_name).await;
        let pending = self
            .pending
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .take();
        if let Some((hash, text)) = pending
            && response.errors.is_empty()
        {
            let ip = ctx
                .data_opt::<crate::limits::ClientIp>()
                .map_or("", |ip| ip.0.as_str());
            self.store.put(hash, text, ip);
        }
        response
    }
}

/// Marks a request that came over `GET`: only queries may run.
pub struct OverGet;

/// Refuses a mutation or subscription in a request that came over `GET` (`post_required`).
pub(super) struct QueriesOnlyOverGet;

impl ExtensionFactory for QueriesOnlyOverGet {
    fn create(&self) -> Arc<dyn Extension> {
        Arc::new(QueriesOnlyOverGet)
    }
}

#[async_trait::async_trait]
impl Extension for QueriesOnlyOverGet {
    async fn parse_query(
        &self,
        ctx: &ExtensionContext<'_>,
        query: &str,
        variables: &Variables,
        next: NextParseQuery<'_>,
    ) -> ServerResult<ExecutableDocument> {
        let document = next.run(ctx, query, variables).await?;
        if ctx.data_opt::<OverGet>().is_some()
            && let Some((_, op)) = document
                .operations
                .iter()
                .find(|(_, op)| op.node.ty != OperationType::Query)
        {
            return Err(coded(
                "post_required",
                "Over GET only queries run. Send mutations and subscriptions with POST.",
            )
            .into_server_error(op.pos));
        }
        Ok(document)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    impl PersistedStore {
        fn len(&self) -> usize {
            self.0
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .texts
                .len()
        }
    }

    #[test]
    fn the_store_keeps_at_most_its_cap_and_drops_the_oldest() {
        let store = PersistedStore::default();
        for i in 0..MAX_PERSISTED + 3 {
            // Each from its own IP, so the per-IP cap doesn't stop the fill.
            store.put(
                format!("{i:064}"),
                format!("query Q{i} {{ __typename }}"),
                &format!("ip{i}"),
            );
        }
        assert_eq!(store.len(), MAX_PERSISTED);
        assert!(store.get(&format!("{:064}", 0)).is_none());
        assert!(store.get(&format!("{:064}", MAX_PERSISTED + 2)).is_some());
    }

    #[test]
    fn a_hit_keeps_a_query_from_being_dropped() {
        let store = PersistedStore::default();
        let key = |i: usize| format!("{i:064}");
        for i in 0..MAX_PERSISTED {
            store.put(
                key(i),
                format!("query Q{i} {{ __typename }}"),
                &format!("ip{i}"),
            );
        }
        // The oldest is used again, so the next store drops the second oldest instead.
        assert!(store.get(&key(0)).is_some());
        store.put(key(MAX_PERSISTED), "query New { __typename }".into(), "new");
        assert!(store.get(&key(0)).is_some(), "a hit refreshes recency");
        assert!(store.get(&key(1)).is_none());
        assert_eq!(store.len(), MAX_PERSISTED);
    }

    #[test]
    fn one_ip_stores_only_so_many_queries_a_minute() {
        let store = PersistedStore::default();
        let n = MAX_STORES_PER_IP as usize;
        for i in 0..n {
            assert!(store.put(format!("{i:064}"), "{ __typename }".into(), "1.2.3.4"));
        }
        assert!(!store.put(format!("{n:064}"), "{ __typename }".into(), "1.2.3.4"));
        assert!(store.get(&format!("{n:064}")).is_none());
        // Another address still may.
        assert!(store.put(format!("{n:064}"), "{ __typename }".into(), "5.6.7.8"));
    }

    #[test]
    fn hashes_are_lowercase_sha256_hex() {
        let h = hash_of("{ __typename }");
        assert!(is_hash(&h));
        assert!(!is_hash(&h.to_uppercase()));
        assert!(!is_hash("abc"));
    }
}
