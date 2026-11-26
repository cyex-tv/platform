//! Developer API keys (ADR 0033): read-only keys for bots and third-party tools.
//!
//! - A signed-in person makes up to `MAX_KEYS` named keys on the website. A key is
//!   `cyex_live_` and 43 random characters, shown once; only its sha256 is kept.
//! - Sent as `Authorization: Bearer cyex_live_...`. It is not a session: `Auth::caller` never
//!   takes it, so a request with a key runs signed out. It may only read: the public REST reads
//!   and `POST /v1/gql` queries. Anything else on `/v1` answers `api_key_read_only`, and so do
//!   GraphQL mutations (`gql::extensions::AppScopes`).
//! - A key counts under its own bucket (`Rates::api_key`, `CYEX_TV_LIMIT_API_KEY`) instead of the
//!   per-IP one, and the `RateLimit-*` headers describe that bucket. A key that isn't valid
//!   answers `api_key_invalid` and still costs the IP a token of its usual limit.
//! - Accepted requests are counted per key in memory and written per UTC day every minute
//!   (`Usage::flush`).
//! - Sign-out doesn't end keys. Revoking does, and so do account deletion and a staff block; while
//!   an account is deleted or blocked its keys are refused.

use std::collections::HashMap;
use std::sync::{Mutex, PoisonError};

use axum::extract::Request;
use axum::http::header::AUTHORIZATION;
use axum::http::{HeaderMap, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use chrono::{Duration as Days, Utc};
use cyex_tv_auth::{Caller, Via, tokens};
use cyex_tv_db::{api_keys, ids};
use cyex_tv_shared::error::{ApiError, ApiResult};
use cyex_tv_shared::model::{ApiKey, ApiKeyDay, CreatedApiKey};
use cyex_tv_shared::ratelimit::Limiter;
use serde_json::json;
use sqlx::Connection;
use uuid::Uuid;

use crate::errors::{Audit, bad, conflict, db_error, limited_as, record_audit, sqlx_error};
use crate::limits::{self, AccountLimit};
use crate::state::AppState;
use crate::views;

/// Every key starts with this.
pub const KEY_PREFIX: &str = "cyex_live_";
/// Characters of a key shown in the list: the prefix and 8 more.
pub const SHOWN: usize = KEY_PREFIX.len() + 8;
/// Keys one account may hold.
pub const MAX_KEYS: i64 = 10;
/// A key's name, at most this many characters.
pub const NAME_MAX: usize = 64;
/// Days of usage a key's list shows.
pub const USAGE_DAYS: i64 = 30;

/// The key a request sent, if its Bearer token has the key prefix.
pub fn presented(headers: &HeaderMap) -> Option<&str> {
    let value = headers.get(AUTHORIZATION)?.to_str().ok()?;
    let (scheme, token) = value.split_once(' ')?;
    let token = token.trim();
    (scheme.eq_ignore_ascii_case("bearer") && token.starts_with(KEY_PREFIX)).then_some(token)
}

/// Whether `raw` could be a key we made, so junk is refused before any lookup.
fn is_well_formed(raw: &str) -> bool {
    raw.strip_prefix(KEY_PREFIX)
        .is_some_and(tokens::is_well_formed)
}

/// The key a request was let in with, for GraphQL (no mutations) and the handlers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KeyUse {
    pub key_id: Uuid,
    pub account_id: Uuid,
}

/// Requests counted per key since the last flush.
#[derive(Default)]
pub struct Usage {
    counts: Mutex<HashMap<Uuid, i64>>,
}

impl Usage {
    pub fn count(&self, key: Uuid) {
        *self
            .counts
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .entry(key)
            .or_default() += 1;
    }

    /// Writes what was counted under today's UTC date. On a failure the counts are put back.
    pub async fn flush(&self, db: &sqlx::PgPool) -> Result<(), cyex_tv_db::DbError> {
        let taken: Vec<(Uuid, i64)> =
            std::mem::take(&mut *self.counts.lock().unwrap_or_else(PoisonError::into_inner))
                .into_iter()
                .collect();
        if taken.is_empty() {
            return Ok(());
        }
        let write = async {
            let mut conn = db.acquire().await?;
            api_keys::add_usage(&mut conn, Utc::now().date_naive(), &taken).await
        };
        if let Err(err) = write.await {
            let mut counts = self.counts.lock().unwrap_or_else(PoisonError::into_inner);
            for (key, n) in taken {
                *counts.entry(key).or_default() += n;
            }
            return Err(err);
        }
        Ok(())
    }
}

fn read_only() -> Response {
    ApiError::new(
        StatusCode::FORBIDDEN,
        "api_key_read_only",
        "API keys read public data only. Sign in on cyex.tv for anything else.",
    )
    .into_response()
}

fn invalid() -> Response {
    ApiError::new(
        StatusCode::UNAUTHORIZED,
        "api_key_invalid",
        "That API key isn't valid. Make a new one on cyex.tv.",
    )
    .into_response()
}

/// The rate limit for a request that sent `raw` as its key (`limits::middleware` hands it over):
/// the key's bucket for a read, `api_key_read_only` for anything else on `/v1`, `api_key_invalid`
/// for a key that isn't one. `ip_limiter` is the IP's usual limit for the route, charged only
/// when the key is refused, so made-up keys can't be tried for free.
pub async fn handle(
    state: &AppState,
    mut request: Request,
    next: Next,
    raw: &str,
    ip: &str,
    ip_limiter: Option<&Limiter>,
    reads: bool,
) -> Response {
    let path = request.uri().path();
    if !path.starts_with("/v1/") {
        return next.run(request).await;
    }
    if !reads {
        return read_only();
    }
    let found = if is_well_formed(raw) {
        let hash = tokens::hash(raw);
        let lookup = async {
            let mut conn = state.db.acquire().await?;
            api_keys::find_active(&mut conn, &hash).await
        };
        match lookup.await {
            Ok(found) => found,
            Err(err) => return db_error(err).into_response(),
        }
    } else {
        None
    };
    let Some(key) = found else {
        if let Some(limiter) = ip_limiter
            && let Err((wait, _)) = limiter.check_quota(ip)
        {
            return limits::too_many(wait);
        }
        return invalid();
    };
    let quota = match state.limits.api_key.check_quota(&ids::to_text(key.id)) {
        Ok(q) => q,
        Err((wait, q)) => {
            tracing::info!(key = %ids::to_text(key.id), "api key rate limited");
            return limits::too_many_quota(wait, q);
        }
    };
    state.key_usage.count(key.id);
    request.extensions_mut().insert(KeyUse {
        key_id: key.id,
        account_id: key.account_id,
    });
    let mut response = next.run(request).await;
    // The key's bucket, over anything a handler set (none runs as an account here).
    limits::set_quota(response.headers_mut(), quota);
    response
}

fn view(key: &api_keys::ApiKey, usage: Vec<ApiKeyDay>) -> ApiKey {
    ApiKey {
        id: ids::to_text(key.id),
        name: key.name.clone(),
        prefix: key.prefix.clone(),
        created_at: views::time(key.created_at),
        last_used_at: key.last_used_at.map(views::time),
        usage,
    }
}

/// The caller's keys, oldest first, each with its last 30 days of usage.
pub async fn my_keys(state: &AppState, caller: &Caller) -> ApiResult<Vec<ApiKey>> {
    let mut conn = state.db.acquire().await.map_err(sqlx_error)?;
    let keys = api_keys::list(&mut conn, caller.account_id)
        .await
        .map_err(db_error)?;
    let from = Utc::now().date_naive() - Days::days(USAGE_DAYS - 1);
    let ids: Vec<Uuid> = keys.iter().map(|k| k.id).collect();
    let mut by_key: HashMap<Uuid, Vec<ApiKeyDay>> = HashMap::new();
    for (key, day, requests) in api_keys::usage_since(&mut conn, &ids, from)
        .await
        .map_err(db_error)?
    {
        by_key.entry(key).or_default().push(ApiKeyDay {
            day: day.format("%Y-%m-%d").to_string(),
            requests,
        });
    }
    Ok(keys
        .iter()
        .map(|k| view(k, by_key.remove(&k.id).unwrap_or_default()))
        .collect())
}

/// A key's name: trimmed, 1 to 64 characters, no control characters.
fn name_of(raw: &str) -> ApiResult<String> {
    let name = raw.trim();
    if name.is_empty() || name.chars().count() > NAME_MAX || name.chars().any(char::is_control) {
        return Err(bad(
            "bad_name",
            "Give the key a name of 1 to 64 characters.",
        ));
    }
    Ok(name.to_owned())
}

/// Makes a key for the caller. Website only: a token can't make a key that outlives it.
pub async fn create(state: &AppState, caller: &Caller, name: &str) -> ApiResult<CreatedApiKey> {
    if caller.via != Via::Cookie {
        return Err(ApiError::new(
            StatusCode::FORBIDDEN,
            "website_only",
            "Make API keys on cyex.tv, signed in there.",
        ));
    }
    let name = name_of(name)?;
    limited_as(
        state,
        AccountLimit::Prefs,
        &ids::to_text(caller.account_id),
        caller.account_id,
        "That's a lot of changes. Try again later.",
    )?;
    let key = format!("{KEY_PREFIX}{}", tokens::random_token());
    let prefix: String = key.chars().take(SHOWN).collect();
    let mut conn = state.db.acquire().await.map_err(sqlx_error)?;
    let mut tx = conn.begin().await.map_err(sqlx_error)?;
    let made = api_keys::create(
        &mut tx,
        caller.account_id,
        &name,
        &prefix,
        &tokens::hash(&key),
        MAX_KEYS,
    )
    .await
    .map_err(db_error)?
    .ok_or_else(|| {
        conflict(
            "too_many_api_keys",
            "You have 10 API keys, the most you may. Revoke one first.",
        )
    })?;
    let id = ids::to_text(made.id);
    record_audit(
        &mut tx,
        caller.account_id,
        Audit::new("account.api_key_created", "api_key", &id)
            .change(None, Some(json!({ "name": name, "prefix": prefix }))),
    )
    .await?;
    tx.commit().await.map_err(sqlx_error)?;
    Ok(CreatedApiKey {
        key,
        api_key: view(&made, Vec::new()),
    })
}

/// Revokes one of the caller's keys at once. False when it isn't theirs or is gone already.
pub async fn revoke(state: &AppState, caller: &Caller, id: &str) -> ApiResult<bool> {
    let Some(id) = ids::from_text(id.trim()) else {
        return Ok(false);
    };
    let mut conn = state.db.acquire().await.map_err(sqlx_error)?;
    let mut tx = conn.begin().await.map_err(sqlx_error)?;
    let done = api_keys::delete(&mut tx, caller.account_id, id)
        .await
        .map_err(db_error)?;
    if done {
        record_audit(
            &mut tx,
            caller.account_id,
            Audit::new("account.api_key_revoked", "api_key", &ids::to_text(id)),
        )
        .await?;
    }
    tx.commit().await.map_err(sqlx_error)?;
    Ok(done)
}

#[cfg(test)]
mod tests {
    use axum::http::HeaderValue;

    use super::*;

    #[test]
    fn keys_are_told_from_tokens() {
        let mut h = HeaderMap::new();
        assert_eq!(presented(&h), None);
        h.insert(AUTHORIZATION, HeaderValue::from_static("Bearer abc"));
        assert_eq!(presented(&h), None);
        h.insert(
            AUTHORIZATION,
            HeaderValue::from_static("bearer  cyex_live_abc "),
        );
        assert_eq!(presented(&h), Some("cyex_live_abc"));
        assert!(!is_well_formed("cyex_live_abc"));
        let key = format!("{KEY_PREFIX}{}", tokens::random_token());
        assert!(is_well_formed(&key));
        // A key is never a session token: `Auth::caller` refuses it before any lookup.
        assert!(!tokens::is_well_formed(&key));
        assert!(name_of("  bot ").is_ok());
        assert!(name_of(" ").is_err() && name_of(&"x".repeat(65)).is_err());
        assert!(name_of("a\u{7}b").is_err());
    }
}
