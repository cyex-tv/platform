//! Rate limits: a token bucket per key (an IP or an account), in memory.
//!
//! In memory means per instance. v1 runs one api and one events instance; with more,
//! the limits would apply per instance, and should move to a shared store.

use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::sync::{Arc, Mutex, PoisonError};
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

    /// Reads `<count>/<window>`: `600/1m`, `30/1h`, `5/30s`, `100/1d` (`600/m` is `600/1m`).
    /// The count is 1 to 1,000,000; the window 1 second to 7 days.
    pub fn parse(raw: &str) -> Result<Self, String> {
        let bad = || format!("{raw:?} isn't a rate like 600/1m, 30/1h or 5/30s");
        let (count, window) = raw.trim().split_once('/').ok_or_else(bad)?;
        let burst: u32 = count.trim().parse().map_err(|_| bad())?;
        if !(1..=1_000_000).contains(&burst) {
            return Err(format!("{raw:?}: the count must be 1 to 1000000"));
        }
        let window = window.trim();
        let unit_at = window.find(|c: char| !c.is_ascii_digit()).ok_or_else(bad)?;
        let (n, unit) = window.split_at(unit_at);
        let n: u64 = if n.is_empty() {
            1
        } else {
            n.parse().map_err(|_| bad())?
        };
        let secs = match unit {
            "s" => n,
            "m" => n.saturating_mul(60),
            "h" => n.saturating_mul(3600),
            "d" => n.saturating_mul(86_400),
            _ => return Err(bad()),
        };
        if !(1..=7 * 86_400).contains(&secs) {
            return Err(format!("{raw:?}: the window must be 1s to 7d"));
        }
        Ok(Self {
            burst,
            per: Duration::from_secs(secs),
        })
    }
}

impl std::fmt::Display for Rate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let secs = self.per.as_secs();
        let (n, unit) = if secs.is_multiple_of(86_400) {
            (secs / 86_400, "d")
        } else if secs.is_multiple_of(3600) {
            (secs / 3600, "h")
        } else if secs.is_multiple_of(60) {
            (secs / 60, "m")
        } else {
            (secs, "s")
        };
        write!(f, "{}/{n}{unit}", self.burst)
    }
}

/// What a limiter has left for a key after a check, for the `RateLimit-*` headers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Quota {
    /// The bucket's size.
    pub limit: u32,
    /// Whole requests left right now.
    pub remaining: u32,
    /// Seconds until the bucket is full again (at least 1 unless it already is).
    pub reset: u64,
}

fn secs_up(d: Duration) -> u64 {
    d.as_secs() + u64::from(d.subsec_nanos() > 0)
}

struct Bucket {
    tokens: f64,
    at: Instant,
    /// The rate this key last counted under (keys may have their own, `check_with`).
    rate: Rate,
}

fn per_token(rate: Rate) -> f64 {
    rate.per.as_secs_f64() / f64::from(rate.burst.max(1))
}

/// One limit over many keys. Every key counts under the limiter's rate, or under its own when
/// checked with `check_with` (an account whose roles raise or lower the limit).
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

    /// The rate keys count under unless checked with their own.
    pub fn rate(&self) -> Rate {
        self.rate
    }

    /// Takes one token for `key`. `Err` holds how long until one is free.
    pub fn check(&self, key: &str) -> Result<(), Duration> {
        self.check_at(key, self.rate, Instant::now()).map(|_| ())
    }

    /// `check`, and what is left afterwards. On `Err`, the quota is empty and `reset` is the wait.
    pub fn check_quota(&self, key: &str) -> Result<Quota, (Duration, Quota)> {
        self.quota_at(key, self.rate, Instant::now())
    }

    /// `check_quota` at `rate`, for a key with its own (an account whose roles change the limit).
    pub fn check_quota_with(&self, key: &str, rate: Rate) -> Result<Quota, (Duration, Quota)> {
        self.quota_at(key, rate, Instant::now())
    }

    fn quota_at(&self, key: &str, rate: Rate, now: Instant) -> Result<Quota, (Duration, Quota)> {
        let burst = rate.burst;
        match self.check_at(key, rate, now) {
            Ok(tokens) => {
                let missing = (f64::from(burst) - tokens).max(0.0) * per_token(rate);
                Ok(Quota {
                    limit: burst,
                    remaining: tokens.floor() as u32,
                    reset: secs_up(Duration::from_secs_f64(missing)),
                })
            }
            Err(wait) => Err((
                wait,
                Quota {
                    limit: burst,
                    remaining: 0,
                    reset: secs_up(wait).max(1),
                },
            )),
        }
    }

    /// Takes one token for `key`, whose bucket holds `rate.burst` and refills over `rate.per`.
    /// A key whose rate changes keeps what it has used, capped at the new burst.
    pub fn check_with(&self, key: &str, rate: Rate) -> Result<(), Duration> {
        self.check_at(key, rate, Instant::now()).map(|_| ())
    }

    /// `check_with` with the clock passed in, so tests can advance time without sleeping.
    /// `Ok` holds the tokens left.
    fn check_at(&self, key: &str, rate: Rate, now: Instant) -> Result<f64, Duration> {
        let burst = f64::from(rate.burst);
        let per_token = per_token(rate);
        let mut buckets = self.buckets.lock().unwrap_or_else(PoisonError::into_inner);
        if buckets.len() >= SWEEP_AT {
            buckets.retain(|_, b| {
                b.tokens + now.duration_since(b.at).as_secs_f64() / self::per_token(b.rate)
                    < f64::from(b.rate.burst)
            });
        }
        let bucket = buckets.entry(key.to_owned()).or_insert(Bucket {
            tokens: burst,
            at: now,
            rate,
        });
        bucket.rate = rate;
        let refill = now.duration_since(bucket.at).as_secs_f64() / per_token;
        bucket.tokens = (bucket.tokens + refill).min(burst);
        bucket.at = now;
        if bucket.tokens >= 1.0 {
            bucket.tokens -= 1.0;
            Ok(bucket.tokens)
        } else {
            Err(Duration::from_secs_f64((1.0 - bucket.tokens) * per_token))
        }
    }
}

/// Sockets one IP may hold open at once. Generous, since many people can share one IP behind
/// carrier-grade NAT; it only stops one client from holding thousands of idle sockets.
pub const SOCKETS_PER_IP: u32 = 50;

/// A cap on what one key may hold open at once (sockets per IP). `try_take` hands out a guard
/// that gives its place back when dropped.
pub struct OpenCap {
    max: u32,
    open: Arc<Mutex<HashMap<String, u32>>>,
}

impl OpenCap {
    pub fn new(max: u32) -> Self {
        Self {
            max,
            open: Arc::default(),
        }
    }

    /// The most one key may hold.
    pub fn max(&self) -> u32 {
        self.max
    }

    /// A place for `key`, or `None` when it holds the most it may.
    pub fn try_take(&self, key: &str) -> Option<OpenSlot> {
        let mut open = self.open.lock().unwrap_or_else(PoisonError::into_inner);
        let count = open.entry(key.to_owned()).or_insert(0);
        if *count >= self.max {
            return None;
        }
        *count += 1;
        Some(OpenSlot {
            key: key.to_owned(),
            open: self.open.clone(),
        })
    }
}

/// One place taken from an [`OpenCap`], given back on drop.
pub struct OpenSlot {
    key: String,
    open: Arc<Mutex<HashMap<String, u32>>>,
}

impl Drop for OpenSlot {
    fn drop(&mut self) {
        let mut open = self.open.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(count) = open.get_mut(&self.key) {
            *count -= 1;
            if *count == 0 {
                open.remove(&self.key);
            }
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
    fn quota_counts_down_and_says_when_it_resets() {
        let l = Limiter::new(Rate::per_minute(3));
        let q = l.check_quota("a").unwrap();
        assert_eq!((q.limit, q.remaining, q.reset), (3, 2, 20));
        assert_eq!(l.check_quota("a").unwrap().remaining, 1);
        assert_eq!(l.check_quota("a").unwrap().remaining, 0);
        let (wait, q) = l.check_quota("a").unwrap_err();
        assert_eq!((q.limit, q.remaining), (3, 0));
        assert!(q.reset >= 1 && q.reset <= 20 && q.reset == secs_up(wait));
    }

    #[test]
    fn a_bucket_allows_its_burst_then_refills() {
        let l = Limiter::new(Rate {
            burst: 3,
            per: Duration::from_millis(300),
        });
        let r = l.rate();
        let t0 = Instant::now();
        for _ in 0..3 {
            assert!(l.check_at("a", r, t0).is_ok());
        }
        let wait = l.check_at("a", r, t0).unwrap_err();
        assert_eq!(wait, Duration::from_millis(100));
        // Keys are separate.
        assert!(l.check_at("b", r, t0).is_ok());
        // Not yet refilled a whole token.
        assert!(l.check_at("a", r, t0 + Duration::from_millis(90)).is_err());
        assert!(l.check_at("a", r, t0 + Duration::from_millis(100)).is_ok());
        assert!(l.check_at("a", r, t0 + Duration::from_millis(100)).is_err());
    }

    #[test]
    fn a_key_can_count_under_its_own_rate() {
        let l = Limiter::new(Rate::per_minute(2));
        let t0 = Instant::now();
        let more = Rate::per_minute(5);
        for _ in 0..5 {
            assert!(l.check_at("raised", more, t0).is_ok());
        }
        assert!(l.check_at("raised", more, t0).is_err());
        for _ in 0..2 {
            assert!(l.check_at("plain", l.rate(), t0).is_ok());
        }
        assert!(l.check_at("plain", l.rate(), t0).is_err());
        // Lowered: one a minute, and the bucket refills at that pace.
        let less = Rate::per_minute(1);
        assert!(l.check_at("lowered", less, t0).is_ok());
        let wait = l.check_at("lowered", less, t0).unwrap_err();
        assert_eq!(wait, Duration::from_secs(60));
    }

    #[test]
    fn rates_parse_from_text() {
        assert_eq!(Rate::parse("600/1m"), Ok(Rate::per_minute(600)));
        assert_eq!(Rate::parse(" 30 / 1h "), Ok(Rate::per_hour(30)));
        assert_eq!(Rate::parse("600/m"), Ok(Rate::per_minute(600)));
        assert_eq!(
            Rate::parse("5/30s"),
            Ok(Rate {
                burst: 5,
                per: Duration::from_secs(30)
            })
        );
        assert_eq!(
            Rate::parse("100/2h").unwrap().per,
            Duration::from_secs(7200)
        );
        assert_eq!(
            Rate::parse("1/1d").unwrap().per,
            Duration::from_secs(86_400)
        );
        for bad in [
            "",
            "600",
            "600/",
            "/1m",
            "x/1m",
            "600/1x",
            "600/1",
            "0/1m",
            "-1/1m",
            "600/0s",
            "600/8d",
            "2000000/1m",
            "1.5/1m",
        ] {
            assert!(Rate::parse(bad).is_err(), "{bad:?}");
        }
        assert_eq!(Rate::per_minute(600).to_string(), "600/1m");
        assert_eq!(Rate::per_hour(30).to_string(), "30/1h");
        assert_eq!(Rate::parse("5/90s").unwrap().to_string(), "5/90s");
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

    #[test]
    fn open_cap_refuses_past_the_max_and_frees_on_drop() {
        let cap = OpenCap::new(2);
        let a = cap.try_take("1.2.3.4").unwrap();
        let _b = cap.try_take("1.2.3.4").unwrap();
        assert!(cap.try_take("1.2.3.4").is_none());
        assert!(cap.try_take("5.6.7.8").is_some());
        drop(a);
        assert!(cap.try_take("1.2.3.4").is_some());
    }
}
