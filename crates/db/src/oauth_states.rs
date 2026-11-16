//! The state of a sign-in in progress (ADR 0009): stored at `login`, taken once at `callback`.

use chrono::{DateTime, Duration, Utc};
use cyex_tv_shared::model::Platform;
use sqlx::{FromRow, PgConnection};
use uuid::Uuid;

use crate::error::DbResult;
use crate::sessions::TokenHash;

#[derive(Clone, Debug, PartialEq, Eq, FromRow)]
pub struct OAuthState {
    pub platform: Platform,
    pub code_verifier: String,
    pub return_to: String,
    pub link_account_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

#[derive(Clone, Copy, Debug)]
pub struct NewOAuthState<'a> {
    pub platform: Platform,
    pub code_verifier: &'a str,
    pub return_to: &'a str,
    pub link_account_id: Option<Uuid>,
    pub ttl: Duration,
}

pub async fn create(
    conn: &mut PgConnection,
    hash: &TokenHash,
    new: &NewOAuthState<'_>,
) -> DbResult<()> {
    sqlx::query(
        "INSERT INTO oauth_states (state_hash, platform, code_verifier, return_to, link_account_id, expires_at)
         VALUES ($1, $2, $3, $4, $5, now() + $6)",
    )
    .bind(&hash[..])
    .bind(new.platform)
    .bind(new.code_verifier)
    .bind(new.return_to)
    .bind(new.link_account_id)
    .bind(new.ttl)
    .execute(conn)
    .await?;
    Ok(())
}

/// Removes and returns the state, if it exists, is for `platform` and hasn't expired. A state can
/// be taken only once, so a replayed callback finds nothing.
pub async fn take(
    conn: &mut PgConnection,
    hash: &TokenHash,
    platform: Platform,
) -> DbResult<Option<OAuthState>> {
    let row: Option<OAuthState> = sqlx::query_as(
        "DELETE FROM oauth_states WHERE state_hash = $1
         RETURNING platform, code_verifier, return_to, link_account_id, created_at, expires_at",
    )
    .bind(&hash[..])
    .fetch_optional(conn)
    .await?;
    Ok(row.filter(|s| s.platform == platform && s.expires_at > Utc::now()))
}

/// Removes states nobody came back for. Run on a schedule.
pub async fn delete_expired(conn: &mut PgConnection) -> DbResult<u64> {
    let done = sqlx::query("DELETE FROM oauth_states WHERE expires_at <= now()")
        .execute(conn)
        .await?;
    Ok(done.rows_affected())
}
