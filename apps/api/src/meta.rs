//! API version, build and deprecations (ADR 0008, ADR 0027): `GET /v1/meta`, the GraphQL `meta`
//! query, the `X-Cyex-Api-Version` header on every answer, and the `Deprecation` / `Sunset` /
//! `Link` headers (and GraphQL `extensions.deprecations`) on a request that used a deprecated
//! input.
//!
//! A deprecated input is noted while the request runs ([`note`], a task-local), and the
//! middleware turns the notes into headers once the answer is built. Requests that never noted
//! one (and sockets, which have no scope) are left alone.

use std::cell::RefCell;

use async_graphql::SimpleObject;
use axum::extract::Request;
use axum::http::{HeaderName, HeaderValue};
use axum::middleware::Next;
use axum::response::Response;
use chrono::NaiveDate;
use serde::Serialize;
use utoipa::ToSchema;

/// The API's version: the crate's, which is the workspace's.
pub const API_VERSION: &str = env!("CARGO_PKG_VERSION");

/// The git commit this was built from (`CYEX_TV_BUILD_SHA`), or `dev` when the build had none.
pub const BUILD: &str = match option_env!("CYEX_TV_BUILD_SHA") {
    Some(sha) if !sha.is_empty() => sha,
    _ => "dev",
};

pub const VERSION_HEADER: &str = "x-cyex-api-version";

/// Id of the deprecation for ULIDs as account, emote and set IDs in inputs (ADR 0027).
pub const ULID_ID_INPUT: &str = "ulid_id_input";

/// One deprecation: id, what, since, sunset, replacement.
type Entry = (
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    &'static str,
);

/// The deprecated things, newest last. Adding one is additive; removing one waits for its sunset.
const DEPRECATIONS: &[Entry] = &[(
    ULID_ID_INPUT,
    "An account, emote or set ID given as a ULID (26 characters) in an input: a GraphQL argument or variable, a REST path or an upload's owner_id.",
    "2026-11-26",
    "2027-06-01",
    "Send the Snowflake (a decimal string) that every answer already carries (ADR 0027).",
)];

/// A deprecated behaviour of the API.
#[derive(Clone, Debug, Serialize, ToSchema, SimpleObject)]
pub struct ApiDeprecation {
    /// Stable name, also what `extensions.deprecations` lists in a GraphQL answer.
    pub id: String,
    /// What is deprecated.
    pub what: String,
    /// The day it was deprecated, `YYYY-MM-DD`.
    pub since: String,
    /// The day it stops working, `YYYY-MM-DD`. Answers that used it carry it as `Sunset`.
    pub sunset: String,
    /// What to use instead.
    pub replacement: String,
}

/// Where the limits are published.
#[derive(Clone, Debug, Serialize, ToSchema, SimpleObject)]
pub struct ApiLimitsLink {
    /// The REST route.
    pub rest: String,
    /// The GraphQL query field.
    pub graphql_field: String,
}

/// `GET /v1/meta` and the GraphQL `meta` query.
#[derive(Clone, Debug, Serialize, ToSchema, SimpleObject)]
pub struct ApiMeta {
    /// The API's version (semver). Within v1 it only gains fields, types and endpoints.
    pub api_version: String,
    /// The git commit it was built from, or `dev`.
    pub build: String,
    /// Where the limits (sizes, counts, rates) are published.
    pub limits: ApiLimitsLink,
    /// Everything deprecated right now, with the sunset date of each.
    pub deprecations: Vec<ApiDeprecation>,
}

pub fn deprecations() -> Vec<ApiDeprecation> {
    DEPRECATIONS
        .iter()
        .map(|(id, what, since, sunset, replacement)| ApiDeprecation {
            id: (*id).to_owned(),
            what: (*what).to_owned(),
            since: (*since).to_owned(),
            sunset: (*sunset).to_owned(),
            replacement: (*replacement).to_owned(),
        })
        .collect()
}

pub fn meta() -> ApiMeta {
    ApiMeta {
        api_version: API_VERSION.to_owned(),
        build: BUILD.to_owned(),
        limits: ApiLimitsLink {
            rest: "/v1/limits".to_owned(),
            graphql_field: "limits".to_owned(),
        },
        deprecations: deprecations(),
    }
}

/// The sunset day of a deprecation as an HTTP date (RFC 8594), `Tue, 01 Jun 2027 00:00:00 GMT`.
fn sunset_http_date(id: &str) -> Option<String> {
    let entry = DEPRECATIONS.iter().find(|d| d.0 == id)?;
    let day = NaiveDate::parse_from_str(entry.3, "%Y-%m-%d").ok()?;
    Some(day.format("%a, %d %b %Y 00:00:00 GMT").to_string())
}

tokio::task_local! {
    static NOTED: RefCell<Vec<&'static str>>;
}

/// Notes that the running request used the deprecation `id`. Does nothing outside a request.
pub fn note(id: &'static str) {
    let _ = NOTED.try_with(|noted| {
        let mut noted = noted.borrow_mut();
        if !noted.contains(&id) {
            noted.push(id);
        }
    });
}

/// Adds `X-Cyex-Api-Version` to every answer, and the deprecation headers when the request
/// noted a deprecated input.
pub async fn middleware(request: Request, next: Next) -> Response {
    let (mut response, noted) = NOTED
        .scope(RefCell::new(Vec::new()), async {
            let response = next.run(request).await;
            (response, NOTED.with(|n| n.borrow().clone()))
        })
        .await;
    let headers = response.headers_mut();
    headers.insert(
        HeaderName::from_static(VERSION_HEADER),
        HeaderValue::from_static(API_VERSION),
    );
    if let Some(sunset) = noted.iter().find_map(|id| sunset_http_date(id)) {
        headers.insert("deprecation", HeaderValue::from_static("true"));
        if let Ok(v) = HeaderValue::from_str(&sunset) {
            headers.insert("sunset", v);
        }
        headers.append(
            "link",
            HeaderValue::from_static("</v1/meta>; rel=\"deprecation\""),
        );
    }
    response
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_ulid_sunset_is_a_valid_http_date() {
        assert_eq!(
            sunset_http_date(ULID_ID_INPUT).as_deref(),
            Some("Tue, 01 Jun 2027 00:00:00 GMT")
        );
    }

    #[test]
    fn every_deprecation_has_dates_in_order() {
        for d in deprecations() {
            let since = NaiveDate::parse_from_str(&d.since, "%Y-%m-%d").unwrap();
            let sunset = NaiveDate::parse_from_str(&d.sunset, "%Y-%m-%d").unwrap();
            assert!(since < sunset, "{}", d.id);
        }
    }
}
