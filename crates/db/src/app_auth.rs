//! App sign-in (ADR 0021): authorization codes, the app session they turn into, and its rotating
//! refresh tokens. Tokens and codes never reach the database; callers pass their sha256.

use chrono::{DateTime, Duration, Utc};
use sqlx::{FromRow, PgConnection};
use uuid::Uuid;

use crate::error::DbResult;
use crate::sessions::{SESSION, Session, SessionKind, TokenHash, USER_AGENT_MAX};

/// An authorization code, as stored.
#[derive(Clone, Debug, PartialEq, Eq, FromRow)]
pub struct AuthCode {
    pub client_id: String,
    pub account_id: Uuid,
    pub redirect_uri: String,
    pub code_challenge: String,
    pub expires_at: DateTime<Utc>,
    pub used_at: Option<DateTime<Utc>>,
    /// The session the code was traded for, once it was.
    pub session_id: Option<Uuid>,
}

#[derive(Clone, Copy, Debug)]
pub struct NewAuthCode<'a> {
    pub client_id: &'a str,
    pub account_id: Uuid,
    pub redirect_uri: &'a str,
    pub code_challenge: &'a str,
    pub ttl: Duration,
}

/// Stores a code. Codes that expired over a day ago are cleared on the way, so the table stays
/// small without a job.
pub async fn create_code(
    conn: &mut PgConnection,
    hash: &TokenHash,
    new: &NewAuthCode<'_>,
) -> DbResult<()> {
    sqlx::query("DELETE FROM app_auth_codes WHERE expires_at < now() - interval '1 day'")
        .execute(&mut *conn)
        .await?;
    sqlx::query(
        "INSERT INTO app_auth_codes (code_hash, client_id, account_id, redirect_uri, code_challenge, expires_at)
         VALUES ($1, $2, $3, $4, $5, now() + $6)",
    )
    .bind(&hash[..])
    .bind(new.client_id)
    .bind(new.account_id)
    .bind(new.redirect_uri)
    .bind(new.code_challenge)
    .bind(new.ttl)
    .execute(conn)
    .await?;
    Ok(())
}

/// Locks and returns a code for the exchange, used or not; the caller decides. Run it inside a
/// transaction, so two exchanges of one code can't both pass.
pub async fn lock_code(conn: &mut PgConnection, hash: &TokenHash) -> DbResult<Option<AuthCode>> {
    Ok(sqlx::query_as(
        "SELECT client_id, account_id, redirect_uri, code_challenge, expires_at, used_at, session_id
         FROM app_auth_codes WHERE code_hash = $1 FOR UPDATE",
    )
    .bind(&hash[..])
    .fetch_optional(conn)
    .await?)
}

/// Marks a code used, noting the session it gave (if any).
pub async fn use_code(
    conn: &mut PgConnection,
    hash: &TokenHash,
    session_id: Option<Uuid>,
) -> DbResult<()> {
    sqlx::query(
        "UPDATE app_auth_codes SET used_at = coalesce(used_at, now()), session_id = $2
         WHERE code_hash = $1",
    )
    .bind(&hash[..])
    .bind(session_id)
    .execute(conn)
    .await?;
    Ok(())
}

/// Whether the account may still sign in: not deleted, not blocked.
pub async fn account_is_live(conn: &mut PgConnection, account_id: Uuid) -> DbResult<bool> {
    Ok(sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM accounts WHERE id = $1 AND deleted_at IS NULL
                          AND (blocked_at IS NULL OR blocked_until <= now()))",
    )
    .bind(account_id)
    .fetch_one(conn)
    .await?)
}

/// A new app session for `client_id`: the access token lives `access_ttl`, the refresh token
/// `refresh_ttl` (the session row's `expires_at`).
pub async fn create_session(
    conn: &mut PgConnection,
    access_hash: &TokenHash,
    account_id: Uuid,
    client_id: &str,
    access_ttl: Duration,
    refresh_ttl: Duration,
    user_agent: Option<&str>,
) -> DbResult<Session> {
    let user_agent: Option<String> = user_agent
        .map(str::trim)
        .filter(|ua| !ua.is_empty())
        .map(|ua| ua.chars().take(USER_AGENT_MAX).collect());
    Ok(sqlx::query_as(&format!(
        "INSERT INTO sessions (id_hash, account_id, kind, expires_at, id, user_agent, app_client_id,
                               access_expires_at)
         VALUES ($1, $2, $3, now() + $4, $5, $6, $7, now() + $8)
         RETURNING {SESSION}"
    ))
    .bind(&access_hash[..])
    .bind(account_id)
    .bind(SessionKind::App)
    .bind(refresh_ttl)
    .bind(crate::ids::new_id())
    .bind(user_agent)
    .bind(client_id)
    .bind(access_ttl)
    .fetch_one(conn)
    .await?)
}

/// Stores a refresh token for a session.
pub async fn create_refresh(
    conn: &mut PgConnection,
    hash: &TokenHash,
    session_id: Uuid,
) -> DbResult<()> {
    sqlx::query("INSERT INTO app_refresh_tokens (token_hash, session_id) VALUES ($1, $2)")
        .bind(&hash[..])
        .bind(session_id)
        .execute(conn)
        .await?;
    Ok(())
}

/// A refresh token, as stored.
#[derive(Clone, Debug, PartialEq, Eq, FromRow)]
pub struct RefreshToken {
    pub session_id: Uuid,
    pub used_at: Option<DateTime<Utc>>,
    /// The session's client.
    pub client_id: Option<String>,
}

/// Locks and returns a refresh token with its session's client. Run it inside a transaction.
pub async fn lock_refresh(
    conn: &mut PgConnection,
    hash: &TokenHash,
) -> DbResult<Option<RefreshToken>> {
    Ok(sqlx::query_as(
        "SELECT r.session_id, r.used_at, s.app_client_id AS client_id
         FROM app_refresh_tokens r JOIN sessions s ON s.id = r.session_id
         WHERE r.token_hash = $1 FOR UPDATE OF r",
    )
    .bind(&hash[..])
    .fetch_optional(conn)
    .await?)
}

/// Marks a refresh token used, and forgets the session's tokens used more than `keep_used` ago.
pub async fn use_refresh(
    conn: &mut PgConnection,
    hash: &TokenHash,
    session_id: Uuid,
    keep_used: Duration,
) -> DbResult<()> {
    sqlx::query("UPDATE app_refresh_tokens SET used_at = now() WHERE token_hash = $1")
        .bind(&hash[..])
        .execute(&mut *conn)
        .await?;
    sqlx::query(
        "DELETE FROM app_refresh_tokens
         WHERE session_id = $1 AND used_at IS NOT NULL AND used_at < now() - $2",
    )
    .bind(session_id)
    .bind(keep_used)
    .execute(conn)
    .await?;
    Ok(())
}

/// Gives a live app session a new access token and slides its refresh window. `None` when the
/// session has ended (refresh window over) or its account is deleted or blocked.
pub async fn rotate_session(
    conn: &mut PgConnection,
    session_id: Uuid,
    access_hash: &TokenHash,
    access_ttl: Duration,
    refresh_ttl: Duration,
) -> DbResult<Option<Session>> {
    Ok(sqlx::query_as(
        "UPDATE sessions s
         SET id_hash = $2, access_expires_at = now() + $3, expires_at = now() + $4,
             last_seen_at = now()
         FROM accounts a
         WHERE s.id = $1 AND s.app_client_id IS NOT NULL AND s.expires_at > now()
           AND a.id = s.account_id AND a.deleted_at IS NULL
           AND (a.blocked_at IS NULL OR a.blocked_until <= now())
         RETURNING s.id_hash, s.account_id, s.kind, s.created_at, s.expires_at, s.last_seen_at,
                   s.id, s.user_agent, s.app_client_id, s.access_expires_at",
    )
    .bind(session_id)
    .bind(&access_hash[..])
    .bind(access_ttl)
    .bind(refresh_ttl)
    .fetch_optional(conn)
    .await?)
}

/// Signs a session out by its public ID (its refresh tokens go with it). True if it existed.
pub async fn delete_session(conn: &mut PgConnection, session_id: Uuid) -> DbResult<bool> {
    let done = sqlx::query("DELETE FROM sessions WHERE id = $1")
        .bind(session_id)
        .execute(conn)
        .await?;
    Ok(done.rows_affected() > 0)
}

/// The session a refresh token belongs to, used or not.
pub async fn session_of_refresh(
    conn: &mut PgConnection,
    hash: &TokenHash,
) -> DbResult<Option<Uuid>> {
    Ok(
        sqlx::query_scalar("SELECT session_id FROM app_refresh_tokens WHERE token_hash = $1")
            .bind(&hash[..])
            .fetch_optional(conn)
            .await?,
    )
}

/// The app sign-in session an access token belongs to, expired access or not.
pub async fn session_of_access(
    conn: &mut PgConnection,
    hash: &TokenHash,
) -> DbResult<Option<Uuid>> {
    Ok(sqlx::query_scalar(
        "SELECT id FROM sessions WHERE id_hash = $1 AND app_client_id IS NOT NULL",
    )
    .bind(&hash[..])
    .fetch_optional(conn)
    .await?)
}
