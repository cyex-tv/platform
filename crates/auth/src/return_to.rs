//! Where to send the browser after signing in. Only the site itself, so the callback can't be
//! used as an open redirect.

use url::Url;

use crate::config::AuthConfig;
use crate::error::AuthError;

const MAX_LEN: usize = 2048;

/// Resolves `return_to` to an absolute URL on the site. Empty means the site's home page. A path
/// (`/settings`) is taken relative to the site; a full URL must have the site's origin.
pub fn resolve(cfg: &AuthConfig, raw: Option<&str>) -> Result<String, AuthError> {
    let raw = raw.map(str::trim).unwrap_or_default();
    if raw.is_empty() {
        return Ok(cfg.site_url.as_str().to_owned());
    }
    if raw.len() > MAX_LEN || raw.chars().any(|c| c.is_control() || c == '\\') {
        return Err(AuthError::BadReturnTo);
    }
    let url = if raw.starts_with('/') {
        // `//evil.example` is a full URL to another host, not a path.
        if raw.starts_with("//") {
            return Err(AuthError::BadReturnTo);
        }
        cfg.site_url.join(raw).map_err(|_| AuthError::BadReturnTo)?
    } else {
        Url::parse(raw).map_err(|_| AuthError::BadReturnTo)?
    };
    if url.origin() != cfg.site_url.origin()
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return Err(AuthError::BadReturnTo);
    }
    Ok(url.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg() -> AuthConfig {
        AuthConfig::from_source(|_| None).unwrap()
    }

    fn ok(raw: &str) -> String {
        resolve(&cfg(), Some(raw)).unwrap()
    }

    fn bad(raw: &str) {
        assert!(
            matches!(resolve(&cfg(), Some(raw)), Err(AuthError::BadReturnTo)),
            "{raw:?} should be refused"
        );
    }

    #[test]
    fn site_urls_and_paths_are_allowed() {
        assert_eq!(resolve(&cfg(), None).unwrap(), "https://cyex.tv/");
        assert_eq!(ok(""), "https://cyex.tv/");
        assert_eq!(
            ok("/settings?tab=sets#top"),
            "https://cyex.tv/settings?tab=sets#top"
        );
        assert_eq!(ok("https://cyex.tv/emotes/1"), "https://cyex.tv/emotes/1");
        assert_eq!(ok("https://CYEX.tv:443/x"), "https://cyex.tv/x");
    }

    #[test]
    fn other_places_are_refused() {
        bad("https://evil.example/");
        bad("//evil.example/");
        bad("/\\evil.example");
        bad("https://cyex.tv.evil.example/");
        bad("https://api.cyex.tv/");
        bad("http://cyex.tv/");
        bad("https://cyex.tv:8443/");
        bad("https://user:pw@cyex.tv/");
        bad("javascript:alert(1)");
        bad("settings");
        bad("/a\nb");
        bad(&format!("/{}", "a".repeat(MAX_LEN)));
    }
}
