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

use std::collections::{HashMap, HashSet};

use chrono::{DateTime, Utc};
use cyex_tv_shared::model::{EmoteState, Platform, SetKind, SetVisibility};
use sqlx::{Connection, FromRow, PgConnection};
use uuid::Uuid;

use crate::error::{DbError, DbResult, unique_violation};
use crate::ids::new_id;

/// Default capacity of a channel set (D20).
pub const CHANNEL_CAPACITY: i32 = 1000;
/// Slots of a personal set without a role that gives more or a staff override (ADR 0038). The
/// migration's `personal_set_capacity` repeats this number.
pub const PERSONAL_CAPACITY: i32 = 3;
/// What a subscriber role gives (its `personal_set_slots` role limit), for the website's wording.
pub const SUBSCRIBER_PERSONAL_CAPACITY: i32 = 10;
/// Capacity of the global set when it's first made.
pub const GLOBAL_CAPACITY: i32 = 1000;

#[derive(Clone, Debug, PartialEq, Eq, FromRow)]
pub struct Set {
    pub id: Uuid,
    /// `None` only for the global set.
    pub owner_account_id: Option<Uuid>,
    pub name: String,
    pub kind: SetKind,
    /// The slots the set has. For a personal set this is the effective number (ADR 0038): the staff
    /// override, else its owner's `personal_set_slots` role limit, else 3. Read, never stored.
    pub capacity: i32,
    /// A personal set's capacity set by staff; `None` when the roles decide. Always `None` for
    /// other kinds.
    pub capacity_override: Option<i32>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub visibility: SetVisibility,
    /// Goes up with every change to what the set shows (ADR 0035; triggers keep it).
    pub version: i64,
    /// The newest version whose change rows were pruned: a delta from before it is a full resync.
    pub change_floor: i64,
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

/// The columns of a [`Set`]. `capacity` of a personal set is read through `personal_set_capacity`
/// (ADR 0038): the staff override, else the owner's `personal_set_slots` role limit, else 3.
const SET: &str = "id, owner_account_id, name, kind,
                   CASE WHEN kind = 'personal'
                        THEN personal_set_capacity(owner_account_id, capacity_override)
                        ELSE capacity END AS capacity,
                   capacity_override, created_at, updated_at, visibility, version, change_floor";
const MEMBER: &str = "set_id, emote_id, name, zero_width, added_by, added_at";
const EDITOR: &str = "set_id, account_id, granted_by, granted_at";

/// Makes a channel or personal set with the default capacity and visibility. One personal set
/// per account.
pub async fn create(
    conn: &mut PgConnection,
    owner: Uuid,
    name: &str,
    kind: SetKind,
) -> DbResult<Set> {
    create_with(conn, owner, name, kind, None).await
}

/// The visibility a new set of `kind` gets when none is asked for: listed, for every kind (every
/// set is public, ADR 0017 amendment 2026/11/22; personal sets were unlisted before).
pub fn default_visibility(_kind: SetKind) -> SetVisibility {
    SetVisibility::Listed
}

/// Whether a set of `kind` may have `visibility`: personal sets are shown in chat to everyone,
/// so never private; the global set is always listed.
pub fn visibility_allowed(kind: SetKind, visibility: SetVisibility) -> bool {
    match kind {
        SetKind::Channel => true,
        SetKind::Personal => visibility != SetVisibility::Private,
        SetKind::Global => visibility == SetVisibility::Listed,
    }
}

/// [`create`] with a visibility (`None`: the default for the kind).
pub async fn create_with(
    conn: &mut PgConnection,
    owner: Uuid,
    name: &str,
    kind: SetKind,
    visibility: Option<SetVisibility>,
) -> DbResult<Set> {
    let capacity = match kind {
        SetKind::Channel => CHANNEL_CAPACITY,
        SetKind::Personal => PERSONAL_CAPACITY,
        SetKind::Global => return Err(DbError::InvalidSetKind),
    };
    let visibility = visibility.unwrap_or_else(|| default_visibility(kind));
    if !visibility_allowed(kind, visibility) {
        return Err(DbError::InvalidVisibility);
    }
    sqlx::query_as(&format!(
        "INSERT INTO emote_sets (id, owner_account_id, name, kind, capacity, visibility)
         VALUES ($1, $2, $3, $4, $5, $6)
         RETURNING {SET}"
    ))
    .bind(new_id())
    .bind(owner)
    .bind(name)
    .bind(kind)
    .bind(capacity)
    .bind(visibility)
    .fetch_one(conn)
    .await
    .map_err(|err| match unique_violation(&err) {
        Some("emote_sets_one_personal") => DbError::PersonalSetExists,
        _ => err.into(),
    })
}

/// An account's personal set, made when it has none (new accounts at sign-up; the migration made
/// one for the rest). Returns the set either way.
pub async fn ensure_personal(conn: &mut PgConnection, owner: Uuid) -> DbResult<Set> {
    // No error to catch and no aborted transaction: the unique index decides.
    sqlx::query(
        "INSERT INTO emote_sets (id, owner_account_id, name, kind, capacity, visibility)
         VALUES ($1, $2, 'Personal', 'personal', $3, 'listed')
         ON CONFLICT DO NOTHING",
    )
    .bind(new_id())
    .bind(owner)
    .bind(PERSONAL_CAPACITY)
    .execute(&mut *conn)
    .await?;
    personal_for(conn, owner).await?.ok_or(DbError::NotFound)
}

/// A new account's default set: a listed channel set named after the account (ADR 0017). The
/// name is cut to the 100 characters a set name may have.
pub async fn create_default(conn: &mut PgConnection, owner: Uuid, name: &str) -> DbResult<Set> {
    let name: String = name.chars().take(100).collect();
    let name = if name.trim().is_empty() {
        "Emotes".to_owned()
    } else {
        name
    };
    create_with(conn, owner, &name, SetKind::Channel, None).await
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

/// Sets by ID, in no particular order. Unknown IDs are left out.
pub async fn get_many(conn: &mut PgConnection, ids: &[Uuid]) -> DbResult<Vec<Set>> {
    Ok(
        sqlx::query_as(&format!("SELECT {SET} FROM emote_sets WHERE id = ANY($1)"))
            .bind(ids)
            .fetch_all(conn)
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

/// The sets of every account in `owners`, by owner then ID: [`list_for_owner`] for many owners
/// in one query.
pub async fn list_for_owners(conn: &mut PgConnection, owners: &[Uuid]) -> DbResult<Vec<Set>> {
    if owners.is_empty() {
        return Ok(Vec::new());
    }
    Ok(sqlx::query_as(&format!(
        "SELECT {SET} FROM emote_sets WHERE owner_account_id = ANY($1) ORDER BY owner_account_id, id"
    ))
    .bind(owners)
    .fetch_all(conn)
    .await?)
}

/// Which of `ids` are sets `owner` owns (any kind but global).
pub async fn owned_ids(conn: &mut PgConnection, owner: Uuid, ids: &[Uuid]) -> DbResult<Vec<Uuid>> {
    Ok(sqlx::query_scalar(
        "SELECT id FROM emote_sets WHERE owner_account_id = $1 AND id = ANY($2) ORDER BY id",
    )
    .bind(owner)
    .bind(ids)
    .fetch_all(conn)
    .await?)
}

/// The sets an account was made editor of.
pub async fn list_editable(conn: &mut PgConnection, account_id: Uuid) -> DbResult<Vec<Set>> {
    Ok(sqlx::query_as(
        "SELECT s.id, s.owner_account_id, s.name, s.kind,
                CASE WHEN s.kind = 'personal'
                     THEN personal_set_capacity(s.owner_account_id, s.capacity_override)
                     ELSE s.capacity END AS capacity,
                s.capacity_override, s.created_at, s.updated_at,
                s.visibility, s.version, s.change_floor
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
                s.id, s.owner_account_id, s.name, s.kind,
                CASE WHEN s.kind = 'personal'
                     THEN personal_set_capacity(s.owner_account_id, s.capacity_override)
                     ELSE s.capacity END AS capacity,
                s.capacity_override, s.created_at, s.updated_at,
                s.visibility, s.version, s.change_floor
         FROM identities i
         JOIN accounts a ON a.id = i.account_id AND a.deleted_at IS NULL
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

/// Changes who can find and open a set.
pub async fn set_visibility(
    conn: &mut PgConnection,
    id: Uuid,
    visibility: SetVisibility,
) -> DbResult<Set> {
    let mut tx = conn.begin().await?;
    let set = lock_set(&mut tx, id).await?;
    if !visibility_allowed(set.kind, visibility) {
        return Err(DbError::InvalidVisibility);
    }
    let set = sqlx::query_as(&format!(
        "UPDATE emote_sets SET visibility = $2, updated_at = now() WHERE id = $1 RETURNING {SET}"
    ))
    .bind(id)
    .bind(visibility)
    .fetch_one(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(set)
}

/// An account's listed sets (not the global set), oldest first, at most `limit`.
pub async fn list_listed(conn: &mut PgConnection, owner: Uuid, limit: i64) -> DbResult<Vec<Set>> {
    Ok(sqlx::query_as(&format!(
        "SELECT {SET} FROM emote_sets
         WHERE owner_account_id = $1 AND visibility = 'listed'
         ORDER BY id LIMIT $2"
    ))
    .bind(owner)
    .bind(limit)
    .fetch_all(conn)
    .await?)
}

/// How many emotes each set holds (every member, whatever its state). Sets with none are missing.
pub async fn counts(conn: &mut PgConnection, set_ids: &[Uuid]) -> DbResult<HashMap<Uuid, i64>> {
    let rows: Vec<(Uuid, i64)> = sqlx::query_as(
        "SELECT set_id, count(*) FROM emote_set_members WHERE set_id = ANY($1) GROUP BY set_id",
    )
    .bind(set_ids)
    .fetch_all(conn)
    .await?;
    Ok(rows.into_iter().collect())
}

/// Which of `set_ids` a channel shows now.
pub async fn active(conn: &mut PgConnection, set_ids: &[Uuid]) -> DbResult<HashSet<Uuid>> {
    let rows: Vec<Uuid> = sqlx::query_scalar(
        "SELECT DISTINCT active_set_id FROM channels WHERE active_set_id = ANY($1)",
    )
    .bind(set_ids)
    .fetch_all(conn)
    .await?;
    Ok(rows.into_iter().collect())
}

/// Changes a set's capacity (raising it needs `sets.edit_any`; that's the api's check). It can't go
/// below the number of emotes already in the set. On a personal set this is the staff override
/// (ADR 0038); `None` there takes the override away, so the owner's roles decide again.
pub async fn set_capacity(conn: &mut PgConnection, id: Uuid, capacity: i32) -> DbResult<Set> {
    set_capacity_or_reset(conn, id, Some(capacity)).await
}

/// [`set_capacity`], and with `None` on a personal set, drops its override.
pub async fn set_capacity_or_reset(
    conn: &mut PgConnection,
    id: Uuid,
    capacity: Option<i32>,
) -> DbResult<Set> {
    let mut tx = conn.begin().await?;
    let set = lock_set(&mut tx, id).await?;
    let personal = set.kind == SetKind::Personal;
    let capacity = match capacity {
        Some(n) => {
            if count(&mut tx, id).await? > i64::from(n) {
                return Err(DbError::CapacityTooLow);
            }
            n
        }
        None if personal => 0,
        None => return Err(DbError::InvalidSetKind),
    };
    let sql = if personal {
        // `capacity` stays as it was: a personal set's number is read, not stored.
        "UPDATE emote_sets SET capacity_override = NULLIF($2, 0), updated_at = now()
         WHERE id = $1 RETURNING "
    } else {
        "UPDATE emote_sets SET capacity = $2, updated_at = now() WHERE id = $1 RETURNING "
    };
    let set = sqlx::query_as(&format!("{sql}{SET}"))
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
    add_member_within(conn, set_id, new, None).await
}

/// [`add_member`] against `capacity` instead of the set's own, when the api holds a stricter or
/// fresher number. A set over its capacity (a personal set whose owner lost a role) takes no
/// new member until it is under: the check is "holds at least capacity", not "equals".
pub async fn add_member_within(
    conn: &mut PgConnection,
    set_id: Uuid,
    new: NewMember<'_>,
    capacity: Option<i32>,
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

    if count(&mut tx, set_id).await? >= i64::from(capacity.unwrap_or(set.capacity)) {
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

// Versions and changes (ADR 0035)

/// One change row: the member (or, for `set`, the set itself) touched at `version`.
#[derive(Clone, Debug, PartialEq, Eq, FromRow)]
pub struct Change {
    pub version: i64,
    pub emote_id: Option<Uuid>,
    /// `added`, `removed`, `renamed` (the member's name or zero-width flag in the set), `emote`
    /// (the emote itself, or its file) or `set` (the set's name).
    pub kind: String,
}

/// How long change rows are kept.
pub const CHANGES_KEEP_DAYS: i64 = 7;
/// How many of a set's newest versions keep their change rows.
pub const CHANGES_KEEP_VERSIONS: i64 = 1000;

/// The changes of a set after `since` up to `upto`, oldest first, at most `limit`.
pub async fn changes_since(
    conn: &mut PgConnection,
    set_id: Uuid,
    since: i64,
    upto: i64,
    limit: i64,
) -> DbResult<Vec<Change>> {
    Ok(sqlx::query_as(
        "SELECT version, emote_id, kind FROM emote_set_changes
         WHERE set_id = $1 AND version > $2 AND version <= $3
         ORDER BY version, id
         LIMIT $4",
    )
    .bind(set_id)
    .bind(since)
    .bind(upto)
    .bind(limit)
    .fetch_all(conn)
    .await?)
}

/// Removes change rows older than `cutoff` or more than `keep_versions` versions behind their
/// set, and raises each set's `change_floor` to the newest version it lost. In batches, like the
/// outbox. Returns how many rows went.
pub async fn prune_changes(
    conn: &mut PgConnection,
    cutoff: DateTime<Utc>,
    keep_versions: i64,
) -> DbResult<u64> {
    const BATCH: i64 = 10_000;
    let mut removed = 0;
    loop {
        let gone: i64 = sqlx::query_scalar(
            "WITH gone AS (
                 DELETE FROM emote_set_changes WHERE id IN (
                     SELECT c.id FROM emote_set_changes c
                     JOIN emote_sets s ON s.id = c.set_id
                     WHERE c.changed_at < $1 OR c.version <= s.version - $2
                     ORDER BY c.id LIMIT $3)
                 RETURNING set_id, version
             ), floors AS (
                 UPDATE emote_sets s SET change_floor = GREATEST(s.change_floor, g.v)
                 FROM (SELECT set_id, max(version) AS v FROM gone GROUP BY set_id) g
                 WHERE s.id = g.set_id
                 RETURNING 1
             )
             SELECT count(*) FROM gone",
        )
        .bind(cutoff)
        .bind(keep_versions)
        .bind(BATCH)
        .fetch_one(&mut *conn)
        .await?;
        removed += u64::try_from(gone).unwrap_or(0);
        if gone < BATCH {
            return Ok(removed);
        }
    }
}
