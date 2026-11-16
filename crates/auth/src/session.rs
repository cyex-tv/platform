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
    /// - A Bearer token must be an app session; when one is sent the cookie is ignored.
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
        if Utc::now() - session.last_seen_at > touch_after()
            && let Err(err) = sessions::touch(&mut conn, &hash, ttl(kind)).await
        {
            tracing::warn!(error = %err, "session touch failed");
        }
        let owner = self.is_owner(&mut conn, session.account_id).await?;
        Ok(Some(Caller {
            account_id: session.account_id,
            owner,
            token_hash: hash,
            kind,
            via,
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
