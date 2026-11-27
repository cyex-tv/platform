//! Idempotency keys for requests that must not run twice (a checkout, a redeem, a review
//! verdict). The client makes one UUID per user action and sends it again when it retries; the
//! first request with it stores its answer for 24 hours, and a repeat gets that answer back
//! without acting again. The same key with another request is `idempotency_key_reused`; a repeat
//! while the first is still running is `idempotency_in_progress` (try again in a moment). A
//! request that fails gives its key back, so retrying it acts again.
//!
//! Only the request's sha256 is stored, never the request itself.

use std::future::Future;

use axum::http::StatusCode;
use cyex_tv_auth::Caller;
use cyex_tv_db::idempotency::{self as db, Claim};
use cyex_tv_shared::error::{ApiError, ApiResult};
use serde::Serialize;
use serde::de::DeserializeOwned;
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::errors::{bad, conflict, db_error};
use crate::state::AppState;

/// The operations that take a key; each has its own key space.
pub mod op {
    pub const START_CHECKOUT: &str = "startCheckout";
    pub const REDEEM_CODE: &str = "redeemCode";
    pub const APPROVE_EMOTE: &str = "approveEmote";
    pub const REVIEW_EMOTES: &str = "reviewEmotes";
    pub const REVIEW_APPROVALS: &str = "reviewApprovals";
}

/// Parses a client's key: a UUID in any of its usual spellings.
pub fn parse_key(raw: &str) -> ApiResult<Uuid> {
    Uuid::parse_str(raw.trim())
        .map_err(|_| bad("bad_idempotency_key", "The idempotency key must be a UUID."))
}

/// The sha256 of what was asked, from its JSON (`serde_json` keeps object keys in order, so the
/// same request hashes the same).
pub fn request_hash(request: &serde_json::Value) -> [u8; 32] {
    Sha256::digest(request.to_string().as_bytes()).into()
}

fn reused() -> ApiError {
    conflict(
        "idempotency_key_reused",
        "That idempotency key was used for a different request. Make a new one for each action.",
    )
}

fn in_progress() -> ApiError {
    ApiError::new(
        StatusCode::CONFLICT,
        "idempotency_in_progress",
        "That request is still being handled. Try again in a moment.",
    )
}

/// Runs `run` once per `key`: without a key it just runs. With one, a repeat gets the stored
/// answer. `request` is what decides the answer (it is hashed, not stored).
pub async fn once<T, F, Fut>(
    state: &AppState,
    caller: &Caller,
    operation: &'static str,
    key: Option<&str>,
    request: serde_json::Value,
    run: F,
) -> ApiResult<T>
where
    T: Serialize + DeserializeOwned,
    F: FnOnce() -> Fut,
    Fut: Future<Output = ApiResult<T>>,
{
    let Some(raw) = key else {
        return run().await;
    };
    let key = parse_key(raw)?;
    let hash = request_hash(&request);
    let account = caller.account_id;
    let mut conn = crate::errors::conn(state).await?;
    match db::claim(&mut conn, account, operation, key, &hash)
        .await
        .map_err(db_error)?
    {
        Claim::New => {}
        Claim::Done {
            request_hash,
            response,
        } => {
            if request_hash != hash {
                return Err(reused());
            }
            return serde_json::from_value(response).map_err(|err| {
                tracing::error!(error = %err, operation, "a stored idempotent answer didn't read back");
                ApiError::internal()
            });
        }
        Claim::Running { request_hash } => {
            return Err(if request_hash == hash {
                in_progress()
            } else {
                reused()
            });
        }
    }
    // Not held while the request runs: it takes connections of its own.
    drop(conn);
    let out = run().await;
    let mut conn = crate::errors::conn(state).await?;
    match &out {
        Ok(value) => {
            let stored = serde_json::to_value(value).map_err(|err| {
                tracing::error!(error = %err, operation, "an idempotent answer didn't serialize");
                ApiError::internal()
            });
            match stored {
                Ok(json) => {
                    if let Err(err) = db::finish(&mut conn, account, operation, key, &json).await {
                        // Done, but not remembered: the key stays claimed (a repeat is told it
                        // is in progress until the key expires), so it never runs twice.
                        tracing::error!(error = %err, operation, "storing an idempotent answer failed");
                    }
                }
                Err(_) => {
                    tracing::error!(
                        operation,
                        "idempotent answer not stored; the key stays claimed"
                    );
                }
            }
        }
        Err(_) => {
            if let Err(err) = db::release(&mut conn, account, operation, key).await {
                tracing::warn!(error = %err, operation, "giving an idempotency key back failed");
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn keys_are_uuids() {
        assert!(parse_key("0b6e1c1e-3f5e-4a43-9a84-3a3c0f6a2b11").is_ok());
        assert!(parse_key(" 0B6E1C1E3F5E4A439A843A3C0F6A2B11 ").is_ok());
        let err = parse_key("not-a-uuid").unwrap_err();
        assert_eq!(err.code, "bad_idempotency_key");
        assert_eq!(err.status, StatusCode::BAD_REQUEST);
    }

    #[test]
    fn the_same_request_hashes_the_same_and_another_does_not() {
        let a = request_hash(&json!({ "itemId": "x", "recipient": null }));
        assert_eq!(
            a,
            request_hash(&json!({ "itemId": "x", "recipient": null }))
        );
        assert_ne!(
            a,
            request_hash(&json!({ "itemId": "y", "recipient": null }))
        );
    }
}
