//! Twitch and Kick: where their OAuth endpoints are, and what their answers look like
//! (ADR 0009, `docs/research/kick-api.md`).

use std::time::Duration;

use cyex_tv_shared::model::Platform;
use cyex_tv_shared::text;
use serde::Deserialize;
use url::Url;

use crate::config::ClientCreds;
use crate::error::AuthError;

#[derive(Clone, Debug)]
pub struct Endpoints {
    pub authorize: Url,
    pub token: Url,
    pub user: Url,
    pub revoke: Url,
}

fn fixed(url: &str) -> Url {
    // These are constants; a typo is caught by the tests below.
    Url::parse(url).expect("a valid built-in URL")
}

impl Endpoints {
    pub fn twitch() -> Self {
        Self {
            authorize: fixed("https://id.twitch.tv/oauth2/authorize"),
            token: fixed("https://id.twitch.tv/oauth2/token"),
            user: fixed("https://api.twitch.tv/helix/users"),
            revoke: fixed("https://id.twitch.tv/oauth2/revoke"),
        }
    }

    pub fn kick() -> Self {
        Self {
            authorize: fixed("https://id.kick.com/oauth/authorize"),
            token: fixed("https://id.kick.com/oauth/token"),
            user: fixed("https://api.kick.com/public/v1/users"),
            revoke: fixed("https://id.kick.com/oauth/revoke"),
        }
    }

    /// The same paths under another base, for tests against a local fixture server.
    pub fn at(base: &Url, platform: Platform) -> Self {
        let real = match platform {
            Platform::Twitch => Self::twitch(),
            Platform::Kick => Self::kick(),
        };
        let moved = |url: &Url| {
            let mut out = base.clone();
            out.set_path(&format!("/{platform}{}", url.path()));
            out
        };
        Self {
            authorize: moved(&real.authorize),
            token: moved(&real.token),
            user: moved(&real.user),
            revoke: moved(&real.revoke),
        }
    }
}

/// What a platform says about the user who signed in, checked before it reaches the database.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlatformUser {
    pub id: String,
    pub login: String,
    pub display_name: String,
    pub avatar: Option<String>,
}

pub struct Provider {
    pub platform: Platform,
    creds: ClientCreds,
    endpoints: Endpoints,
}

/// How long a platform gets to answer one request.
pub const TIMEOUT: Duration = Duration::from_secs(10);

impl Provider {
    pub fn new(platform: Platform, creds: ClientCreds, endpoints: Endpoints) -> Self {
        Self {
            platform,
            creds,
            endpoints,
        }
    }

    /// Kick requires PKCE S256. Twitch doesn't document it, so it gets `state` only.
    pub fn uses_pkce(&self) -> bool {
        self.platform == Platform::Kick
    }

    fn scope(&self) -> &'static str {
        match self.platform {
            // Identity only: `helix/users` needs no scope for the caller's own public profile.
            Platform::Twitch => "",
            Platform::Kick => "user:read",
        }
    }

    /// The platform's consent page.
    pub fn authorize_url(&self, redirect_uri: &str, state: &str, challenge: Option<&str>) -> Url {
        let mut url = self.endpoints.authorize.clone();
        {
            let mut query = url.query_pairs_mut();
            query
                .append_pair("response_type", "code")
                .append_pair("client_id", &self.creds.client_id)
                .append_pair("redirect_uri", redirect_uri)
                .append_pair("scope", self.scope())
                .append_pair("state", state);
            if let Some(challenge) = challenge {
                query
                    .append_pair("code_challenge", challenge)
                    .append_pair("code_challenge_method", "S256");
            }
        }
        url
    }

    /// Trades the code for an access token.
    pub async fn exchange(
        &self,
        http: &reqwest::Client,
        code: &str,
        verifier: Option<&str>,
        redirect_uri: &str,
    ) -> Result<String, AuthError> {
        let mut form = vec![
            ("grant_type", "authorization_code"),
            ("client_id", self.creds.client_id.as_str()),
            ("client_secret", self.creds.client_secret.as_str()),
            ("code", code),
            ("redirect_uri", redirect_uri),
        ];
        if let Some(verifier) = verifier {
            form.push(("code_verifier", verifier));
        }
        let response = http
            .post(self.endpoints.token.clone())
            .form(&form)
            .send()
            .await
            .map_err(|err| {
                AuthError::Platform(format!("{} token request: {err}", self.platform))
            })?;
        let token: TokenResponse = read_json(response, self.platform, "token").await?;
        if token.access_token.is_empty() {
            return Err(AuthError::Platform(format!(
                "{} sent an empty token",
                self.platform
            )));
        }
        Ok(token.access_token)
    }

    /// Reads the signed-in user with the access token.
    pub async fn user(
        &self,
        http: &reqwest::Client,
        access_token: &str,
    ) -> Result<PlatformUser, AuthError> {
        let mut request = http
            .get(self.endpoints.user.clone())
            .bearer_auth(access_token);
        if self.platform == Platform::Twitch {
            request = request.header("Client-Id", &self.creds.client_id);
        }
        let response = request
            .send()
            .await
            .map_err(|err| AuthError::Platform(format!("{} user request: {err}", self.platform)))?;
        let user = match self.platform {
            Platform::Twitch => {
                let body: DataList<TwitchUser> = read_json(response, self.platform, "user").await?;
                body.data
                    .into_iter()
                    .next()
                    .map(|u| (u.id, u.login, u.display_name, u.profile_image_url))
            }
            Platform::Kick => {
                let body: DataList<KickUser> = read_json(response, self.platform, "user").await?;
                // Kick has no separate login; `name` is the username. Logins are matched
                // ignoring case, so it is stored as Kick sends it.
                body.data.into_iter().next().map(|u| {
                    (
                        u.user_id.to_string(),
                        u.name.clone(),
                        u.name,
                        u.profile_picture,
                    )
                })
            }
        };
        let (id, login, display_name, avatar) = user
            .ok_or_else(|| AuthError::Platform(format!("{} user list was empty", self.platform)))?;
        check_user(self.platform, id, login, display_name, avatar)
    }

    /// Revokes the access token. cyex.tv keeps no platform tokens (ADR 0009); this tells the
    /// platform so too. Failures are only logged.
    pub async fn revoke(&self, http: &reqwest::Client, access_token: &str) {
        let request = match self.platform {
            Platform::Twitch => http.post(self.endpoints.revoke.clone()).form(&[
                ("client_id", self.creds.client_id.as_str()),
                ("token", access_token),
            ]),
            Platform::Kick => http
                .post(self.endpoints.revoke.clone())
                .query(&[("token", access_token), ("token_hint_type", "access_token")]),
        };
        match request.send().await {
            Ok(response) if response.status().is_success() => {}
            Ok(response) => {
                tracing::warn!(platform = %self.platform, status = %response.status(), "token revoke refused");
            }
            Err(err) => {
                tracing::warn!(platform = %self.platform, error = %err, "token revoke failed")
            }
        }
    }
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
}

#[derive(Deserialize)]
struct DataList<T> {
    data: Vec<T>,
}

#[derive(Deserialize)]
struct TwitchUser {
    id: String,
    login: String,
    display_name: String,
    profile_image_url: Option<String>,
}

#[derive(Deserialize)]
struct KickUser {
    user_id: i64,
    name: String,
    profile_picture: Option<String>,
}

async fn read_json<T: serde::de::DeserializeOwned>(
    response: reqwest::Response,
    platform: Platform,
    what: &str,
) -> Result<T, AuthError> {
    let status = response.status();
    let body = response
        .bytes()
        .await
        .map_err(|err| AuthError::Platform(format!("{platform} {what} body: {err}")))?;
    if !status.is_success() {
        let text = String::from_utf8_lossy(&body);
        let short: String = text.chars().take(200).collect();
        return Err(AuthError::Platform(format!(
            "{platform} {what} answered {status}: {short}"
        )));
    }
    serde_json::from_slice(&body)
        .map_err(|err| AuthError::Platform(format!("{platform} {what} JSON: {err}")))
}

fn check_user(
    platform: Platform,
    id: String,
    login: String,
    display_name: String,
    avatar: Option<String>,
) -> Result<PlatformUser, AuthError> {
    let bad = |what: &str| AuthError::Platform(format!("{platform} sent a bad {what}"));
    if id.is_empty() || id.len() > 20 || !id.bytes().all(|b| b.is_ascii_digit()) {
        return Err(bad("user ID"));
    }
    let login = login.trim().to_owned();
    if login.is_empty()
        || login.chars().count() > 64
        || login.chars().any(char::is_whitespace)
        || text::has_unsafe(&login, false)
    {
        return Err(bad("login"));
    }
    // Hidden characters are stripped, not refused: the user is already signing in.
    let display_name = text::strip_hidden(display_name.trim());
    let display_name = display_name.trim();
    let display_name = if display_name.is_empty()
        || display_name.chars().count() > 64
        || display_name.chars().any(char::is_control)
    {
        login.clone()
    } else {
        display_name.to_owned()
    };
    let avatar = avatar
        .filter(|url| url.len() <= 2048 && Url::parse(url).is_ok_and(|u| u.scheme() == "https"));
    Ok(PlatformUser {
        id,
        login,
        display_name,
        avatar,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn provider(platform: Platform) -> Provider {
        let endpoints = match platform {
            Platform::Twitch => Endpoints::twitch(),
            Platform::Kick => Endpoints::kick(),
        };
        Provider::new(
            platform,
            ClientCreds {
                client_id: "cid".to_owned(),
                client_secret: "secret".to_owned(),
            },
            endpoints,
        )
    }

    fn query(url: &Url) -> Vec<(String, String)> {
        url.query_pairs()
            .map(|(k, v)| (k.into_owned(), v.into_owned()))
            .collect()
    }

    #[test]
    fn twitch_authorize_url_asks_for_identity_only() {
        let url = provider(Platform::Twitch).authorize_url("https://api.cyex.tv/cb", "st", None);
        assert_eq!(url.host_str(), Some("id.twitch.tv"));
        assert_eq!(url.path(), "/oauth2/authorize");
        let q = query(&url);
        assert!(q.contains(&("scope".into(), String::new())));
        assert!(q.contains(&("state".into(), "st".into())));
        assert!(q.contains(&("redirect_uri".into(), "https://api.cyex.tv/cb".into())));
        assert!(!q.iter().any(|(k, _)| k == "code_challenge"));
        assert!(!provider(Platform::Twitch).uses_pkce());
    }

    #[test]
    fn kick_authorize_url_has_pkce() {
        let kick = provider(Platform::Kick);
        assert!(kick.uses_pkce());
        let url = kick.authorize_url("https://api.cyex.tv/cb", "st", Some("chal"));
        assert_eq!(url.host_str(), Some("id.kick.com"));
        let q = query(&url);
        assert!(q.contains(&("scope".into(), "user:read".into())));
        assert!(q.contains(&("code_challenge".into(), "chal".into())));
        assert!(q.contains(&("code_challenge_method".into(), "S256".into())));
        assert!(q.contains(&("response_type".into(), "code".into())));
    }

    #[test]
    fn endpoints_can_move_for_tests() {
        let base = Url::parse("http://127.0.0.1:9999").unwrap();
        let e = Endpoints::at(&base, Platform::Kick);
        assert_eq!(e.token.as_str(), "http://127.0.0.1:9999/kick/oauth/token");
        assert_eq!(
            e.user.as_str(),
            "http://127.0.0.1:9999/kick/public/v1/users"
        );
    }

    #[test]
    fn users_are_checked() {
        let ok = check_user(
            Platform::Twitch,
            "123".into(),
            "name".into(),
            "  ".into(),
            Some("http://insecure.example/a.png".into()),
        )
        .unwrap();
        assert_eq!(ok.display_name, "name");
        assert_eq!(ok.avatar, None);

        for (id, login) in [("", "a"), ("12a", "a"), ("1", ""), ("1", "has space")] {
            assert!(
                check_user(Platform::Kick, id.into(), login.into(), "x".into(), None).is_err(),
                "{id:?} {login:?}"
            );
        }
    }

    #[test]
    fn hidden_characters_are_stripped_from_display_names() {
        let user = |login: &str, display: &str| {
            check_user(
                Platform::Twitch,
                "1".into(),
                login.into(),
                display.into(),
                None,
            )
        };
        assert_eq!(
            user("bob", "Bo\u{202E}b\u{200B}").unwrap().display_name,
            "Bob"
        );
        assert_eq!(user("bob", "\u{3164}").unwrap().display_name, "bob");
        assert!(user("b\u{200B}ob", "Bob").is_err());
    }
}
