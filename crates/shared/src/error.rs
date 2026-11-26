//! The one error shape every endpoint returns: `{ "error": "<code>", "message": "<text>" }`.

use axum::Json;
use axum::http::header::RETRY_AFTER;
use axum::http::{HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// An error answer. `error` is a stable code clients can match on; `message` is for people.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct ErrorBody {
    /// One of the `ErrorCode` values.
    pub error: String,
    pub message: String,
    /// The numbers behind a limit error (`LIMIT_ERRORS`): what the limit is and where the caller
    /// stands, so a client can show them. Missing on every other error.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schema(required = false)]
    pub details: Option<ErrorDetails>,
}

/// The numbers of a limit error. GraphQL carries them as `extensions.limit` and
/// `extensions.current`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct ErrorDetails {
    /// What is allowed: a count, or bytes for a file.
    pub limit: u64,
    /// What the caller has or asked for. At a state limit (`set_full`, `too_many_pending`, ...)
    /// this equals `limit`: the cap is reached. At a request-size cap it is the size sent.
    pub current: u64,
}

#[derive(Debug)]
pub struct ApiError {
    pub status: StatusCode,
    pub code: &'static str,
    pub message: String,
    /// Seconds to wait before trying again, sent as `Retry-After`. Only 429 answers send it; one
    /// without a wait of its own says `DEFAULT_RETRY_AFTER`.
    pub retry_after: Option<u64>,
    /// The numbers of a limit error, sent as `details` (REST) or `extensions.limit` and
    /// `extensions.current` (GraphQL).
    pub details: Option<ErrorDetails>,
}

/// The `Retry-After` of a 429 that has no wait of its own, in seconds.
pub const DEFAULT_RETRY_AFTER: u64 = 60;

pub type ApiResult<T> = Result<T, ApiError>;

impl ApiError {
    pub fn new(status: StatusCode, code: &'static str, message: impl Into<String>) -> Self {
        Self {
            status,
            code,
            message: message.into(),
            retry_after: None,
            details: None,
        }
    }

    /// A limit error: its status comes from `LIMIT_ERRORS`, and it carries the numbers.
    pub fn limit(
        code: &'static str,
        message: impl Into<String>,
        limit: impl Into<u64>,
        current: impl Into<u64>,
    ) -> Self {
        Self::new(crate::limit_errors::status_of(code), code, message)
            .with_details(limit.into(), current.into())
    }

    /// Adds the numbers of a limit error to any error.
    #[must_use]
    pub fn with_details(mut self, limit: u64, current: u64) -> Self {
        self.details = Some(ErrorDetails { limit, current });
        self
    }

    /// Tells the client how many seconds to wait (sent as `Retry-After` on a 429).
    #[must_use]
    pub fn retry_after(mut self, seconds: u64) -> Self {
        self.retry_after = Some(seconds.max(1));
        self
    }

    /// Part of the v1 contract, not built yet.
    pub fn not_implemented() -> Self {
        Self::new(
            StatusCode::NOT_IMPLEMENTED,
            "not_implemented",
            "This endpoint is part of the v1 API but isn't built yet.",
        )
    }

    pub fn unavailable(message: impl Into<String>) -> Self {
        Self::new(StatusCode::SERVICE_UNAVAILABLE, "unavailable", message)
    }

    pub fn internal() -> Self {
        Self::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "internal",
            "Something went wrong on our side.",
        )
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let body = ErrorBody {
            error: self.code.to_owned(),
            message: self.message,
            details: self.details,
        };
        let wait = (self.status == StatusCode::TOO_MANY_REQUESTS)
            .then(|| self.retry_after.unwrap_or(DEFAULT_RETRY_AFTER));
        let mut response = (self.status, Json(body)).into_response();
        if let Some(wait) = wait {
            response
                .headers_mut()
                .insert(RETRY_AFTER, HeaderValue::from(wait));
        }
        response
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_429_carries_retry_after() {
        let plain = ApiError::new(StatusCode::TOO_MANY_REQUESTS, "rate_limited", "x");
        assert_eq!(plain.into_response().headers()[RETRY_AFTER], "60");
        let timed = ApiError::new(StatusCode::TOO_MANY_REQUESTS, "locked_out", "x").retry_after(0);
        assert_eq!(timed.into_response().headers()[RETRY_AFTER], "1");
        let other = ApiError::new(StatusCode::BAD_REQUEST, "bad", "x").retry_after(5);
        assert!(other.into_response().headers().get(RETRY_AFTER).is_none());
    }
}

#[cfg(test)]
mod details_tests {
    use super::*;

    fn body(error: &ApiError) -> serde_json::Value {
        serde_json::to_value(ErrorBody {
            error: error.code.to_owned(),
            message: error.message.clone(),
            details: error.details,
        })
        .unwrap()
    }

    #[test]
    fn a_limit_error_sends_details_and_other_errors_do_not() {
        let limit = ApiError::limit("set_full", "The set is full.", 5u32, 5u32);
        assert_eq!(limit.status, StatusCode::CONFLICT);
        assert_eq!(
            body(&limit)["details"],
            serde_json::json!({ "limit": 5, "current": 5 })
        );
        let plain = body(&ApiError::new(StatusCode::NOT_FOUND, "not_found", "x"));
        assert!(plain.get("details").is_none(), "{plain}");
        // A request-size cap keeps its own status.
        let cap = ApiError::limit("too_many", "x", 100u32, 101u32);
        assert_eq!(cap.status, StatusCode::BAD_REQUEST);
    }
}
