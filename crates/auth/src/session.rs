//! Who is calling: a site session from the cookie, or an app session from a Bearer token.

use axum::http::header::AUTHORIZATION;
use axum::http::{HeaderMap, Method};
use chrono::{Duration, Utc};
use cyex_tv_db::sessions::{self, SessionKind, TokenHash};
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::AuthError;
use crate::flow::Auth;
use crate::{cookies, origin, tokens};

/// How the caller proved who they are.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Via {
    Cookie,
    Bearer,
}

/// A signed-in caller.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Caller {
    pub account_id: Uuid,
    /// Named by `CYEX_TV_OWNER_IDS`: holds every permission (ADR 0010).
    pub owner: bool,
    pub token_hash: TokenHash,
    pub kind: SessionKind,
    pub via: Via,
    /// The app that signed in through `/v1/auth/app` (ADR 0021). Such a session may run only the
    /// mutations its scopes name (`app::allows_mutation`).
    pub app_client: Option<String>,
}

/// How long a session lasts after its last use (ADR 0009): site 30 days, app 90 days.
pub fn ttl(kind: SessionKind) -> Duration {
    match kind {
        SessionKind::Site => Duration::days(30),
        SessionKind::App => Duration::days(90),
    }
}

/// A session is written back (`last_seen_at`, sliding expiry) at most this often.
pub fn touch_after() -> Duration {
    Duration::hours(1)
}

/// The token from `Authorization: Bearer <token>`, if the header has that form.
fn bearer(headers: &HeaderMap) -> Option<&str> {
    let value = headers.get(AUTHORIZATION)?.to_str().ok()?;
    let (scheme, token) = value.split_once(' ')?;
    scheme.eq_ignore_ascii_case("bearer").then(|| token.trim())
}

impl Auth {
    /// The caller, if the request carries a live session.
    ///
    /// - A Bearer token must be an app sign-in's session (one that names its client, ADR 0021);
    ///   when one is sent the cookie is ignored.
    /// - The cookie must be a site session. On a write (anything but GET, HEAD and OPTIONS) it
    ///   counts only with `Origin` set to the site; otherwise the request fails with `bad_origin`
    ///   rather than quietly running signed out.
    pub async fn caller(
        &self,
        db: &PgPool,
        method: &Method,
        headers: &HeaderMap,
    ) -> Result<Option<Caller>, AuthError> {
        let (token, kind, via) = if let Some(token) = bearer(headers) {
            (token.to_owned(), SessionKind::App, Via::Bearer)
        } else if let Some(token) = cookies::read(headers, cookies::SESSION) {
            if !origin::is_safe(method) && !origin::is_allowed(self.config(), headers) {
                return Err(AuthError::BadOrigin);
            }
            (token, SessionKind::Site, Via::Cookie)
        } else {
            return Ok(None);
        };
        if !tokens::is_well_formed(&token) {
            return Ok(None);
        }

        let hash = tokens::hash(&token);
        let mut conn = db.acquire().await?;
        let Some(session) = sessions::find_active(&mut conn, &hash).await? else {
            return Ok(None);
        };
        if session.kind != kind {
            return Ok(None);
        }
        // Unscoped app sessions are retired (ADR 0021, amendment of 2026/11/20): a Bearer token
        // counts only for an app that signed in through `/v1/auth/app`. The database refuses
        // new ones too; this is the second line.
        if via == Via::Bearer && session.app_client_id.is_none() {
            return Ok(None);
        }
        if Utc::now() - session.last_seen_at > touch_after()
            && let Err(err) = sessions::touch(&mut conn, &hash, ttl(kind)).await
        {
            tracing::warn!(error = %err, "session touch failed");
        }
        // An app sign-in session (ADR 0021) is never the owner: staff grants don't count for apps.
        let owner =
            session.app_client_id.is_none() && self.is_owner(&mut conn, session.account_id).await?;
        Ok(Some(Caller {
            account_id: session.account_id,
            owner,
            token_hash: hash,
            kind,
            via,
            app_client: session.app_client_id,
        }))
    }

    /// The cookie's session when its account's deletion is pending and can still be restored
    /// (ADR 0019). `caller` never answers with these; this one may only see and undo the
    /// deletion, or sign out. Cookie only, with the same `Origin` rule.
    pub async fn restorable_caller(
        &self,
        db: &PgPool,
        method: &Method,
        headers: &HeaderMap,
    ) -> Result<Option<Caller>, AuthError> {
        if bearer(headers).is_some() {
            return Ok(None);
        }
        let Some(token) = cookies::read(headers, cookies::SESSION) else {
            return Ok(None);
        };
        if !origin::is_safe(method) && !origin::is_allowed(self.config(), headers) {
            return Err(AuthError::BadOrigin);
        }
        if !tokens::is_well_formed(&token) {
            return Ok(None);
        }
        let hash = tokens::hash(&token);
        let mut conn = db.acquire().await?;
        let Some(session) = sessions::find_restorable(&mut conn, &hash).await? else {
            return Ok(None);
        };
        Ok(Some(Caller {
            account_id: session.account_id,
            owner: false,
            token_hash: hash,
            kind: SessionKind::Site,
            via: Via::Cookie,
            app_client: None,
        }))
    }

    /// Whether the account is one of the owners in `CYEX_TV_OWNER_IDS`.
    pub async fn is_owner(
        &self,
        conn: &mut sqlx::PgConnection,
        account_id: Uuid,
    ) -> Result<bool, AuthError> {
        let owners = &self.config().owners;
        if owners.is_empty() {
            return Ok(false);
        }
        Ok(
            cyex_tv_db::accounts::is_any_of(conn, account_id, &owners.identities, &owners.accounts)
                .await?,
        )
    }

    /// Ends the caller's session. Returns the `Set-Cookie` that clears the cookie, which is sent
    /// even when there was no session, so a stale cookie goes away too.
    pub async fn sign_out(
        &self,
        db: &PgPool,
        caller: Option<&Caller>,
    ) -> Result<String, AuthError> {
        if let Some(caller) = caller {
            let mut conn = db.acquire().await?;
            sessions::delete(&mut conn, &caller.token_hash).await?;
        }
        Ok(cookies::clear_session(self.config()))
    }
}

#[cfg(test)]
mod tests {
    use axum::http::HeaderValue;

    use super::*;

    #[test]
    fn bearer_is_parsed() {
        let mut headers = HeaderMap::new();
        assert_eq!(bearer(&headers), None);
        headers.insert(AUTHORIZATION, HeaderValue::from_static("Bearer abc"));
        assert_eq!(bearer(&headers), Some("abc"));
        headers.insert(AUTHORIZATION, HeaderValue::from_static("bearer  abc "));
        assert_eq!(bearer(&headers), Some("abc"));
        headers.insert(AUTHORIZATION, HeaderValue::from_static("Basic abc"));
        assert_eq!(bearer(&headers), None);
    }

    #[test]
    fn lifetimes_match_adr_0009() {
        assert_eq!(ttl(SessionKind::Site), Duration::days(30));
        assert_eq!(ttl(SessionKind::App), Duration::days(90));
    }
}
