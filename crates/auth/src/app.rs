//! App sign-in (ADR 0021): how a desktop app such as cyex deck gets an `app` session.
//!
//! OAuth 2.0 authorization code flow with PKCE (RFC 7636, S256 only) for public clients, with a
//! loopback redirect as RFC 8252 describes it:
//!
//! 1. The app opens `GET /v1/auth/app/authorize` in the browser. The client and the loopback
//!    `redirect_uri` are checked here; nothing is redirected to an address that fails.
//! 2. The browser lands on the site's consent page, signing in with Twitch or Kick first if needed.
//! 3. Allow (or Deny) posts to `/v1/auth/app/consent` with the site cookie. The answer is where to
//!    send the browser: the loopback address with a one-time `code` (or `error=access_denied`).
//! 4. The app trades the code and its verifier at `POST /v1/auth/app/token` for an access token
//!    (1 hour) and a refresh token (60 days, rotated on every use).
//!
//! A refresh token presented twice is taken as stolen: the whole session is signed out (reuse
//! detection, RFC 9700 4.14.2). A code presented twice signs out the session it gave.

use chrono::Duration;
use cyex_tv_db::app_auth::{self, NewAuthCode};
use sqlx::{Connection, PgPool};
use url::Url;

use crate::error::AuthError;
use crate::flow::Auth;
use crate::session::{Caller, Via};
use crate::tokens;

/// What an app may do once signed in. Shown on the consent page.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Scope {
    pub id: &'static str,
    /// For the consent page: "cyex deck wants to: …".
    pub description: &'static str,
    /// The GraphQL mutations the scope allows. Every query stays open to app sessions.
    pub mutations: &'static [&'static str],
}

pub const SCOPE_SET_EMOTES: Scope = Scope {
    id: "sets.emotes",
    description: "Manage emotes in sets you can edit",
    mutations: &["addSetEmote", "updateSetEmote", "removeSetEmote"],
};

pub const SCOPE_COSMETICS: Scope = Scope {
    id: "cosmetics.select",
    description: "Pick your cosmetics",
    mutations: &["selectCosmetics"],
};

/// A registered app. Public clients only: there is no secret, PKCE stands in for it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AppClient {
    pub id: &'static str,
    pub name: &'static str,
    /// Paths allowed on the loopback redirect. Any port is allowed (RFC 8252 7.3).
    pub redirect_paths: &'static [&'static str],
    pub scopes: &'static [Scope],
}

/// The cyex deck desktop app.
pub const DECK: AppClient = AppClient {
    id: "cyex-deck",
    name: "cyex deck",
    redirect_paths: &["/callback"],
    scopes: &[SCOPE_SET_EMOTES, SCOPE_COSMETICS],
};

/// Every registered app. New ones are added here, in code, with an ADR note.
pub const CLIENTS: &[AppClient] = &[DECK];

pub fn client(id: &str) -> Option<&'static AppClient> {
    CLIENTS.iter().find(|c| c.id == id)
}

/// Whether an app session of `client_id` may run the mutation `field`.
pub fn allows_mutation(client_id: &str, field: &str) -> bool {
    client(client_id).is_some_and(|c| c.scopes.iter().any(|s| s.mutations.contains(&field)))
}

/// The scopes of a client, space-separated, as the token answer's `scope`.
pub fn scope_string(client: &AppClient) -> String {
    client
        .scopes
        .iter()
        .map(|s| s.id)
        .collect::<Vec<_>>()
        .join(" ")
}

/// How long an access token lives.
pub fn access_ttl() -> Duration {
    Duration::hours(1)
}

/// How long a refresh token lives; every refresh starts the window again.
pub fn refresh_ttl() -> Duration {
    Duration::days(60)
}

/// How long a code may wait for its exchange.
pub fn code_ttl() -> Duration {
    Duration::minutes(2)
}

/// Used refresh tokens are remembered this long, to notice a replay.
pub fn keep_used_refresh() -> Duration {
    Duration::days(7)
}

/// The longest `state` taken.
pub const STATE_MAX: usize = 512;

/// Why an app sign-in step failed. The token and revoke endpoints answer with the OAuth error
/// code (`error`); the authorize and consent steps with the API's usual body.
#[derive(Debug, thiserror::Error)]
pub enum AppAuthError {
    #[error("unknown client")]
    UnknownClient,
    #[error("bad redirect_uri")]
    BadRedirectUri,
    #[error("{0}")]
    InvalidRequest(&'static str),
    #[error("invalid grant")]
    InvalidGrant,
    #[error("unsupported grant type")]
    UnsupportedGrantType,
    #[error("only the site session may consent")]
    NeedsSiteSession,
    #[error(transparent)]
    Auth(#[from] AuthError),
}

impl From<cyex_tv_db::DbError> for AppAuthError {
    fn from(err: cyex_tv_db::DbError) -> Self {
        Self::Auth(err.into())
    }
}

impl From<sqlx::Error> for AppAuthError {
    fn from(err: sqlx::Error) -> Self {
        Self::Auth(err.into())
    }
}

impl AppAuthError {
    /// The OAuth error code (RFC 6749 5.2).
    pub fn oauth_code(&self) -> &'static str {
        match self {
            Self::UnknownClient => "invalid_client",
            Self::BadRedirectUri | Self::InvalidRequest(_) | Self::NeedsSiteSession => {
                "invalid_request"
            }
            Self::InvalidGrant => "invalid_grant",
            Self::UnsupportedGrantType => "unsupported_grant_type",
            Self::Auth(AuthError::Db(_)) => "server_error",
            Self::Auth(_) => "invalid_grant",
        }
    }

    /// The API's own code, for the authorize and consent steps.
    pub fn api_code(&self) -> &'static str {
        match self {
            Self::UnknownClient => "unknown_client",
            Self::BadRedirectUri => "bad_redirect_uri",
            Self::InvalidRequest(_) => "bad_request",
            Self::InvalidGrant => "invalid_grant",
            Self::UnsupportedGrantType => "unsupported_grant_type",
            Self::NeedsSiteSession => "signed_out",
            Self::Auth(err) => err.code(),
        }
    }

    pub fn message(&self) -> String {
        match self {
            Self::UnknownClient => "Unknown client_id.".to_owned(),
            Self::BadRedirectUri => "redirect_uri must be http://127.0.0.1:<port>/callback or \
                                     http://[::1]:<port>/callback."
                .to_owned(),
            Self::InvalidRequest(why) => (*why).to_owned(),
            Self::InvalidGrant => {
                "The code or refresh token is wrong, expired or already used.".to_owned()
            }
            Self::UnsupportedGrantType => {
                "grant_type must be authorization_code or refresh_token.".to_owned()
            }
            Self::NeedsSiteSession => "Sign in on cyex.tv to allow an app.".to_owned(),
            Self::Auth(err) => err.message().to_owned(),
        }
    }
}

/// Checks a loopback redirect: `http`, the host `127.0.0.1` or `[::1]` (RFC 8252 8.3 advises
/// against `localhost`), any port, a registered path, and no user info, query or fragment.
pub fn check_redirect(client: &AppClient, raw: &str) -> Result<Url, AppAuthError> {
    if raw.len() > 512 {
        return Err(AppAuthError::BadRedirectUri);
    }
    let url = Url::parse(raw).map_err(|_| AppAuthError::BadRedirectUri)?;
    let loopback = match url.host() {
        Some(url::Host::Ipv4(ip)) => ip == std::net::Ipv4Addr::LOCALHOST,
        Some(url::Host::Ipv6(ip)) => ip == std::net::Ipv6Addr::LOCALHOST,
        _ => false,
    };
    let ok = url.scheme() == "http"
        && loopback
        && url.username().is_empty()
        && url.password().is_none()
        && url.query().is_none()
        && url.fragment().is_none()
        && client.redirect_paths.contains(&url.path());
    if ok {
        Ok(url)
    } else {
        Err(AppAuthError::BadRedirectUri)
    }
}

/// An S256 challenge: 43 base64url characters (a sha256).
fn is_challenge(raw: &str) -> bool {
    raw.len() == 43
        && raw
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

/// A PKCE verifier (RFC 7636 4.1): 43 to 128 of `[A-Za-z0-9-._~]`.
fn is_verifier(raw: &str) -> bool {
    (43..=128).contains(&raw.len())
        && raw
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'.' | b'_' | b'~'))
}

/// What `/v1/auth/app/authorize` and the consent step read.
#[derive(Clone, Debug, Default)]
pub struct AuthorizeRequest<'a> {
    pub response_type: Option<&'a str>,
    pub client_id: Option<&'a str>,
    pub redirect_uri: Option<&'a str>,
    pub code_challenge: Option<&'a str>,
    pub code_challenge_method: Option<&'a str>,
    pub state: Option<&'a str>,
}

/// A request that passed the checks.
#[derive(Clone, Debug)]
pub struct Checked {
    pub client: &'static AppClient,
    pub redirect: Url,
    pub code_challenge: String,
    pub state: String,
}

/// What `authorize` answers.
#[derive(Debug)]
pub enum Authorize {
    /// On to the site's consent page.
    Consent(String),
    /// Back to the app with an OAuth error (the redirect itself was fine).
    Back(String),
}

/// `redirect` with the given query pairs.
fn back_to(redirect: &Url, pairs: &[(&str, &str)]) -> String {
    let mut url = redirect.clone();
    {
        let mut q = url.query_pairs_mut();
        for (k, v) in pairs {
            q.append_pair(k, v);
        }
    }
    url.to_string()
}

/// The client and redirect first: when they fail, nothing may be sent to the redirect.
fn client_and_redirect(
    req: &AuthorizeRequest<'_>,
) -> Result<(&'static AppClient, Url), AppAuthError> {
    let client = req
        .client_id
        .and_then(client)
        .ok_or(AppAuthError::UnknownClient)?;
    let redirect = check_redirect(client, req.redirect_uri.unwrap_or_default())?;
    Ok((client, redirect))
}

/// The rest, once the redirect is known to be safe.
fn the_rest(req: &AuthorizeRequest<'_>) -> Result<(String, String), &'static str> {
    if req.response_type.is_some_and(|t| t != "code") {
        return Err("response_type must be code.");
    }
    if req.code_challenge_method != Some("S256") {
        return Err("code_challenge_method must be S256.");
    }
    let challenge = req
        .code_challenge
        .filter(|c| is_challenge(c))
        .ok_or("code_challenge must be the S256 challenge (43 base64url characters).")?;
    let state = req
        .state
        .filter(|s| !s.is_empty() && s.len() <= STATE_MAX)
        .ok_or("state is required (1 to 512 characters).")?;
    Ok((challenge.to_owned(), state.to_owned()))
}

/// Checks the whole request. `Err(Ok(location))` is an OAuth error to send back to the app.
pub fn check(req: &AuthorizeRequest<'_>) -> Result<Checked, Result<String, AppAuthError>> {
    let (client, redirect) = client_and_redirect(req).map_err(Err)?;
    match the_rest(req) {
        Ok((code_challenge, state)) => Ok(Checked {
            client,
            redirect,
            code_challenge,
            state,
        }),
        Err(why) => {
            let mut pairs = vec![("error", "invalid_request"), ("error_description", why)];
            if let Some(state) = req.state.filter(|s| s.len() <= STATE_MAX) {
                pairs.push(("state", state));
            }
            Err(Ok(back_to(&redirect, &pairs)))
        }
    }
}

/// The token endpoint's form (RFC 6749 4.1.3 and 6).
#[derive(Clone, Debug, Default)]
pub struct TokenRequest<'a> {
    pub grant_type: Option<&'a str>,
    pub client_id: Option<&'a str>,
    pub code: Option<&'a str>,
    pub code_verifier: Option<&'a str>,
    pub redirect_uri: Option<&'a str>,
    pub refresh_token: Option<&'a str>,
    /// The app's `User-Agent`, kept with the session.
    pub user_agent: Option<&'a str>,
}

/// A successful token answer (RFC 6749 5.1).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Tokens {
    pub access_token: String,
    /// Seconds.
    pub expires_in: i64,
    pub refresh_token: String,
    pub scope: String,
    /// The session's public ID, as `mySessions` lists it.
    pub session_id: uuid::Uuid,
}

impl Auth {
    /// Step 1: where to send the browser.
    pub fn app_authorize(&self, req: &AuthorizeRequest<'_>) -> Result<Authorize, AppAuthError> {
        match check(req) {
            Ok(checked) => {
                let mut url = self.config().site_url.clone();
                url.set_path("/authorize");
                url.query_pairs_mut()
                    .append_pair("client_id", checked.client.id)
                    .append_pair("redirect_uri", checked.redirect.as_str())
                    .append_pair("code_challenge", &checked.code_challenge)
                    .append_pair("code_challenge_method", "S256")
                    .append_pair("state", &checked.state);
                Ok(Authorize::Consent(url.to_string()))
            }
            Err(Ok(location)) => Ok(Authorize::Back(location)),
            Err(Err(err)) => Err(err),
        }
    }

    /// Step 3: the signed-in user allows or denies. Only the site cookie may answer, never an app
    /// token. Returns where to send the browser.
    pub async fn app_consent(
        &self,
        db: &PgPool,
        caller: Option<&Caller>,
        req: &AuthorizeRequest<'_>,
        allow: bool,
    ) -> Result<String, AppAuthError> {
        let caller = match caller {
            Some(c) if c.via == Via::Cookie && c.app_client.is_none() => c,
            _ => return Err(AppAuthError::NeedsSiteSession),
        };
        let checked = match check(req) {
            Ok(checked) => checked,
            Err(Ok(location)) => return Ok(location),
            Err(Err(err)) => return Err(err),
        };
        if !allow {
            return Ok(back_to(
                &checked.redirect,
                &[("error", "access_denied"), ("state", &checked.state)],
            ));
        }
        let code = tokens::random_token();
        let mut conn = db.acquire().await?;
        app_auth::create_code(
            &mut conn,
            &tokens::hash(&code),
            &NewAuthCode {
                client_id: checked.client.id,
                account_id: caller.account_id,
                redirect_uri: checked.redirect.as_str(),
                code_challenge: &checked.code_challenge,
                ttl: code_ttl(),
            },
        )
        .await?;
        Ok(back_to(
            &checked.redirect,
            &[("code", &code), ("state", &checked.state)],
        ))
    }

    /// Step 4, and every refresh after it.
    pub async fn app_token(
        &self,
        db: &PgPool,
        req: &TokenRequest<'_>,
    ) -> Result<Tokens, AppAuthError> {
        let client = req
            .client_id
            .and_then(client)
            .ok_or(AppAuthError::UnknownClient)?;
        match req.grant_type {
            Some("authorization_code") => self.exchange_code(db, client, req).await,
            Some("refresh_token") => self.refresh(db, client, req).await,
            Some(_) => Err(AppAuthError::UnsupportedGrantType),
            None => Err(AppAuthError::InvalidRequest("grant_type is required.")),
        }
    }

    async fn exchange_code(
        &self,
        db: &PgPool,
        client: &'static AppClient,
        req: &TokenRequest<'_>,
    ) -> Result<Tokens, AppAuthError> {
        let code = req
            .code
            .filter(|c| tokens::is_well_formed(c))
            .ok_or(AppAuthError::InvalidGrant)?;
        let verifier = req
            .code_verifier
            .ok_or(AppAuthError::InvalidRequest("code_verifier is required."))?;
        if !is_verifier(verifier) {
            return Err(AppAuthError::InvalidGrant);
        }
        let hash = tokens::hash(code);
        let mut conn = db.acquire().await?;
        let mut tx = conn.begin().await?;
        let Some(stored) = app_auth::lock_code(&mut tx, &hash).await? else {
            return Err(AppAuthError::InvalidGrant);
        };
        if stored.used_at.is_some() {
            // A replayed code: whatever it gave is signed out (RFC 6749 4.1.2).
            if let Some(session) = stored.session_id {
                app_auth::delete_session(&mut tx, session).await?;
            }
            tx.commit().await?;
            return Err(AppAuthError::InvalidGrant);
        }
        let fits = stored.client_id == client.id
            && stored.expires_at > chrono::Utc::now()
            && req.redirect_uri == Some(stored.redirect_uri.as_str())
            && tokens::pkce_challenge(verifier) == stored.code_challenge
            && app_auth::account_is_live(&mut tx, stored.account_id).await?;
        if !fits {
            // One try per code: a wrong verifier burns it.
            app_auth::use_code(&mut tx, &hash, None).await?;
            tx.commit().await?;
            return Err(AppAuthError::InvalidGrant);
        }
        let access = tokens::random_token();
        let refresh = tokens::random_token();
        let session = app_auth::create_session(
            &mut tx,
            &tokens::hash(&access),
            stored.account_id,
            client.id,
            access_ttl(),
            refresh_ttl(),
            req.user_agent,
        )
        .await?;
        app_auth::create_refresh(&mut tx, &tokens::hash(&refresh), session.id).await?;
        app_auth::use_code(&mut tx, &hash, Some(session.id)).await?;
        tx.commit().await?;
        Ok(Tokens {
            access_token: access,
            expires_in: access_ttl().num_seconds(),
            refresh_token: refresh,
            scope: scope_string(client),
            session_id: session.id,
        })
    }

    async fn refresh(
        &self,
        db: &PgPool,
        client: &'static AppClient,
        req: &TokenRequest<'_>,
    ) -> Result<Tokens, AppAuthError> {
        let presented = req
            .refresh_token
            .filter(|t| tokens::is_well_formed(t))
            .ok_or(AppAuthError::InvalidGrant)?;
        let hash = tokens::hash(presented);
        let mut conn = db.acquire().await?;
        let mut tx = conn.begin().await?;
        let Some(stored) = app_auth::lock_refresh(&mut tx, &hash).await? else {
            return Err(AppAuthError::InvalidGrant);
        };
        if stored.used_at.is_some() {
            // Reuse: someone holds an old token. Sign the whole session out.
            app_auth::delete_session(&mut tx, stored.session_id).await?;
            tx.commit().await?;
            tracing::warn!(
                client = client.id,
                "refresh token reused; session signed out"
            );
            return Err(AppAuthError::InvalidGrant);
        }
        if stored.client_id.as_deref() != Some(client.id) {
            return Err(AppAuthError::InvalidGrant);
        }
        let access = tokens::random_token();
        let refresh = tokens::random_token();
        let Some(session) = app_auth::rotate_session(
            &mut tx,
            stored.session_id,
            &tokens::hash(&access),
            access_ttl(),
            refresh_ttl(),
        )
        .await?
        else {
            return Err(AppAuthError::InvalidGrant);
        };
        app_auth::use_refresh(&mut tx, &hash, session.id, keep_used_refresh()).await?;
        app_auth::create_refresh(&mut tx, &tokens::hash(&refresh), session.id).await?;
        tx.commit().await?;
        Ok(Tokens {
            access_token: access,
            expires_in: access_ttl().num_seconds(),
            refresh_token: refresh,
            scope: scope_string(client),
            session_id: session.id,
        })
    }

    /// RFC 7009: signs out the app session an access or refresh token belongs to. Unknown tokens
    /// are fine (the answer is the same), so this tells nobody whether a token existed.
    pub async fn app_revoke(&self, db: &PgPool, token: &str) -> Result<(), AppAuthError> {
        if !tokens::is_well_formed(token) {
            return Ok(());
        }
        let hash = tokens::hash(token);
        let mut conn = db.acquire().await?;
        let session = match app_auth::session_of_refresh(&mut conn, &hash).await? {
            Some(id) => Some(id),
            None => app_auth::session_of_access(&mut conn, &hash).await?,
        };
        if let Some(id) = session {
            app_auth::delete_session(&mut conn, id).await?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_deck_is_registered() {
        let deck = client("cyex-deck").unwrap();
        assert_eq!(deck.name, "cyex deck");
        assert_eq!(scope_string(deck), "sets.emotes cosmetics.select");
        assert!(client("other").is_none());
    }

    #[test]
    fn only_loopback_redirects_pass() {
        for ok in [
            "http://127.0.0.1:53682/callback",
            "http://127.0.0.1/callback",
            "http://[::1]:8080/callback",
        ] {
            assert!(check_redirect(&DECK, ok).is_ok(), "{ok}");
        }
        for bad in [
            "https://127.0.0.1:5000/callback",
            "http://localhost:5000/callback",
            "http://127.0.0.2:5000/callback",
            "http://evil.example/callback",
            "http://127.0.0.1.evil.example/callback",
            "http://127.0.0.1:5000/other",
            "http://127.0.0.1:5000/callback?x=1",
            "http://127.0.0.1:5000/callback#x",
            "http://user@127.0.0.1:5000/callback",
            "cyexdeck://callback",
            "",
        ] {
            assert!(check_redirect(&DECK, bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn scopes_allow_their_mutations_only() {
        for m in [
            "addSetEmote",
            "updateSetEmote",
            "removeSetEmote",
            "selectCosmetics",
        ] {
            assert!(allows_mutation("cyex-deck", m), "{m}");
        }
        for m in ["deleteSet", "revokeSession", "deleteAccount", "createSet"] {
            assert!(!allows_mutation("cyex-deck", m), "{m}");
        }
        assert!(!allows_mutation("other", "addSetEmote"));
    }

    fn req<'a>(challenge: &'a str, state: &'a str) -> AuthorizeRequest<'a> {
        AuthorizeRequest {
            response_type: Some("code"),
            client_id: Some("cyex-deck"),
            redirect_uri: Some("http://127.0.0.1:4000/callback"),
            code_challenge: Some(challenge),
            code_challenge_method: Some("S256"),
            state: Some(state),
        }
    }

    #[test]
    fn requests_are_checked_in_order() {
        let challenge = tokens::pkce_challenge(&tokens::random_token());
        assert!(check(&req(&challenge, "s")).is_ok());
        // A bad challenge goes back to the (safe) redirect with an error and the state.
        let Err(Ok(back)) = check(&req("short", "s")) else {
            panic!("expected a redirect back");
        };
        assert!(back.starts_with("http://127.0.0.1:4000/callback?error=invalid_request"));
        assert!(back.ends_with("&state=s"));
        // No state: back without one.
        assert!(matches!(check(&req(&challenge, "")), Err(Ok(_))));
        // A bad redirect is never redirected to.
        let mut bad = req(&challenge, "s");
        bad.redirect_uri = Some("https://evil.example/callback");
        assert!(matches!(
            check(&bad),
            Err(Err(AppAuthError::BadRedirectUri))
        ));
        let mut plain = req(&challenge, "s");
        plain.code_challenge_method = Some("plain");
        assert!(matches!(check(&plain), Err(Ok(_))));
    }

    #[test]
    fn verifiers_follow_rfc_7636() {
        assert!(is_verifier(&tokens::random_token()));
        assert!(is_verifier(&"a~.-_".repeat(20)));
        assert!(!is_verifier("short"));
        assert!(!is_verifier(&"a".repeat(129)));
        assert!(!is_verifier(&format!("{}+", "a".repeat(50))));
    }

    #[test]
    fn lifetimes() {
        assert_eq!(access_ttl(), Duration::hours(1));
        assert_eq!(refresh_ttl(), Duration::days(60));
        assert!(code_ttl() <= Duration::minutes(10));
    }
}
