//! Error helpers the api modules share: the 400, 403, 404, 409 and 429 answers, the database error
//! and the lookups every route starts with.

use axum::http::StatusCode;
use cyex_tv_auth::AuthError;
use cyex_tv_db::{DbError, ids};
use cyex_tv_shared::error::{ApiError, ApiResult};
use cyex_tv_shared::ratelimit::Limiter;
use uuid::Uuid;

use crate::state::AppState;

pub fn bad(code: &'static str, message: impl Into<String>) -> ApiError {
    ApiError::new(StatusCode::BAD_REQUEST, code, message)
}

pub fn conflict(code: &'static str, message: &str) -> ApiError {
    ApiError::new(StatusCode::CONFLICT, code, message)
}

/// `429 rate_limited` with what to tell the person.
pub fn rate_limited(message: &str) -> ApiError {
    ApiError::new(StatusCode::TOO_MANY_REQUESTS, "rate_limited", message)
}

/// Counts one use of `key` against `limiter`; over the limit it is `429 rate_limited` with the
/// feature's own `message`.
pub fn limited(limiter: &Limiter, key: &str, message: &str) -> ApiResult<()> {
    limiter.check(key).map_err(|_| rate_limited(message))
}

pub fn sqlx_error(err: sqlx::Error) -> ApiError {
    db_error(DbError::Sqlx(err))
}

pub fn not_found() -> ApiError {
    ApiError::new(StatusCode::NOT_FOUND, "not_found", "Not found.")
}

pub fn forbidden(message: &str) -> ApiError {
    ApiError::new(StatusCode::FORBIDDEN, "forbidden", message)
}

/// A database error as an API error with its rule's code.
pub fn db_error(err: DbError) -> ApiError {
    match err {
        DbError::NotFound => not_found(),
        DbError::PersonalSetExists => {
            conflict("personal_set_exists", "You already have a personal set.")
        }
        DbError::InvalidSetKind => bad(
            "invalid_set_kind",
            "Sets of this kind can't be made or changed this way.",
        ),
        DbError::SetFull => conflict("set_full", "The set is full."),
        DbError::CapacityTooLow => conflict(
            "capacity_too_low",
            "The set already holds more emotes than that.",
        ),
        DbError::NameTaken => conflict("name_taken", "That name is already used in this set."),
        DbError::AlreadyInSet => conflict("already_in_set", "That emote is already in this set."),
        DbError::NotApproved => conflict(
            "not_approved",
            "That emote isn't approved yet. Until it is, it can only go in its owner's channel sets.",
        ),
        DbError::EmoteUnavailable => conflict("emote_unavailable", "That emote can't be used."),
        DbError::RoleNameTaken => conflict("role_name_taken", "A role with that name exists."),
        DbError::UnknownPermission(_) => bad("unknown_permission", "Unknown permission name."),
        DbError::TooManyEditors => conflict(
            "too_many_editors",
            "An account can have at most 50 editors. Remove one to add someone else.",
        ),
        DbError::AlreadyEditor => conflict("already_editor", "They are an editor already."),
        DbError::CosmeticNameTaken => conflict(
            "name_taken",
            "A badge or paint with that name exists already.",
        ),
        DbError::NotOwned => conflict("not_owned", "You can only show cosmetics the account owns."),
        DbError::CodeTaken => {
            tracing::error!("a new redeem code collided with an existing one");
            ApiError::internal()
        }
        DbError::PendingOrder => conflict(
            "checkout_busy",
            "A checkout for this is starting already. Try again in a moment.",
        ),
        DbError::InvalidVisibility => bad(
            "bad_visibility",
            "Personal sets can't be private, and the global set is always listed.",
        ),
        err @ (DbError::IdentityTaken | DbError::PlatformAlreadyLinked | DbError::LastIdentity) => {
            AuthError::from(err).into()
        }
        DbError::Sqlx(err) => {
            tracing::error!(error = %err, "database error");
            ApiError::internal()
        }
    }
}

pub(crate) async fn conn(
    state: &AppState,
) -> ApiResult<sqlx::pool::PoolConnection<sqlx::Postgres>> {
    state.db.acquire().await.map_err(sqlx_error)
}

/// Parses an ID from the API. A malformed one names nothing, so it is `not_found`.
pub fn parse_id(raw: &str) -> ApiResult<Uuid> {
    ids::from_text(raw.trim()).ok_or_else(not_found)
}

#[cfg(test)]
mod tests {
    use cyex_tv_shared::ratelimit::{Limiter, Rate};

    use super::*;

    #[test]
    fn limited_passes_until_the_burst_is_used_then_says_the_features_message() {
        let limiter = Limiter::new(Rate::per_hour(2));
        assert!(limited(&limiter, "a", "Slow down.").is_ok());
        assert!(limited(&limiter, "a", "Slow down.").is_ok());
        let err = limited(&limiter, "a", "Slow down.").unwrap_err();
        assert_eq!(err.status, StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(err.code, "rate_limited");
        assert_eq!(err.message, "Slow down.");
        // Another key has its own count.
        assert!(limited(&limiter, "b", "Slow down.").is_ok());
    }
}
