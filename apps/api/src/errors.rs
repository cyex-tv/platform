//! Error helpers the api modules share: the 400, 403, 404, 409 and 429 answers, the database error
//! and the lookups every route starts with.

use axum::http::StatusCode;
use cyex_tv_auth::AuthError;
use cyex_tv_db::{DbError, ids};
use cyex_tv_shared::error::{ApiError, ApiResult};
use cyex_tv_shared::ratelimit::Limiter;
use uuid::Uuid;

use crate::limits::AccountLimit;
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

/// Counts one use of the per-account `limit` for `key`, at what `account`'s roles allow
/// (`Limits::take`); over it is `429 rate_limited` with the feature's own `message`.
pub fn limited_as(
    state: &AppState,
    limit: AccountLimit,
    key: &str,
    account: Uuid,
    message: &str,
) -> ApiResult<()> {
    state
        .limits
        .take(limit, key, Some(account))
        .map_err(|_| rate_limited(message))
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

/// The deepest page a list serves. A page past it is empty (the total is still counted), so a
/// huge page number never makes the database step over that many rows.
pub const MAX_PAGE: u32 = 1000;

/// One page of a list: the page and size as answered, and the rows to read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Paging {
    /// From 1, as asked.
    pub page: u32,
    /// Clamped to 1 to the list's maximum.
    pub per_page: u32,
    /// Rows to read: `per_page`, or 0 past [`MAX_PAGE`].
    pub limit: i64,
    pub offset: i64,
}

impl Paging {
    /// Past [`MAX_PAGE`]: no rows are read.
    pub fn past_end(&self) -> bool {
        self.page > MAX_PAGE
    }
}

/// `page` (from 1, default 1) and `per_page` (`default`, clamped to 1 to `max`) as the rows to
/// read. Every paged list goes through this.
pub fn paging(page: Option<u32>, per_page: Option<u32>, default: u32, max: u32) -> Paging {
    let per_page = per_page.unwrap_or(default).clamp(1, max);
    let page = page.unwrap_or(1).max(1);
    if page > MAX_PAGE {
        return Paging {
            page,
            per_page,
            limit: 0,
            offset: 0,
        };
    }
    Paging {
        page,
        per_page,
        limit: i64::from(per_page),
        offset: i64::from(page - 1) * i64::from(per_page),
    }
}

#[cfg(test)]
mod tests {
    use cyex_tv_shared::ratelimit::{Limiter, Rate};

    use super::*;

    #[test]
    fn paging_clamps_and_stops_past_the_last_page() {
        let p = paging(None, None, 50, 100);
        assert_eq!((p.page, p.per_page, p.limit, p.offset), (1, 50, 50, 0));
        let p = paging(Some(0), Some(500), 50, 100);
        assert_eq!((p.page, p.per_page, p.limit, p.offset), (1, 100, 100, 0));
        let p = paging(Some(3), Some(0), 50, 100);
        assert_eq!((p.page, p.per_page, p.limit, p.offset), (3, 1, 1, 2));
        let p = paging(Some(MAX_PAGE), Some(100), 50, 100);
        assert_eq!(p.offset, 99_900);
        assert!(!p.past_end());
        let p = paging(Some(1_000_000_000), Some(100), 50, 100);
        assert!(p.past_end());
        assert_eq!(
            (p.page, p.per_page, p.limit, p.offset),
            (1_000_000_000, 100, 0, 0)
        );
    }

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
