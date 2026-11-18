//! Request rate limits (PLAN.md Phase 7), per IP for every route class and per account on
//! GraphQL. A request over a limit gets `429 rate_limited` with `Retry-After`.
//!
//! The upload's 20 an hour per account lives in `upload`, since it counts what was made. Editor
//! changes are checked per account in `editors`, cosmetics writes in `cosmetics`, set clones in
//! `catalog`.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Mutex, PoisonError};
use std::time::{Duration, Instant};

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
    /// Adding, changing and removing account editors, per account (ADR 0014).
    pub editors: Rate,
    /// Cosmetics writes (staff changes, grants, selections), per account (ADR 0015); staff shop
    /// and code writes count here too (ADR 0016).
    pub cosmetics: Rate,
    /// Starting a checkout, per account (ADR 0016).
    pub checkout: Rate,
    /// Redeem tries, right or wrong, per account (ADR 0016).
    pub redeem: Rate,
    /// `cloneSet`, per account: each copies up to a whole set.
    pub set_clones: Rate,
    /// `exportMyData`, per account: each reads everything about the account.
    pub data_export: Rate,
    /// `searchAccounts`, per account, or per IP when signed out.
    pub account_search: Rate,
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
            editors: Rate::per_hour(60),
            cosmetics: Rate::per_hour(300),
            checkout: Rate::per_hour(30),
            redeem: Rate::per_hour(30),
            set_clones: Rate::per_hour(30),
            data_export: Rate::per_hour(5),
            account_search: Rate::per_minute(60),
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
    pub editors: Limiter,
    pub cosmetics: Limiter,
    pub checkout: Limiter,
    pub redeem: Limiter,
    /// Wrong redeem codes lock an account and an IP out for a while (ADR 0016).
    pub redeem_lockout: RedeemLockout,
    pub set_clones: Limiter,
    pub data_export: Limiter,
    pub account_search: Limiter,
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
            editors: Limiter::new(rates.editors),
            cosmetics: Limiter::new(rates.cosmetics),
            checkout: Limiter::new(rates.checkout),
            redeem: Limiter::new(rates.redeem),
            redeem_lockout: RedeemLockout::default(),
            set_clones: Limiter::new(rates.set_clones),
            data_export: Limiter::new(rates.data_export),
            account_search: Limiter::new(rates.account_search),
            trust_cloudflare,
        }
    }
}

/// Counts failures per key; `max` of them inside `window` (from the first) lock the key until the
/// window ends. In memory, like the other limits.
pub struct Lockout {
    max: u32,
    window: Duration,
    entries: Mutex<HashMap<String, (u32, Instant)>>,
}

impl Lockout {
    pub fn new(max: u32, window: Duration) -> Self {
        Self {
            max,
            window,
            entries: Mutex::default(),
        }
    }

    /// How long `key` is still locked out, if it is.
    pub fn locked(&self, key: &str) -> Option<Duration> {
        let entries = self.entries.lock().unwrap_or_else(PoisonError::into_inner);
        let (count, since) = entries.get(key)?;
        let left = self.window.checked_sub(since.elapsed())?;
        (*count >= self.max).then_some(left)
    }

    pub fn fail(&self, key: &str) {
        let now = Instant::now();
        let mut entries = self.entries.lock().unwrap_or_else(PoisonError::into_inner);
        if entries.len() >= 10_000 {
            let window = self.window;
            entries.retain(|_, (_, since)| now.duration_since(*since) < window);
        }
        let entry = entries.entry(key.to_owned()).or_insert((0, now));
        if now.duration_since(entry.1) >= self.window {
            *entry = (0, now);
        }
        entry.0 += 1;
    }
}

/// Wrong redeem codes: 5 per account or 20 per IP within 15 minutes lock it out until the 15
/// minutes are over.
pub struct RedeemLockout {
    pub account: Lockout,
    pub ip: Lockout,
}

impl Default for RedeemLockout {
    fn default() -> Self {
        Self {
            account: Lockout::new(5, Duration::from_secs(15 * 60)),
            ip: Lockout::new(20, Duration::from_secs(15 * 60)),
        }
    }
}

impl Default for Limits {
    fn default() -> Self {
        Self::new(Rates::default(), false)
    }
}

/// The client's IP as the limits saw it, for limits applied inside GraphQL resolvers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClientIp(pub String);

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
        (&Method::POST, "/v1/emotes" | "/v1/cosmetics/images") => Some(&limits.upload),
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
    let mut request = request;
    request.extensions_mut().insert(ClientIp(ip.clone()));
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
        assert!(same(
            class(&l, &Method::POST, "/v1/cosmetics/images"),
            &l.upload
        ));
        assert!(same(
            class(&l, &Method::GET, "/v1/cosmetics/users"),
            &l.reads
        ));
        assert!(same(class(&l, &Method::GET, "/v1/sets/global"), &l.reads));
        assert!(same(
            class(&l, &Method::POST, "/v1/personal-sets/lookup"),
            &l.reads
        ));
        assert!(class(&l, &Method::GET, "/health").is_none());
    }

    #[test]
    fn lockouts_hold_for_the_window() {
        let l = Lockout::new(3, Duration::from_millis(200));
        for _ in 0..2 {
            l.fail("a");
        }
        assert!(l.locked("a").is_none());
        l.fail("a");
        assert!(l.locked("a").is_some());
        assert!(l.locked("b").is_none());
        std::thread::sleep(Duration::from_millis(220));
        assert!(l.locked("a").is_none());
        // A new window starts over.
        l.fail("a");
        assert!(l.locked("a").is_none());
    }

    #[test]
    fn the_answer_says_when_to_retry() {
        let r = too_many(std::time::Duration::from_millis(1500));
        assert_eq!(r.status(), StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(r.headers()[RETRY_AFTER], "2");
    }
}
