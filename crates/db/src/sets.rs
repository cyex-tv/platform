//! Emote sets, their members and their editors.
//!
//! The rules that keep the data sound are checked here, under a lock on the set row:
//! - a set holds at most `capacity` emotes (D20, D21);
//! - names are unique within a set;
//! - a pending emote only goes in its owner's own sets (D10);
//! - a personal set takes approved emotes only (D21);
//! - rejected, taken-down and deleted emotes go nowhere.
//!
//! Who may edit a set is the api's check; `can_edit` answers the owner-or-editor part.

use chrono::{DateTime, Utc};
use cyex_tv_shared::model::{EmoteState, Platform, SetKind};
use sqlx::{Connection, FromRow, PgConnection};
use uuid::Uuid;

use crate::error::{DbError, DbResult, unique_violation};
use crate::ids::new_id;

/// Default capacity of a channel set (D20).
pub const CHANNEL_CAPACITY: i32 = 1000;
/// Default capacity of a personal set (D21).
pub const PERSONAL_CAPACITY: i32 = 5;
/// Capacity of the global set when it's first made.
pub const GLOBAL_CAPACITY: i32 = 1000;

#[derive(Clone, Debug, PartialEq, Eq, FromRow)]
pub struct Set {
    pub id: Uuid,
    /// `None` only for the global set.
    pub owner_account_id: Option<Uuid>,
    pub name: String,
    pub kind: SetKind,
    pub capacity: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, PartialEq, Eq, FromRow)]
pub struct Member {
    pub set_id: Uuid,
    pub emote_id: Uuid,
    /// The name in this set.
    pub name: String,
    pub zero_width: bool,
    pub added_by: Option<Uuid>,
    pub added_at: DateTime<Utc>,
}

#[derive(Clone, Debug, PartialEq, Eq, FromRow)]
pub struct Editor {
    pub set_id: Uuid,
    pub account_id: Uuid,
    pub granted_by: Option<Uuid>,
    pub granted_at: DateTime<Utc>,
}

/// A personal set found for a chatter.
#[derive(Clone, Debug, PartialEq, Eq, FromRow)]
pub struct PersonalSet {
    pub platform_user_id: String,
    #[sqlx(flatten)]
    pub set: Set,
}

/// What to add to a set. `None` takes the emote's own name and zero-width flag.
#[derive(Clone, Copy, Debug)]
pub struct NewMember<'a> {
    pub emote_id: Uuid,
    pub name: Option<&'a str>,
    pub zero_width: Option<bool>,
    pub added_by: Option<Uuid>,
}

const SET: &str = "id, owner_account_id, name, kind, capacity, created_at, updated_at";
const MEMBER: &str = "set_id, emote_id, name, zero_width, added_by, added_at";
const EDITOR: &str = "set_id, account_id, granted_by, granted_at";

/// Makes a channel or personal set with the default capacity. One personal set per account.
pub async fn create(
    conn: &mut PgConnection,
    owner: Uuid,
    name: &str,
    kind: SetKind,
) -> DbResult<Set> {
    let capacity = match kind {
        SetKind::Channel => CHANNEL_CAPACITY,
        SetKind::Personal => PERSONAL_CAPACITY,
        SetKind::Global => return Err(DbError::InvalidSetKind),
    };
    sqlx::query_as(&format!(
        "INSERT INTO emote_sets (id, owner_account_id, name, kind, capacity)
         VALUES ($1, $2, $3, $4, $5)
         RETURNING {SET}"
    ))
    .bind(new_id())
    .bind(owner)
    .bind(name)
    .bind(kind)
    .bind(capacity)
    .fetch_one(conn)
    .await
    .map_err(|err| match unique_violation(&err) {
        Some("emote_sets_one_personal") => DbError::PersonalSetExists,
        _ => err.into(),
    })
}

/// The global set (D12), made on first use.
pub async fn ensure_global(conn: &mut PgConnection) -> DbResult<Set> {
    let select = format!("SELECT {SET} FROM emote_sets WHERE kind = 'global'");
    if let Some(set) = sqlx::query_as(&select).fetch_optional(&mut *conn).await? {
        return Ok(set);
    }
    sqlx::query(
        "INSERT INTO emote_sets (id, name, kind, capacity)
         VALUES ($1, 'Global', 'global', $2)
         ON CONFLICT DO NOTHING",
    )
    .bind(new_id())
    .bind(GLOBAL_CAPACITY)
    .execute(&mut *conn)
    .await?;
    Ok(sqlx::query_as(&select).fetch_one(conn).await?)
}

pub async fn get(conn: &mut PgConnection, id: Uuid) -> DbResult<Option<Set>> {
    Ok(
        sqlx::query_as(&format!("SELECT {SET} FROM emote_sets WHERE id = $1"))
            .bind(id)
            .fetch_optional(conn)
            .await?,
    )
}

/// The sets an account owns, oldest first.
pub async fn list_for_owner(conn: &mut PgConnection, owner: Uuid) -> DbResult<Vec<Set>> {
    Ok(sqlx::query_as(&format!(
        "SELECT {SET} FROM emote_sets WHERE owner_account_id = $1 ORDER BY id"
    ))
    .bind(owner)
    .fetch_all(conn)
    .await?)
}

/// The sets an account was made editor of.
pub async fn list_editable(conn: &mut PgConnection, account_id: Uuid) -> DbResult<Vec<Set>> {
    Ok(sqlx::query_as(
        "SELECT s.id, s.owner_account_id, s.name, s.kind, s.capacity, s.created_at, s.updated_at
         FROM emote_sets s
         JOIN set_editors e ON e.set_id = s.id
         WHERE e.account_id = $1
         ORDER BY s.id",
    )
    .bind(account_id)
    .fetch_all(conn)
    .await?)
}

pub async fn personal_for(conn: &mut PgConnection, owner: Uuid) -> DbResult<Option<Set>> {
    Ok(sqlx::query_as(&format!(
        "SELECT {SET} FROM emote_sets WHERE owner_account_id = $1 AND kind = 'personal'"
    ))
    .bind(owner)
    .fetch_optional(conn)
    .await?)
}

/// The non-empty personal sets of chatters, by their platform user IDs.
pub async fn personal_for_users(
    conn: &mut PgConnection,
    platform: Platform,
    platform_user_ids: &[String],
) -> DbResult<Vec<PersonalSet>> {
    Ok(sqlx::query_as(
        "SELECT i.platform_user_id,
                s.id, s.owner_account_id, s.name, s.kind, s.capacity, s.created_at, s.updated_at
         FROM identities i
         JOIN emote_sets s ON s.owner_account_id = i.account_id AND s.kind = 'personal'
         WHERE i.platform = $1
           AND i.platform_user_id = ANY($2)
           AND EXISTS (SELECT 1 FROM emote_set_members m WHERE m.set_id = s.id)
         ORDER BY i.platform_user_id",
    )
    .bind(platform)
    .bind(platform_user_ids)
    .fetch_all(conn)
    .await?)
}

pub async fn rename(conn: &mut PgConnection, id: Uuid, name: &str) -> DbResult<Set> {
    sqlx::query_as(&format!(
        "UPDATE emote_sets SET name = $2, updated_at = now() WHERE id = $1 RETURNING {SET}"
    ))
    .bind(id)
    .bind(name)
    .fetch_optional(conn)
    .await?
    .ok_or(DbError::NotFound)
}

/// Changes a set's capacity (raising it needs `sets.edit_any`; that's the api's check). It can't go
/// below the number of emotes already in the set.
pub async fn set_capacity(conn: &mut PgConnection, id: Uuid, capacity: i32) -> DbResult<Set> {
    let mut tx = conn.begin().await?;
    lock_set(&mut tx, id).await?;
    if count(&mut tx, id).await? > i64::from(capacity) {
        return Err(DbError::CapacityTooLow);
    }
    let set = sqlx::query_as(&format!(
        "UPDATE emote_sets SET capacity = $2, updated_at = now() WHERE id = $1 RETURNING {SET}"
    ))
    .bind(id)
    .bind(capacity)
    .fetch_one(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(set)
}

/// Deletes a set with its members and editors. Channels using it fall back to no active set.
pub async fn delete(conn: &mut PgConnection, id: Uuid) -> DbResult<bool> {
    let done = sqlx::query("DELETE FROM emote_sets WHERE id = $1 AND kind <> 'global'")
        .bind(id)
        .execute(conn)
        .await?;
    Ok(done.rows_affected() > 0)
}

async fn lock_set(conn: &mut PgConnection, id: Uuid) -> DbResult<Set> {
    sqlx::query_as(&format!(
        "SELECT {SET} FROM emote_sets WHERE id = $1 FOR UPDATE"
    ))
    .bind(id)
    .fetch_optional(conn)
    .await?
    .ok_or(DbError::NotFound)
}

pub async fn count(conn: &mut PgConnection, set_id: Uuid) -> DbResult<i64> {
    Ok(
        sqlx::query_scalar("SELECT count(*) FROM emote_set_members WHERE set_id = $1")
            .bind(set_id)
            .fetch_one(conn)
            .await?,
    )
}

#[derive(FromRow)]
struct EmoteForSet {
    owner_account_id: Option<Uuid>,
    name: String,
    zero_width: bool,
    state: EmoteState,
}

/// Adds an emote to a set, checking capacity, unique names and the emote's state.
pub async fn add_member(
    conn: &mut PgConnection,
    set_id: Uuid,
    new: NewMember<'_>,
) -> DbResult<Member> {
    let mut tx = conn.begin().await?;
    let set = lock_set(&mut tx, set_id).await?;

    let emote: EmoteForSet = sqlx::query_as(
        "SELECT owner_account_id, name, zero_width, state
         FROM emotes WHERE id = $1 AND deleted_at IS NULL",
    )
    .bind(new.emote_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(DbError::EmoteUnavailable)?;
    match emote.state {
        EmoteState::Approved => {}
        EmoteState::Pending => {
            let own_set =
                set.owner_account_id.is_some() && set.owner_account_id == emote.owner_account_id;
            if set.kind == SetKind::Personal || !own_set {
                return Err(DbError::NotApproved);
            }
        }
        EmoteState::Rejected | EmoteState::TakenDown => return Err(DbError::EmoteUnavailable),
    }

    if count(&mut tx, set_id).await? >= i64::from(set.capacity) {
        return Err(DbError::SetFull);
    }

    let member = sqlx::query_as(&format!(
        "INSERT INTO emote_set_members (set_id, emote_id, name, zero_width, added_by)
         VALUES ($1, $2, $3, $4, $5)
         RETURNING {MEMBER}"
    ))
    .bind(set_id)
    .bind(new.emote_id)
    .bind(new.name.unwrap_or(&emote.name))
    .bind(new.zero_width.unwrap_or(emote.zero_width))
    .bind(new.added_by)
    .fetch_one(&mut *tx)
    .await
    .map_err(member_conflict)?;
    tx.commit().await?;
    Ok(member)
}

fn member_conflict(err: sqlx::Error) -> DbError {
    match unique_violation(&err) {
        Some("emote_set_members_pkey") => DbError::AlreadyInSet,
        Some("emote_set_members_set_id_name_key") => DbError::NameTaken,
        _ => err.into(),
    }
}

/// Renames an emote in a set, or changes its zero-width flag there.
pub async fn update_member(
    conn: &mut PgConnection,
    set_id: Uuid,
    emote_id: Uuid,
    name: Option<&str>,
    zero_width: Option<bool>,
) -> DbResult<Member> {
    sqlx::query_as(&format!(
        "UPDATE emote_set_members
         SET name = coalesce($3, name), zero_width = coalesce($4, zero_width)
         WHERE set_id = $1 AND emote_id = $2
         RETURNING {MEMBER}"
    ))
    .bind(set_id)
    .bind(emote_id)
    .bind(name)
    .bind(zero_width)
    .fetch_optional(conn)
    .await
    .map_err(member_conflict)?
    .ok_or(DbError::NotFound)
}

pub async fn remove_member(
    conn: &mut PgConnection,
    set_id: Uuid,
    emote_id: Uuid,
) -> DbResult<Member> {
    sqlx::query_as(&format!(
        "DELETE FROM emote_set_members WHERE set_id = $1 AND emote_id = $2 RETURNING {MEMBER}"
    ))
    .bind(set_id)
    .bind(emote_id)
    .fetch_optional(conn)
    .await?
    .ok_or(DbError::NotFound)
}

/// A set's emotes in the order they were added.
pub async fn members(conn: &mut PgConnection, set_id: Uuid) -> DbResult<Vec<Member>> {
    Ok(sqlx::query_as(&format!(
        "SELECT {MEMBER} FROM emote_set_members WHERE set_id = $1 ORDER BY added_at, emote_id"
    ))
    .bind(set_id)
    .fetch_all(conn)
    .await?)
}

/// Makes an account an editor of a set. False if it already was.
pub async fn add_editor(
    conn: &mut PgConnection,
    set_id: Uuid,
    account_id: Uuid,
    granted_by: Option<Uuid>,
) -> DbResult<bool> {
    let done = sqlx::query(
        "INSERT INTO set_editors (set_id, account_id, granted_by) VALUES ($1, $2, $3)
         ON CONFLICT DO NOTHING",
    )
    .bind(set_id)
    .bind(account_id)
    .bind(granted_by)
    .execute(conn)
    .await?;
    Ok(done.rows_affected() > 0)
}

pub async fn remove_editor(
    conn: &mut PgConnection,
    set_id: Uuid,
    account_id: Uuid,
) -> DbResult<bool> {
    let done = sqlx::query("DELETE FROM set_editors WHERE set_id = $1 AND account_id = $2")
        .bind(set_id)
        .bind(account_id)
        .execute(conn)
        .await?;
    Ok(done.rows_affected() > 0)
}

pub async fn editors(conn: &mut PgConnection, set_id: Uuid) -> DbResult<Vec<Editor>> {
    Ok(sqlx::query_as(&format!(
        "SELECT {EDITOR} FROM set_editors WHERE set_id = $1 ORDER BY granted_at"
    ))
    .bind(set_id)
    .fetch_all(conn)
    .await?)
}

/// True if the account owns the set or is one of its editors. Permissions (`sets.edit_any`,
/// `global_set.edit`) are checked by the api on top.
pub async fn can_edit(conn: &mut PgConnection, account_id: Uuid, set_id: Uuid) -> DbResult<bool> {
    Ok(sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM emote_sets WHERE id = $2 AND owner_account_id = $1)
             OR EXISTS (SELECT 1 FROM set_editors WHERE set_id = $2 AND account_id = $1)",
    )
    .bind(account_id)
    .bind(set_id)
    .fetch_one(conn)
    .await?)
}
