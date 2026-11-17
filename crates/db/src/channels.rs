//! Channels: one row per Twitch or Kick channel, with its active set and settings (ADR 0009).

use chrono::{DateTime, Utc};
use cyex_tv_shared::model::Platform;
use sqlx::{FromRow, PgConnection};
use uuid::Uuid;

use crate::error::{DbError, DbResult};
use crate::ids::new_id;

#[derive(Clone, Debug, PartialEq, Eq, FromRow)]
pub struct Channel {
    pub id: Uuid,
    pub account_id: Uuid,
    pub platform: Platform,
    /// Twitch: the broadcaster's user ID. Kick: `broadcaster_user_id`.
    pub platform_channel_id: String,
    pub login: String,
    pub active_set_id: Option<Uuid>,
    pub allow_personal_emotes: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

const CHANNEL: &str = "id, account_id, platform, platform_channel_id, login, active_set_id, \
                       allow_personal_emotes, created_at, updated_at";

/// Makes the channel for a broadcaster's identity, or refreshes its login and owner.
pub async fn upsert(
    conn: &mut PgConnection,
    account_id: Uuid,
    platform: Platform,
    platform_channel_id: &str,
    login: &str,
) -> DbResult<Channel> {
    Ok(sqlx::query_as(&format!(
        "INSERT INTO channels (id, account_id, platform, platform_channel_id, login)
         VALUES ($1, $2, $3, $4, $5)
         ON CONFLICT (platform, platform_channel_id) DO UPDATE
         SET account_id = EXCLUDED.account_id, login = EXCLUDED.login, updated_at = now()
         RETURNING {CHANNEL}"
    ))
    .bind(new_id())
    .bind(account_id)
    .bind(platform)
    .bind(platform_channel_id)
    .bind(login)
    .fetch_one(conn)
    .await?)
}

pub async fn get(conn: &mut PgConnection, id: Uuid) -> DbResult<Option<Channel>> {
    Ok(
        sqlx::query_as(&format!("SELECT {CHANNEL} FROM channels WHERE id = $1"))
            .bind(id)
            .fetch_optional(conn)
            .await?,
    )
}

/// Channels by ID, in no particular order. Unknown IDs are left out.
pub async fn get_many(conn: &mut PgConnection, ids: &[Uuid]) -> DbResult<Vec<Channel>> {
    Ok(sqlx::query_as(&format!(
        "SELECT {CHANNEL} FROM channels WHERE id = ANY($1)"
    ))
    .bind(ids)
    .fetch_all(conn)
    .await?)
}

pub async fn find(
    conn: &mut PgConnection,
    platform: Platform,
    platform_channel_id: &str,
) -> DbResult<Option<Channel>> {
    Ok(sqlx::query_as(&format!(
        "SELECT {CHANNEL} FROM channels WHERE platform = $1 AND platform_channel_id = $2"
    ))
    .bind(platform)
    .bind(platform_channel_id)
    .fetch_optional(conn)
    .await?)
}

/// Looks a channel up by login, ignoring case. Logins can move between channels, so the most
/// recently refreshed one wins.
pub async fn find_by_login(
    conn: &mut PgConnection,
    platform: Platform,
    login: &str,
) -> DbResult<Option<Channel>> {
    Ok(sqlx::query_as(&format!(
        "SELECT {CHANNEL} FROM channels
         WHERE platform = $1 AND lower(login) = lower($2)
         ORDER BY updated_at DESC
         LIMIT 1"
    ))
    .bind(platform)
    .bind(login)
    .fetch_optional(conn)
    .await?)
}

pub async fn list_for_account(conn: &mut PgConnection, account_id: Uuid) -> DbResult<Vec<Channel>> {
    Ok(sqlx::query_as(&format!(
        "SELECT {CHANNEL} FROM channels WHERE account_id = $1 ORDER BY platform DESC"
    ))
    .bind(account_id)
    .fetch_all(conn)
    .await?)
}

/// Changes the channel's active set, or clears it. Whether the caller may use that set is the
/// api's check.
pub async fn set_active_set(
    conn: &mut PgConnection,
    id: Uuid,
    set_id: Option<Uuid>,
) -> DbResult<Channel> {
    sqlx::query_as(&format!(
        "UPDATE channels SET active_set_id = $2, updated_at = now() WHERE id = $1 RETURNING {CHANNEL}"
    ))
    .bind(id)
    .bind(set_id)
    .fetch_optional(conn)
    .await?
    .ok_or(DbError::NotFound)
}

pub async fn set_allow_personal_emotes(
    conn: &mut PgConnection,
    id: Uuid,
    allow: bool,
) -> DbResult<Channel> {
    sqlx::query_as(&format!(
        "UPDATE channels SET allow_personal_emotes = $2, updated_at = now()
         WHERE id = $1
         RETURNING {CHANNEL}"
    ))
    .bind(id)
    .bind(allow)
    .fetch_optional(conn)
    .await?
    .ok_or(DbError::NotFound)
}

/// The channels showing a set as their active one.
pub async fn using_set(conn: &mut PgConnection, set_id: Uuid) -> DbResult<Vec<Channel>> {
    Ok(sqlx::query_as(&format!(
        "SELECT {CHANNEL} FROM channels WHERE active_set_id = $1 ORDER BY id"
    ))
    .bind(set_id)
    .fetch_all(conn)
    .await?)
}

/// Removes the account's channel on `platform`, when that identity is unlinked. True if there
/// was one.
pub async fn delete_for(
    conn: &mut PgConnection,
    account_id: Uuid,
    platform: Platform,
) -> DbResult<bool> {
    let done = sqlx::query("DELETE FROM channels WHERE account_id = $1 AND platform = $2")
        .bind(account_id)
        .bind(platform)
        .execute(conn)
        .await?;
    Ok(done.rows_affected() > 0)
}
