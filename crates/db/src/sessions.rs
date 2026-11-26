//! Site and app sessions (ADR 0009). The token itself never reaches the database; callers pass its
//! sha256.

use chrono::{DateTime, Duration, Utc};
use sqlx::encode::IsNull;
use sqlx::error::BoxDynError;
use sqlx::postgres::{PgArgumentBuffer, PgTypeInfo, PgValueRef};
use sqlx::{Decode, Encode, FromRow, PgConnection, Postgres, Type};
use uuid::Uuid;

use crate::error::DbResult;

/// The sha256 of a session token.
pub type TokenHash = [u8; 32];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SessionKind {
    /// The cyex.tv cookie.
    Site,
    /// A Bearer token for the deck or another app.
    App,
    /// A personal Bearer token the person minted on the website (ADR 0028), for using the API as
    /// themselves. One per account, 3 days, never slid, ended whenever the account signs a
    /// session out (ADR 0029).
    Console,
}

impl SessionKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Site => "site",
            Self::App => "app",
            Self::Console => "console",
        }
    }
}

impl Type<Postgres> for SessionKind {
    fn type_info() -> PgTypeInfo {
        <&str as Type<Postgres>>::type_info()
    }

    fn compatible(ty: &PgTypeInfo) -> bool {
        <&str as Type<Postgres>>::compatible(ty)
    }
}

impl Encode<'_, Postgres> for SessionKind {
    fn encode_by_ref(&self, buf: &mut PgArgumentBuffer) -> Result<IsNull, BoxDynError> {
        <&str as Encode<Postgres>>::encode(self.as_str(), buf)
    }
}

impl<'r> Decode<'r, Postgres> for SessionKind {
    fn decode(value: PgValueRef<'r>) -> Result<Self, BoxDynError> {
        match <&str as Decode<Postgres>>::decode(value)? {
            "site" => Ok(Self::Site),
            "app" => Ok(Self::App),
            "console" => Ok(Self::Console),
            other => Err(format!("unknown session kind: {other}").into()),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, FromRow)]
pub struct Session {
    pub id_hash: Vec<u8>,
    pub account_id: Uuid,
    pub kind: SessionKind,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub last_seen_at: DateTime<Utc>,
    /// The public handle (`mySessions`, `revokeSession`). Never the token or its hash.
    pub id: Uuid,
    pub user_agent: Option<String>,
    /// The app client that signed in through `/v1/auth/app` (ADR 0021); `None` otherwise.
    pub app_client_id: Option<String>,
    /// When the access token of an app sign-in (ADR 0021) dies; a refresh gives a new one.
    /// `expires_at` is then when the refresh token stops working. `None` for other sessions.
    pub access_expires_at: Option<DateTime<Utc>>,
}

pub(crate) const SESSION: &str = "id_hash, account_id, kind, created_at, expires_at, last_seen_at, id, user_agent,      app_client_id, access_expires_at";

/// The longest user agent kept.
pub const USER_AGENT_MAX: usize = 512;

/// Makes a session. Only `SessionKind::Site` works here: an `app` session must name its client
/// (`app_auth`, ADR 0021), and the database refuses one without (`sessions_app_has_client`); a
/// `console` session comes from [`replace_console`].
pub async fn create(
    conn: &mut PgConnection,
    hash: &TokenHash,
    account_id: Uuid,
    kind: SessionKind,
    ttl: Duration,
) -> DbResult<Session> {
    create_with(conn, hash, account_id, kind, ttl, None).await
}

/// [`create`], noting the user agent that signed in (cut to [`USER_AGENT_MAX`] characters).
pub async fn create_with(
    conn: &mut PgConnection,
    hash: &TokenHash,
    account_id: Uuid,
    kind: SessionKind,
    ttl: Duration,
    user_agent: Option<&str>,
) -> DbResult<Session> {
    let user_agent: Option<String> = user_agent
        .map(str::trim)
        .filter(|ua| !ua.is_empty())
        .map(|ua| ua.chars().take(USER_AGENT_MAX).collect());
    Ok(sqlx::query_as(&format!(
        "INSERT INTO sessions (id_hash, account_id, kind, expires_at, id, user_agent)
         VALUES ($1, $2, $3, now() + $4, $5, $6)
         RETURNING {SESSION}"
    ))
    .bind(&hash[..])
    .bind(account_id)
    .bind(kind)
    .bind(ttl)
    .bind(crate::ids::new_id())
    .bind(user_agent)
    .fetch_one(conn)
    .await?)
}

/// A new console token's session (ADR 0028) for the account, signing out the one it had (live or
/// expired). Run it in a transaction: the delete and the insert go together, and the unique index
/// `sessions_one_console` refuses a second one made at the same moment. Returns the new session
/// and the public ID of the one it replaced, if any.
pub async fn replace_console(
    conn: &mut PgConnection,
    hash: &TokenHash,
    account_id: Uuid,
    ttl: Duration,
    user_agent: Option<&str>,
) -> DbResult<(Session, Option<Uuid>)> {
    let old: Option<(Uuid, DateTime<Utc>)> = sqlx::query_as(
        "DELETE FROM sessions WHERE account_id = $1 AND kind = 'console' RETURNING id, expires_at",
    )
    .bind(account_id)
    .fetch_optional(&mut *conn)
    .await?;
    let session = create_with(
        conn,
        hash,
        account_id,
        SessionKind::Console,
        ttl,
        user_agent,
    )
    .await?;
    // An expired one was signed out already; only a live one counts as replaced.
    let replaced = old.and_then(|(id, expires)| (expires > Utc::now()).then_some(id));
    Ok((session, replaced))
}

/// Ends the account's console token (ADR 0029: it rotates whenever the person signs a session
/// out, one or all). The next one is made on request; the ended one never works again, since
/// every request looks its session up. Returns how many ended (0 or 1).
pub async fn end_console(conn: &mut PgConnection, account_id: Uuid) -> DbResult<u64> {
    let done = sqlx::query("DELETE FROM sessions WHERE account_id = $1 AND kind = 'console'")
        .bind(account_id)
        .execute(conn)
        .await?;
    Ok(done.rows_affected())
}

/// [`end_console`] for the account a session (by its public ID) belongs to, as an app signing
/// itself out (RFC 7009) knows only its session.
pub async fn end_console_of_session(conn: &mut PgConnection, session_id: Uuid) -> DbResult<u64> {
    let done = sqlx::query(
        "DELETE FROM sessions WHERE kind = 'console'
           AND account_id = (SELECT account_id FROM sessions WHERE id = $1)",
    )
    .bind(session_id)
    .execute(conn)
    .await?;
    Ok(done.rows_affected())
}

/// An account's live sessions, the most recently used first.
pub async fn list_for_account(conn: &mut PgConnection, account_id: Uuid) -> DbResult<Vec<Session>> {
    Ok(sqlx::query_as(&format!(
        "SELECT {SESSION} FROM sessions
         WHERE account_id = $1 AND expires_at > now()
         ORDER BY last_seen_at DESC, created_at DESC"
    ))
    .bind(account_id)
    .fetch_all(conn)
    .await?)
}

/// Signs out one of the account's sessions by its public ID. False when the account has no such
/// session: another account's ID signs nobody out.
pub async fn delete_by_id(conn: &mut PgConnection, account_id: Uuid, id: Uuid) -> DbResult<bool> {
    let done = sqlx::query("DELETE FROM sessions WHERE account_id = $1 AND id = $2")
        .bind(account_id)
        .bind(id)
        .execute(conn)
        .await?;
    Ok(done.rows_affected() > 0)
}

/// The session for a token, if it hasn't expired and its account isn't deleted or blocked.
pub async fn find_active(conn: &mut PgConnection, hash: &TokenHash) -> DbResult<Option<Session>> {
    Ok(sqlx::query_as(
        "SELECT s.id_hash, s.account_id, s.kind, s.created_at, s.expires_at, s.last_seen_at,
                s.id, s.user_agent, s.app_client_id, s.access_expires_at
         FROM sessions s
         JOIN accounts a ON a.id = s.account_id
         WHERE s.id_hash = $1 AND s.expires_at > now()
           AND (s.access_expires_at IS NULL OR s.access_expires_at > now())
           AND a.deleted_at IS NULL AND (a.blocked_at IS NULL OR a.blocked_until <= now())",
    )
    .bind(&hash[..])
    .fetch_optional(conn)
    .await?)
}

/// A site session of an account whose deletion is pending and can still be restored (ADR 0019).
/// `find_active` never returns these: such a session can only look at and undo the deletion.
pub async fn find_restorable(
    conn: &mut PgConnection,
    hash: &TokenHash,
) -> DbResult<Option<Session>> {
    Ok(sqlx::query_as(
        "SELECT s.id_hash, s.account_id, s.kind, s.created_at, s.expires_at, s.last_seen_at,
                s.id, s.user_agent, s.app_client_id, s.access_expires_at
         FROM sessions s
         JOIN accounts a ON a.id = s.account_id
         WHERE s.id_hash = $1 AND s.expires_at > now() AND s.kind = 'site'
           AND a.erase_after > now() AND (a.blocked_at IS NULL OR a.blocked_until <= now())",
    )
    .bind(&hash[..])
    .fetch_optional(conn)
    .await?)
}

/// Marks the session as used and slides its expiry to `now + ttl` (never shortens it). Callers
/// decide how often to call it, so a busy client doesn't write on every request. An app sign-in's
/// session (ADR 0021) keeps its expiry: only a refresh moves it. So does a console token
/// (ADR 0028): it ends when it was made to.
pub async fn touch(
    conn: &mut PgConnection,
    hash: &TokenHash,
    ttl: Duration,
) -> DbResult<Option<Session>> {
    Ok(sqlx::query_as(&format!(
        "UPDATE sessions
         SET last_seen_at = now(),
             expires_at = CASE WHEN access_expires_at IS NULL AND kind <> 'console'
                               THEN greatest(expires_at, now() + $2) ELSE expires_at END
         WHERE id_hash = $1 AND expires_at > now()
         RETURNING {SESSION}"
    ))
    .bind(&hash[..])
    .bind(ttl)
    .fetch_optional(conn)
    .await?)
}

/// Signs one session out. True if it existed.
pub async fn delete(conn: &mut PgConnection, hash: &TokenHash) -> DbResult<bool> {
    let done = sqlx::query("DELETE FROM sessions WHERE id_hash = $1")
        .bind(&hash[..])
        .execute(conn)
        .await?;
    Ok(done.rows_affected() > 0)
}

/// Signs an account out everywhere, except the session `keep` if given (rotation on a privilege
/// change keeps the caller's own new session).
pub async fn delete_for_account(
    conn: &mut PgConnection,
    account_id: Uuid,
    keep: Option<&TokenHash>,
) -> DbResult<u64> {
    let done = sqlx::query(
        "DELETE FROM sessions WHERE account_id = $1 AND ($2::bytea IS NULL OR id_hash <> $2)",
    )
    .bind(account_id)
    .bind(keep.map(|h| &h[..]))
    .execute(conn)
    .await?;
    Ok(done.rows_affected())
}

/// Removes expired sessions. Run on a schedule.
pub async fn delete_expired(conn: &mut PgConnection) -> DbResult<u64> {
    let done = sqlx::query("DELETE FROM sessions WHERE expires_at <= now()")
        .execute(conn)
        .await?;
    Ok(done.rows_affected())
}
