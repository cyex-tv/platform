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
    /// Uploads blocked. Like every hold here, read as it is now: a hold whose end has passed
    /// reads as `None`.
    pub restricted_at: Option<DateTime<Utc>>,
    pub deleted_at: Option<DateTime<Utc>>,
    /// Blocked by `users.restrict` or `users.block`: can't sign in.
    pub blocked_at: Option<DateTime<Utc>>,
    /// When the uploads hold ends; `None` with `restricted_at` set means for good.
    pub restricted_until: Option<DateTime<Utc>>,
    pub blocked_until: Option<DateTime<Utc>>,
    /// Can't file reports or tickets.
    pub tickets_blocked_at: Option<DateTime<Utc>>,
    pub tickets_blocked_until: Option<DateTime<Utc>>,
    /// Set while a deletion is pending (with `deleted_at`): the account is hidden, and signing in
    /// before this time can restore it; after it, the worker erases it (ADR 0019).
    pub erase_after: Option<DateTime<Utc>>,
}

impl Account {
    /// Deleted, but still within the undo window.
    pub fn restorable(&self, now: DateTime<Utc>) -> bool {
        self.deleted_at.is_some() && self.erase_after.is_some_and(|t| t > now)
    }

    /// Any hold at all (uploads, tickets or the whole account).
    pub fn held(&self) -> bool {
        self.restricted_at.is_some()
            || self.blocked_at.is_some()
            || self.tickets_blocked_at.is_some()
    }
}

/// One of an account's three holds (migration `timed_restrictions`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Hold {
    /// No uploads (`restricted_at`).
    Uploads,
    /// No reports or tickets (`tickets_blocked_at`).
    Tickets,
    /// No sign-in, no sessions (`blocked_at`).
    Account,
}

impl Hold {
    pub const ALL: [Hold; 3] = [Hold::Uploads, Hold::Tickets, Hold::Account];

    /// The `account_restrictions.kind` value.
    pub fn kind(self) -> &'static str {
        match self {
            Self::Uploads => "uploads",
            Self::Tickets => "tickets",
            Self::Account => "account",
        }
    }

    pub fn from_kind(kind: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|h| h.kind() == kind)
    }

    fn columns(self) -> (&'static str, &'static str) {
        match self {
            Self::Uploads => ("restricted_at", "restricted_until"),
            Self::Tickets => ("tickets_blocked_at", "tickets_blocked_until"),
            Self::Account => ("blocked_at", "blocked_until"),
        }
    }

    /// When it started and when it ends, as the account is now.
    pub fn of(self, account: &Account) -> (Option<DateTime<Utc>>, Option<DateTime<Utc>>) {
        match self {
            Self::Uploads => (account.restricted_at, account.restricted_until),
            Self::Tickets => (account.tickets_blocked_at, account.tickets_blocked_until),
            Self::Account => (account.blocked_at, account.blocked_until),
        }
    }
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

// Holds are read as they are now: one that has ended reads as none.
const ACCOUNT: &str = "id, created_at, deleted_at,
    CASE WHEN restricted_until IS NULL OR restricted_until > now() THEN restricted_at END AS restricted_at,
    CASE WHEN restricted_until > now() THEN restricted_until END AS restricted_until,
    CASE WHEN blocked_until IS NULL OR blocked_until > now() THEN blocked_at END AS blocked_at,
    CASE WHEN blocked_until > now() THEN blocked_until END AS blocked_until,
    CASE WHEN tickets_blocked_until IS NULL OR tickets_blocked_until > now() THEN tickets_blocked_at END AS tickets_blocked_at,
    CASE WHEN tickets_blocked_until > now() THEN tickets_blocked_until END AS tickets_blocked_until,
    erase_after";
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

/// Copies the hold an identity left behind (see the migrations `identity_holds` and
/// `timed_restrictions`) onto `account`, and uses it up. Where both hold the same kind, the longer
/// one stays. The account as it is afterwards.
async fn apply_hold(
    conn: &mut PgConnection,
    account: Account,
    profile: Profile<'_>,
) -> DbResult<Account> {
    type Time = Option<DateTime<Utc>>;
    let hold: Option<(Time, Time, Time, Time, Time, Time)> = sqlx::query_as(
        "DELETE FROM identity_holds WHERE platform = $1 AND platform_user_id = $2
         RETURNING restricted_at, restricted_until, blocked_at, blocked_until,
                   tickets_blocked_at, tickets_blocked_until",
    )
    .bind(profile.platform)
    .bind(profile.platform_user_id)
    .fetch_optional(&mut *conn)
    .await?;
    let Some((r_at, r_until, b_at, b_until, t_at, t_until)) = hold else {
        return Ok(account);
    };
    let mut out = account;
    for (hold, at, until) in [
        (Hold::Uploads, r_at, r_until),
        (Hold::Account, b_at, b_until),
        (Hold::Tickets, t_at, t_until),
    ] {
        let Some(at) = at else { continue };
        if until.is_some_and(|u| u <= Utc::now()) {
            continue;
        }
        out = merge_hold(conn, out.id, hold, at, until).await?;
    }
    Ok(out)
}

/// Puts `hold` on an account from `at` until `until` (`None`: for good), keeping a longer one it
/// already has.
async fn merge_hold(
    conn: &mut PgConnection,
    account_id: Uuid,
    hold: Hold,
    at: DateTime<Utc>,
    until: Option<DateTime<Utc>>,
) -> DbResult<Account> {
    let (at_col, until_col) = hold.columns();
    let live = format!("({at_col} IS NOT NULL AND ({until_col} IS NULL OR {until_col} > now()))");
    Ok(sqlx::query_as(&format!(
        "UPDATE accounts
         SET {until_col} = CASE
                 WHEN NOT {live} THEN $3::timestamptz
                 WHEN {until_col} IS NULL OR $3::timestamptz IS NULL THEN NULL
                 ELSE greatest({until_col}, $3::timestamptz) END,
             {at_col} = CASE WHEN {live} THEN {at_col} ELSE $2 END
         WHERE id = $1
         RETURNING {ACCOUNT}"
    ))
    .bind(account_id)
    .bind(at)
    .bind(until)
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
         SET restricted_at = CASE WHEN $2 THEN
                 CASE WHEN restricted_until IS NULL OR restricted_until > now()
                      THEN coalesce(restricted_at, now()) ELSE now() END
             END,
             restricted_until = NULL
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
         SET blocked_at = CASE WHEN $2 THEN
                 CASE WHEN blocked_until IS NULL OR blocked_until > now()
                      THEN coalesce(blocked_at, now()) ELSE now() END
             END,
             blocked_until = NULL
         WHERE id = $1
         RETURNING {ACCOUNT}"
    ))
    .bind(account_id)
    .bind(blocked)
    .fetch_optional(conn)
    .await?
    .ok_or(DbError::NotFound)
}

/// Puts `hold` on an account until `until` (`None`: for good), replacing the end of one it had of
/// that kind; or lifts it (`on` false). A new hold starts now; a running one keeps its start.
pub async fn set_hold(
    conn: &mut PgConnection,
    account_id: Uuid,
    hold: Hold,
    on: bool,
    until: Option<DateTime<Utc>>,
) -> DbResult<Account> {
    let (at_col, until_col) = hold.columns();
    sqlx::query_as(&format!(
        "UPDATE accounts
         SET {at_col} = CASE WHEN $2 THEN
                 CASE WHEN {at_col} IS NOT NULL AND ({until_col} IS NULL OR {until_col} > now())
                      THEN {at_col} ELSE now() END
             END,
             {until_col} = CASE WHEN $2 THEN $3::timestamptz END
         WHERE id = $1
         RETURNING {ACCOUNT}"
    ))
    .bind(account_id)
    .bind(on)
    .bind(until)
    .fetch_optional(conn)
    .await?
    .ok_or(DbError::NotFound)
}

/// One hold placed on an account, from `account_restrictions`.
#[derive(Clone, Debug, PartialEq, Eq, FromRow)]
pub struct Restriction {
    pub id: Uuid,
    pub account_id: Uuid,
    pub kind: String,
    pub reason: Option<String>,
    pub ends_at: Option<DateTime<Utc>>,
    pub created_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub lifted_at: Option<DateTime<Utc>>,
    pub lifted_by: Option<Uuid>,
}

const RESTRICTION: &str =
    "id, account_id, kind, reason, ends_at, created_by, created_at, lifted_at, lifted_by";

/// Records a hold being placed. A running record of the same kind is marked lifted by `by`: the
/// new one replaces it.
pub async fn record_hold(
    conn: &mut PgConnection,
    account_id: Uuid,
    hold: Hold,
    reason: Option<&str>,
    ends_at: Option<DateTime<Utc>>,
    by: Option<Uuid>,
) -> DbResult<Restriction> {
    end_records(conn, account_id, hold, by).await?;
    Ok(sqlx::query_as(&format!(
        "INSERT INTO account_restrictions (id, account_id, kind, reason, ends_at, created_by)
         VALUES ($1, $2, $3, $4, $5, $6)
         RETURNING {RESTRICTION}"
    ))
    .bind(new_id())
    .bind(account_id)
    .bind(hold.kind())
    .bind(reason)
    .bind(ends_at)
    .bind(by)
    .fetch_one(conn)
    .await?)
}

/// Marks the running records of `hold` lifted by `by`. How many there were.
pub async fn end_records(
    conn: &mut PgConnection,
    account_id: Uuid,
    hold: Hold,
    by: Option<Uuid>,
) -> DbResult<u64> {
    let done = sqlx::query(
        "UPDATE account_restrictions SET lifted_at = now(), lifted_by = $3
         WHERE account_id = $1 AND kind = $2 AND lifted_at IS NULL
           AND (ends_at IS NULL OR ends_at > now())",
    )
    .bind(account_id)
    .bind(hold.kind())
    .bind(by)
    .execute(conn)
    .await?;
    Ok(done.rows_affected())
}

/// Every hold placed on an account, newest first (at most `limit`).
pub async fn restrictions(
    conn: &mut PgConnection,
    account_id: Uuid,
    limit: i64,
) -> DbResult<Vec<Restriction>> {
    Ok(sqlx::query_as(&format!(
        "SELECT {RESTRICTION} FROM account_restrictions WHERE account_id = $1
         ORDER BY created_at DESC, id DESC LIMIT $2"
    ))
    .bind(account_id)
    .bind(limit)
    .fetch_all(conn)
    .await?)
}
