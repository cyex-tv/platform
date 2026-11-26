//! CORS for browsers calling `api.cyex.tv` from other origins.
//!
//! - **The site** (`CYEX_TV_SITE_URL`, `https://cyex.tv`): credentialed, every method the API has,
//!   so the website can call GraphQL and upload with the session cookie. Its writes still pass the
//!   Origin check (ADR 0009).
//! - **Anyone else:** the public reads only (`GET` of channels, sets and emotes, GraphQL queries over
//!   `GET`, and the personal-set, set and channel lookups), with `Access-Control-Allow-Origin: *` and no credentials, so chat clients running in
//!   other pages (the extension on twitch.tv) can read them. Nothing signed-in is offered to other
//!   origins: the cookie is `SameSite=Lax` and isn't sent anyway, and Bearer clients aren't browsers.

use axum::extract::{Request, State};
use axum::http::header::{
    ACCESS_CONTROL_ALLOW_CREDENTIALS, ACCESS_CONTROL_ALLOW_HEADERS, ACCESS_CONTROL_ALLOW_METHODS,
    ACCESS_CONTROL_ALLOW_ORIGIN, ACCESS_CONTROL_EXPOSE_HEADERS, ACCESS_CONTROL_MAX_AGE, ORIGIN,
    VARY,
};
use axum::http::{HeaderMap, HeaderValue, Method, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};

use crate::state::AppState;

const SITE_METHODS: &str = "GET, POST, OPTIONS";
const SITE_HEADERS: &str = "content-type, authorization";
const PUBLIC_METHODS: &str = "GET, HEAD, POST, OPTIONS";
const PUBLIC_HEADERS: &str = "content-type, if-none-match";
const EXPOSED: &str = "etag, retry-after, ratelimit-limit, ratelimit-remaining, ratelimit-reset, x-cyex-api-version, deprecation, sunset, link";
/// How long a browser may keep a preflight answer.
const MAX_AGE: &str = "600";

/// The public reads any origin may call.
pub fn is_public_read(method: &Method, path: &str) -> bool {
    let read = matches!(*method, Method::GET | Method::HEAD | Method::OPTIONS);
    (read
        && (path == "/v1/meta"
            || path.starts_with("/v1/channels/")
            || path.starts_with("/v1/sets/")
            || path.starts_with("/v1/emotes/")))
        // GraphQL queries over GET (persisted queries, ADR 0035): cookies are never read there.
        // A simple GET needs no preflight, so OPTIONS (which would open POST) stays closed.
        || (path == "/v1/gql" && matches!(*method, Method::GET | Method::HEAD))
        || (matches!(
            path,
            "/v1/personal-sets/lookup" | "/v1/sets/lookup" | "/v1/channels/lookup"
        ) && matches!(*method, Method::POST | Method::OPTIONS))
}

enum Who {
    Site,
    Public,
    None,
}

fn who(state: &AppState, headers: &HeaderMap, method: &Method, path: &str) -> Who {
    let Some(origin) = headers.get(ORIGIN).and_then(|v| v.to_str().ok()) else {
        return Who::None;
    };
    if origin == state.auth.config().site_origin() {
        Who::Site
    } else if is_public_read(method, path) {
        Who::Public
    } else {
        Who::None
    }
}

fn set(headers: &mut HeaderMap, name: axum::http::HeaderName, value: &str) {
    if let Ok(v) = HeaderValue::from_str(value) {
        headers.insert(name, v);
    }
}

fn decorate(headers: &mut HeaderMap, who: &Who, site_origin: &str, preflight: bool) {
    match who {
        Who::Site => {
            set(headers, ACCESS_CONTROL_ALLOW_ORIGIN, site_origin);
            set(headers, ACCESS_CONTROL_ALLOW_CREDENTIALS, "true");
            if preflight {
                set(headers, ACCESS_CONTROL_ALLOW_METHODS, SITE_METHODS);
                set(headers, ACCESS_CONTROL_ALLOW_HEADERS, SITE_HEADERS);
            }
        }
        Who::Public => {
            set(headers, ACCESS_CONTROL_ALLOW_ORIGIN, "*");
            if preflight {
                set(headers, ACCESS_CONTROL_ALLOW_METHODS, PUBLIC_METHODS);
                set(headers, ACCESS_CONTROL_ALLOW_HEADERS, PUBLIC_HEADERS);
            }
        }
        Who::None => return,
    }
    set(headers, ACCESS_CONTROL_EXPOSE_HEADERS, EXPOSED);
    if preflight {
        set(headers, ACCESS_CONTROL_MAX_AGE, MAX_AGE);
    }
    headers.append(VARY, HeaderValue::from_static("origin"));
}

pub async fn middleware(State(state): State<AppState>, request: Request, next: Next) -> Response {
    let site_origin = state.auth.config().site_origin();
    let method = request.method().clone();
    let path = request.uri().path().to_owned();
    let who = who(&state, request.headers(), &method, &path);
    let preflight = method == Method::OPTIONS
        && request
            .headers()
            .contains_key("access-control-request-method");
    if preflight {
        let mut response = StatusCode::NO_CONTENT.into_response();
        decorate(response.headers_mut(), &who, &site_origin, true);
        return response;
    }
    let mut response = next.run(request).await;
    decorate(response.headers_mut(), &who, &site_origin, false);
    response
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn public_reads_are_the_chat_client_routes() {
        assert!(is_public_read(&Method::GET, "/v1/channels/twitch/1"));
        assert!(is_public_read(&Method::GET, "/v1/sets/global"));
        assert!(is_public_read(&Method::GET, "/v1/emotes/x"));
        assert!(is_public_read(&Method::GET, "/v1/meta"));
        assert!(is_public_read(&Method::POST, "/v1/personal-sets/lookup"));
        assert!(is_public_read(&Method::POST, "/v1/sets/lookup"));
        assert!(is_public_read(&Method::POST, "/v1/channels/lookup"));
        assert!(is_public_read(&Method::GET, "/v1/gql"));
        assert!(!is_public_read(&Method::OPTIONS, "/v1/gql"));
        assert!(!is_public_read(&Method::POST, "/v1/gql"));
        assert!(!is_public_read(&Method::POST, "/v1/emotes"));
        assert!(!is_public_read(&Method::GET, "/v1/auth/twitch/login"));
    }
}
