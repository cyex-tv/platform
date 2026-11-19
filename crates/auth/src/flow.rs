//! The sign-in flow (ADR 0009):
//!
//! 1. `start`: store a fresh `state` (its hash) with the PKCE verifier and `return_to`, set the
//!    pre-login cookie to `state`, and send the browser to the platform.
//! 2. `finish`: the platform sends the browser back with `code` and `state`. The `state` must match
//!    the cookie and a stored row, which is taken so it can't be replayed. The code is traded for
//!    an access token, the user is read, the token is revoked, and the account is signed in to (or
//!    the identity linked). A new site session replaces the old one.

use std::sync::Arc;

use chrono::Duration;
use cyex_tv_db::accounts::{self, Profile};
use cyex_tv_db::oauth_states::{self, NewOAuthState};
use cyex_tv_db::sessions::{self, SessionKind};
use cyex_tv_db::{channels, sets};
use cyex_tv_shared::model::Platform;
use sqlx::{Connection, PgPool};
use uuid::Uuid;

use crate::config::{AuthConfig, ProviderConfig};
use crate::error::AuthError;
use crate::providers::{self, Provider};
use crate::session::{Caller, Via, ttl};
use crate::{cookies, return_to, tokens};

/// How long a sign-in may take between leaving for the platform and coming back.
pub fn state_ttl() -> Duration {
    Duration::minutes(10)
}

pub struct Auth {
    cfg: AuthConfig,
    http: reqwest::Client,
    twitch: Option<Arc<Provider>>,
    kick: Option<Arc<Provider>>,
}

/// Where to send the browser to sign in, and the pre-login cookie to set on the way.
#[derive(Debug)]
pub struct Started {
    pub location: String,
    pub set_cookie: String,
}

/// A finished sign-in: where to send the browser and the cookies to set.
#[derive(Debug)]
pub struct Finished {
    pub location: String,
    pub set_cookies: Vec<String>,
    pub account_id: Uuid,
    /// The sign-in made a new account.
    pub created: bool,
    /// The identity was linked to the caller's account rather than signed in with.
    pub linked: bool,
}

/// The callback's query string.
#[derive(Clone, Debug, Default)]
pub struct Callback<'a> {
    pub code: Option<&'a str>,
    pub state: Option<&'a str>,
    pub error: Option<&'a str>,
    /// The browser's `User-Agent`, kept with the session to name the device (`mySessions`).
    pub user_agent: Option<&'a str>,
}

impl Auth {
    pub fn new(cfg: AuthConfig) -> Result<Self, reqwest::Error> {
        // reqwest is built without a crypto provider (see the workspace Cargo.toml); use ring.
        // Installing twice fails harmlessly, so a second `Auth` (tests) is fine.
        let _ = rustls::crypto::ring::default_provider().install_default();
        let http = reqwest::Client::builder()
            .timeout(providers::TIMEOUT)
            .redirect(reqwest::redirect::Policy::none())
            .user_agent("cyex.tv (+https://cyex.tv)")
            .build()?;
        let provider = |platform, pc: &Option<ProviderConfig>| {
            pc.as_ref().map(|pc| {
                Arc::new(Provider::new(
                    platform,
                    pc.creds.clone(),
                    pc.endpoints.clone(),
                ))
            })
        };
        Ok(Self {
            twitch: provider(Platform::Twitch, &cfg.twitch),
            kick: provider(Platform::Kick, &cfg.kick),
            http,
            cfg,
        })
    }

    pub fn config(&self) -> &AuthConfig {
        &self.cfg
    }

    fn provider(&self, platform: Platform) -> Result<&Arc<Provider>, AuthError> {
        match platform {
            Platform::Twitch => self.twitch.as_ref(),
            Platform::Kick => self.kick.as_ref(),
        }
        .ok_or(AuthError::NotConfigured)
    }

    /// The callback URL registered with the platform: `{public_url}/v1/auth/{platform}/callback`.
    pub fn redirect_uri(&self, platform: Platform) -> String {
        let base = self.cfg.public_url.as_str().trim_end_matches('/');
        format!("{base}/v1/auth/{platform}/callback")
    }

    /// Step 1. With `link`, the caller must be signed in with the site cookie, and the platform
    /// is linked to their account at the end instead of signed in with.
    pub async fn start(
        &self,
        db: &PgPool,
        platform: Platform,
        return_to: Option<&str>,
        link: bool,
        caller: Option<&Caller>,
    ) -> Result<Started, AuthError> {
        let provider = self.provider(platform)?;
        let return_to = return_to::resolve(&self.cfg, return_to)?;
        let link_account_id = match (link, caller) {
            (false, _) => None,
            (true, Some(caller)) if caller.via == Via::Cookie => Some(caller.account_id),
            (true, _) => return Err(AuthError::SignedOut),
        };

        let state = tokens::random_token();
        let verifier = tokens::random_token();
        let mut conn = db.acquire().await?;
        oauth_states::create(
            &mut conn,
            &tokens::hash(&state),
            &NewOAuthState {
                platform,
                code_verifier: &verifier,
                return_to: &return_to,
                link_account_id,
                ttl: state_ttl(),
            },
        )
        .await?;

        let challenge = provider
            .uses_pkce()
            .then(|| tokens::pkce_challenge(&verifier));
        let location = provider
            .authorize_url(&self.redirect_uri(platform), &state, challenge.as_deref())
            .into();
        Ok(Started {
            location,
            set_cookie: cookies::state(&self.cfg, &state, state_ttl()),
        })
    }

    /// Step 2. `cookie_state` is the pre-login cookie; `caller` is whoever the request's session
    /// cookie belongs to, if anyone (their session is replaced).
    pub async fn finish(
        &self,
        db: &PgPool,
        platform: Platform,
        query: &Callback<'_>,
        cookie_state: Option<&str>,
        caller: Option<&Caller>,
    ) -> Result<Finished, AuthError> {
        let provider = self.provider(platform)?;
        let state = query.state.filter(|s| tokens::is_well_formed(s));
        let stored = match state {
            Some(state) => {
                let mut conn = db.acquire().await?;
                oauth_states::take(&mut conn, &tokens::hash(state), platform).await?
            }
            None => None,
        };
        if query.error.is_some() {
            return Err(AuthError::Declined);
        }
        let (Some(state), Some(code), Some(stored)) = (state, query.code, stored) else {
            return Err(AuthError::BadState);
        };
        // The state must come back in the cookie of the browser that started the sign-in, so a
        // callback link sent to someone else signs nobody in.
        if cookie_state != Some(state) {
            return Err(AuthError::BadState);
        }
        if let Some(link_to) = stored.link_account_id {
            let same = caller.is_some_and(|c| c.via == Via::Cookie && c.account_id == link_to);
            if !same {
                return Err(AuthError::BadState);
            }
        }

        let redirect_uri = self.redirect_uri(platform);
        let verifier = provider
            .uses_pkce()
            .then_some(stored.code_verifier.as_str());
        let access_token = provider
            .exchange(&self.http, code, verifier, &redirect_uri)
            .await?;
        let user = provider.user(&self.http, &access_token).await;
        {
            let provider = Arc::clone(provider);
            let http = self.http.clone();
            tokio::spawn(async move { provider.revoke(&http, &access_token).await });
        }
        let user = user?;
        let profile = Profile {
            platform,
            platform_user_id: &user.id,
            login: &user.login,
            display_name: &user.display_name,
            avatar: user.avatar.as_deref(),
        };

        let mut conn = db.acquire().await?;
        let (account_id, created, linked) = match stored.link_account_id {
            Some(account_id) => {
                // The account was deleted while the link was under way.
                match accounts::link(&mut conn, account_id, profile).await {
                    Err(cyex_tv_db::DbError::NotFound) => return Err(AuthError::AccountDeleted),
                    other => other?,
                };
                // A hold the identity carried may have blocked the account.
                if accounts::get(&mut conn, account_id)
                    .await?
                    .is_some_and(|a| a.blocked_at.is_some())
                {
                    sessions::delete_for_account(&mut conn, account_id, None).await?;
                    return Err(AuthError::AccountBlocked);
                }
                (account_id, false, true)
            }
            None => {
                let signed_in = accounts::sign_in(&mut conn, profile).await?;
                if signed_in.account.deleted_at.is_some() {
                    return Err(AuthError::AccountDeleted);
                }
                if signed_in.account.blocked_at.is_some() {
                    return Err(AuthError::AccountBlocked);
                }
                // A new account starts with a channel set named after it (ADR 0017).
                if signed_in.created {
                    let name = match user.display_name.trim() {
                        "" => user.login.trim(),
                        name => name,
                    };
                    sets::create_default(&mut conn, signed_in.account.id, name).await?;
                }
                (signed_in.account.id, signed_in.created, false)
            }
        };

        // Every identity is a channel on its platform (a Twitch broadcaster's channel ID is
        // their user ID; Kick's `broadcaster_user_id` equals `user_id`).
        channels::upsert(&mut conn, account_id, platform, &user.id, &user.login).await?;

        // A fresh session on every sign-in and link, so a token planted before sign-in is
        // worth nothing after it.
        let token = tokens::random_token();
        let mut tx = conn.begin().await?;
        sessions::create_with(
            &mut tx,
            &tokens::hash(&token),
            account_id,
            SessionKind::Site,
            ttl(SessionKind::Site),
            query.user_agent,
        )
        .await?;
        if let Some(old) = caller.filter(|c| c.via == Via::Cookie) {
            sessions::delete(&mut tx, &old.token_hash).await?;
        }
        tx.commit().await?;

        Ok(Finished {
            location: stored.return_to,
            set_cookies: vec![
                cookies::session(&self.cfg, &token, ttl(SessionKind::Site)),
                cookies::clear_state(&self.cfg),
            ],
            account_id,
            created,
            linked,
        })
    }
}
