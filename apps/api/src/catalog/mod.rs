//! Channels, sets, set members and editors: the rules, shared by GraphQL and the REST reads.
//!
//! Who may change a set:
//! - the global set: holders of `global_set.edit`;
//! - any other set: its owner, its editors, and the owner's account editors holding the flag the
//!   change needs whose grant covers the set (every set, or a list naming it; ADR 0014); anyone
//!   else needs `sets.edit_any`;
//! - deleting a set and managing its editors: its owner, account editors with `sets` (delete) or
//!   `editors` and `emotes` (set editors), or `sets.edit_any`.
//!
//! Every set is public (ADR 0017, amendment 2026/11/22): only staff with `sets.edit_any` can make
//! one `UNLISTED` or `PRIVATE` (moderation), so the rules below are for those.
//!
//! Who may open a set (`set`, `GET /v1/sets/{id}`): anyone, unless it is `PRIVATE`; then its
//! owner, its editors, the owner's account editors and `sets.edit_any`, or anyone while a channel
//! shows it (chat shows it to everyone then).
//!
//! Every write runs in one transaction with its outbox event (ADR 0005) and, when it grants
//! something or relies on a permission instead of ownership, its audit row.

use std::collections::HashSet;

use cyex_tv_auth::{Caller, rbac};
use cyex_tv_db::sets::{self, NewMember, Set};
use cyex_tv_db::{DbError, accounts, channels, emotes, ids, outbox, reads, roles};
use cyex_tv_shared::error::{ApiError, ApiResult};
use cyex_tv_shared::model::{
    AccountSummary, AddSetEmote, Channel, ClonedSet, CreateSet, EditorPermission, EmotePage,
    EmoteSet, EmoteState, EventType, PersonalSetEntry, Platform, SetEmote, SetEmotePage, SetKind,
    SetSummary, SetVisibility, UpdateChannelSettings, UpdateSet, UpdateSetEmote,
};
use cyex_tv_shared::permissions;
use cyex_tv_shared::text;
use serde_json::json;
use sqlx::{Connection, PgConnection};
use uuid::Uuid;

use crate::editors::{has_flag, has_flag_on_all_sets, has_flag_on_set};
use crate::errors::{Audit, record_audit};
use crate::errors::{bad, conflict, rate_limited_in, sqlx_error};
use crate::limits::AccountLimit;
use crate::paging::Paged;

use crate::state::AppState;
use crate::views::{self, Viewer};

/// The most chatters one personal-set lookup takes.
pub const LOOKUP_LIMIT: usize = 100;

// Errors

pub(crate) use crate::errors::conn;
pub use crate::errors::{db_error, forbidden, not_found, parse_id};

// Permissions

/// The permissions the caller holds: every one for the owner (ADR 0010), otherwise the union of
/// their roles'. An app sign-in session (ADR 0021) holds only what every account holds (the
/// built-in User role): staff grants and the owner's never count for an app, so a deck token
/// can't edit any set through `sets.edit_any` or read what staff read.
pub async fn permissions_of(conn: &mut PgConnection, caller: &Caller) -> ApiResult<Vec<String>> {
    if caller.app_client.is_some() {
        return roles::everyones_permissions(conn).await.map_err(db_error);
    }
    if caller.owner {
        return Ok(rbac::all_permissions());
    }
    roles::permissions_of(conn, caller.account_id)
        .await
        .map_err(db_error)
}

pub async fn holds(conn: &mut PgConnection, caller: &Caller, permission: &str) -> ApiResult<bool> {
    Ok(has(&permissions_of(conn, caller).await?, permission))
}

/// Whether `held` (from [`permissions_of`]) has `permission`: for a handler that asks more than
/// once and loads the set a single time.
pub fn has(held: &[String], permission: &str) -> bool {
    held.iter().any(|p| p == permission)
}

/// Whether the caller holds at least one of `any`, with one load of their permissions.
pub async fn holds_any(conn: &mut PgConnection, caller: &Caller, any: &[&str]) -> ApiResult<bool> {
    let held = permissions_of(conn, caller).await?;
    Ok(any.iter().any(|p| has(&held, p)))
}

/// The 403 for a caller holding none of `any`: "This needs X.", "X or Y.", "X, Y or Z.".
fn needs(any: &[&str]) -> String {
    match any {
        [] => "This needs a permission.".to_owned(),
        [one] => format!("This needs {one}."),
        [init @ .., last] => format!("This needs {} or {last}.", init.join(", ")),
    }
}

/// Passes when the caller holds one of `any` (the owner holds all; an app session only what
/// everyone does), otherwise `403 forbidden` saying which are needed.
pub async fn require_any(conn: &mut PgConnection, caller: &Caller, any: &[&str]) -> ApiResult<()> {
    if holds_any(conn, caller, any).await? {
        Ok(())
    } else {
        Err(forbidden(&needs(any)))
    }
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
    /// A per-set editor.
    Editor,
    /// An account editor of the set's owner holding this flag (ADR 0014): audited.
    AccountEditor(EditorPermission),
    /// Through a permission rather than the set: audited.
    Permission(&'static str),
}

/// May the caller change `set` in a way an account editor needs `need` for? Per-set editors may
/// rename the set and change its members, as before.
async fn edit_grant(
    conn: &mut PgConnection,
    caller: &Caller,
    set: &Set,
    need: EditorPermission,
) -> ApiResult<Grant> {
    let account = caller.account_id;
    // A set the caller can't open answers like one that doesn't exist, whatever the change
    // (ADR 0017): forbidden here would say it exists.
    if !may_view(conn, Some(caller), set).await? {
        return Err(not_found());
    }
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
    if let Some(owner) = set.owner_account_id
        && has_flag_on_set(conn, owner, account, need, set.id).await?
    {
        return Ok(Grant::AccountEditor(need));
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

/// May the caller delete `set` or manage its editors? Per-set editors may not.
async fn owner_grant(
    conn: &mut PgConnection,
    caller: &Caller,
    set: &Set,
    need: EditorPermission,
) -> ApiResult<Grant> {
    match edit_grant(conn, caller, set, need).await? {
        Grant::Editor => Err(forbidden("Only the set's owner can do this.")),
        grant => Ok(grant),
    }
}

/// Audits a set change. The owner's own changes and those made through an account grant are
/// audited too, and tagged with the owner as their subject: they are what the owner's public
/// activity feed shows (`activity.rs`). Staff acting on a permission are audited as before and
/// never reach the feed. Per-set editors are not audited.
async fn audit_if_permission(
    conn: &mut PgConnection,
    caller: &Caller,
    grant: Grant,
    action: &str,
    set: &Set,
    before: Option<serde_json::Value>,
    after: Option<serde_json::Value>,
) -> ApiResult<()> {
    let (after, subject) = match grant {
        Grant::Permission(_) => (after, None),
        Grant::AccountEditor(_) => (Some(via_account_editor(after)), set.owner_account_id),
        Grant::Owner => (after, set.owner_account_id),
        Grant::Editor => return Ok(()),
    };
    record_audit(
        conn,
        caller.account_id,
        Audit::new(action, "set", &ids::to_text(set.id))
            .change(before, after)
            .subject(subject),
    )
    .await
}

/// Marks an audited change as made through an account editor grant.
fn via_account_editor(after: Option<serde_json::Value>) -> serde_json::Value {
    match after {
        Some(serde_json::Value::Object(mut map)) => {
            map.insert("via".to_owned(), json!("account_editor"));
            serde_json::Value::Object(map)
        }
        None => json!({ "via": "account_editor" }),
        Some(other) => json!({ "via": "account_editor", "value": other }),
    }
}

// Events

/// The topics a change to `set` is published on: the set, the channels showing it, and for a
/// personal set its owner's identities. A private set gets no `set:` topic (ADR 0017: it answers
/// like a set that doesn't exist, and every topic is public); unlisted sets keep it, since their
/// ID is already enough to open them. A channel showing the set makes it public, so those stay.
pub(crate) async fn set_topics(conn: &mut PgConnection, set: &Set) -> ApiResult<Vec<String>> {
    let mut topics = Vec::new();
    if set.visibility != SetVisibility::Private {
        topics.push(outbox::set_topic(set.id));
    }
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

pub(crate) async fn publish_member_event(
    conn: &mut PgConnection,
    set: &Set,
    event_type: EventType,
    emote_id: Uuid,
    name: Option<&str>,
) -> ApiResult<()> {
    // Every topic is public, and public reads hide emotes that aren't approved: an event naming
    // one would leak a name nobody has reviewed. Clients learn it when it is approved.
    if name.is_some() {
        let approved = emotes::get(conn, emote_id)
            .await
            .map_err(db_error)?
            .is_some_and(|e| e.state == EmoteState::Approved);
        if !approved {
            return Ok(());
        }
    }
    let topics = set_topics(conn, set).await?;
    if topics.is_empty() {
        return Ok(());
    }
    // ADR 0005: clients watch personal sets through `personal.updated`.
    let event_type = if set.kind == SetKind::Personal {
        EventType::PersonalUpdated
    } else {
        event_type
    };
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

/// `set.created` on the owner's `user:` topics, for listed sets only: an unlisted or private
/// set's ID is its secret, and every topic is public.
async fn publish_set_created(conn: &mut PgConnection, set: &Set) -> ApiResult<()> {
    let Some(owner) = set.owner_account_id else {
        return Ok(());
    };
    if set.visibility != SetVisibility::Listed {
        return Ok(());
    }
    let topics: Vec<String> = accounts::identities(conn, owner)
        .await
        .map_err(db_error)?
        .iter()
        .map(|i| outbox::user_topic(i.platform, &i.platform_user_id))
        .collect();
    if topics.is_empty() {
        return Ok(());
    }
    outbox::publish(
        conn,
        &outbox::NewEvent {
            event_type: EventType::SetCreated,
            topics,
            set_id: Some(set.id),
            payload: json!({ "set_id": ids::to_text(set.id), "name": set.name }),
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
    if len == 0 || len > 100 || text::has_unsafe(name, false) {
        return Err(bad("bad_name", "Set names are 1 to 100 characters."));
    }
    Ok(name.to_owned())
}

/// Names emotes are typed as in a set: 1 to 100 characters, no spaces.
pub(crate) fn member_name(raw: &str) -> ApiResult<&str> {
    let len = raw.chars().count();
    if len == 0 || len > 100 || raw.chars().any(char::is_whitespace) || text::has_unsafe(raw, false)
    {
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

/// The most slots staff with `sets.edit_any` may give any set (ADR 0017, amendment 2026/11/22).
pub const STAFF_CAPACITY_MAX: u32 = 1_000_000;

/// The most slots the owner's side (the owner, their account editors and per-set editors) may give
/// `set` without a staff permission: a channel set the owner's `set_capacity_max` role limit (1.000
/// unless a role raises it), the personal set its default, and never less than it has now (staff
/// may have raised it: lowering it again stays allowed). The global set is staff-only anyway.
/// `set_full` with the set's capacity: it holds that many, and that is the most.
pub(crate) fn set_full(capacity: i32) -> ApiError {
    let capacity = u64::try_from(capacity).unwrap_or(0);
    ApiError::limit("set_full", "The set is full.", capacity, capacity)
}

pub(crate) async fn user_capacity_max(state: &AppState, set: &Set) -> u32 {
    let current = u32::try_from(set.capacity).unwrap_or(0);
    let limit = match (set.kind, set.owner_account_id) {
        (SetKind::Channel, Some(owner)) => state
            .limits
            .load(&state.db, owner)
            .await
            .get(AccountLimit::SetCapacityMax),
        (kind, _) => u32::try_from(default_capacity(kind)).unwrap_or(1),
    };
    limit.max(current)
}

/// What the caller may set `id`'s capacity to; `None` when they can't change it (or can't see
/// the set: the same answer, so it says nothing about private sets).
pub async fn set_capacity_range(
    state: &AppState,
    caller: &Caller,
    id: &str,
) -> ApiResult<Option<cyex_tv_shared::model::SetCapacityRange>> {
    use axum::http::StatusCode;
    let id = parse_id(id)?;
    let mut conn = conn(state).await?;
    let Some(set) = sets::get(&mut conn, id).await.map_err(db_error)? else {
        return Ok(None);
    };
    match edit_grant(&mut conn, caller, &set, EditorPermission::Sets).await {
        Ok(_) => {}
        Err(e) if matches!(e.status, StatusCode::FORBIDDEN | StatusCode::NOT_FOUND) => {
            return Ok(None);
        }
        Err(e) => return Err(e),
    }
    let used = sets::count(&mut conn, id).await.map_err(db_error)?;
    let used = u32::try_from(used).unwrap_or(u32::MAX);
    let owner_max = user_capacity_max(state, &set).await;
    let staff =
        set.kind == SetKind::Global || holds(&mut conn, caller, permissions::SETS_EDIT_ANY).await?;
    let max = if staff { STAFF_CAPACITY_MAX } else { owner_max };
    Ok(Some(cyex_tv_shared::model::SetCapacityRange {
        min: used.max(1),
        max,
        owner_max,
        used,
        staff,
    }))
}

/// 1000000 as "1.000.000" (cyex number format).
fn dotted(n: u32) -> String {
    let digits = n.to_string();
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push('.');
        }
        out.push(c);
    }
    out
}

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

/// An account's public summary; `None` when unknown, deleted or blocked.
pub async fn account(state: &AppState, id: &str) -> ApiResult<Option<AccountSummary>> {
    let Some(id) = ids::from_text(id.trim()) else {
        return Ok(None);
    };
    let mut conn = conn(state).await?;
    let Some(acc) = accounts::get(&mut conn, id).await.map_err(db_error)? else {
        return Ok(None);
    };
    if acc.deleted_at.is_some() || acc.blocked_at.is_some() {
        return Ok(None);
    }
    let summaries = reads::summaries(&mut conn, &[id]).await.map_err(db_error)?;
    Ok(summaries.get(&id).map(AccountSummary::from))
}

/// How far into the list `offset` may reach (a page past it is empty).
pub const EMOTE_CHANNELS_OFFSET_MAX: u32 = 10_000;

/// Whether anyone may see the emote and what happens to it: approved, not private, not deleted,
/// and not of an account whose deletion is pending (ADR 0019). Unlisted emotes count: their page
/// is open to anyone with the link.
pub(crate) async fn public_emote(conn: &mut PgConnection, id: Uuid) -> ApiResult<bool> {
    let row = reads::emotes(conn, &[id])
        .await
        .map_err(db_error)?
        .into_iter()
        .next()
        .filter(|e| e.state == EmoteState::Approved && !e.private);
    Ok(match row {
        Some(e) => !crate::moderation::owner_hidden(conn, e.owner).await?,
        None => false,
    })
}

/// Channels showing a public emote, `offset` rows in (for paging). Unknown, pending, private and
/// deleted emotes have none.
pub async fn emote_channels(
    state: &AppState,
    id: &str,
    limit: Option<u32>,
    offset: Option<u32>,
) -> ApiResult<Vec<Channel>> {
    let Some(id) = ids::from_text(id.trim()) else {
        return Ok(Vec::new());
    };
    let limit = Paged::EmoteChannels.params().size(limit);
    let offset = offset.unwrap_or(0);
    if offset > EMOTE_CHANNELS_OFFSET_MAX {
        return Ok(Vec::new());
    }
    let mut conn = conn(state).await?;
    if !public_emote(&mut conn, id).await? {
        return Ok(Vec::new());
    }
    let chs = reads::emote_channels(&mut conn, id, i64::from(limit), i64::from(offset))
        .await
        .map_err(db_error)?;
    views::channels_of(&mut conn, &chs).await.map_err(db_error)
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
    if !may_view(&mut conn, caller, &set).await? {
        return Ok(None);
    }
    let viewer = viewer(&mut conn, caller).await?;
    views::set(&mut conn, &set, &viewer, &state.cdn_url)
        .await
        .map(Some)
        .map_err(db_error)
}

/// A page of a set's emotes, oldest first, with the same visibility rule as `set`: `query` keeps
/// the members whose name in the set or own name contains it; `perPage` is 1 to 200 (default 120).
pub async fn set_emotes(
    state: &AppState,
    caller: Option<&Caller>,
    id: &str,
    query: Option<&str>,
    page: Option<u32>,
    per_page: Option<u32>,
    after: Option<&str>,
) -> ApiResult<SetEmotePage> {
    let empty = SetEmotePage {
        items: Vec::new(),
        total: 0,
        next: None,
    };
    let Some(id) = ids::from_text(id.trim()) else {
        return Ok(empty);
    };
    let paged = Paged::SetEmotes.paging(page, per_page);
    // The cursor belongs to this set and filter.
    let scope = format!(
        "setEmotes|{}|{}",
        ids::to_text(id),
        query.map(str::trim).unwrap_or_default()
    );
    let after = crate::cursor::decode(after, &scope, &reads::member_keys())?;
    let (limit, offset) = match after {
        Some(_) => (i64::from(paged.per_page), 0),
        None => (paged.limit, paged.offset),
    };
    let mut conn = conn(state).await?;
    let Some(set) = sets::get(&mut conn, id).await.map_err(db_error)? else {
        return Ok(empty);
    };
    if !may_view(&mut conn, caller, &set).await? {
        return Ok(empty);
    }
    let viewer = viewer(&mut conn, caller).await?;
    let (items, total, next) = views::set_emotes(
        &mut conn,
        &set,
        &viewer,
        &state.cdn_url,
        query,
        limit,
        offset,
        after.as_deref(),
    )
    .await
    .map_err(db_error)?;
    Ok(SetEmotePage {
        items,
        total: u64::try_from(total).unwrap_or(0),
        next: crate::cursor::next(&scope, next.as_ref()),
    })
}

/// Whether the caller may open `set` (the module's rule). A private set the caller may not see
/// answers like one that doesn't exist.
pub(crate) async fn may_view(
    conn: &mut PgConnection,
    caller: Option<&Caller>,
    set: &Set,
) -> ApiResult<bool> {
    // The sets of an account whose deletion is pending are hidden with it (ADR 0019), from
    // everyone but staff.
    if let Some(owner) = set.owner_account_id
        && accounts::get(conn, owner)
            .await
            .map_err(db_error)?
            .is_some_and(|a| a.deleted_at.is_some())
    {
        return match caller {
            Some(c) => holds(conn, c, permissions::SETS_EDIT_ANY).await,
            None => Ok(false),
        };
    }
    if set.visibility != SetVisibility::Private {
        return Ok(true);
    }
    if !sets::active(conn, &[set.id])
        .await
        .map_err(db_error)?
        .is_empty()
    {
        return Ok(true);
    }
    let Some(caller) = caller else {
        return Ok(false);
    };
    let account = caller.account_id;
    if set.owner_account_id == Some(account) {
        return Ok(true);
    }
    if let Some(owner) = set.owner_account_id
        && cyex_tv_db::editors::covers_set(conn, owner, account, set.id)
            .await
            .map_err(db_error)?
    {
        return Ok(true);
    }
    if sets::can_edit(conn, account, set.id)
        .await
        .map_err(db_error)?
    {
        return Ok(true);
    }
    holds(conn, caller, permissions::SETS_EDIT_ANY).await
}

/// A live account's public profile: linked logins, public roles, each channel's active set.
/// `None` when the account is unknown, deleted or blocked.
pub async fn account_profile(
    state: &AppState,
    account_id: &str,
) -> ApiResult<Option<cyex_tv_shared::model::AccountProfile>> {
    use cyex_tv_shared::model::{AccountProfile, ActiveChannelSet, PublicIdentity, PublicRole};
    let Some(id) = ids::from_text(account_id.trim()) else {
        return Ok(None);
    };
    let mut conn = conn(state).await?;
    let Some(row) = accounts::get(&mut conn, id).await.map_err(db_error)? else {
        return Ok(None);
    };
    if row.deleted_at.is_some() || row.blocked_at.is_some() {
        return Ok(None);
    }
    let Some(summary) = reads::summaries(&mut conn, &[id])
        .await
        .map_err(db_error)?
        .get(&id)
        .map(AccountSummary::from)
    else {
        return Ok(None);
    };
    let identities = accounts::identities(&mut conn, id)
        .await
        .map_err(db_error)?
        .into_iter()
        .map(|i| PublicIdentity {
            platform: i.platform,
            login: i.login,
            display_name: i.display_name,
        })
        .collect();
    let roles = roles::public_roles_of(&mut conn, id)
        .await
        .map_err(db_error)?
        .into_iter()
        .map(|r| PublicRole {
            id: ids::to_text(r.id),
            name: r.name,
        })
        .collect();
    let channel_rows: Vec<_> = channels::list_for_account(&mut conn, id)
        .await
        .map_err(db_error)?
        .into_iter()
        .filter(|ch| ch.active_set_id.is_some())
        .collect();
    let set_ids: Vec<Uuid> = channel_rows
        .iter()
        .filter_map(|c| c.active_set_id)
        .collect();
    let by_id: std::collections::HashMap<Uuid, Set> = sets::get_many(&mut conn, &set_ids)
        .await
        .map_err(db_error)?
        .into_iter()
        .map(|s| (s.id, s))
        .collect();
    let active_sets = channel_rows
        .into_iter()
        .filter_map(|ch| {
            let set = by_id.get(&ch.active_set_id?)?;
            Some(ActiveChannelSet {
                platform: ch.platform,
                login: ch.login,
                set_id: ids::to_text(set.id),
                set_name: set.name.clone(),
            })
        })
        .collect();
    let editors = crate::editors::public_editors(&mut conn, id).await?;
    Ok(Some(AccountProfile {
        account: summary,
        identities,
        roles,
        active_sets,
        created_at: views::time(row.created_at),
        editors,
        collection_public: cyex_tv_db::prefs::get(&mut conn, id)
            .await
            .map_err(db_error)?
            .show_collection,
    }))
}

/// Cards per `accountSets` answer, and emotes per card.
pub const ACCOUNT_SETS_MAX: i64 = 50;

/// An account's listed sets, oldest first, as profile cards. Empty when the account is unknown,
/// deleted or blocked.
pub async fn account_sets(
    state: &AppState,
    caller: Option<&Caller>,
    account_id: &str,
    preview: Option<u32>,
) -> ApiResult<Vec<SetSummary>> {
    let Some(id) = ids::from_text(account_id.trim()) else {
        return Ok(Vec::new());
    };
    let preview = {
        let size = Paged::SetPreview.params();
        preview.unwrap_or(size.default).min(size.max) as usize
    };
    let mut conn = conn(state).await?;
    let live = accounts::get(&mut conn, id)
        .await
        .map_err(db_error)?
        .is_some_and(|a| a.deleted_at.is_none() && a.blocked_at.is_none());
    if !live {
        return Ok(Vec::new());
    }
    let rows = sets::list_listed(&mut conn, id, ACCOUNT_SETS_MAX)
        .await
        .map_err(db_error)?;
    let viewer = viewer(&mut conn, caller).await?;
    views::set_summaries(&mut conn, &rows, &viewer, &state.cdn_url, preview)
        .await
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
        return Err(ApiError::limit(
            "too_many",
            "Look up at most 100 users at once.",
            LOOKUP_LIMIT as u64,
            cyex_tv_shared::limit_errors::n(user_ids.len()),
        ));
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

/// The caller's own emotes in every state (pending, rejected and taken down too), newest first.
pub async fn my_emotes(
    state: &AppState,
    caller: &Caller,
    page: Option<u32>,
    per_page: Option<u32>,
    after: Option<&str>,
) -> ApiResult<EmotePage> {
    let paged = Paged::ReviewQueue.paging(page, per_page);
    let mut conn = conn(state).await?;
    owner_emotes(state, &mut conn, caller.account_id, paged, after).await
}

/// One page of an account's own emotes in every state, newest first (`myEmotes`,
/// `accountEmotes`), by number or after a cursor. The cursor is bound to the account.
pub(crate) async fn owner_emotes(
    state: &AppState,
    conn: &mut PgConnection,
    owner: Uuid,
    paged: crate::paging::Paging,
    after: Option<&str>,
) -> ApiResult<EmotePage> {
    let scope = format!("ownerEmotes|{}", ids::to_text(owner));
    let after = crate::cursor::decode(after, &scope, &reads::owner_keys())?;
    let (limit, offset) = match after {
        Some(_) => (i64::from(paged.per_page), 0),
        None => (paged.limit, paged.offset),
    };
    let (page, total) = reads::emotes_by_owner_page(conn, owner, limit, offset, after.as_deref())
        .await
        .map_err(db_error)?;
    Ok(EmotePage {
        items: views::emotes(conn, &page.rows, &state.cdn_url)
            .await
            .map_err(db_error)?,
        total: u64::try_from(total).unwrap_or(0),
        next: crate::cursor::next(&scope, page.next.as_ref()),
    })
}

/// A set's editors, oldest grant first. For whoever may change the set.
pub async fn set_editors(
    state: &AppState,
    caller: &Caller,
    set_id: &str,
) -> ApiResult<Vec<AccountSummary>> {
    let set_id = parse_id(set_id)?;
    let mut conn = conn(state).await?;
    let set = load_set(&mut conn, set_id).await?;
    if edit_grant(&mut conn, caller, &set, EditorPermission::Emotes)
        .await
        .is_err()
    {
        edit_grant(&mut conn, caller, &set, EditorPermission::Editors).await?;
    }
    let editors = sets::editors(&mut conn, set_id).await.map_err(db_error)?;
    let account_ids: Vec<Uuid> = editors.iter().map(|e| e.account_id).collect();
    let summaries = reads::summaries(&mut conn, &account_ids)
        .await
        .map_err(db_error)?;
    Ok(editors
        .iter()
        .filter_map(|e| summaries.get(&e.account_id))
        .map(|s| AccountSummary {
            id: ids::to_text(s.account_id),
            display_name: s.display_name.clone(),
            avatar: s.avatar.clone(),
        })
        .collect())
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

pub(crate) async fn load_set(conn: &mut PgConnection, id: Uuid) -> ApiResult<Set> {
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
    let owner = match input.owner_id.as_deref().map(str::trim) {
        Some(raw) if !raw.is_empty() => parse_id(raw)?,
        _ => caller.account_id,
    };
    let mut conn = conn(state).await?;
    let mut tx = conn.begin().await.map_err(sqlx_error)?;
    let by_editor = owner != caller.account_id;
    if by_editor {
        may_make_sets(&mut tx, owner, caller.account_id).await?;
    }
    staff_only_visibility(&mut tx, caller, input.visibility).await?;
    let set = sets::create_with(&mut tx, owner, &name, input.kind, input.visibility)
        .await
        .map_err(db_error)?;
    let grant = if by_editor {
        Grant::AccountEditor(EditorPermission::Sets)
    } else {
        Grant::Owner
    };
    audit_if_permission(
        &mut tx,
        caller,
        grant,
        "set.created",
        &set,
        None,
        Some(json!({
            "name": set.name,
            "kind": set.kind,
            "owner_id": ids::to_text(owner),
            "visibility": set.visibility,
        })),
    )
    .await?;
    publish_set_created(&mut tx, &set).await?;
    tx.commit().await.map_err(sqlx_error)?;
    drop(conn);
    built(state, caller, set.id).await
}

/// Every set is public: asking for `UNLISTED` or `PRIVATE` needs `sets.edit_any` (staff hiding a
/// set, moderation). `LISTED`, or nothing, is fine for anyone.
async fn staff_only_visibility(
    conn: &mut PgConnection,
    caller: &Caller,
    visibility: Option<SetVisibility>,
) -> ApiResult<()> {
    match visibility {
        Some(v) if v != SetVisibility::Listed => {
            if holds(conn, caller, permissions::SETS_EDIT_ANY).await? {
                Ok(())
            } else {
                Err(public_only())
            }
        }
        _ => Ok(()),
    }
}

fn public_only() -> ApiError {
    bad(
        "bad_visibility",
        "Every set is public. Only staff can take a set off the listings.",
    )
}

/// An account editor making a set for the owner needs `sets` on every set: one limited to some
/// sets couldn't work on the new one.
async fn may_make_sets(conn: &mut PgConnection, owner: Uuid, editor: Uuid) -> ApiResult<()> {
    if has_flag_on_all_sets(conn, owner, editor, EditorPermission::Sets).await? {
        return Ok(());
    }
    if has_flag(conn, owner, editor, EditorPermission::Sets).await? {
        return Err(forbidden(
            "You work on some of this account's sets only, so you can't make new ones for it.",
        ));
    }
    Err(forbidden(
        "Only the account's owner and its editors with the sets permission can make sets for it.",
    ))
}

/// Copies a set the caller may open into a new channel set owned by the caller, or by an account
/// they edit with `sets`. Emotes go in as `addSetEmote` would put them (capacity, approved only,
/// others' private emotes left out), in the source's order, keeping the source's names.
pub async fn clone_set(
    state: &AppState,
    caller: &Caller,
    id: &str,
    name: Option<&str>,
    owner_id: Option<&str>,
    visibility: Option<SetVisibility>,
) -> ApiResult<ClonedSet> {
    let source_id = parse_id(id)?;
    let owner = match owner_id.map(str::trim) {
        Some(raw) if !raw.is_empty() => parse_id(raw)?,
        _ => caller.account_id,
    };
    let name = name.map(set_name).transpose()?;
    state
        .limits
        .take(
            crate::limits::AccountLimit::SetClones,
            &ids::to_text(caller.account_id),
            Some(caller.account_id),
        )
        .map_err(|wait| {
            rate_limited_in("That's a lot of copied sets. Try again in a while.", wait)
        })?;
    let mut conn = conn(state).await?;
    let mut tx = conn.begin().await.map_err(sqlx_error)?;
    let source = load_set(&mut tx, source_id).await?;
    if !may_view(&mut tx, Some(caller), &source).await? {
        return Err(not_found());
    }
    let by_editor = owner != caller.account_id;
    if by_editor {
        may_make_sets(&mut tx, owner, caller.account_id).await?;
    }
    staff_only_visibility(&mut tx, caller, visibility).await?;
    let name = match name {
        Some(name) => name,
        None => set_name(&source.name)?,
    };
    let set = sets::create_with(&mut tx, owner, &name, SetKind::Channel, visibility)
        .await
        .map_err(db_error)?;

    // As the caller sees the source: pending emotes only when they may see them.
    let viewer = viewer(&mut tx, Some(caller)).await?;
    let shown = views::set(&mut tx, &source, &viewer, &state.cdn_url)
        .await
        .map_err(db_error)?;
    let total = sets::count(&mut tx, source.id).await.map_err(db_error)?;
    let (mut copied, mut skipped) = (0u32, 0u32);
    for member in &shown.emotes {
        let Some(emote_id) = ids::from_text(&member.emote.id) else {
            skipped += 1;
            continue;
        };
        // Private emotes: the caller's own, or, for an account editor, the new owner's own.
        let emote_owner = member
            .emote
            .owner
            .as_ref()
            .and_then(|o| ids::from_text(&o.id));
        let own =
            emote_owner == Some(caller.account_id) || (by_editor && emote_owner == Some(owner));
        if member.emote.flags.private && !own {
            skipped += 1;
            continue;
        }
        let added = sets::add_member(
            &mut tx,
            set.id,
            NewMember {
                emote_id,
                name: Some(&member.name),
                zero_width: Some(member.zero_width),
                added_by: Some(caller.account_id),
            },
        )
        .await;
        match added {
            Ok(_) => copied += 1,
            Err(
                DbError::SetFull
                | DbError::NotApproved
                | DbError::EmoteUnavailable
                | DbError::NameTaken
                | DbError::AlreadyInSet,
            ) => skipped += 1,
            Err(err) => return Err(db_error(err)),
        }
    }
    // Members the caller couldn't see weren't copied either.
    skipped += u32::try_from(total)
        .unwrap_or(0)
        .saturating_sub(copied + skipped);
    // A copy is a new set: the same audit row as `createSet`, so it shows in the activity feed.
    let grant = if by_editor {
        Grant::AccountEditor(EditorPermission::Sets)
    } else {
        Grant::Owner
    };
    audit_if_permission(
        &mut tx,
        caller,
        grant,
        "set.created",
        &set,
        None,
        Some(json!({
            "name": set.name,
            "kind": set.kind,
            "owner_id": ids::to_text(owner),
            "visibility": set.visibility,
            "source_id": ids::to_text(source.id),
            "copied": copied,
            "skipped": skipped,
        })),
    )
    .await?;
    publish_set_created(&mut tx, &set).await?;
    tx.commit().await.map_err(sqlx_error)?;
    drop(conn);
    Ok(ClonedSet {
        set: built(state, caller, set.id).await?,
        copied,
        skipped,
    })
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
    let grant = edit_grant(&mut tx, caller, &set, EditorPermission::Sets).await?;

    let mut after = set.clone();
    if let Some(name) = &name {
        after = sets::rename(&mut tx, id, name).await.map_err(db_error)?;
    }
    if let Some(capacity) = input.capacity {
        if capacity == 0 || capacity > STAFF_CAPACITY_MAX {
            return Err(bad(
                "bad_capacity",
                "Capacity is a whole number from 1 to 1.000.000.",
            ));
        }
        let staff = holds(&mut tx, caller, permissions::SETS_EDIT_ANY).await?;
        let mine = user_capacity_max(state, &set).await;
        let mut grant_used = grant;
        if capacity > mine && set.kind != SetKind::Global {
            if !staff {
                return Err(ApiError::limit(
                    "capacity_over_limit",
                    format!("This set can have up to {} slots.", dotted(mine)),
                    mine,
                    capacity,
                ));
            }
            grant_used = Grant::Permission(permissions::SETS_EDIT_ANY);
        }
        let capacity = i32::try_from(capacity).map_err(|_| {
            bad(
                "bad_capacity",
                "Capacity is a whole number from 1 to 1.000.000.",
            )
        })?;
        after = sets::set_capacity(&mut tx, id, capacity)
            .await
            .map_err(db_error)?;
        audit_if_permission(
            &mut tx,
            caller,
            grant_used,
            "set.capacity_changed",
            &set,
            Some(json!({ "capacity": set.capacity })),
            Some(json!({ "capacity": after.capacity })),
        )
        .await?;
    }
    if let Some(visibility) = input.visibility
        && visibility != set.visibility
    {
        if set.kind == SetKind::Global {
            return Err(db_error(DbError::InvalidVisibility));
        }
        // Every set is public: only staff with `sets.edit_any` change it (hiding a set, or
        // listing one they hid), audited as staff.
        if !holds(&mut tx, caller, permissions::SETS_EDIT_ANY).await? {
            return Err(public_only());
        }
        let grant = Grant::Permission(permissions::SETS_EDIT_ANY);
        after = sets::set_visibility(&mut tx, id, visibility)
            .await
            .map_err(db_error)?;
        audit_if_permission(
            &mut tx,
            caller,
            grant,
            "set.visibility_changed",
            &set,
            Some(json!({ "visibility": set.visibility })),
            Some(json!({ "visibility": after.visibility })),
        )
        .await?;
    }
    // Renames and visibility changes make no client event; every instance's cached reads of the
    // set (and of channels showing it) still have to go.
    let forget = if name.is_some() || after.visibility != set.visibility {
        Some(forget_set_reads(&mut tx, &set).await?)
    } else {
        None
    };
    if name.is_some() {
        audit_if_permission(
            &mut tx,
            caller,
            grant,
            "set.renamed",
            &set,
            Some(json!({ "name": set.name })),
            Some(json!({ "name": after.name })),
        )
        .await?;
    }
    tx.commit().await.map_err(sqlx_error)?;
    if let Some(keys) = forget {
        state.forget_cached(&keys);
    }
    drop(conn);
    built(state, caller, id).await
}

/// Publishes `cache.forget` for the cached reads showing `set` (itself, the global set read for
/// the global set, and the channels showing it) and returns their keys, for the caller to drop
/// on its own instance after the commit.
async fn forget_set_reads(conn: &mut PgConnection, set: &Set) -> ApiResult<Vec<String>> {
    let mut keys = vec![crate::read_cache::set_key(set.id)];
    if set.kind == SetKind::Global {
        keys.push(crate::read_cache::GLOBAL_SET_KEY.to_owned());
    }
    for ch in channels::using_set(conn, set.id).await.map_err(db_error)? {
        keys.push(outbox::channel_topic(ch.platform, &ch.platform_channel_id));
    }
    outbox::publish_cache_forget(conn, &keys)
        .await
        .map_err(db_error)?;
    Ok(keys)
}

pub async fn delete_set(state: &AppState, caller: &Caller, id: &str) -> ApiResult<bool> {
    let id = parse_id(id)?;
    let mut conn = conn(state).await?;
    let mut tx = conn.begin().await.map_err(sqlx_error)?;
    let set = load_set(&mut tx, id).await?;
    if set.kind == SetKind::Global {
        return Err(db_error(DbError::InvalidSetKind));
    }
    let grant = owner_grant(&mut tx, caller, &set, EditorPermission::Sets).await?;
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
    let forget = forget_set_reads(&mut tx, &set).await?;
    audit_if_permission(
        &mut tx,
        caller,
        grant,
        "set.deleted",
        &set,
        Some(json!({ "name": set.name, "kind": set.kind })),
        None,
    )
    .await?;
    tx.commit().await.map_err(sqlx_error)?;
    state.forget_cached(&forget);
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
    let grant = edit_grant(&mut tx, caller, &set, EditorPermission::Emotes).await?;

    let emote = emotes::get(&mut tx, emote_id)
        .await
        .map_err(db_error)?
        .ok_or_else(|| db_error(DbError::EmoteUnavailable))?;
    // An emote of an account whose deletion is pending is hidden with it (ADR 0019).
    if emote.owner_account_id != Some(caller.account_id)
        && crate::moderation::owner_hidden(&mut tx, emote.owner_account_id).await?
    {
        return Err(db_error(DbError::EmoteUnavailable));
    }
    // An account editor with `emotes` may use the owner's own private emotes in the owner's sets.
    let owners_own = matches!(grant, Grant::AccountEditor(_))
        && emote.owner_account_id.is_some()
        && emote.owner_account_id == set.owner_account_id;
    if emote.private && emote.owner_account_id != Some(caller.account_id) && !owners_own {
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
    .map_err(|err| match err {
        DbError::SetFull => set_full(set.capacity),
        err => db_error(err),
    })?;
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
        &set,
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
    let grant = edit_grant(&mut tx, caller, &set, EditorPermission::Emotes).await?;
    let previous = reads::member(&mut tx, set_id, emote_id)
        .await
        .map_err(db_error)?;
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
        &set,
        previous.map(|p| {
            json!({ "emote_id": ids::to_text(emote_id), "name": p.name, "zero_width": p.zero_width })
        }),
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
    let grant = edit_grant(&mut tx, caller, &set, EditorPermission::Emotes).await?;
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
        &set,
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
    let grant = owner_grant(&mut tx, caller, &set, EditorPermission::Editors).await?;
    // A set editor can change emotes, so granting one needs `emotes` as well.
    if let (Grant::AccountEditor(_), Some(owner)) = (grant, set.owner_account_id)
        && !has_flag(&mut tx, owner, caller.account_id, EditorPermission::Emotes).await?
    {
        return Err(forbidden("You can only grant permissions you hold."));
    }
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
        record_audit(
            &mut tx,
            caller.account_id,
            Audit::new("set.editor_added", "set", &ids::to_text(set_id))
                .change(None, Some(json!({ "account_id": ids::to_text(target.id) }))),
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
        owner_grant(&mut tx, caller, &set, EditorPermission::Editors).await?;
    }
    let removed = sets::remove_editor(&mut tx, set_id, account_id)
        .await
        .map_err(db_error)?;
    if removed {
        record_audit(
            &mut tx,
            caller.account_id,
            Audit::new("set.editor_removed", "set", &ids::to_text(set_id)).change(
                Some(json!({ "account_id": ids::to_text(account_id) })),
                None,
            ),
        )
        .await?;
    }
    tx.commit().await.map_err(sqlx_error)?;
    Ok(removed)
}

/// The channel's owner, or the owner's account editors (audited): picking the active set needs
/// `sets` or `channels`, the other settings need `channels` (ADR 0014, amendment 2026/11/22). The
/// active set must be one of the owner's channel sets, and one an editor's grant covers; an empty
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
    let by_editor = ch.account_id != caller.account_id;
    if by_editor {
        let channels = has_flag(
            &mut tx,
            ch.account_id,
            caller.account_id,
            EditorPermission::Channels,
        )
        .await?;
        let sets = has_flag(
            &mut tx,
            ch.account_id,
            caller.account_id,
            EditorPermission::Sets,
        )
        .await?;
        // Picking the active set is managing sets; `channels` keeps it too, as it always had it.
        if input.active_set_id.is_some() && !(sets || channels) {
            return Err(forbidden(
                "Only the channel's owner and its account editors with the sets permission can pick its active set.",
            ));
        }
        if input.allow_personal_emotes.is_some() && !channels {
            return Err(forbidden(
                "Only the channel's owner and its account editors with the channel settings permission can change its settings.",
            ));
        }
        if !(sets || channels) {
            return Err(forbidden(
                "Only the channel's owner and its account editors can change it.",
            ));
        }
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
                    "The active set must be one of the channel owner's channel sets.",
                ));
            }
            if by_editor
                && !cyex_tv_db::editors::covers_set(
                    &mut tx,
                    ch.account_id,
                    caller.account_id,
                    set.id,
                )
                .await
                .map_err(db_error)?
            {
                return Err(forbidden(
                    "You can only show sets you work on. Ask the owner to add this one to your sets.",
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
    // Editors are always audited; the owner's own changes are too, for their activity feed.
    let changed = updated.active_set_id != ch.active_set_id
        || updated.allow_personal_emotes != ch.allow_personal_emotes;
    if by_editor || changed {
        let after = json!({
            "active_set_id": updated.active_set_id.map(ids::to_text),
            "allow_personal_emotes": updated.allow_personal_emotes,
        });
        record_audit(
            &mut tx,
            caller.account_id,
            Audit::new("channel.settings_changed", "channel", &ids::to_text(ch.id))
                .change(
                    Some(json!({
                        "active_set_id": ch.active_set_id.map(ids::to_text),
                        "allow_personal_emotes": ch.allow_personal_emotes,
                    })),
                    Some(if by_editor {
                        via_account_editor(Some(after))
                    } else {
                        after
                    }),
                )
                .subject(Some(ch.account_id)),
        )
        .await?;
    }
    tx.commit().await.map_err(sqlx_error)?;
    views::channel(&mut conn, &updated)
        .await
        .map_err(db_error)?
        .ok_or_else(not_found)
}

// The sets you can change for a channel (for apps such as cyex deck, ADR 0021)

/// Sets one `channelEditableSets` answer looks at, at most.
pub const CHANNEL_EDITABLE_SETS_MAX: usize = 200;

/// The channel, found by its platform ID or its login (exactly one), with the sets of its account
/// the caller may add, rename and remove emotes in: the same check `addSetEmote` makes, so an
/// app can tell before it sends. `None` when the channel is unknown or its account is hidden.
pub async fn channel_editable_sets(
    state: &AppState,
    caller: &Caller,
    platform: Platform,
    id: Option<&str>,
    login: Option<&str>,
) -> ApiResult<Option<cyex_tv_shared::model::ChannelEditableSets>> {
    use cyex_tv_shared::model::{ChannelEditableSets, ChannelRole, EditableSet, SetAccess};

    let id = id.map(str::trim).filter(|s| !s.is_empty());
    let login = login
        .map(|l| l.trim().trim_start_matches('@'))
        .filter(|s| !s.is_empty());
    let mut conn = conn(state).await?;
    let found = match (id, login) {
        (Some(id), None) if id.len() <= 64 => channels::find(&mut conn, platform, id).await,
        (None, Some(login)) if login.len() <= 64 => {
            channels::find_by_login(&mut conn, platform, login).await
        }
        _ => {
            return Err(bad(
                "bad_channel",
                "Give the channel's id or its login (one of them, up to 64 characters).",
            ));
        }
    }
    .map_err(db_error)?;
    let Some(ch) = found else {
        return Ok(None);
    };
    let Some(channel) = views::channel(&mut conn, &ch).await.map_err(db_error)? else {
        return Ok(None);
    };

    let mut rows = sets::list_for_owner(&mut conn, ch.account_id)
        .await
        .map_err(db_error)?;
    rows.retain(|s| s.kind != SetKind::Global);
    rows.truncate(CHANNEL_EDITABLE_SETS_MAX);
    let mut editable = Vec::new();
    let mut access = Vec::new();
    for set in rows {
        match edit_grant(&mut conn, caller, &set, EditorPermission::Emotes).await {
            Ok(grant) => {
                access.push(match grant {
                    Grant::Owner => SetAccess::Owner,
                    Grant::AccountEditor(_) => SetAccess::AccountEditor,
                    Grant::Editor => SetAccess::SetEditor,
                    Grant::Permission(_) => SetAccess::Staff,
                });
                editable.push(set);
            }
            Err(err)
                if matches!(
                    err.status,
                    axum::http::StatusCode::FORBIDDEN | axum::http::StatusCode::NOT_FOUND
                ) => {}
            Err(err) => return Err(err),
        }
    }

    let role = if ch.account_id == caller.account_id {
        Some(ChannelRole::Owner)
    } else if cyex_tv_db::editors::get(&mut conn, ch.account_id, caller.account_id)
        .await
        .map_err(db_error)?
        .is_some()
    {
        Some(ChannelRole::AccountEditor)
    } else if access.contains(&SetAccess::SetEditor) {
        Some(ChannelRole::SetEditor)
    } else {
        None
    };

    let viewer = viewer(&mut conn, Some(caller)).await?;
    let summaries = views::set_summaries(&mut conn, &editable, &viewer, &state.cdn_url, 0)
        .await
        .map_err(db_error)?;
    let active = channel.active_set_id.clone();
    let sets: Vec<EditableSet> = summaries
        .into_iter()
        .zip(access)
        .map(|(set, access)| EditableSet {
            active_here: active.as_deref() == Some(set.id.as_str()),
            set,
            access,
        })
        .collect();
    Ok(Some(ChannelEditableSets {
        can_edit_active_set: sets.iter().any(|s| s.active_here),
        role,
        sets,
        channel,
    }))
}

#[cfg(test)]
mod tests {
    use axum::http::StatusCode;

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
    fn a_missing_permission_is_a_403_naming_what_is_needed() {
        let one = forbidden(&needs(&[permissions::EMOTES_REVIEW]));
        assert_eq!(one.status, StatusCode::FORBIDDEN);
        assert_eq!(one.code, "forbidden");
        assert_eq!(one.message, "This needs emotes.review.");
        let two = forbidden(&needs(&[
            permissions::EMOTES_REVIEW,
            permissions::REVIEW_SETTINGS,
        ]));
        assert_eq!(two.message, "This needs emotes.review or review.settings.");
        let three = needs(&[
            permissions::EMOTES_REVIEW,
            permissions::EMOTES_TAKEDOWN,
            permissions::REPORTS_MANAGE,
        ]);
        assert_eq!(
            three,
            "This needs emotes.review, emotes.takedown or reports.manage."
        );
        let held = vec!["emotes.review".to_owned()];
        assert!(has(&held, "emotes.review"));
        assert!(!has(&held, "emotes.takedown"));
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
