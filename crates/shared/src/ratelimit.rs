//! Rate limits (PLAN.md Phase 7): a token bucket per key (an IP or an account), in memory.
//!
//! In memory means per instance. v1 runs one api and one events instance (Phase 9); with more,
//! the limits would apply per instance, and should move to a shared store.

use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::sync::{Mutex, PoisonError};
use std::time::{Duration, Instant};

use axum::http::HeaderMap;

/// `burst` requests at once, refilled evenly over `per`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rate {
    pub burst: u32,
    pub per: Duration,
}

impl Rate {
    pub const fn per_minute(n: u32) -> Self {
        Self {
            burst: n,
            per: Duration::from_secs(60),
        }
    }

    pub const fn per_hour(n: u32) -> Self {
        Self {
            burst: n,
            per: Duration::from_secs(3600),
        }
    }
}

struct Bucket {
    tokens: f64,
    at: Instant,
}

/// One limit over many keys.
pub struct Limiter {
    rate: Rate,
    buckets: Mutex<HashMap<String, Bucket>>,
}

/// How many keys are kept before full (idle) buckets are swept out.
const SWEEP_AT: usize = 10_000;

impl Limiter {
    pub fn new(rate: Rate) -> Self {
        Self {
            rate,
            buckets: Mutex::default(),
        }
    }

    fn per_token(&self) -> f64 {
        self.rate.per.as_secs_f64() / f64::from(self.rate.burst.max(1))
    }

    /// Takes one token for `key`. `Err` holds how long until one is free.
    pub fn check(&self, key: &str) -> Result<(), Duration> {
        let now = Instant::now();
        let burst = f64::from(self.rate.burst);
        let per_token = self.per_token();
        let mut buckets = self.buckets.lock().unwrap_or_else(PoisonError::into_inner);
        if buckets.len() >= SWEEP_AT {
            buckets.retain(|_, b| {
                b.tokens + now.duration_since(b.at).as_secs_f64() / per_token < burst
            });
        }
        let bucket = buckets.entry(key.to_owned()).or_insert(Bucket {
            tokens: burst,
            at: now,
        });
        let refill = now.duration_since(bucket.at).as_secs_f64() / per_token;
        bucket.tokens = (bucket.tokens + refill).min(burst);
        bucket.at = now;
        if bucket.tokens >= 1.0 {
            bucket.tokens -= 1.0;
            Ok(())
        } else {
            Err(Duration::from_secs_f64((1.0 - bucket.tokens) * per_token))
        }
    }
}

/// The peer address when the server was started with connection info (`main` does that; tests
/// calling the router directly have none).
pub struct Peer(pub Option<SocketAddr>);

impl<S: Send + Sync> axum::extract::FromRequestParts<S> for Peer {
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        Ok(Self(
            parts
                .extensions
                .get::<axum::extract::ConnectInfo<SocketAddr>>()
                .map(|c| c.0),
        ))
    }
}

/// The client's IP. Behind Cloudflare (`trust_cloudflare`), every connection comes from the
/// tunnel, so the client is in `CF-Connecting-IP`; anywhere else that header could be forged and
/// only the peer address counts.
pub fn client_ip(headers: &HeaderMap, peer: Option<SocketAddr>, trust_cloudflare: bool) -> String {
    if trust_cloudflare
        && let Some(ip) = headers
            .get("cf-connecting-ip")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.trim().parse::<IpAddr>().ok())
    {
        return ip.to_string();
    }
    peer.map_or_else(|| "unknown".to_owned(), |p| p.ip().to_string())
}

#[cfg(test)]
mod tests {
    use axum::http::HeaderValue;

    use super::*;

    #[test]
    fn a_bucket_allows_its_burst_then_refills() {
        let l = Limiter::new(Rate {
            burst: 3,
            per: Duration::from_millis(300),
        });
        for _ in 0..3 {
            assert!(l.check("a").is_ok());
        }
        let wait = l.check("a").unwrap_err();
        assert!(wait <= Duration::from_millis(100), "{wait:?}");
        // Keys are separate.
        assert!(l.check("b").is_ok());
        std::thread::sleep(Duration::from_millis(120));
        assert!(l.check("a").is_ok());
    }

    #[test]
    fn the_client_ip_comes_from_cloudflare_only_when_trusted() {
        let mut headers = HeaderMap::new();
        headers.insert("cf-connecting-ip", HeaderValue::from_static("203.0.113.7"));
        let peer: SocketAddr = "10.0.0.2:5555".parse().unwrap();
        assert_eq!(client_ip(&headers, Some(peer), true), "203.0.113.7");
        assert_eq!(client_ip(&headers, Some(peer), false), "10.0.0.2");
        headers.insert("cf-connecting-ip", HeaderValue::from_static("not an ip"));
        assert_eq!(client_ip(&headers, Some(peer), true), "10.0.0.2");
        assert_eq!(client_ip(&HeaderMap::new(), None, true), "unknown");
    }
}
