//! Sign-in config, from `CYEX_TV_*` variables (see `.env.example`).

use std::fmt;

use cyex_tv_shared::config::ConfigError;
use url::Url;

use crate::providers::Endpoints;
use crate::rbac::Owners;

/// An OAuth client registered with a platform. `Debug` hides the secret.
#[derive(Clone)]
pub struct ClientCreds {
    pub client_id: String,
    pub client_secret: String,
}

impl fmt::Debug for ClientCreds {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ClientCreds")
            .field("client_id", &self.client_id)
            .field("client_secret", &"<redacted>")
            .finish()
    }
}

#[derive(Clone, Debug)]
pub struct ProviderConfig {
    pub creds: ClientCreds,
    /// The platform's real endpoints, unless a test points them somewhere else.
    pub endpoints: Endpoints,
}

#[derive(Clone, Debug)]
pub struct AuthConfig {
    /// Where this API is reached, for the OAuth redirect URIs. `https://api.cyex.tv`.
    pub public_url: Url,
    /// The site. `return_to` must be on it, and cookie callers must send its origin.
    pub site_url: Url,
    /// `Domain` of the session cookie. `None` makes it host-only (local development).
    pub cookie_domain: Option<String>,
    pub cookie_secure: bool,
    /// `None` when the platform's client isn't configured: its sign-in answers 503.
    pub twitch: Option<ProviderConfig>,
    pub kick: Option<ProviderConfig>,
    /// `CYEX_TV_OWNER_IDS` (ADR 0010).
    pub owners: Owners,
}

impl AuthConfig {
    /// Reads from any lookup, so tests don't touch the process environment.
    pub fn from_source(get: impl Fn(&str) -> Option<String>) -> Result<Self, ConfigError> {
        let public_url = url_or(&get, "CYEX_TV_PUBLIC_URL", "https://api.cyex.tv")?;
        let site_url = url_or(&get, "CYEX_TV_SITE_URL", "https://cyex.tv")?;

        let cookie_domain = match get("CYEX_TV_COOKIE_DOMAIN") {
            Some(raw) if raw.trim().is_empty() => None,
            Some(raw) => Some(raw.trim().trim_start_matches('.').to_owned()),
            None => default_cookie_domain(&site_url),
        };
        let cookie_secure = match get("CYEX_TV_COOKIE_SECURE").as_deref().map(str::trim) {
            None => site_url.scheme() == "https",
            Some("true") => true,
            Some("false") => false,
            Some(other) => {
                return Err(ConfigError::Invalid {
                    name: "CYEX_TV_COOKIE_SECURE",
                    reason: format!("{other:?} is not true or false"),
                });
            }
        };

        let owners = Owners::parse(&get("CYEX_TV_OWNER_IDS").unwrap_or_default())?;

        Ok(Self {
            owners,
            twitch: creds(
                &get,
                "CYEX_TV_TWITCH_CLIENT_ID",
                "CYEX_TV_TWITCH_CLIENT_SECRET",
            )?
            .map(|creds| ProviderConfig {
                creds,
                endpoints: Endpoints::twitch(),
            }),
            kick: creds(&get, "CYEX_TV_KICK_CLIENT_ID", "CYEX_TV_KICK_CLIENT_SECRET")?.map(
                |creds| ProviderConfig {
                    creds,
                    endpoints: Endpoints::kick(),
                },
            ),
            public_url,
            site_url,
            cookie_domain,
            cookie_secure,
        })
    }

    /// The site's origin, as browsers send it in `Origin`: `https://cyex.tv`.
    pub fn site_origin(&self) -> String {
        self.site_url.origin().ascii_serialization()
    }
}

/// The site's host, unless it's `localhost` or an IP address, where browsers refuse `Domain`.
fn default_cookie_domain(site: &Url) -> Option<String> {
    match site.host() {
        Some(url::Host::Domain(host)) if host != "localhost" => Some(host.to_owned()),
        _ => None,
    }
}

fn url_or(
    get: &impl Fn(&str) -> Option<String>,
    name: &'static str,
    default: &str,
) -> Result<Url, ConfigError> {
    let raw = get(name).unwrap_or_else(|| default.to_owned());
    let url = Url::parse(raw.trim()).map_err(|err| ConfigError::Invalid {
        name,
        reason: err.to_string(),
    })?;
    if !matches!(url.scheme(), "http" | "https") || url.host().is_none() {
        return Err(ConfigError::Invalid {
            name,
            reason: "must be an http or https URL".to_owned(),
        });
    }
    Ok(url)
}

/// Both or neither: a client ID without its secret is a mistake worth failing on.
fn creds(
    get: &impl Fn(&str) -> Option<String>,
    id_name: &'static str,
    secret_name: &'static str,
) -> Result<Option<ClientCreds>, ConfigError> {
    let id = get(id_name).filter(|v| !v.trim().is_empty());
    let secret = get(secret_name).filter(|v| !v.trim().is_empty());
    match (id, secret) {
        (None, None) => Ok(None),
        (Some(client_id), Some(client_secret)) => Ok(Some(ClientCreds {
            client_id: client_id.trim().to_owned(),
            client_secret: client_secret.trim().to_owned(),
        })),
        (Some(_), None) => Err(ConfigError::Missing(secret_name)),
        (None, Some(_)) => Err(ConfigError::Missing(id_name)),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    fn load(pairs: &[(&str, &str)]) -> Result<AuthConfig, ConfigError> {
        let map: HashMap<String, String> = pairs
            .iter()
            .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
            .collect();
        AuthConfig::from_source(|name| map.get(name).cloned())
    }

    #[test]
    fn production_defaults() {
        let cfg = load(&[]).unwrap();
        assert_eq!(cfg.public_url.as_str(), "https://api.cyex.tv/");
        assert_eq!(cfg.site_origin(), "https://cyex.tv");
        assert_eq!(cfg.cookie_domain.as_deref(), Some("cyex.tv"));
        assert!(cfg.cookie_secure);
        assert!(cfg.twitch.is_none() && cfg.kick.is_none());
    }

    #[test]
    fn localhost_gets_a_host_only_insecure_cookie() {
        let cfg = load(&[("CYEX_TV_SITE_URL", "http://localhost:5173")]).unwrap();
        assert_eq!(cfg.cookie_domain, None);
        assert!(!cfg.cookie_secure);
        assert_eq!(cfg.site_origin(), "http://localhost:5173");
    }

    #[test]
    fn cookie_settings_can_be_set() {
        let cfg = load(&[
            ("CYEX_TV_COOKIE_DOMAIN", ".example.test"),
            ("CYEX_TV_COOKIE_SECURE", "false"),
        ])
        .unwrap();
        assert_eq!(cfg.cookie_domain.as_deref(), Some("example.test"));
        assert!(!cfg.cookie_secure);
        let cfg = load(&[("CYEX_TV_COOKIE_DOMAIN", "")]).unwrap();
        assert_eq!(cfg.cookie_domain, None);
        assert!(load(&[("CYEX_TV_COOKIE_SECURE", "yes")]).is_err());
    }

    #[test]
    fn clients_need_both_halves() {
        let cfg = load(&[
            ("CYEX_TV_KICK_CLIENT_ID", "kid"),
            ("CYEX_TV_KICK_CLIENT_SECRET", "ksecret"),
        ])
        .unwrap();
        let kick = cfg.kick.unwrap();
        assert_eq!(kick.creds.client_id, "kid");
        assert!(!format!("{kick:?}").contains("ksecret"));
        assert!(cfg.twitch.is_none());

        assert_eq!(
            load(&[("CYEX_TV_TWITCH_CLIENT_ID", "x")]).unwrap_err(),
            ConfigError::Missing("CYEX_TV_TWITCH_CLIENT_SECRET")
        );
        assert_eq!(
            load(&[("CYEX_TV_KICK_CLIENT_SECRET", "x")]).unwrap_err(),
            ConfigError::Missing("CYEX_TV_KICK_CLIENT_ID")
        );
    }

    #[test]
    fn owners_are_read() {
        let cfg = load(&[("CYEX_TV_OWNER_IDS", "twitch:12345678,kick:7654321")]).unwrap();
        assert_eq!(cfg.owners.identities, ["twitch:12345678", "kick:7654321"]);
        assert!(load(&[]).unwrap().owners.is_empty());
        assert!(load(&[("CYEX_TV_OWNER_IDS", "twitch:x")]).is_err());
    }

    #[test]
    fn urls_must_be_http() {
        assert!(load(&[("CYEX_TV_SITE_URL", "ftp://cyex.tv")]).is_err());
        assert!(load(&[("CYEX_TV_PUBLIC_URL", "not a url")]).is_err());
    }
}
