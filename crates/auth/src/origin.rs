//! CSRF by Origin (ADR 0009): a write that relies on the session cookie must come from the site.
//! Bearer callers are exempt, since a browser never attaches a Bearer token on its own.

use axum::http::header::ORIGIN;
use axum::http::{HeaderMap, Method};

use crate::config::AuthConfig;

/// Methods that change nothing and need no Origin check.
pub fn is_safe(method: &Method) -> bool {
    matches!(*method, Method::GET | Method::HEAD | Method::OPTIONS)
}

/// Whether the request's `Origin` is exactly the site's. A missing `Origin` fails: browsers send
/// it on every cross-origin request and on same-origin writes.
pub fn is_allowed(cfg: &AuthConfig, headers: &HeaderMap) -> bool {
    headers
        .get(ORIGIN)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|origin| origin == cfg.site_origin())
}

#[cfg(test)]
mod tests {
    use axum::http::HeaderValue;

    use super::*;

    fn with_origin(origin: Option<&'static str>) -> HeaderMap {
        let mut headers = HeaderMap::new();
        if let Some(origin) = origin {
            headers.insert(ORIGIN, HeaderValue::from_static(origin));
        }
        headers
    }

    #[test]
    fn only_the_site_origin_passes() {
        let cfg = AuthConfig::from_source(|_| None).unwrap();
        assert!(is_allowed(&cfg, &with_origin(Some("https://cyex.tv"))));
        for bad in [
            None,
            Some("null"),
            Some("https://cyex.tv/"),
            Some("http://cyex.tv"),
            Some("https://api.cyex.tv"),
            Some("https://cyex.tv.evil.example"),
            Some("https://evil.example"),
        ] {
            assert!(!is_allowed(&cfg, &with_origin(bad)), "{bad:?}");
        }
    }

    #[test]
    fn reads_are_safe() {
        assert!(is_safe(&Method::GET));
        assert!(is_safe(&Method::HEAD));
        assert!(!is_safe(&Method::POST));
        assert!(!is_safe(&Method::DELETE));
    }
}
