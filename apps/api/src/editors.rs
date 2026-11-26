//! Account editors (ADR 0014): who may act on an account's sets and channels, and the rules for
//! changing who that is.
//!
//! - The owner of an account holds every flag and is never an editor of it.
//! - An editor with `editors` manages the account's editors, but only grants flags they hold and
//!   only changes or removes editors whose flags are all ones they hold.
//! - An editor can always remove themselves.
//! - A grant covers every set of the owner, new ones included, or only a list of them. A manager
//!   limited to some sets only grants within their list and only changes editors inside it.
//! - `sets` and `channels` are separate (amendment 2026/11/22): `sets` makes, renames, deletes and
//!   resizes sets and picks a channel's active set; `channels` changes channel settings (and may
//!   still pick the active set, as it always could). Grants are listed as stored.
//! - Grants of blocked or deleted accounts, and grants on them, count for nothing.
//! - The emote flags (`emote_upload`, `emote_manage`) need a grant on every set
//!   (`has_emote_flag`): an account's emotes aren't in any one set. `emote_transfer` is retired
//!   (amendment 2026/11/22): never granted, and a stored one grants nothing.
//!
//! Every change writes `audit_log` with before and after, in the change's transaction.

use std::collections::{BTreeSet, HashMap};

use axum::http::StatusCode;
use cyex_tv_auth::Caller;
use cyex_tv_db::editors::{self, Grant};
use cyex_tv_db::{accounts, channels, ids, reads, sets};
use cyex_tv_shared::error::{ApiError, ApiResult};
use cyex_tv_shared::model::{
    AccountEditor, AccountSummary, AddAccountEditor, Channel, ChannelRole, EditableChannel,
    EditedAccount, EditorPermission, EmoteSet,
};
use cyex_tv_shared::permissions;
use serde_json::json;
use sqlx::{Connection, PgConnection};
use uuid::Uuid;

use crate::catalog::{db_error, forbidden, holds, not_found, parse_id, viewer};
use crate::errors::{bad, conflict, limited_as, sqlx_error};
use crate::limits::AccountLimit;
use crate::state::AppState;
use crate::views;

pub type Flags = BTreeSet<EditorPermission>;

/// Stored names to the flags they grant. A name this version doesn't know grants nothing.
fn flags_of(names: &[String]) -> Flags {
    names
        .iter()
        .filter_map(|n| EditorPermission::parse(n))
        .collect()
}

/// Stored names as listed by the API: as stored, unknown names left out.
fn listed(names: &[String]) -> Vec<EditorPermission> {
    let flags: Flags = names
        .iter()
        .filter_map(|n| EditorPermission::parse(n))
        .collect();
    flags.into_iter().collect()
}

/// Which of the owner's sets a grant covers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Scope {
    /// Every set, new ones included.
    All,
    Only(BTreeSet<Uuid>),
}

impl Scope {
    fn of(all_sets: bool, sets: &[Uuid]) -> Self {
        if all_sets {
            Self::All
        } else {
            Self::Only(sets.iter().copied().collect())
        }
    }

    /// Everything `self` covers, `other` covers too.
    fn within(&self, other: &Scope) -> bool {
        match (self, other) {
            (_, Scope::All) => true,
            (Scope::All, Scope::Only(_)) => false,
            (Scope::Only(mine), Scope::Only(theirs)) => mine.is_subset(theirs),
        }
    }

    pub fn is_all(&self) -> bool {
        matches!(self, Scope::All)
    }

    fn ids(&self) -> Vec<String> {
        match self {
            Scope::All => Vec::new(),
            Scope::Only(ids) => ids.iter().map(|id| ids::to_text(*id)).collect(),
        }
    }
}

fn names_of(flags: &Flags) -> Vec<String> {
    flags.iter().map(|f| f.as_str().to_owned()).collect()
}

fn all_flags() -> Flags {
    EditorPermission::ALL.into_iter().collect()
}

/// The flags `editor` holds on `owner`: every flag for the owner themselves, `None` for anyone
/// who isn't a live editor of a live account.
pub async fn flags_on(
    conn: &mut PgConnection,
    owner: Uuid,
    editor: Uuid,
) -> ApiResult<Option<Flags>> {
    if owner == editor {
        return Ok(Some(all_flags()));
    }
    Ok(editors::permissions_of(conn, owner, editor)
        .await
        .map_err(db_error)?
        .map(|names| flags_of(&names)))
}

/// The flags and set scope `editor` holds on `owner`: everything for the owner, `None` for
/// anyone who isn't a live editor of a live account.
async fn access_on(
    conn: &mut PgConnection,
    owner: Uuid,
    editor: Uuid,
) -> ApiResult<Option<(Flags, Scope)>> {
    if owner == editor {
        return Ok(Some((all_flags(), Scope::All)));
    }
    let Some(names) = editors::permissions_of(conn, owner, editor)
        .await
        .map_err(db_error)?
    else {
        return Ok(None);
    };
    let scope = scope_of(conn, owner, editor).await?;
    Ok(Some((flags_of(&names), scope)))
}

async fn scope_of(conn: &mut PgConnection, owner: Uuid, editor: Uuid) -> ApiResult<Scope> {
    let all = editors::get(conn, owner, editor)
        .await
        .map_err(db_error)?
        .is_none_or(|g| g.all_sets);
    if all {
        return Ok(Scope::All);
    }
    let sets = editors::scoped_sets(conn, owner, editor)
        .await
        .map_err(db_error)?;
    Ok(Scope::Only(sets.into_iter().collect()))
}

/// True if `editor` is an account editor of `owner` holding `flag` whose grant covers every
/// set: what making new sets needs. False for the owner, as `has_flag`.
pub async fn has_flag_on_all_sets(
    conn: &mut PgConnection,
    owner: Uuid,
    editor: Uuid,
    flag: EditorPermission,
) -> ApiResult<bool> {
    if !has_flag(conn, owner, editor, flag).await? {
        return Ok(false);
    }
    Ok(scope_of(conn, owner, editor).await?.is_all())
}

/// True if `editor` may use an emote flag (`emote_upload`, `emote_manage`) on
/// `owner`'s emotes: an account editor holding it on a grant that covers every set. Emotes belong
/// to the account, not to a set, so a grant limited to some sets is too narrow for them. False for
/// the owner, as `has_flag`.
pub async fn has_emote_flag(
    conn: &mut PgConnection,
    owner: Uuid,
    editor: Uuid,
    flag: EditorPermission,
) -> ApiResult<bool> {
    debug_assert!(matches!(
        flag,
        EditorPermission::EmoteUpload | EditorPermission::EmoteManage
    ));
    has_flag_on_all_sets(conn, owner, editor, flag).await
}

/// True if `editor` is an account editor of `owner` holding `flag` and their grant covers `set`.
/// False for the owner, as `has_flag`.
pub async fn has_flag_on_set(
    conn: &mut PgConnection,
    owner: Uuid,
    editor: Uuid,
    flag: EditorPermission,
    set: Uuid,
) -> ApiResult<bool> {
    Ok(has_flag(conn, owner, editor, flag).await?
        && editors::covers_set(conn, owner, editor, set)
            .await
            .map_err(db_error)?)
}

/// True if `editor` is an account editor of `owner` holding `flag`. False for the owner: callers
/// check ownership first, so this answers only the editor part. Says nothing about which sets:
/// set changes use `has_flag_on_set`.
pub async fn has_flag(
    conn: &mut PgConnection,
    owner: Uuid,
    editor: Uuid,
    flag: EditorPermission,
) -> ApiResult<bool> {
    if owner == editor {
        return Ok(false);
    }
    Ok(editors::permissions_of(conn, owner, editor)
        .await
        .map_err(db_error)?
        .is_some_and(|names| flags_of(&names).contains(&flag)))
}

/// Who may manage an account's editors, and with what.
struct Manager {
    flags: Flags,
    scope: Scope,
    /// Through `editors.manage_any` rather than as the owner or through an account grant: a staff
    /// action, marked as such in the audit log.
    staff: bool,
}

/// The flags and sets the caller may manage editors of `owner` with: all of them for the owner
/// and for staff with `editors.manage_any`, theirs for an editor holding `editors`. Staff act only
/// on accounts below their own highest role, and never on the owners' (`CYEX_TV_OWNER_IDS`):
/// otherwise an Admin could make themselves an editor of the owner with every flag.
async fn manager_access(
    state: &AppState,
    conn: &mut PgConnection,
    caller: &Caller,
    owner: Uuid,
) -> ApiResult<Manager> {
    if let Some((flags, scope)) = access_on(conn, owner, caller.account_id).await?
        && flags.contains(&EditorPermission::Editors)
    {
        return Ok(Manager {
            flags,
            scope,
            staff: false,
        });
    }
    if holds(conn, caller, permissions::EDITORS_MANAGE_ANY).await? {
        if accounts::get(conn, owner)
            .await
            .map_err(db_error)?
            .is_none()
        {
            return Err(not_found());
        }
        if !caller.owner
            && state
                .auth
                .is_owner(conn, owner)
                .await
                .map_err(ApiError::from)?
        {
            return Err(ApiError::new(
                StatusCode::FORBIDDEN,
                "owner_protected",
                "Only the owner manages the owner's editors.",
            ));
        }
        let target_highest = cyex_tv_db::roles::highest_position(conn, owner)
            .await
            .map_err(db_error)?;
        crate::admin::actor(conn, caller)
            .await?
            .may_act_on(target_highest)
            .map_err(crate::admin::rule)?;
        return Ok(Manager {
            flags: all_flags(),
            scope: Scope::All,
            staff: true,
        });
    }
    Err(forbidden(
        "Only the account's owner and its editors with the editors permission can manage its editors.",
    ))
}

/// What a grant asks for. The retired `EMOTE_TRANSFER` is dropped (a client may still send it
/// with others); asked for alone, it's refused with a sentence saying why.
fn requested(permissions: &[EditorPermission]) -> ApiResult<Flags> {
    let flags: Flags = permissions
        .iter()
        .copied()
        .filter(|p| *p != EditorPermission::EmoteTransfer)
        .collect();
    if flags.is_empty() && permissions.contains(&EditorPermission::EmoteTransfer) {
        return Err(bad(
            "retired_permission",
            "Editors can't be given the transfer permission any more: only an emote's owner can give it away.",
        ));
    }
    if flags.is_empty() {
        return Err(bad(
            "no_permissions",
            "Pick at least one permission. To take every permission away, remove the editor.",
        ));
    }
    Ok(flags)
}

/// What `flags` grant is all within `manager`.
fn no_escalation(manager: &Flags, flags: &Flags) -> ApiResult<()> {
    if flags.is_subset(manager) {
        Ok(())
    } else {
        Err(forbidden("You can only grant permissions you hold."))
    }
}

fn no_wider_scope(manager: &Scope, scope: &Scope) -> ApiResult<()> {
    if scope.within(manager) {
        Ok(())
    } else {
        Err(forbidden(
            "You can only grant sets you may work on yourself.",
        ))
    }
}

fn outranked(manager: &Flags, current: &Flags) -> ApiResult<()> {
    if current.is_subset(manager) {
        Ok(())
    } else {
        Err(forbidden(
            "That editor holds permissions you don't, so only the owner can change them.",
        ))
    }
}

fn outscoped(manager: &Scope, current: &Scope) -> ApiResult<()> {
    if current.within(manager) {
        Ok(())
    } else {
        Err(forbidden(
            "That editor works on sets you don't, so only the owner can change them.",
        ))
    }
}

/// The sets asked for, as a scope: none means every set. Each must be one of the owner's.
async fn requested_scope(
    conn: &mut PgConnection,
    owner: Uuid,
    set_ids: Option<&[String]>,
) -> ApiResult<Scope> {
    let raw = set_ids.unwrap_or_default();
    if raw.is_empty() {
        return Ok(Scope::All);
    }
    if raw.len() > editors::MAX_SCOPED_SETS {
        return Err(bad(
            "too_many_sets",
            "Pick at most 200 sets, or let them work on every set.",
        ));
    }
    let wanted: BTreeSet<Uuid> = raw
        .iter()
        .map(|r| {
            ids::from_text(r.trim()).ok_or_else(|| bad("bad_set", "Pick sets of this account."))
        })
        .collect::<ApiResult<_>>()?;
    let list: Vec<Uuid> = wanted.iter().copied().collect();
    let owned = sets::owned_ids(conn, owner, &list)
        .await
        .map_err(db_error)?;
    if owned.len() != wanted.len() {
        return Err(bad("bad_set", "Pick sets of this account."));
    }
    Ok(Scope::Only(wanted))
}

fn too_many_editors() -> ApiError {
    conflict(
        "too_many_editors",
        "An account can have at most 50 editors. Remove one to add someone else.",
    )
}

fn rate_limited(state: &AppState, caller: &Caller) -> ApiResult<()> {
    limited_as(
        state,
        AccountLimit::Editors,
        &ids::to_text(caller.account_id),
        caller.account_id,
        "That's a lot of editor changes. Try again in a while.",
    )
}

fn owner_of(caller: &Caller, raw: Option<&str>) -> ApiResult<Uuid> {
    match raw.map(str::trim).filter(|s| !s.is_empty()) {
        Some(raw) => parse_id(raw),
        None => Ok(caller.account_id),
    }
}

fn summary_of(s: &reads::Summary) -> AccountSummary {
    AccountSummary {
        id: ids::to_text(s.account_id),
        display_name: s.display_name.clone(),
        avatar: s.avatar.clone(),
    }
}

async fn editor_views(
    conn: &mut PgConnection,
    owner: Uuid,
    grants: &[Grant],
) -> ApiResult<Vec<AccountEditor>> {
    let mut wanted: Vec<Uuid> = grants.iter().map(|g| g.editor_id).collect();
    wanted.extend(grants.iter().filter_map(|g| g.granted_by));
    let summaries = reads::summaries(conn, &wanted).await.map_err(db_error)?;
    let scoped = editors::scoped_sets_of_account(conn, owner)
        .await
        .map_err(db_error)?;
    Ok(grants
        .iter()
        .filter_map(|g| {
            let scope = Scope::of(
                g.all_sets,
                scoped.get(&g.editor_id).map_or(&[][..], Vec::as_slice),
            );
            Some(AccountEditor {
                account: summary_of(summaries.get(&g.editor_id)?),
                permissions: listed(&g.permissions),
                granted_by: g
                    .granted_by
                    .and_then(|id| summaries.get(&id))
                    .map(summary_of),
                granted_at: views::time(g.granted_at),
                updated_at: views::time(g.updated_at),
                all_sets: scope.is_all(),
                set_ids: scope.ids(),
            })
        })
        .collect())
}

async fn audit(
    conn: &mut PgConnection,
    caller: &Caller,
    action: &str,
    owner: Uuid,
    before: Option<(&Grant, &Scope)>,
    after: Option<(&Grant, &Scope)>,
    staff: bool,
) -> ApiResult<()> {
    let shape = |(g, scope): (&Grant, &Scope)| {
        json!({
            "editor_id": ids::to_text(g.editor_id),
            "permissions": g.permissions,
            "all_sets": scope.is_all(),
            "set_ids": scope.ids(),
        })
    };
    // Adding and removing editors shows on the owner's public activity feed (names only, never
    // the permissions; ADR 0016), so the row names its subject. A change someone else made is
    // marked: through `editors.manage_any` (a staff action, which the feed leaves out), or as an
    // account editor (one with `EDITORS`, or an editor leaving), like other editor changes.
    let mut after = after.map(shape);
    if staff {
        let mark = after.get_or_insert_with(|| json!({}));
        mark["via"] = json!(permissions::EDITORS_MANAGE_ANY);
    } else if caller.account_id != owner {
        let mark = after.get_or_insert_with(|| json!({}));
        mark["via"] = json!("account_editor");
    }
    cyex_tv_db::audit::write_for(
        conn,
        &cyex_tv_db::audit::NewEntry {
            actor: Some(caller.account_id),
            action,
            target_kind: "account",
            target_id: &ids::to_text(owner),
            before: before.map(shape),
            after,
        },
        Some(owner),
    )
    .await
    .map_err(db_error)?;
    Ok(())
}

// Reads

/// An account's editors: yours, those of an account you edit with `editors`, or anyone's with
/// `editors.manage_any`.
pub async fn account_editors(
    state: &AppState,
    caller: &Caller,
    owner_id: Option<&str>,
) -> ApiResult<Vec<AccountEditor>> {
    let owner = owner_of(caller, owner_id)?;
    let mut conn = state.db.acquire().await.map_err(sqlx_error)?;
    manager_access(state, &mut conn, caller, owner).await?;
    let grants = editors::list_for_account(&mut conn, owner)
        .await
        .map_err(db_error)?;
    editor_views(&mut conn, owner, &grants).await
}

/// Who edits an account, for anyone: live editors, names only, never their permissions or sets.
pub async fn public_editors(
    conn: &mut PgConnection,
    owner: Uuid,
) -> ApiResult<Vec<AccountSummary>> {
    let ids = editors::live_editors_of(conn, owner)
        .await
        .map_err(db_error)?;
    let summaries = reads::summaries(conn, &ids).await.map_err(db_error)?;
    Ok(ids
        .iter()
        .filter_map(|id| summaries.get(id))
        .map(summary_of)
        .collect())
}

/// The accounts the caller edits, with their flags there.
pub async fn editor_of(state: &AppState, caller: &Caller) -> ApiResult<Vec<EditedAccount>> {
    let mut conn = state.db.acquire().await.map_err(sqlx_error)?;
    let grants = editors::list_for_editor(&mut conn, caller.account_id)
        .await
        .map_err(db_error)?;
    let owners: Vec<Uuid> = grants.iter().map(|g| g.account_id).collect();
    let summaries = reads::summaries(&mut conn, &owners)
        .await
        .map_err(db_error)?;
    let scoped = editors::scoped_sets_of_editor(&mut conn, caller.account_id)
        .await
        .map_err(db_error)?;
    Ok(grants
        .iter()
        .filter_map(|g| {
            let scope = Scope::of(
                g.all_sets,
                scoped.get(&g.account_id).map_or(&[][..], Vec::as_slice),
            );
            Some(EditedAccount {
                account: summary_of(summaries.get(&g.account_id)?),
                permissions: listed(&g.permissions),
                granted_at: views::time(g.granted_at),
                all_sets: scope.is_all(),
                set_ids: scope.ids(),
            })
        })
        .collect())
}

/// Whether a grant lets its editor work on the owner's sets: `emotes`, `sets` or `channels` (which
/// picks a channel's active set). A grant of only `profile` or `editors`, say, lists no sets.
fn works_on_sets(grant: &Grant) -> bool {
    let flags = flags_of(&grant.permissions);
    [
        EditorPermission::Emotes,
        EditorPermission::Sets,
        EditorPermission::Channels,
    ]
    .iter()
    .any(|f| flags.contains(f))
}

/// The sets of every account the caller edits, by account then age: the ones each grant covers,
/// for grants that work on sets (`works_on_sets`).
pub async fn editor_sets(state: &AppState, caller: &Caller) -> ApiResult<Vec<EmoteSet>> {
    let mut conn = state.db.acquire().await.map_err(sqlx_error)?;
    let grants: Vec<Grant> = editors::list_for_editor(&mut conn, caller.account_id)
        .await
        .map_err(db_error)?
        .into_iter()
        .filter(works_on_sets)
        .collect();
    let scoped = editors::scoped_sets_of_editor(&mut conn, caller.account_id)
        .await
        .map_err(db_error)?;
    let owners: Vec<Uuid> = grants.iter().map(|g| g.account_id).collect();
    let mut by_owner: HashMap<Uuid, Vec<sets::Set>> = HashMap::new();
    for set in sets::list_for_owners(&mut conn, &owners)
        .await
        .map_err(db_error)?
    {
        if let Some(owner) = set.owner_account_id {
            by_owner.entry(owner).or_default().push(set);
        }
    }
    let mut all = Vec::new();
    for g in &grants {
        let owned = by_owner.remove(&g.account_id).unwrap_or_default();
        if g.all_sets {
            all.extend(owned);
        } else {
            let only = scoped.get(&g.account_id).map_or(&[][..], Vec::as_slice);
            all.extend(owned.into_iter().filter(|s| only.contains(&s.id)));
        }
    }
    let viewer = viewer(&mut conn, Some(caller)).await?;
    views::sets(&mut conn, &all, &viewer, &state.cdn_url)
        .await
        .map_err(db_error)
}

/// The channels of every account the caller edits.
pub async fn editor_channels(state: &AppState, caller: &Caller) -> ApiResult<Vec<Channel>> {
    let mut conn = state.db.acquire().await.map_err(sqlx_error)?;
    let grants = editors::list_for_editor(&mut conn, caller.account_id)
        .await
        .map_err(db_error)?;
    let accounts: Vec<Uuid> = grants.iter().map(|g| g.account_id).collect();
    let mut by_account: HashMap<Uuid, Vec<channels::Channel>> = HashMap::new();
    for ch in channels::list_for_accounts(&mut conn, &accounts)
        .await
        .map_err(db_error)?
    {
        by_account.entry(ch.account_id).or_default().push(ch);
    }
    // In grant order, as before, with one query for every channel's account.
    let ordered: Vec<channels::Channel> = grants
        .iter()
        .flat_map(|g| by_account.remove(&g.account_id).unwrap_or_default())
        .collect();
    views::channels_of(&mut conn, &ordered)
        .await
        .map_err(db_error)
}

/// The channels the caller can manage, each once under their highest role, ordered by role, then
/// login (ignoring case), then platform.
pub async fn my_editable_channels(
    state: &AppState,
    caller: &Caller,
) -> ApiResult<Vec<EditableChannel>> {
    let mut conn = state.db.acquire().await.map_err(sqlx_error)?;
    // Accounts in the order of the role they give: the caller, account editors, set editors.
    let mut roles: Vec<(Uuid, ChannelRole)> = vec![(caller.account_id, ChannelRole::Owner)];
    for g in editors::list_for_editor(&mut conn, caller.account_id)
        .await
        .map_err(db_error)?
    {
        roles.push((g.account_id, ChannelRole::AccountEditor));
    }
    for set in sets::list_editable(&mut conn, caller.account_id)
        .await
        .map_err(db_error)?
    {
        if let Some(owner) = set.owner_account_id {
            roles.push((owner, ChannelRole::SetEditor));
        }
    }
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    for (account, role) in roles {
        if !seen.insert(account) {
            continue;
        }
        // Set editors of an account that is blocked or deleted manage nothing.
        if role == ChannelRole::SetEditor
            && !accounts::get(&mut conn, account)
                .await
                .map_err(db_error)?
                .is_some_and(|a| a.deleted_at.is_none() && a.blocked_at.is_none())
        {
            continue;
        }
        for ch in channels::list_for_account(&mut conn, account)
            .await
            .map_err(db_error)?
        {
            if let Some(channel) = views::channel(&mut conn, &ch).await.map_err(db_error)? {
                out.push(EditableChannel { role, channel });
            }
        }
    }
    out.sort_by_cached_key(|e| {
        (
            e.role,
            e.channel.login.to_lowercase(),
            e.channel.platform.to_string(),
        )
    });
    Ok(out)
}

// Writes

/// The account to add: by ID, or by a Twitch or Kick login. Unknown, deleted and blocked accounts
/// are `not_found`.
async fn target(conn: &mut PgConnection, input: &AddAccountEditor) -> ApiResult<Uuid> {
    let id = match (
        input.account_id.as_deref().map(str::trim),
        input.platform,
        input.login.as_deref().map(str::trim),
    ) {
        (Some(raw), _, _) if !raw.is_empty() => parse_id(raw)?,
        (_, Some(platform), Some(login)) if !login.is_empty() && login.len() <= 64 => {
            accounts::find_by_login(conn, platform, login.trim_start_matches('@'))
                .await
                .map_err(db_error)?
                .ok_or_else(not_found)?
                .account_id
        }
        _ => {
            return Err(bad(
                "bad_editor",
                "Give the editor's account ID, or a platform and a login.",
            ));
        }
    };
    let live = accounts::get(conn, id)
        .await
        .map_err(db_error)?
        .is_some_and(|a| a.deleted_at.is_none() && a.blocked_at.is_none());
    if live { Ok(id) } else { Err(not_found()) }
}

async fn view_of(conn: &mut PgConnection, grant: &Grant) -> ApiResult<AccountEditor> {
    editor_views(conn, grant.account_id, std::slice::from_ref(grant))
        .await?
        .into_iter()
        .next()
        .ok_or_else(not_found)
}

/// `editor.added`, `editor.changed` and `editor.removed`: the editor hears unless they did it
/// themselves, the owner hears unless they did it. The payload names both accounts and who acted,
/// with display names as they read now, and the permissions after the change.
async fn notify_grant(
    conn: &mut PgConnection,
    caller: &Caller,
    kind: &str,
    owner: Uuid,
    editor: Uuid,
    permissions: &[String],
) -> ApiResult<()> {
    let names = reads::summaries(conn, &[owner, editor, caller.account_id])
        .await
        .map_err(db_error)?;
    let name = |id: Uuid| names.get(&id).map(|s| s.display_name.clone());
    let payload = |role: &str| {
        json!({
            "role": role,
            "owner_id": ids::to_text(owner),
            "owner_name": name(owner),
            "editor_id": ids::to_text(editor),
            "editor_name": name(editor),
            "actor_id": ids::to_text(caller.account_id),
            "actor_name": name(caller.account_id),
            "permissions": permissions,
        })
    };
    if caller.account_id != editor {
        crate::account::notify(conn, editor, kind, &payload("editor")).await?;
    }
    if caller.account_id != owner {
        crate::account::notify(conn, owner, kind, &payload("owner")).await?;
    }
    Ok(())
}

pub async fn add(
    state: &AppState,
    caller: &Caller,
    input: &AddAccountEditor,
) -> ApiResult<AccountEditor> {
    let owner = owner_of(caller, input.owner_id.as_deref())?;
    let flags = requested(&input.permissions)?;
    rate_limited(state, caller)?;
    let mut conn = state.db.acquire().await.map_err(sqlx_error)?;
    let mut tx = conn.begin().await.map_err(sqlx_error)?;
    editors::lock_owner(&mut tx, owner)
        .await
        .map_err(db_error)?;
    let m = manager_access(state, &mut tx, caller, owner).await?;
    no_escalation(&m.flags, &flags)?;
    let scope = requested_scope(&mut tx, owner, input.set_ids.as_deref()).await?;
    no_wider_scope(&m.scope, &scope)?;
    let editor = target(&mut tx, input).await?;
    if editor == owner {
        return Err(bad(
            "already_owner",
            "The account's owner can do all of this already.",
        ));
    }
    let grant = editors::add(
        &mut tx,
        owner,
        editor,
        &names_of(&flags),
        Some(caller.account_id),
    )
    .await
    .map_err(|err| match err {
        cyex_tv_db::DbError::TooManyEditors => too_many_editors(),
        cyex_tv_db::DbError::AlreadyEditor => conflict(
            "already_editor",
            "They are an editor already. Change their permissions instead.",
        ),
        err => db_error(err),
    })?;
    let grant = match &scope {
        Scope::All => grant,
        Scope::Only(sets) => {
            let list: Vec<Uuid> = sets.iter().copied().collect();
            editors::set_scope(&mut tx, owner, editor, Some(&list))
                .await
                .map_err(db_error)?
        }
    };
    audit(
        &mut tx,
        caller,
        "account.editor_added",
        owner,
        None,
        Some((&grant, &scope)),
        m.staff,
    )
    .await?;
    notify_grant(
        &mut tx,
        caller,
        "editor.added",
        owner,
        editor,
        &grant.permissions,
    )
    .await?;
    let view = view_of(&mut tx, &grant).await?;
    tx.commit().await.map_err(sqlx_error)?;
    Ok(view)
}

/// Changes an editor's permissions and, when `set_ids` is given, their sets (empty: every set).
pub async fn update(
    state: &AppState,
    caller: &Caller,
    owner_id: Option<&str>,
    account_id: &str,
    permissions: &[EditorPermission],
    set_ids: Option<&[String]>,
) -> ApiResult<AccountEditor> {
    let owner = owner_of(caller, owner_id)?;
    let editor = parse_id(account_id)?;
    let flags = requested(permissions)?;
    rate_limited(state, caller)?;
    let mut conn = state.db.acquire().await.map_err(sqlx_error)?;
    let mut tx = conn.begin().await.map_err(sqlx_error)?;
    editors::lock_owner(&mut tx, owner)
        .await
        .map_err(db_error)?;
    let m = manager_access(state, &mut tx, caller, owner).await?;
    let before = editors::get(&mut tx, owner, editor)
        .await
        .map_err(db_error)?
        .ok_or_else(not_found)?;
    let before_scope = scope_of(&mut tx, owner, editor).await?;
    outranked(&m.flags, &flags_of(&before.permissions))?;
    outscoped(&m.scope, &before_scope)?;
    no_escalation(&m.flags, &flags)?;
    let scope = match set_ids {
        Some(ids) => requested_scope(&mut tx, owner, Some(ids)).await?,
        None => before_scope.clone(),
    };
    no_wider_scope(&m.scope, &scope)?;
    let mut after = editors::update(&mut tx, owner, editor, &names_of(&flags))
        .await
        .map_err(db_error)?;
    if scope != before_scope {
        let list: Option<Vec<Uuid>> = match &scope {
            Scope::All => None,
            Scope::Only(sets) => Some(sets.iter().copied().collect()),
        };
        after = editors::set_scope(&mut tx, owner, editor, list.as_deref())
            .await
            .map_err(db_error)?;
    }
    audit(
        &mut tx,
        caller,
        "account.editor_updated",
        owner,
        Some((&before, &before_scope)),
        Some((&after, &scope)),
        m.staff,
    )
    .await?;
    if after.permissions != before.permissions || scope != before_scope {
        notify_grant(
            &mut tx,
            caller,
            "editor.changed",
            owner,
            editor,
            &after.permissions,
        )
        .await?;
    }
    let view = view_of(&mut tx, &after).await?;
    tx.commit().await.map_err(sqlx_error)?;
    Ok(view)
}

/// The owner or an editor with `editors` removes an editor, or an editor leaves. False when they
/// weren't one.
pub async fn remove(
    state: &AppState,
    caller: &Caller,
    owner_id: Option<&str>,
    account_id: &str,
) -> ApiResult<bool> {
    let owner = owner_of(caller, owner_id)?;
    let editor = parse_id(account_id)?;
    rate_limited(state, caller)?;
    let mut conn = state.db.acquire().await.map_err(sqlx_error)?;
    let mut tx = conn.begin().await.map_err(sqlx_error)?;
    editors::lock_owner(&mut tx, owner)
        .await
        .map_err(db_error)?;
    let current_scope = scope_of(&mut tx, owner, editor).await?;
    let mut staff = false;
    if editor != caller.account_id {
        let m = manager_access(state, &mut tx, caller, owner).await?;
        staff = m.staff;
        if let Some(current) = editors::get(&mut tx, owner, editor)
            .await
            .map_err(db_error)?
        {
            outranked(&m.flags, &flags_of(&current.permissions))?;
            outscoped(&m.scope, &current_scope)?;
        }
    }
    let removed = editors::remove(&mut tx, owner, editor)
        .await
        .map_err(db_error)?;
    if let Some(grant) = &removed {
        audit(
            &mut tx,
            caller,
            "account.editor_removed",
            owner,
            Some((grant, &current_scope)),
            None,
            staff,
        )
        .await?;
        notify_grant(&mut tx, caller, "editor.removed", owner, editor, &[]).await?;
    }
    tx.commit().await.map_err(sqlx_error)?;
    Ok(removed.is_some())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set(flags: &[EditorPermission]) -> Flags {
        flags.iter().copied().collect()
    }

    #[test]
    fn no_flag_beyond_your_own() {
        use EditorPermission::*;
        let mine = set(&[Emotes, Editors]);
        assert!(no_escalation(&mine, &set(&[Emotes])).is_ok());
        assert!(no_escalation(&mine, &set(&[Emotes, Editors])).is_ok());
        assert!(no_escalation(&mine, &set(&[Sets])).is_err());
        assert!(outranked(&mine, &set(&[Emotes, Sets])).is_err());
        assert!(outranked(&mine, &set(&[Emotes])).is_ok());
        assert!(no_escalation(&all_flags(), &set(&[Profile, Sets])).is_ok());
    }

    #[test]
    fn empty_grants_are_refused() {
        assert_eq!(requested(&[]).unwrap_err().code, "no_permissions");
        assert_eq!(
            requested(&[EditorPermission::Sets, EditorPermission::Sets])
                .unwrap()
                .len(),
            1
        );
    }

    #[test]
    fn stored_names_round_trip_and_unknown_ones_grant_nothing() {
        let flags = flags_of(&[
            "emotes".to_owned(),
            "profile".to_owned(),
            "admin".to_owned(),
        ]);
        assert_eq!(
            flags,
            set(&[EditorPermission::Emotes, EditorPermission::Profile])
        );
        assert_eq!(
            names_of(&all_flags()),
            [
                "emotes",
                "sets",
                "editors",
                "profile",
                "channels",
                "emote_upload",
                "emote_manage",
                "billing_payment",
                "billing_subscription"
            ]
        );
    }

    /// `EMOTE_TRANSFER` is retired (amendment 2026/11/22): never granted, a stored one grants
    /// nothing and isn't listed, alone it's refused, with others it's dropped.
    #[test]
    fn transfer_flag_is_retired() {
        use EditorPermission::*;
        assert!(!EditorPermission::ALL.contains(&EmoteTransfer));
        assert_eq!(EditorPermission::parse("emote_transfer"), None);
        assert_eq!(
            requested(&[EmoteTransfer]).unwrap_err().code,
            "retired_permission"
        );
        assert_eq!(
            requested(&[EmoteManage, EmoteTransfer]).unwrap(),
            set(&[EmoteManage])
        );
        let stored = ["emote_manage".to_owned(), "emote_transfer".to_owned()];
        assert_eq!(flags_of(&stored), set(&[EmoteManage]));
        assert_eq!(listed(&stored), [EmoteManage]);
        assert!(!all_flags().contains(&EmoteTransfer));
    }

    #[test]
    fn sets_and_channels_are_separate() {
        use EditorPermission::*;
        assert_eq!(requested(&[Sets]).unwrap(), set(&[Sets]));
        assert_eq!(flags_of(&["sets".to_owned()]), set(&[Sets]));
        assert_eq!(listed(&["sets".to_owned(), "nope".to_owned()]), [Sets]);
        // `sets` alone is enough to grant `sets`, and grants nothing about channels.
        assert!(no_escalation(&set(&[Sets]), &set(&[Sets])).is_ok());
        let manager = flags_of(&["sets".to_owned(), "editors".to_owned()]);
        assert!(no_escalation(&manager, &set(&[Channels])).is_err());
        // And the other way round.
        let channels = flags_of(&["channels".to_owned(), "editors".to_owned()]);
        assert!(no_escalation(&channels, &set(&[Channels])).is_ok());
        assert!(no_escalation(&channels, &set(&[Sets])).is_err());
    }

    #[test]
    fn scopes_never_widen() {
        let a = Uuid::from_u128(1);
        let b = Uuid::from_u128(2);
        let only = |ids: &[Uuid]| Scope::Only(ids.iter().copied().collect());
        assert!(only(&[a]).within(&Scope::All));
        assert!(only(&[a]).within(&only(&[a, b])));
        assert!(!only(&[a, b]).within(&only(&[a])));
        assert!(!Scope::All.within(&only(&[a, b])));
        assert!(Scope::All.within(&Scope::All));
        assert!(no_wider_scope(&only(&[a]), &Scope::All).is_err());
        assert!(outscoped(&only(&[a]), &only(&[b])).is_err());
        assert!(outscoped(&Scope::All, &only(&[b])).is_ok());
        assert_eq!(Scope::of(true, &[a]), Scope::All);
        assert_eq!(Scope::of(false, &[]), only(&[]));
    }
}
