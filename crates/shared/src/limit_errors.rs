//! Which HTTP status each limit error uses, in one table (ADR 0008, API review item 5).
//!
//! The rule:
//! - A limit the **account or a set has reached** (the request is fine, the state is full) is
//!   `409 Conflict`: `set_full`, `too_many_pending`, `too_many_editors`, `limit_reached`.
//!   New limit errors of this kind use 409 too (`ApiError::limit` takes the status from here).
//! - A cap on **one request's size** (too many ids, tags or reasons in one call) is
//!   `400 Bad Request`, a file over its byte cap is `413`. These shipped that way and clients
//!   match on them, so they keep their status.
//! - `capacity_over_limit` is a state limit that shipped as 400 and stays 400 for compatibility.
//! - Rate limits are `429` with `Retry-After` (see `rate_limited`), not in this table.
//!
//! Every error here carries `details { limit, current }` over REST and `extensions.limit` and
//! `extensions.current` in GraphQL. At a state limit `current` equals `limit` (the cap is
//! reached); for a request cap it is the number sent. `too_large` carries them only when the
//! bytes are over (`limit` is the byte cap, `current` the size sent); a side or frame cap names
//! its numbers in the message.

use axum::http::StatusCode;

/// One limit error.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LimitError {
    pub code: &'static str,
    /// The HTTP status (REST). GraphQL answers 200 with the code in `extensions.code`.
    pub status: u16,
    /// What `limit` counts.
    pub counts: &'static str,
    /// Why the status is not 409, when it is not.
    pub note: &'static str,
}

const fn l(
    code: &'static str,
    status: u16,
    counts: &'static str,
    note: &'static str,
) -> LimitError {
    LimitError {
        code,
        status,
        counts,
        note,
    }
}

/// Every limit error and its status.
pub const LIMIT_ERRORS: &[LimitError] = &[
    l("set_full", 409, "emotes the set can hold", ""),
    l("too_many_pending", 409, "emotes waiting for review", ""),
    l("too_many_editors", 409, "editors an account can have", ""),
    l(
        "too_many_api_keys",
        409,
        "developer API keys an account can have",
        "",
    ),
    l(
        "too_many_webhooks",
        409,
        "webhooks a channel can have, or webhooks asking for cosmetics events",
        "",
    ),
    l(
        "limit_reached",
        409,
        "hidden emotes, hidden creators or favourites",
        "",
    ),
    l(
        "capacity_over_limit",
        400,
        "slots a set can have (the owner's `set_capacity_max`)",
        "Shipped as 400; kept so clients that match it keep working.",
    ),
    l(
        "too_many_sets",
        400,
        "sets one editor can be limited to",
        "A cap on one request's size.",
    ),
    l(
        "too_many",
        400,
        "items in one request",
        "A cap on one request's size.",
    ),
    l(
        "bad_batch",
        400,
        "items in one batch",
        "A cap on one request's size.",
    ),
    l(
        "too_many_tags",
        400,
        "tags on an emote",
        "A cap on one request's size.",
    ),
    l(
        "too_many_reasons",
        400,
        "ready reject reasons",
        "A cap on one request's size.",
    ),
    l(
        "too_large",
        413,
        "bytes of an uploaded file",
        "The standard status for a body over its cap.",
    ),
    l(
        "too_many_topics",
        400,
        "topics in one subscription",
        "GraphQL subscription; the answer is a GraphQL error, 400 is its REST equivalent.",
    ),
    l(
        "too_many_subscriptions",
        400,
        "subscriptions on one socket",
        "GraphQL subscription; the answer is a GraphQL error, 400 is its REST equivalent.",
    ),
    l(
        "too_many_fields",
        400,
        "root fields in one query",
        "GraphQL query; the answer is a GraphQL error, 400 is its REST equivalent.",
    ),
];

/// The status a limit error uses. Panics in debug builds for a code that is not in the table.
pub fn status_of(code: &str) -> StatusCode {
    let found = LIMIT_ERRORS.iter().find(|e| e.code == code);
    debug_assert!(found.is_some(), "{code} is not in LIMIT_ERRORS");
    found
        .and_then(|e| StatusCode::from_u16(e.status).ok())
        .unwrap_or(StatusCode::CONFLICT)
}

/// A count as the `u64` the details carry.
pub fn n(count: usize) -> u64 {
    u64::try_from(count).unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error_codes::ERROR_CODES;

    #[test]
    fn every_limit_error_is_a_listed_code_once_and_state_limits_are_409() {
        for (i, e) in LIMIT_ERRORS.iter().enumerate() {
            assert!(
                ERROR_CODES.iter().any(|c| c.code == e.code),
                "{} is not an error code",
                e.code
            );
            assert!(LIMIT_ERRORS[i + 1..].iter().all(|o| o.code != e.code));
            // A status other than 409 says why.
            assert_eq!(e.status == 409, e.note.is_empty(), "{}", e.code);
        }
        assert_eq!(status_of("set_full"), StatusCode::CONFLICT);
        assert_eq!(status_of("too_large"), StatusCode::PAYLOAD_TOO_LARGE);
    }
}
