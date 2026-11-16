//! Request rate limits (PLAN.md Phase 7), per IP for every route class and per account on
//! GraphQL. A request over a limit gets `429 rate_limited` with `Retry-After`.
//!
//! The upload's 20 an hour per account lives in `upload`, since it counts what was made.

use std::net::SocketAddr;

use axum::extract::{ConnectInfo, Request, State};
use axum::http::header::RETRY_AFTER;
use axum::http::{HeaderValue, Method, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use cyex_tv_shared::error::ApiError;
use cyex_tv_shared::ratelimit::{Limiter, Rate, client_ip};

use crate::state::AppState;

#[derive(Clone, Copy, Debug)]
pub struct Rates {
    /// `/v1/auth/*`, per IP.
    pub auth: Rate,
    /// `POST /v1/gql`, per IP.
    pub gql: Rate,
    /// `POST /v1/gql`, per signed-in account.
    pub gql_account: Rate,
    /// `POST /v1/emotes`, per IP.
    pub upload: Rate,
    /// The public REST reads, per IP. Chat clients poll these; the CDN takes most of it.
    pub reads: Rate,
    /// Opening `/v1/gql/ws`, per IP.
    pub socket: Rate,
}

impl Default for Rates {
    fn default() -> Self {
        Self {
            auth: Rate::per_minute(30),
            gql: Rate::per_minute(300),
            gql_account: Rate::per_minute(600),
            upload: Rate::per_hour(30),
            reads: Rate::per_minute(1200),
            socket: Rate::per_minute(60),
        }
    }
}

pub struct Limits {
    pub auth: Limiter,
    pub gql: Limiter,
    pub gql_account: Limiter,
    pub upload: Limiter,
    pub reads: Limiter,
    pub socket: Limiter,
    /// Take the client IP from `CF-Connecting-IP` (`CYEX_TV_TRUST_CLOUDFLARE=true`).
    pub trust_cloudflare: bool,
}

impl Limits {
    pub fn new(rates: Rates, trust_cloudflare: bool) -> Self {
        Self {
            auth: Limiter::new(rates.auth),
            gql: Limiter::new(rates.gql),
            gql_account: Limiter::new(rates.gql_account),
            upload: Limiter::new(rates.upload),
            reads: Limiter::new(rates.reads),
            socket: Limiter::new(rates.socket),
            trust_cloudflare,
        }
    }
}

impl Default for Limits {
    fn default() -> Self {
        Self::new(Rates::default(), false)
    }
}

/// The 429 answer.
pub fn too_many(wait: std::time::Duration) -> Response {
    let mut response = ApiError::new(
        StatusCode::TOO_MANY_REQUESTS,
        "rate_limited",
        "Too many requests. Slow down and try again shortly.",
    )
    .into_response();
    let secs = wait.as_secs() + u64::from(wait.subsec_nanos() > 0);
    if let Ok(v) = HeaderValue::from_str(&secs.max(1).to_string()) {
        response.headers_mut().insert(RETRY_AFTER, v);
    }
    response
}

/// Which per-IP limit a request falls under, if any (health checks have none).
fn class<'a>(limits: &'a Limits, method: &Method, path: &str) -> Option<&'a Limiter> {
    if path.starts_with("/v1/auth/") {
        return Some(&limits.auth);
    }
    match (method, path) {
        (&Method::POST, "/v1/gql") => Some(&limits.gql),
        (_, "/v1/gql/ws") => Some(&limits.socket),
        (&Method::POST, "/v1/emotes") => Some(&limits.upload),
        (&Method::POST, "/v1/personal-sets/lookup") => Some(&limits.reads),
        (&Method::GET | &Method::HEAD, p) if p.starts_with("/v1/") => Some(&limits.reads),
        _ => None,
    }
}

pub async fn middleware(State(state): State<AppState>, request: Request, next: Next) -> Response {
    let peer = request
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .map(|c| c.0);
    let ip = client_ip(request.headers(), peer, state.limits.trust_cloudflare);
    if let Some(limiter) = class(&state.limits, request.method(), request.uri().path())
        && let Err(wait) = limiter.check(&ip)
    {
        tracing::info!(%ip, path = request.uri().path(), "rate limited");
        return too_many(wait);
    }
    next.run(request).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn routes_fall_into_their_classes() {
        let l = Limits::default();
        let same = |a: Option<&Limiter>, b: &Limiter| a.is_some_and(|a| std::ptr::eq(a, b));
        assert!(same(
            class(&l, &Method::GET, "/v1/auth/twitch/login"),
            &l.auth
        ));
        assert!(same(class(&l, &Method::POST, "/v1/auth/logout"), &l.auth));
        assert!(same(class(&l, &Method::POST, "/v1/gql"), &l.gql));
        assert!(same(class(&l, &Method::GET, "/v1/gql/ws"), &l.socket));
        assert!(same(class(&l, &Method::POST, "/v1/emotes"), &l.upload));
        assert!(same(class(&l, &Method::GET, "/v1/sets/global"), &l.reads));
        assert!(same(
            class(&l, &Method::POST, "/v1/personal-sets/lookup"),
            &l.reads
        ));
        assert!(class(&l, &Method::GET, "/health").is_none());
    }

    #[test]
    fn the_answer_says_when_to_retry() {
        let r = too_many(std::time::Duration::from_millis(1500));
        assert_eq!(r.status(), StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(r.headers()[RETRY_AFTER], "2");
    }
}
