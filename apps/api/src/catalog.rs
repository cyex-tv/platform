//! Channels, sets, set members and editors: the rules, shared by GraphQL and the REST reads.
//!
//! Who may change a set:
//! - the global set: holders of `global_set.edit`;
//! - any other set: its owner and its editors; anyone else needs `sets.edit_any`;
//! - deleting a set and managing its editors: its owner (or `sets.edit_any`).
//!
//! Every write runs in one transaction with its outbox event (ADR 0005) and, when it grants
//! something or relies on a permission instead of ownership, its audit row.

use std::collections::HashSet;

use axum::http::StatusCode;
use cyex_tv_auth::{AuthError, Caller, rbac};
use cyex_tv_db::sets::{self, NewMember, Set};
use cyex_tv_db::{DbError, accounts, audit, channels, emotes, ids, outbox, roles};
use cyex_tv_shared::error::{ApiError, ApiResult};
use cyex_tv_shared::model::{
    AddSetEmote, Channel, CreateSet, EmoteSet, EventType, PersonalSetEntry, Platform, SetEmote,
    SetKind, UpdateChannelSettings, UpdateSet, UpdateSetEmote,
};
use cyex_tv_shared::permissions;
use serde_json::json;
use sqlx::{Connection, PgConnection};
use uuid::Uuid;

use crate::state::AppState;
use crate::views::{self, Viewer};

/// The most chatters one personal-set lookup takes.
pub const LOOKUP_LIMIT: usize = 100;

// Errors

fn bad(code: &'static str, message: &str) -> ApiError {
    ApiError::new(StatusCode::BAD_REQUEST, code, message)
}

fn conflict(code: &'static str, message: &str) -> ApiError {
    ApiError::new(StatusCode::CONFLICT, code, message)
}

pub fn not_found() -> ApiError {
    ApiError::new(StatusCode::NOT_FOUND, "not_found", "Not found.")
}

pub fn forbidden(message: &str) -> ApiError {
    ApiError::new(StatusCode::FORBIDDEN, "forbidden", message)
}

/// A database error as an API error with its rule's code.
pub fn db_error(err: DbError) -> ApiError {
    match err {
        DbError::NotFound => not_found(),
        DbError::PersonalSetExists => {
            conflict("personal_set_exists", "You already have a personal set.")
        }
        DbError::InvalidSetKind => bad(
            "invalid_set_kind",
            "Sets of this kind can't be made or changed this way.",
        ),
        DbError::SetFull => conflict("set_full", "The set is full."),
        DbError::CapacityTooLow => conflict(
            "capacity_too_low",
            "The set already holds more emotes than that.",
        ),
        DbError::NameTaken => conflict("name_taken", "That name is already used in this set."),
        DbError::AlreadyInSet => conflict("already_in_set", "That emote is already in this set."),
        DbError::NotApproved => conflict(
            "not_approved",
            "That emote isn't approved yet. Until it is, it can only go in its owner's channel sets.",
        ),
        DbError::EmoteUnavailable => conflict("emote_unavailable", "That emote can't be used."),
        DbError::RoleNameTaken => conflict("role_name_taken", "A role with that name exists."),
        DbError::UnknownPermission(_) => bad("unknown_permission", "Unknown permission name."),
        err @ (DbError::IdentityTaken | DbError::PlatformAlreadyLinked | DbError::LastIdentity) => {
            AuthError::from(err).into()
        }
        DbError::Sqlx(err) => {
            tracing::error!(error = %err, "database error");
            ApiError::internal()
        }
    }
}

fn sqlx_error(err: sqlx::Error) -> ApiError {
    db_error(DbError::Sqlx(err))
}

async fn conn(state: &AppState) -> ApiResult<sqlx::pool::PoolConnection<sqlx::Postgres>> {
    state.db.acquire().await.map_err(sqlx_error)
}

/// Parses an ID from the API. A malformed one names nothing, so it is `not_found`.
pub fn parse_id(raw: &str) -> ApiResult<Uuid> {
    ids::from_text(raw.trim()).ok_or_else(not_found)
}

// Permissions

/// The permissions the caller holds: every one for the owner (ADR 0010), otherwise the union of
/// their roles'.
pub async fn permissions_of(conn: &mut PgConnection, caller: &Caller) -> ApiResult<Vec<String>> {
    if caller.owner {
        return Ok(rbac::all_permissions());
    }
    roles::permissions_of(conn, caller.account_id)
        .await
        .map_err(db_error)
}

pub async fn holds(conn: &mut PgConnection, caller: &Caller, permission: &str) -> ApiResult<bool> {
    Ok(permissions_of(conn, caller)
        .await?
        .iter()
        .any(|p| p == permission))
}

/// Whose eyes an answer is built for.
pub async fn viewer(conn: &mut PgConnection, caller: Option<&Caller>) -> ApiResult<Viewer> {
    let Some(caller) = caller else {
        return Ok(Viewer::public());
    };
    Ok(Viewer {
        account_id: Some(caller.account_id),
        reviewer: holds(conn, caller, permissions::EMOTES_REVIEW).await?,
    })
}

/// How the caller came to be allowed a set change.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Grant {
    Owner,
    Editor,
    /// Through a permission rather than the set: audited.
    Permission(&'static str),
}

/// May the caller change what's in `set` (rename, members)?
async fn edit_grant(conn: &mut PgConnection, caller: &Caller, set: &Set) -> ApiResult<Grant> {
    let account = caller.account_id;
    if set.kind == SetKind::Global {
        return if holds(conn, caller, permissions::GLOBAL_SET_EDIT).await? {
            Ok(Grant::Permission(permissions::GLOBAL_SET_EDIT))
        } else {
            Err(forbidden("Changing the global set needs global_set.edit."))
        };
    }
    if set.owner_account_id == Some(account) {
        return Ok(Grant::Owner);
    }
    if sets::can_edit(conn, account, set.id)
        .await
        .map_err(db_error)?
    {
        return Ok(Grant::Editor);
    }
    if holds(conn, caller, permissions::SETS_EDIT_ANY).await? {
        return Ok(Grant::Permission(permissions::SETS_EDIT_ANY));
    }
    Err(forbidden("Only the set's owner and editors can change it."))
}

/// May the caller delete `set` or manage its editors?
async fn owner_grant(conn: &mut PgConnection, caller: &Caller, set: &Set) -> ApiResult<Grant> {
    match edit_grant(conn, caller, set).await? {
        Grant::Editor => Err(forbidden("Only the set's owner can do this.")),
        grant => Ok(grant),
    }
}

async fn audit_if_permission(
    conn: &mut PgConnection,
    caller: &Caller,
    grant: Grant,
    action: &str,
    set_id: Uuid,
    before: Option<serde_json::Value>,
    after: Option<serde_json::Value>,
) -> ApiResult<()> {
    if let Grant::Permission(_) = grant {
        write_audit(
            conn,
            caller,
            action,
            "set",
            &ids::to_text(set_id),
            before,
            after,
        )
        .await?;
    }
    Ok(())
}

async fn write_audit(
    conn: &mut PgConnection,
    caller: &Caller,
    action: &str,
    target_kind: &str,
    target_id: &str,
    before: Option<serde_json::Value>,
    after: Option<serde_json::Value>,
) -> ApiResult<()> {
    audit::write(
        conn,
        &audit::NewEntry {
            actor: Some(caller.account_id),
            action,
            target_kind,
            target_id,
            before,
            after,
        },
    )
    .await
    .map_err(db_error)?;
    Ok(())
}

// Events

/// The topics a change to `set` is published on: the set, the channels showing it, and for a
/// personal set its owner's identities.
async fn set_topics(conn: &mut PgConnection, set: &Set) -> ApiResult<Vec<String>> {
    let mut topics = vec![outbox::set_topic(set.id)];
    for ch in channels::using_set(conn, set.id).await.map_err(db_error)? {
        topics.push(outbox::channel_topic(ch.platform, &ch.platform_channel_id));
    }
    if set.kind == SetKind::Personal
        && let Some(owner) = set.owner_account_id
    {
        for identity in accounts::identities(conn, owner).await.map_err(db_error)? {
            topics.push(outbox::user_topic(
                identity.platform,
                &identity.platform_user_id,
            ));
        }
    }
    Ok(topics)
}

async fn publish_member_event(
    conn: &mut PgConnection,
    set: &Set,
    event_type: EventType,
    emote_id: Uuid,
    name: Option<&str>,
) -> ApiResult<()> {
    let topics = set_topics(conn, set).await?;
    outbox::publish(
        conn,
        &outbox::NewEvent {
            event_type,
            topics,
            set_id: Some(set.id),
            payload: json!({
                "set_id": ids::to_text(set.id),
                "emote_id": ids::to_text(emote_id),
                "name": name,
            }),
        },
    )
    .await
    .map_err(db_error)?;
    Ok(())
}

// Checks on input

/// Set names: 1 to 100 characters after trimming, no control characters.
fn set_name(raw: &str) -> ApiResult<String> {
    let name = raw.trim();
    let len = name.chars().count();
    if len == 0 || len > 100 || name.chars().any(char::is_control) {
        return Err(bad("bad_name", "Set names are 1 to 100 characters."));
    }
    Ok(name.to_owned())
}

/// Names emotes are typed as in a set: 1 to 100 characters, no spaces.
fn member_name(raw: &str) -> ApiResult<&str> {
    let len = raw.chars().count();
    if len == 0 || len > 100 || raw.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return Err(bad(
            "bad_name",
            "Emote names are 1 to 100 characters, without spaces.",
        ));
    }
    Ok(raw)
}

fn default_capacity(kind: SetKind) -> i32 {
    match kind {
        SetKind::Channel => sets::CHANNEL_CAPACITY,
        SetKind::Personal => sets::PERSONAL_CAPACITY,
        SetKind::Global => sets::GLOBAL_CAPACITY,
    }
}

const MAX_CAPACITY: u32 = 100_000;

// Reads

pub async fn channel(state: &AppState, platform: Platform, id: &str) -> ApiResult<Option<Channel>> {
    let mut conn = conn(state).await?;
    let Some(ch) = channels::find(&mut conn, platform, id.trim())
        .await
        .map_err(db_error)?
    else {
        return Ok(None);
    };
    views::channel(&mut conn, &ch).await.map_err(db_error)
}

pub async fn channel_by_login(
    state: &AppState,
    platform: Platform,
    login: &str,
) -> ApiResult<Option<Channel>> {
    let mut conn = conn(state).await?;
    let Some(ch) = channels::find_by_login(&mut conn, platform, login.trim())
        .await
        .map_err(db_error)?
    else {
        return Ok(None);
    };
    views::channel(&mut conn, &ch).await.map_err(db_error)
}

pub async fn set(
    state: &AppState,
    caller: Option<&Caller>,
    id: &str,
) -> ApiResult<Option<EmoteSet>> {
    let Some(id) = ids::from_text(id.trim()) else {
        return Ok(None);
    };
    let mut conn = conn(state).await?;
    let Some(set) = sets::get(&mut conn, id).await.map_err(db_error)? else {
        return Ok(None);
    };
    let viewer = viewer(&mut conn, caller).await?;
    views::set(&mut conn, &set, &viewer, &state.cdn_url)
        .await
        .map(Some)
        .map_err(db_error)
}

pub async fn global_set(state: &AppState, caller: Option<&Caller>) -> ApiResult<EmoteSet> {
    let mut conn = conn(state).await?;
    let set = sets::ensure_global(&mut conn).await.map_err(db_error)?;
    let viewer = viewer(&mut conn, caller).await?;
    views::set(&mut conn, &set, &viewer, &state.cdn_url)
        .await
        .map_err(db_error)
}

/// Personal sets of chatters, by platform user ID. Unknown users, users without a personal set
/// and empty sets are left out. As chat clients see them: approved emotes only.
pub async fn personal_sets(
    state: &AppState,
    platform: Platform,
    user_ids: &[String],
) -> ApiResult<Vec<PersonalSetEntry>> {
    if user_ids.len() > LOOKUP_LIMIT {
        return Err(bad("too_many", "Look up at most 100 users at once."));
    }
    let mut seen = HashSet::new();
    let wanted: Vec<String> = user_ids
        .iter()
        .map(|id| id.trim().to_owned())
        .filter(|id| !id.is_empty() && id.len() <= 20 && id.bytes().all(|b| b.is_ascii_digit()))
        .filter(|id| seen.insert(id.clone()))
        .collect();
    if wanted.is_empty() {
        return Ok(Vec::new());
    }
    let mut conn = conn(state).await?;
    let found = sets::personal_for_users(&mut conn, platform, &wanted)
        .await
        .map_err(db_error)?;
    let set_rows: Vec<Set> = found.iter().map(|p| p.set.clone()).collect();
    let built = views::sets(&mut conn, &set_rows, &Viewer::public(), &state.cdn_url)
        .await
        .map_err(db_error)?;
    Ok(found
        .into_iter()
        .zip(built)
        .filter(|(_, set)| !set.emotes.is_empty())
        .map(|(p, set)| PersonalSetEntry {
            platform_user_id: p.platform_user_id,
            set,
        })
        .collect())
}

/// One user's personal set, as chat clients see it.
pub async fn personal_set(
    state: &AppState,
    platform: Platform,
    user_id: &str,
) -> ApiResult<Option<EmoteSet>> {
    Ok(personal_sets(state, platform, &[user_id.to_owned()])
        .await?
        .into_iter()
        .next()
        .map(|entry| entry.set))
}

/// The sets the caller owns, then the ones they edit.
pub async fn my_sets(state: &AppState, caller: &Caller) -> ApiResult<Vec<EmoteSet>> {
    let mut conn = conn(state).await?;
    let mut all = sets::list_for_owner(&mut conn, caller.account_id)
        .await
        .map_err(db_error)?;
    all.extend(
        sets::list_editable(&mut conn, caller.account_id)
            .await
            .map_err(db_error)?,
    );
    let viewer = viewer(&mut conn, Some(caller)).await?;
    views::sets(&mut conn, &all, &viewer, &state.cdn_url)
        .await
        .map_err(db_error)
}

pub async fn my_channels(state: &AppState, caller: &Caller) -> ApiResult<Vec<Channel>> {
    let mut conn = conn(state).await?;
    let mut out = Vec::new();
    for ch in channels::list_for_account(&mut conn, caller.account_id)
        .await
        .map_err(db_error)?
    {
        if let Some(view) = views::channel(&mut conn, &ch).await.map_err(db_error)? {
            out.push(view);
        }
    }
    Ok(out)
}

// Writes

async fn load_set(conn: &mut PgConnection, id: Uuid) -> ApiResult<Set> {
    sets::get(conn, id)
        .await
        .map_err(db_error)?
        .ok_or_else(not_found)
}

async fn built(state: &AppState, caller: &Caller, set_id: Uuid) -> ApiResult<EmoteSet> {
    let mut conn = conn(state).await?;
    let set = load_set(&mut conn, set_id).await?;
    let viewer = viewer(&mut conn, Some(caller)).await?;
    views::set(&mut conn, &set, &viewer, &state.cdn_url)
        .await
        .map_err(db_error)
}

pub async fn create_set(
    state: &AppState,
    caller: &Caller,
    input: &CreateSet,
) -> ApiResult<EmoteSet> {
    let name = set_name(&input.name)?;
    if input.kind == SetKind::Global {
        return Err(db_error(DbError::InvalidSetKind));
    }
    let mut conn = conn(state).await?;
    let set = sets::create(&mut conn, caller.account_id, &name, input.kind)
        .await
        .map_err(db_error)?;
    drop(conn);
    built(state, caller, set.id).await
}

pub async fn update_set(
    state: &AppState,
    caller: &Caller,
    id: &str,
    input: &UpdateSet,
) -> ApiResult<EmoteSet> {
    let id = parse_id(id)?;
    let name = input.name.as_deref().map(set_name).transpose()?;
    let mut conn = conn(state).await?;
    let mut tx = conn.begin().await.map_err(sqlx_error)?;
    let set = load_set(&mut tx, id).await?;
    let grant = edit_grant(&mut tx, caller, &set).await?;

    let mut after = set.clone();
    if let Some(name) = &name {
        after = sets::rename(&mut tx, id, name).await.map_err(db_error)?;
    }
    if let Some(capacity) = input.capacity {
        if capacity == 0 || capacity > MAX_CAPACITY {
            return Err(bad("bad_capacity", "Capacity is 1 to 100000."));
        }
        let capacity =
            i32::try_from(capacity).map_err(|_| bad("bad_capacity", "Capacity is 1 to 100000."))?;
        let mut grant_used = grant;
        if capacity > default_capacity(set.kind) && set.kind != SetKind::Global {
            if !holds(&mut tx, caller, permissions::SETS_EDIT_ANY).await? {
                return Err(forbidden(
                    "Raising a set above its default capacity needs sets.edit_any.",
                ));
            }
            grant_used = Grant::Permission(permissions::SETS_EDIT_ANY);
        }
        after = sets::set_capacity(&mut tx, id, capacity)
            .await
            .map_err(db_error)?;
        audit_if_permission(
            &mut tx,
            caller,
            grant_used,
            "set.capacity_changed",
            id,
            Some(json!({ "capacity": set.capacity })),
            Some(json!({ "capacity": after.capacity })),
        )
        .await?;
    }
    if name.is_some() {
        audit_if_permission(
            &mut tx,
            caller,
            grant,
            "set.renamed",
            id,
            Some(json!({ "name": set.name })),
            Some(json!({ "name": after.name })),
        )
        .await?;
    }
    tx.commit().await.map_err(sqlx_error)?;
    drop(conn);
    built(state, caller, id).await
}

pub async fn delete_set(state: &AppState, caller: &Caller, id: &str) -> ApiResult<bool> {
    let id = parse_id(id)?;
    let mut conn = conn(state).await?;
    let mut tx = conn.begin().await.map_err(sqlx_error)?;
    let set = load_set(&mut tx, id).await?;
    if set.kind == SetKind::Global {
        return Err(db_error(DbError::InvalidSetKind));
    }
    let grant = owner_grant(&mut tx, caller, &set).await?;
    if !channels::using_set(&mut tx, id)
        .await
        .map_err(db_error)?
        .is_empty()
    {
        return Err(conflict(
            "set_in_use",
            "A channel shows this set. Pick another active set first.",
        ));
    }
    let deleted = sets::delete(&mut tx, id).await.map_err(db_error)?;
    audit_if_permission(
        &mut tx,
        caller,
        grant,
        "set.deleted",
        id,
        Some(json!({ "name": set.name, "kind": set.kind })),
        None,
    )
    .await?;
    tx.commit().await.map_err(sqlx_error)?;
    Ok(deleted)
}

pub async fn add_set_emote(
    state: &AppState,
    caller: &Caller,
    set_id: &str,
    emote_id: &str,
    input: &AddSetEmote,
) -> ApiResult<SetEmote> {
    let set_id = parse_id(set_id)?;
    let emote_id = parse_id(emote_id)?;
    let alias = input.name.as_deref().map(member_name).transpose()?;
    let mut conn = conn(state).await?;
    let mut tx = conn.begin().await.map_err(sqlx_error)?;
    let set = load_set(&mut tx, set_id).await?;
    let grant = edit_grant(&mut tx, caller, &set).await?;

    let emote = emotes::get(&mut tx, emote_id)
        .await
        .map_err(db_error)?
        .ok_or_else(|| db_error(DbError::EmoteUnavailable))?;
    if emote.private && emote.owner_account_id != Some(caller.account_id) {
        return Err(conflict(
            "emote_private",
            "Only its owner can add a private emote to sets.",
        ));
    }
    let member = sets::add_member(
        &mut tx,
        set_id,
        NewMember {
            emote_id,
            name: alias,
            zero_width: input.zero_width,
            added_by: Some(caller.account_id),
        },
    )
    .await
    .map_err(db_error)?;
    publish_member_event(
        &mut tx,
        &set,
        EventType::SetEmoteAdded,
        emote_id,
        Some(&member.name),
    )
    .await?;
    audit_if_permission(
        &mut tx,
        caller,
        grant,
        "set.emote_added",
        set_id,
        None,
        Some(json!({ "emote_id": ids::to_text(emote_id), "name": member.name })),
    )
    .await?;
    tx.commit().await.map_err(sqlx_error)?;

    views::member(&mut conn, set_id, emote_id, &state.cdn_url)
        .await
        .map_err(db_error)?
        .ok_or_else(not_found)
}

pub async fn update_set_emote(
    state: &AppState,
    caller: &Caller,
    set_id: &str,
    emote_id: &str,
    input: &UpdateSetEmote,
) -> ApiResult<SetEmote> {
    let set_id = parse_id(set_id)?;
    let emote_id = parse_id(emote_id)?;
    let name = input.name.as_deref().map(member_name).transpose()?;
    let mut conn = conn(state).await?;
    let mut tx = conn.begin().await.map_err(sqlx_error)?;
    let set = load_set(&mut tx, set_id).await?;
    let grant = edit_grant(&mut tx, caller, &set).await?;
    let member = sets::update_member(&mut tx, set_id, emote_id, name, input.zero_width)
        .await
        .map_err(db_error)?;
    publish_member_event(
        &mut tx,
        &set,
        EventType::SetEmoteRenamed,
        emote_id,
        Some(&member.name),
    )
    .await?;
    audit_if_permission(
        &mut tx,
        caller,
        grant,
        "set.emote_updated",
        set_id,
        None,
        Some(json!({ "emote_id": ids::to_text(emote_id), "name": member.name, "zero_width": member.zero_width })),
    )
    .await?;
    tx.commit().await.map_err(sqlx_error)?;

    views::member(&mut conn, set_id, emote_id, &state.cdn_url)
        .await
        .map_err(db_error)?
        .ok_or_else(not_found)
}

/// False when the emote wasn't in the set.
pub async fn remove_set_emote(
    state: &AppState,
    caller: &Caller,
    set_id: &str,
    emote_id: &str,
) -> ApiResult<bool> {
    let set_id = parse_id(set_id)?;
    let emote_id = parse_id(emote_id)?;
    let mut conn = conn(state).await?;
    let mut tx = conn.begin().await.map_err(sqlx_error)?;
    let set = load_set(&mut tx, set_id).await?;
    let grant = edit_grant(&mut tx, caller, &set).await?;
    let member = match sets::remove_member(&mut tx, set_id, emote_id).await {
        Ok(member) => member,
        Err(DbError::NotFound) => return Ok(false),
        Err(err) => return Err(db_error(err)),
    };
    publish_member_event(
        &mut tx,
        &set,
        EventType::SetEmoteRemoved,
        emote_id,
        Some(&member.name),
    )
    .await?;
    audit_if_permission(
        &mut tx,
        caller,
        grant,
        "set.emote_removed",
        set_id,
        Some(json!({ "emote_id": ids::to_text(emote_id), "name": member.name })),
        None,
    )
    .await?;
    tx.commit().await.map_err(sqlx_error)?;
    Ok(true)
}

/// False when they were an editor already. Always audited: it grants access.
pub async fn add_set_editor(
    state: &AppState,
    caller: &Caller,
    set_id: &str,
    account_id: &str,
) -> ApiResult<bool> {
    let set_id = parse_id(set_id)?;
    let account_id = parse_id(account_id)?;
    let mut conn = conn(state).await?;
    let mut tx = conn.begin().await.map_err(sqlx_error)?;
    let set = load_set(&mut tx, set_id).await?;
    if set.kind == SetKind::Global {
        return Err(db_error(DbError::InvalidSetKind));
    }
    owner_grant(&mut tx, caller, &set).await?;
    if set.owner_account_id == Some(account_id) {
        return Err(bad("already_owner", "The set's owner can edit it already."));
    }
    let target = accounts::get(&mut tx, account_id)
        .await
        .map_err(db_error)?
        .filter(|a| a.deleted_at.is_none())
        .ok_or_else(not_found)?;
    let added = sets::add_editor(&mut tx, set_id, target.id, Some(caller.account_id))
        .await
        .map_err(db_error)?;
    if added {
        write_audit(
            &mut tx,
            caller,
            "set.editor_added",
            "set",
            &ids::to_text(set_id),
            None,
            Some(json!({ "account_id": ids::to_text(target.id) })),
        )
        .await?;
    }
    tx.commit().await.map_err(sqlx_error)?;
    Ok(added)
}

/// The owner removes an editor, or an editor leaves. False when they weren't one.
pub async fn remove_set_editor(
    state: &AppState,
    caller: &Caller,
    set_id: &str,
    account_id: &str,
) -> ApiResult<bool> {
    let set_id = parse_id(set_id)?;
    let account_id = parse_id(account_id)?;
    let mut conn = conn(state).await?;
    let mut tx = conn.begin().await.map_err(sqlx_error)?;
    let set = load_set(&mut tx, set_id).await?;
    if account_id != caller.account_id {
        owner_grant(&mut tx, caller, &set).await?;
    }
    let removed = sets::remove_editor(&mut tx, set_id, account_id)
        .await
        .map_err(db_error)?;
    if removed {
        write_audit(
            &mut tx,
            caller,
            "set.editor_removed",
            "set",
            &ids::to_text(set_id),
            Some(json!({ "account_id": ids::to_text(account_id) })),
            None,
        )
        .await?;
    }
    tx.commit().await.map_err(sqlx_error)?;
    Ok(removed)
}

/// Only the channel's owner. The active set must be one of the owner's channel sets; an empty
/// `activeSetId` clears it.
pub async fn update_channel_settings(
    state: &AppState,
    caller: &Caller,
    platform: Platform,
    id: &str,
    input: &UpdateChannelSettings,
) -> ApiResult<Channel> {
    let mut conn = conn(state).await?;
    let mut tx = conn.begin().await.map_err(sqlx_error)?;
    let ch = channels::find(&mut tx, platform, id.trim())
        .await
        .map_err(db_error)?
        .ok_or_else(not_found)?;
    if ch.account_id != caller.account_id {
        return Err(forbidden("Only the channel's owner can change it."));
    }
    let mut updated = ch.clone();
    if let Some(raw) = input.active_set_id.as_deref() {
        let set_id = if raw.trim().is_empty() {
            None
        } else {
            let set = load_set(&mut tx, parse_id(raw)?).await?;
            if set.kind != SetKind::Channel || set.owner_account_id != Some(ch.account_id) {
                return Err(bad(
                    "invalid_active_set",
                    "The active set must be one of your channel sets.",
                ));
            }
            Some(set.id)
        };
        if set_id != ch.active_set_id {
            updated = channels::set_active_set(&mut tx, ch.id, set_id)
                .await
                .map_err(db_error)?;
            outbox::publish(
                &mut tx,
                &outbox::NewEvent {
                    event_type: EventType::ChannelActiveSetChanged,
                    topics: vec![outbox::channel_topic(ch.platform, &ch.platform_channel_id)],
                    set_id,
                    payload: json!({ "set_id": set_id.map(ids::to_text) }),
                },
            )
            .await
            .map_err(db_error)?;
        }
    }
    if let Some(allow) = input.allow_personal_emotes {
        updated = channels::set_allow_personal_emotes(&mut tx, ch.id, allow)
            .await
            .map_err(db_error)?;
    }
    tx.commit().await.map_err(sqlx_error)?;
    views::channel(&mut conn, &updated)
        .await
        .map_err(db_error)?
        .ok_or_else(not_found)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_are_checked() {
        assert_eq!(set_name("  My set ").unwrap(), "My set");
        assert!(set_name("   ").is_err());
        assert!(set_name(&"a".repeat(101)).is_err());
        assert!(set_name("a\u{0}b").is_err());
        assert_eq!(member_name("KEKW").unwrap(), "KEKW");
        assert!(member_name("two words").is_err());
        assert!(member_name("").is_err());
        assert!(member_name(&"é".repeat(100)).is_ok());
        assert!(member_name(&"é".repeat(101)).is_err());
    }

    #[test]
    fn default_capacities_match_the_plan() {
        assert_eq!(default_capacity(SetKind::Channel), 1000);
        assert_eq!(default_capacity(SetKind::Personal), 5);
    }

    #[test]
    fn rule_errors_keep_their_codes() {
        for (err, code, status) in [
            (DbError::SetFull, "set_full", StatusCode::CONFLICT),
            (DbError::NotApproved, "not_approved", StatusCode::CONFLICT),
            (DbError::NotFound, "not_found", StatusCode::NOT_FOUND),
            (DbError::LastIdentity, "last_identity", StatusCode::CONFLICT),
            (
                DbError::InvalidSetKind,
                "invalid_set_kind",
                StatusCode::BAD_REQUEST,
            ),
        ] {
            let api = db_error(err);
            assert_eq!(api.code, code);
            assert_eq!(api.status, status);
        }
    }
}
