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
}

impl SessionKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Site => "site",
            Self::App => "app",
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
}

const SESSION: &str = "id_hash, account_id, kind, created_at, expires_at, last_seen_at";

pub async fn create(
    conn: &mut PgConnection,
    hash: &TokenHash,
    account_id: Uuid,
    kind: SessionKind,
    ttl: Duration,
) -> DbResult<Session> {
    Ok(sqlx::query_as(&format!(
        "INSERT INTO sessions (id_hash, account_id, kind, expires_at)
         VALUES ($1, $2, $3, now() + $4)
         RETURNING {SESSION}"
    ))
    .bind(&hash[..])
    .bind(account_id)
    .bind(kind)
    .bind(ttl)
    .fetch_one(conn)
    .await?)
}

/// The session for a token, if it hasn't expired and its account isn't deleted.
pub async fn find_active(conn: &mut PgConnection, hash: &TokenHash) -> DbResult<Option<Session>> {
    Ok(sqlx::query_as(
        "SELECT s.id_hash, s.account_id, s.kind, s.created_at, s.expires_at, s.last_seen_at
         FROM sessions s
         JOIN accounts a ON a.id = s.account_id
         WHERE s.id_hash = $1 AND s.expires_at > now() AND a.deleted_at IS NULL",
    )
    .bind(&hash[..])
    .fetch_optional(conn)
    .await?)
}

/// Marks the session as used and slides its expiry to `now + ttl` (never shortens it). Callers
/// decide how often to call it, so a busy client doesn't write on every request.
pub async fn touch(
    conn: &mut PgConnection,
    hash: &TokenHash,
    ttl: Duration,
) -> DbResult<Option<Session>> {
    Ok(sqlx::query_as(&format!(
        "UPDATE sessions
         SET last_seen_at = now(), expires_at = greatest(expires_at, now() + $2)
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
