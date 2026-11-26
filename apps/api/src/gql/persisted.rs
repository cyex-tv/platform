//! Automatic persisted queries and GraphQL over `GET` (ADR 0032).
//!
//! A client sends `extensions.persistedQuery = { version: 1, sha256Hash }` (the SHA-256 of the
//! query text, lowercase hex). Without the text, a known hash runs the query stored for it; an
//! unknown one answers `PersistedQueryNotFound` (`persisted_query_not_found`), and the client sends
//! the text with the hash once, which stores it. The text is put back into the request before it
//! is parsed, so every other rule (depth, complexity, the root-field cap, Snowflake IDs) runs on
//! it as on any request. The store is per instance and bounded; a miss on another instance only
//! costs the client one more round trip.
//!
//! Over `GET` only queries run: a document with a mutation or subscription is refused
//! (`post_required`, `405`), so a link or an image tag can never change anything.

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex, PoisonError};

use async_graphql::extensions::{
    Extension, ExtensionContext, ExtensionFactory, NextParseQuery, NextPrepareRequest,
};
use async_graphql::parser::types::{ExecutableDocument, OperationType};
use async_graphql::{Pos, Request, ServerResult, Variables};
use sha2::{Digest, Sha256};

use super::coded;

/// How many persisted queries one instance keeps (the oldest go first).
pub const MAX_PERSISTED: usize = 5_000;
/// The longest query text that is stored. Longer ones still run when sent in full.
pub const MAX_PERSISTED_BYTES: usize = 32 * 1024;

/// The message Apollo-style clients look for to resend the query text.
pub const NOT_FOUND_MESSAGE: &str = "PersistedQueryNotFound";

#[derive(Default)]
struct Store {
    texts: HashMap<String, String>,
    order: VecDeque<String>,
}

/// The persisted queries of this instance, by hash.
#[derive(Clone, Default)]
pub struct PersistedStore(Arc<Mutex<Store>>);

impl PersistedStore {
    pub fn get(&self, hash: &str) -> Option<String> {
        let store = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        store.texts.get(hash).cloned()
    }

    pub fn put(&self, hash: String, text: String) {
        let mut store = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        if store.texts.contains_key(&hash) {
            return;
        }
        while store.texts.len() >= MAX_PERSISTED {
            match store.order.pop_front() {
                Some(old) => {
                    store.texts.remove(&old);
                }
                None => break,
            }
        }
        store.order.push_back(hash.clone());
        store.texts.insert(hash, text);
    }

    #[cfg(test)]
    fn len(&self) -> usize {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .texts
            .len()
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
        Arc::new(PersistedRequest(self.0.clone()))
    }
}

struct PersistedRequest(PersistedStore);

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
                match self.0.get(&hash) {
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
            } else if request.query.len() <= MAX_PERSISTED_BYTES {
                self.0.put(hash, request.query.clone());
            }
        }
        next.run(ctx, request).await
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

    #[test]
    fn the_store_keeps_at_most_its_cap_and_drops_the_oldest() {
        let store = PersistedStore::default();
        for i in 0..MAX_PERSISTED + 3 {
            store.put(format!("{i:064}"), format!("query Q{i} {{ __typename }}"));
        }
        assert_eq!(store.len(), MAX_PERSISTED);
        assert!(store.get(&format!("{:064}", 0)).is_none());
        assert!(store.get(&format!("{:064}", MAX_PERSISTED + 2)).is_some());
    }

    #[test]
    fn hashes_are_lowercase_sha256_hex() {
        let h = hash_of("{ __typename }");
        assert!(is_hash(&h));
        assert!(!is_hash(&h.to_uppercase()));
        assert!(!is_hash("abc"));
    }
}
