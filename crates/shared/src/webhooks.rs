//! Outbound webhooks (ADR 0034): which event types a webhook may ask for, which URLs it may point
//! at, which addresses a delivery may connect to, and how a delivery is signed.
//!
//! The address checks are the SSRF guard. The api checks a URL when it is saved (`check_url`),
//! and the worker checks every address the host resolves to right before it connects
//! (`is_public_ip`), connects only to those addresses, and never follows a redirect.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

use hmac::{Hmac, Mac};
use sha2::Sha256;

/// The event types a channel webhook may subscribe to: the channel's set and emote events, and the
/// cosmetics catalog's (ADR 0005, 0015).
pub const EVENT_TYPES: &[&str] = &[
    "set.emote_added",
    "set.emote_removed",
    "set.emote_renamed",
    "channel.active_set_changed",
    "emote.updated",
    "cosmetic.created",
    "cosmetic.updated",
    "cosmetic.deleted",
];

/// The event type of a delivery sent by "Send test".
pub const TEST_EVENT: &str = "webhook.test";

/// The header that carries the signature.
pub const SIGNATURE_HEADER: &str = "X-Cyex-Signature";

/// The longest URL a webhook may have.
pub const URL_MAX: usize = 2000;

/// The only port deliveries go to: 443, the https default. Webhook URLs can't name another, so a
/// URL can't aim at a service on another port of a public host.
pub const PORT: u16 = 443;

/// Whether `event_type` is one a webhook may subscribe to.
pub fn is_event_type(event_type: &str) -> bool {
    EVENT_TYPES.contains(&event_type)
}

/// Why a URL can't be a webhook's.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UrlProblem {
    TooLong,
    NotAUrl,
    NotHttps,
    Credentials,
    PrivateHost,
    Port,
}

impl UrlProblem {
    pub fn message(self) -> &'static str {
        match self {
            Self::TooLong => "That URL is too long (2000 characters at most).",
            Self::NotAUrl => "That isn't a URL.",
            Self::NotHttps => "Webhook URLs must start with https://.",
            Self::Credentials => "Leave the user name and password out of the URL.",
            Self::PrivateHost => "That URL points at a private or local address.",
            Self::Port => "Webhook URLs use the https port, 443. Leave the port out.",
        }
    }
}

/// Host names that never lead anywhere public.
fn is_private_name(host: &str) -> bool {
    let host = host.trim_end_matches('.').to_ascii_lowercase();
    host == "localhost"
        || !host.contains('.')
        || [
            ".localhost",
            ".local",
            ".internal",
            ".lan",
            ".home.arpa",
            ".localdomain",
        ]
        .iter()
        .any(|suffix| host.ends_with(suffix))
}

/// Checks a URL a webhook is saved with: https, no credentials, a public-looking host (an IP
/// literal must be public). Returns it normalised, without a fragment. DNS is checked again at
/// every delivery, since names can change what they point at.
pub fn check_url(raw: &str) -> Result<url::Url, UrlProblem> {
    let raw = raw.trim();
    if raw.len() > URL_MAX {
        return Err(UrlProblem::TooLong);
    }
    let mut url = url::Url::parse(raw).map_err(|_| UrlProblem::NotAUrl)?;
    if url.scheme() != "https" {
        return Err(UrlProblem::NotHttps);
    }
    if !url.username().is_empty() || url.password().is_some() {
        return Err(UrlProblem::Credentials);
    }
    match url.host() {
        None => return Err(UrlProblem::NotAUrl),
        Some(url::Host::Ipv4(ip)) if !is_public_ip(IpAddr::V4(ip)) => {
            return Err(UrlProblem::PrivateHost);
        }
        Some(url::Host::Ipv6(ip)) if !is_public_ip(IpAddr::V6(ip)) => {
            return Err(UrlProblem::PrivateHost);
        }
        Some(url::Host::Domain(name)) if is_private_name(name) => {
            return Err(UrlProblem::PrivateHost);
        }
        Some(_) => {}
    }
    if url.port_or_known_default() != Some(PORT) {
        return Err(UrlProblem::Port);
    }
    url.set_fragment(None);
    if url.as_str().len() > URL_MAX {
        return Err(UrlProblem::TooLong);
    }
    Ok(url)
}

/// Whether a delivery may connect to `ip`: only globally routable unicast addresses. Private,
/// loopback, link-local (the cloud metadata address 169.254.169.254 among them), carrier-grade
/// NAT, documentation, benchmarking, multicast and reserved ranges are refused. IPv6 addresses
/// that carry an IPv4 address (mapped, compatible, NAT64, 6to4, Teredo) are refused, or judged by
/// the IPv4 address they carry, so a private one can't hide inside.
pub fn is_public_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => is_public_v4(v4),
        IpAddr::V6(v6) => is_public_v6(v6),
    }
}

fn is_public_v4(ip: Ipv4Addr) -> bool {
    let [a, b, c, _] = ip.octets();
    let blocked = a == 0 // "this network"
        || a == 10 // private
        || (a == 100 && (64..128).contains(&b)) // carrier-grade NAT
        || a == 127 // loopback
        || (a == 169 && b == 254) // link-local, cloud metadata
        || (a == 172 && (16..32).contains(&b)) // private
        || (a == 192 && b == 0 && c == 0) // IETF protocol assignments
        || (a == 192 && b == 0 && c == 2) // TEST-NET-1
        || (a == 192 && b == 88 && c == 99) // 6to4 relay anycast
        || (a == 192 && b == 168) // private
        || (a == 198 && (b == 18 || b == 19)) // benchmarking
        || (a == 198 && b == 51 && c == 100) // TEST-NET-2
        || (a == 203 && b == 0 && c == 113) // TEST-NET-3
        || a >= 224; // multicast, reserved, broadcast
    !blocked
}

fn is_public_v6(ip: Ipv6Addr) -> bool {
    let s = ip.segments();
    // IPv4-mapped (::ffff:a.b.c.d): judged as the IPv4 address it is.
    if let Some(v4) = ip.to_ipv4_mapped() {
        return is_public_v4(v4);
    }
    // Unspecified, loopback and the deprecated IPv4-compatible form (::a.b.c.d).
    if s[..6] == [0, 0, 0, 0, 0, 0] {
        return false;
    }
    // IPv4/IPv4 translation (::ffff:0:a.b.c.d, RFC 2765).
    if s[..4] == [0, 0, 0, 0] && s[4] == 0xffff && s[5] == 0 {
        return false;
    }
    // Only global unicast (2000::/3) is ever public.
    if s[0] & 0xe000 != 0x2000 {
        // Covers ULA fc00::/7 (fd00:ec2::254, AWS metadata, too), link-local fe80::/10,
        // site-local fec0::/10, multicast ff00::/8, NAT64 64:ff9b::/96 and discard 100::/64.
        return false;
    }
    let blocked = (s[0] == 0x2001 && s[1] == 0x0db8) // documentation
        || (s[0] == 0x2001 && s[1] == 0) // Teredo: carries an IPv4 address
        || (s[0] == 0x2001 && (s[1] & 0xfff0) == 0x0010) // ORCHID
        || (s[0] == 0x2001 && (s[1] & 0xfff0) == 0x0020) // ORCHIDv2
        || s[0] == 0x2002 // 6to4: carries an IPv4 address
        || (s[0] == 0x3fff && s[1] < 0x1000); // documentation (RFC 9637)
    !blocked
}

/// The `X-Cyex-Signature` value for `body` sent at `timestamp` (Unix seconds):
/// `t=<timestamp>,v1=<hex HMAC-SHA256 of "<timestamp>.<body>" under the secret>`.
pub fn signature(secret: &str, timestamp: i64, body: &[u8]) -> String {
    format!("t={timestamp},v1={}", sign_hex(secret, timestamp, body))
}

/// The header for a delivery signed with every secret in `secrets` (the current one first, and
/// the one before a rotation while it still signs): `t=<timestamp>,v1=<sig>,v1=<sig>`. A receiver
/// accepts the delivery when any `v1` matches its secret.
pub fn signature_all(secrets: &[&str], timestamp: i64, body: &[u8]) -> String {
    let mut out = format!("t={timestamp}");
    for secret in secrets {
        out.push_str(",v1=");
        out.push_str(&sign_hex(secret, timestamp, body));
    }
    out
}

/// A URL as people who didn't make the webhook see it: its host, and `/…` when it has a path or
/// query, which can hold a token (chat-service webhooks do).
pub fn redacted(url: &str) -> String {
    match url::Url::parse(url) {
        Ok(u) => {
            let host = u.host_str().unwrap_or_default();
            let more = u.path() != "/" || u.query().is_some();
            format!("{}://{host}{}", u.scheme(), if more { "/…" } else { "/" })
        }
        Err(_) => "…".to_owned(),
    }
}

fn sign_hex(secret: &str, timestamp: i64, body: &[u8]) -> String {
    // HMAC takes a key of any length.
    let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).expect("any key length");
    mac.update(timestamp.to_string().as_bytes());
    mac.update(b".");
    mac.update(body);
    hex(&mac.finalize().into_bytes())
}

fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        out.push(DIGITS[usize::from(b >> 4)] as char);
        out.push(DIGITS[usize::from(b & 15)] as char);
    }
    out
}

/// How a receiver checks a delivery: the signature matches and is at most `tolerance` seconds
/// from `now`. The docs show the same steps in other languages.
pub fn verify(secret: &str, header: &str, body: &[u8], now: i64, tolerance: i64) -> bool {
    let mut timestamp = None;
    let mut given = Vec::new();
    for part in header.split(',') {
        match part.trim().split_once('=') {
            Some(("t", t)) => timestamp = t.parse::<i64>().ok(),
            Some(("v1", v)) => given.push(v.to_owned()),
            _ => {}
        }
    }
    let Some(t) = timestamp else { return false };
    if (now - t).abs() > tolerance {
        return false;
    }
    let expected = sign_hex(secret, t, body);
    given
        .iter()
        .any(|g| constant_eq(g.as_bytes(), expected.as_bytes()))
}

fn constant_eq(a: &[u8], b: &[u8]) -> bool {
    a.len() == b.len() && a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v4(s: &str) -> bool {
        is_public_ip(s.parse().unwrap())
    }

    #[test]
    fn private_and_special_ipv4_is_refused() {
        for ip in [
            "0.0.0.0",
            "0.1.2.3",
            "10.0.0.1",
            "10.255.255.255",
            "100.64.0.1",
            "100.127.255.254",
            "127.0.0.1",
            "127.1.2.3",
            "169.254.169.254",
            "169.254.0.1",
            "172.16.0.1",
            "172.31.255.255",
            "192.0.0.8",
            "192.0.2.1",
            "192.88.99.1",
            "192.168.1.1",
            "198.18.0.1",
            "198.19.255.255",
            "198.51.100.7",
            "203.0.113.9",
            "224.0.0.1",
            "239.255.255.250",
            "240.0.0.1",
            "255.255.255.255",
        ] {
            assert!(!v4(ip), "{ip} must be refused");
        }
    }

    #[test]
    fn public_ipv4_is_allowed() {
        for ip in [
            "1.1.1.1",
            "8.8.8.8",
            "100.63.255.255",
            "100.128.0.1",
            "172.15.255.255",
            "172.32.0.1",
            "192.169.0.1",
            "198.20.0.1",
            "93.184.216.34",
        ] {
            assert!(v4(ip), "{ip} is public");
        }
    }

    #[test]
    fn private_and_special_ipv6_is_refused() {
        for ip in [
            "::",
            "::1",
            "::ffff:127.0.0.1",
            "::ffff:10.0.0.1",
            "::ffff:169.254.169.254",
            "::ffff:192.168.0.1",
            "::127.0.0.1",
            "::8.8.8.8",
            "::ffff:0:10.0.0.1",
            "64:ff9b::a00:1",
            "64:ff9b::808:808",
            "100::1",
            "fc00::1",
            "fd00:ec2::254",
            "fe80::1",
            "fe80::1%1".split('%').next().unwrap(),
            "fec0::1",
            "ff02::1",
            "2001:db8::1",
            "2001:0:4136:e378:8000:63bf:3fff:fdd2",
            "2001:10::1",
            "2001:20::1",
            "2002:a00:1::1",
            "2002:c0a8:101::1",
            "3fff::1",
        ] {
            assert!(!v4(ip), "{ip} must be refused");
        }
    }

    #[test]
    fn public_ipv6_is_allowed() {
        for ip in [
            "2606:4700:4700::1111",
            "2001:4860:4860::8888",
            "2a00:1450:4001:80b::200e",
            "::ffff:8.8.8.8",
        ] {
            assert!(v4(ip), "{ip} is public");
        }
    }

    #[test]
    fn urls_are_checked_when_saved() {
        assert_eq!(
            check_url(" https://hooks.example.com/cyex#x ")
                .unwrap()
                .as_str(),
            "https://hooks.example.com/cyex"
        );
        assert!(check_url("https://203.0.114.1:443/hook").is_ok());
        assert_eq!(
            check_url("https://hooks.example.com:8443/hook").unwrap_err(),
            UrlProblem::Port
        );
        assert_eq!(
            check_url("https://hooks.example.com:22/").unwrap_err(),
            UrlProblem::Port
        );
        assert!(check_url("https://[2606:4700:4700::1111]/hook").is_ok());
        for (raw, problem) in [
            ("http://hooks.example.com/", UrlProblem::NotHttps),
            ("ftp://hooks.example.com/", UrlProblem::NotHttps),
            ("hooks.example.com", UrlProblem::NotAUrl),
            ("https://", UrlProblem::NotAUrl),
            (
                "https://user:pw@hooks.example.com/",
                UrlProblem::Credentials,
            ),
            ("https://localhost/hook", UrlProblem::PrivateHost),
            ("https://api.localhost/hook", UrlProblem::PrivateHost),
            ("https://intranet/hook", UrlProblem::PrivateHost),
            ("https://printer.local/", UrlProblem::PrivateHost),
            ("https://metadata.google.internal/", UrlProblem::PrivateHost),
            ("https://127.0.0.1/", UrlProblem::PrivateHost),
            (
                "https://169.254.169.254/latest/meta-data",
                UrlProblem::PrivateHost,
            ),
            ("https://0x7f000001/", UrlProblem::PrivateHost),
            ("https://2130706433/", UrlProblem::PrivateHost),
            ("https://[::1]/", UrlProblem::PrivateHost),
            ("https://[::ffff:127.0.0.1]/", UrlProblem::PrivateHost),
            ("https://[fd00::1]/", UrlProblem::PrivateHost),
        ] {
            assert_eq!(check_url(raw).unwrap_err(), problem, "{raw}");
        }
        let long = format!("https://example.com/{}", "a".repeat(URL_MAX));
        assert_eq!(check_url(&long).unwrap_err(), UrlProblem::TooLong);
    }

    #[test]
    fn deliveries_are_signed_and_verify() {
        let body = br#"{"event":{"type":"webhook.test"}}"#;
        let sig = signature("cyexhook_test_not_a_real_secret", 1_764_000_000, body);
        // Known answer, so other languages can check their code against it (the docs show it).
        assert_eq!(
            sig,
            "t=1764000000,v1=b21cbc9c5f20a666e719401e1763ca92d8bd0a2177157ef4f4abf3ed4bb90025"
        );
        assert!(sig.starts_with("t=1764000000,v1="));
        assert_eq!(sig.len(), "t=1764000000,v1=".len() + 64);
        assert!(verify(
            "cyexhook_test_not_a_real_secret",
            &sig,
            body,
            1_764_000_100,
            300
        ));
        // Too old, wrong secret, changed body, junk.
        assert!(!verify(
            "cyexhook_test_not_a_real_secret",
            &sig,
            body,
            1_764_000_301,
            300
        ));
        assert!(!verify(
            "cyexhook_test_other",
            &sig,
            body,
            1_764_000_000,
            300
        ));
        assert!(!verify(
            "cyexhook_test_not_a_real_secret",
            &sig,
            b"{}",
            1_764_000_000,
            300
        ));
        assert!(!verify(
            "cyexhook_test_not_a_real_secret",
            "v1=abc",
            body,
            1_764_000_000,
            300
        ));
        // The same input always signs the same; another timestamp signs differently.
        assert_eq!(
            sig,
            signature("cyexhook_test_not_a_real_secret", 1_764_000_000, body)
        );
        assert_ne!(
            sig,
            signature("cyexhook_test_not_a_real_secret", 1_764_000_001, body)
        );
    }

    #[test]
    fn a_rotation_signs_with_both_secrets() {
        let body = b"{}";
        let sig = signature_all(&["cyexhook_test_new", "cyexhook_test_old"], 10, body);
        assert_eq!(sig.matches("v1=").count(), 2);
        assert!(sig.starts_with(&signature("cyexhook_test_new", 10, body)));
        assert!(verify("cyexhook_test_new", &sig, body, 10, 300));
        assert!(verify("cyexhook_test_old", &sig, body, 10, 300));
        assert!(!verify("cyexhook_test_other", &sig, body, 10, 300));
        assert_eq!(
            signature_all(&["cyexhook_test_new"], 10, body),
            signature("cyexhook_test_new", 10, body)
        );
    }

    #[test]
    fn others_see_only_the_host() {
        assert_eq!(
            redacted("https://chat.example.com/api/webhooks/123/s3cr3t-token"),
            "https://chat.example.com/…"
        );
        assert_eq!(
            redacted("https://hooks.example.com/?token=x"),
            "https://hooks.example.com/…"
        );
        assert_eq!(
            redacted("https://hooks.example.com/"),
            "https://hooks.example.com/"
        );
        assert!(!redacted("https://a.example/x?k=secret").contains("secret"));
    }

    #[test]
    fn a_known_signature() {
        // HMAC-SHA256(key "key", "1.{}"), as any HMAC library computes it.
        assert_eq!(
            signature("key", 1, b"{}"),
            "t=1,v1=1ba6b8171186efc613e8bcc0cbdab2748f24984d7c5a84faa2637afa0e40d224"
        );
        assert!(is_event_type("set.emote_added") && !is_event_type("personal.updated"));
    }
}
