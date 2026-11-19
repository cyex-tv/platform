//! Accounts and their linked Twitch and Kick identities (ADR 0009).

use chrono::{DateTime, Utc};
use cyex_tv_shared::model::Platform;
use sqlx::{Connection, FromRow, PgConnection};
use uuid::Uuid;

use crate::error::{DbError, DbResult, unique_violation};
use crate::ids::new_id;

#[derive(Clone, Debug, PartialEq, Eq, FromRow)]
pub struct Account {
    pub id: Uuid,
    pub created_at: DateTime<Utc>,
    pub restricted_at: Option<DateTime<Utc>>,
    pub deleted_at: Option<DateTime<Utc>>,
    /// Blocked by `users.restrict`: can't sign in.
    pub blocked_at: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, PartialEq, Eq, FromRow)]
pub struct Identity {
    pub platform: Platform,
    pub platform_user_id: String,
    pub account_id: Uuid,
    pub login: String,
    pub display_name: String,
    pub avatar: Option<String>,
    pub linked_at: DateTime<Utc>,
    pub refreshed_at: DateTime<Utc>,
}

/// What a platform's user endpoint told us at sign-in.
#[derive(Clone, Copy, Debug)]
pub struct Profile<'a> {
    pub platform: Platform,
    pub platform_user_id: &'a str,
    pub login: &'a str,
    pub display_name: &'a str,
    pub avatar: Option<&'a str>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SignIn {
    pub account: Account,
    pub identity: Identity,
    /// True when this sign-in made the account.
    pub created: bool,
}

const ACCOUNT: &str = "id, created_at, restricted_at, deleted_at, blocked_at";
const IDENTITY: &str =
    "platform, platform_user_id, account_id, login, display_name, avatar, linked_at, refreshed_at";

/// Signs in with a platform identity: refreshes the profile of a known identity, or makes a new
/// account for an unknown one. Two first sign-ins racing each other end up on the same account.
pub async fn sign_in(conn: &mut PgConnection, profile: Profile<'_>) -> DbResult<SignIn> {
    if let Some(identity) = refresh(conn, profile).await? {
        return existing(conn, identity).await;
    }

    let mut tx = conn.begin().await?;
    let account: Account = sqlx::query_as(&format!(
        "INSERT INTO accounts (id) VALUES ($1) RETURNING {ACCOUNT}"
    ))
    .bind(new_id())
    .fetch_one(&mut *tx)
    .await?;
    let identity: Option<Identity> = sqlx::query_as(&format!(
        "INSERT INTO identities (platform, platform_user_id, account_id, login, display_name, avatar)
         VALUES ($1, $2, $3, $4, $5, $6)
         ON CONFLICT (platform, platform_user_id) DO NOTHING
         RETURNING {IDENTITY}"
    ))
    .bind(profile.platform)
    .bind(profile.platform_user_id)
    .bind(account.id)
    .bind(profile.login)
    .bind(profile.display_name)
    .bind(profile.avatar)
    .fetch_optional(&mut *tx)
    .await?;

    match identity {
        Some(identity) => {
            // An identity that left a restricted or blocked account brings that along.
            let account = apply_hold(&mut tx, account, profile).await?;
            tx.commit().await?;
            Ok(SignIn {
                account,
                identity,
                created: true,
            })
        }
        // Someone signed in with the same identity a moment ago: use that account.
        None => {
            tx.rollback().await?;
            let identity = refresh(conn, profile).await?.ok_or(DbError::NotFound)?;
            existing(conn, identity).await
        }
    }
}

async fn existing(conn: &mut PgConnection, identity: Identity) -> DbResult<SignIn> {
    let account = get(conn, identity.account_id)
        .await?
        .ok_or(DbError::NotFound)?;
    Ok(SignIn {
        account,
        identity,
        created: false,
    })
}

/// Updates a known identity's login, name and avatar. `None` when the identity is unknown.
async fn refresh(conn: &mut PgConnection, profile: Profile<'_>) -> DbResult<Option<Identity>> {
    Ok(sqlx::query_as(&format!(
        "UPDATE identities
         SET login = $3, display_name = $4, avatar = $5, refreshed_at = now()
         WHERE platform = $1 AND platform_user_id = $2
         RETURNING {IDENTITY}"
    ))
    .bind(profile.platform)
    .bind(profile.platform_user_id)
    .bind(profile.login)
    .bind(profile.display_name)
    .bind(profile.avatar)
    .fetch_optional(conn)
    .await?)
}

pub async fn get(conn: &mut PgConnection, id: Uuid) -> DbResult<Option<Account>> {
    Ok(
        sqlx::query_as(&format!("SELECT {ACCOUNT} FROM accounts WHERE id = $1"))
            .bind(id)
            .fetch_optional(conn)
            .await?,
    )
}

/// An account's identities, Twitch first.
pub async fn identities(conn: &mut PgConnection, account_id: Uuid) -> DbResult<Vec<Identity>> {
    Ok(sqlx::query_as(&format!(
        "SELECT {IDENTITY} FROM identities WHERE account_id = $1 ORDER BY platform DESC"
    ))
    .bind(account_id)
    .fetch_all(conn)
    .await?)
}

/// The identities of many accounts at once, Twitch first, by account.
pub async fn identities_many(
    conn: &mut PgConnection,
    account_ids: &[Uuid],
) -> DbResult<std::collections::HashMap<Uuid, Vec<Identity>>> {
    let rows: Vec<Identity> = sqlx::query_as(&format!(
        "SELECT {IDENTITY} FROM identities WHERE account_id = ANY($1)
         ORDER BY account_id, platform DESC"
    ))
    .bind(account_ids)
    .fetch_all(conn)
    .await?;
    let mut out: std::collections::HashMap<Uuid, Vec<Identity>> = std::collections::HashMap::new();
    for row in rows {
        out.entry(row.account_id).or_default().push(row);
    }
    Ok(out)
}

pub async fn find_identity(
    conn: &mut PgConnection,
    platform: Platform,
    platform_user_id: &str,
) -> DbResult<Option<Identity>> {
    Ok(sqlx::query_as(&format!(
        "SELECT {IDENTITY} FROM identities WHERE platform = $1 AND platform_user_id = $2"
    ))
    .bind(platform)
    .bind(platform_user_id)
    .fetch_optional(conn)
    .await?)
}

/// The account whose identity on `platform` has this login (case-insensitive). Logins change and
/// can be reused, so the identity refreshed last wins.
pub async fn find_by_login(
    conn: &mut PgConnection,
    platform: Platform,
    login: &str,
) -> DbResult<Option<Identity>> {
    Ok(sqlx::query_as(&format!(
        "SELECT {IDENTITY} FROM identities
         WHERE platform = $1 AND lower(login) = lower($2)
         ORDER BY refreshed_at DESC
         LIMIT 1"
    ))
    .bind(platform)
    .bind(login)
    .fetch_optional(conn)
    .await?)
}

/// Copies the hold an identity left behind (see the migration `identity_holds`) onto `account`,
/// and uses it up. The account as it is afterwards.
async fn apply_hold(
    conn: &mut PgConnection,
    account: Account,
    profile: Profile<'_>,
) -> DbResult<Account> {
    type Times = (Option<DateTime<Utc>>, Option<DateTime<Utc>>);
    let hold: Option<Times> = sqlx::query_as(
        "DELETE FROM identity_holds WHERE platform = $1 AND platform_user_id = $2
         RETURNING restricted_at, blocked_at",
    )
    .bind(profile.platform)
    .bind(profile.platform_user_id)
    .fetch_optional(&mut *conn)
    .await?;
    let Some((restricted_at, blocked_at)) = hold else {
        return Ok(account);
    };
    Ok(sqlx::query_as(&format!(
        "UPDATE accounts
         SET restricted_at = coalesce(restricted_at, $2), blocked_at = coalesce(blocked_at, $3)
         WHERE id = $1
         RETURNING {ACCOUNT}"
    ))
    .bind(account.id)
    .bind(restricted_at)
    .bind(blocked_at)
    .fetch_one(conn)
    .await?)
}

/// Links another platform's identity to a signed-in account. Linking one that's already on this
/// account just refreshes it. The account row is held (`FOR SHARE`) and has to be live, so a
/// deletion running at the same time can't end up with an identity on a deleted account
/// (`NotFound`). A hold the identity left behind is copied onto the account.
pub async fn link(
    conn: &mut PgConnection,
    account_id: Uuid,
    profile: Profile<'_>,
) -> DbResult<Identity> {
    let mut tx = conn.begin().await?;
    let account: Account = sqlx::query_as(&format!(
        "SELECT {ACCOUNT} FROM accounts WHERE id = $1 AND deleted_at IS NULL FOR SHARE"
    ))
    .bind(account_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(DbError::NotFound)?;
    if let Some(found) = find_identity(&mut tx, profile.platform, profile.platform_user_id).await? {
        if found.account_id != account_id {
            return Err(DbError::IdentityTaken);
        }
        let identity = refresh(&mut tx, profile).await?.ok_or(DbError::NotFound)?;
        tx.commit().await?;
        return Ok(identity);
    }
    let identity: Identity = sqlx::query_as(&format!(
        "INSERT INTO identities (platform, platform_user_id, account_id, login, display_name, avatar)
         VALUES ($1, $2, $3, $4, $5, $6)
         RETURNING {IDENTITY}"
    ))
    .bind(profile.platform)
    .bind(profile.platform_user_id)
    .bind(account_id)
    .bind(profile.login)
    .bind(profile.display_name)
    .bind(profile.avatar)
    .fetch_one(&mut *tx)
    .await
    .map_err(|err| match unique_violation(&err) {
        Some("identities_pkey") => DbError::IdentityTaken,
        Some("identities_account_id_platform_key") => DbError::PlatformAlreadyLinked,
        _ => err.into(),
    })?;
    apply_hold(&mut tx, account, profile).await?;
    tx.commit().await?;
    Ok(identity)
}

/// Unlinks the account's identity on `platform`. The last identity stays.
pub async fn unlink(
    conn: &mut PgConnection,
    account_id: Uuid,
    platform: Platform,
) -> DbResult<Identity> {
    let mut tx = conn.begin().await?;
    // Lock the account's identities, so two unlinks at once can't remove both.
    let linked: Vec<Identity> = sqlx::query_as(&format!(
        "SELECT {IDENTITY} FROM identities WHERE account_id = $1 FOR UPDATE"
    ))
    .bind(account_id)
    .fetch_all(&mut *tx)
    .await?;
    if !linked.iter().any(|i| i.platform == platform) {
        return Err(DbError::NotFound);
    }
    if linked.len() == 1 {
        return Err(DbError::LastIdentity);
    }
    let removed: Identity = sqlx::query_as(&format!(
        "DELETE FROM identities WHERE account_id = $1 AND platform = $2 RETURNING {IDENTITY}"
    ))
    .bind(account_id)
    .bind(platform)
    .fetch_one(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(removed)
}

/// Restricts an account (`users.restrict`) or lifts the restriction.
pub async fn set_restricted(
    conn: &mut PgConnection,
    account_id: Uuid,
    restricted: bool,
) -> DbResult<Account> {
    sqlx::query_as(&format!(
        "UPDATE accounts
         SET restricted_at = CASE WHEN $2 THEN coalesce(restricted_at, now()) END
         WHERE id = $1
         RETURNING {ACCOUNT}"
    ))
    .bind(account_id)
    .bind(restricted)
    .fetch_optional(conn)
    .await?
    .ok_or(DbError::NotFound)
}

/// Whether the account is one of `account_ids`, or has one of `identity_keys`
/// (`platform:platform_user_id`). Used for the owner list.
pub async fn is_any_of(
    conn: &mut PgConnection,
    account_id: Uuid,
    identity_keys: &[String],
    account_ids: &[Uuid],
) -> DbResult<bool> {
    Ok(sqlx::query_scalar(
        "SELECT $1 = ANY($3) OR EXISTS (
             SELECT 1 FROM identities
             WHERE account_id = $1 AND (platform || ':' || platform_user_id) = ANY($2)
         )",
    )
    .bind(account_id)
    .bind(identity_keys)
    .bind(account_ids)
    .fetch_one(conn)
    .await?)
}

/// Blocks an account (`users.restrict`) or lifts the block.
pub async fn set_blocked(
    conn: &mut PgConnection,
    account_id: Uuid,
    blocked: bool,
) -> DbResult<Account> {
    sqlx::query_as(&format!(
        "UPDATE accounts
         SET blocked_at = CASE WHEN $2 THEN coalesce(blocked_at, now()) END
         WHERE id = $1
         RETURNING {ACCOUNT}"
    ))
    .bind(account_id)
    .bind(blocked)
    .fetch_optional(conn)
    .await?
    .ok_or(DbError::NotFound)
}
