//! The two cookies: the site session (ADR 0009) and the short pre-login cookie holding `state`.

use axum::http::HeaderMap;
use axum::http::header::COOKIE;
use chrono::Duration;

use crate::config::AuthConfig;

/// The site session. `Domain=cyex.tv`, so the site and the API both receive it.
pub const SESSION: &str = "cyex_tv_session";
/// The sign-in in progress. Host-only on the API and only sent to `/v1/auth`.
pub const STATE: &str = "cyex_tv_oauth";

const STATE_PATH: &str = "/v1/auth";

/// `Set-Cookie` for a new session token.
pub fn session(cfg: &AuthConfig, token: &str, max_age: Duration) -> String {
    build(
        cfg,
        SESSION,
        token,
        "/",
        cfg.cookie_domain.as_deref(),
        max_age.num_seconds(),
    )
}

/// `Set-Cookie` that removes the session cookie.
pub fn clear_session(cfg: &AuthConfig) -> String {
    build(cfg, SESSION, "", "/", cfg.cookie_domain.as_deref(), 0)
}

/// `Set-Cookie` for the pre-login `state`.
pub fn state(cfg: &AuthConfig, state: &str, max_age: Duration) -> String {
    build(cfg, STATE, state, STATE_PATH, None, max_age.num_seconds())
}

pub fn clear_state(cfg: &AuthConfig) -> String {
    build(cfg, STATE, "", STATE_PATH, None, 0)
}

fn build(
    cfg: &AuthConfig,
    name: &str,
    value: &str,
    path: &str,
    domain: Option<&str>,
    max_age: i64,
) -> String {
    let mut out = format!("{name}={value}; Path={path}");
    if let Some(domain) = domain {
        out.push_str("; Domain=");
        out.push_str(domain);
    }
    out.push_str(&format!("; Max-Age={}", max_age.max(0)));
    if cfg.cookie_secure {
        out.push_str("; Secure");
    }
    out.push_str("; HttpOnly; SameSite=Lax");
    out
}

/// The value of cookie `name` from the request's `Cookie` headers. With duplicates, the first
/// non-empty one wins (browsers send the most specific path first).
pub fn read(headers: &HeaderMap, name: &str) -> Option<String> {
    headers
        .get_all(COOKIE)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .flat_map(|line| line.split(';'))
        .filter_map(|pair| pair.trim().split_once('='))
        .find(|(key, value)| *key == name && !value.is_empty())
        .map(|(_, value)| value.trim_matches('"').to_owned())
}

#[cfg(test)]
mod tests {
    use axum::http::HeaderValue;

    use super::*;

    fn cfg() -> AuthConfig {
        AuthConfig::from_source(|_| None).unwrap()
    }

    #[test]
    fn session_cookie_matches_adr_0009() {
        assert_eq!(
            session(&cfg(), "tok", Duration::days(30)),
            "cyex_tv_session=tok; Path=/; Domain=cyex.tv; Max-Age=2592000; Secure; HttpOnly; SameSite=Lax"
        );
        assert_eq!(
            clear_session(&cfg()),
            "cyex_tv_session=; Path=/; Domain=cyex.tv; Max-Age=0; Secure; HttpOnly; SameSite=Lax"
        );
    }

    #[test]
    fn state_cookie_is_host_only_and_scoped_to_auth() {
        assert_eq!(
            state(&cfg(), "st", Duration::minutes(10)),
            "cyex_tv_oauth=st; Path=/v1/auth; Max-Age=600; Secure; HttpOnly; SameSite=Lax"
        );
    }

    #[test]
    fn local_development_drops_domain_and_secure() {
        let local = AuthConfig::from_source(|name| {
            (name == "CYEX_TV_SITE_URL").then(|| "http://localhost:5173".to_owned())
        })
        .unwrap();
        assert_eq!(
            session(&local, "tok", Duration::days(1)),
            "cyex_tv_session=tok; Path=/; Max-Age=86400; HttpOnly; SameSite=Lax"
        );
    }

    #[test]
    fn reads_from_any_cookie_header() {
        let mut headers = HeaderMap::new();
        headers.append(COOKIE, HeaderValue::from_static("a=1; cyex_tv_oauth=; b=2"));
        headers.append(
            COOKIE,
            HeaderValue::from_static("cyex_tv_session=abc;cyex_tv_oauth=xyz"),
        );
        assert_eq!(read(&headers, SESSION).as_deref(), Some("abc"));
        assert_eq!(read(&headers, STATE).as_deref(), Some("xyz"));
        assert_eq!(read(&headers, "b").as_deref(), Some("2"));
        assert_eq!(read(&headers, "missing"), None);
    }
}
