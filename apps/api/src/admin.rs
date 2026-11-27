//! Roles, role assignment and the audit log (ADR 0010). The rules themselves are in
//! `cyex_tv_auth::rbac`; this module loads who is acting, applies the rules, and writes the
//! change with its audit row in one transaction.
//!
//! Every role change takes one advisory lock, so two admins demoting each other at the same time
//! can't both pass the lock-out check.
//!
//! Built-in roles (migration `default_roles`): "User" is everyone's without an assignment, so it
//! is never assigned or taken away and its position stays 0; "Admin" can't be deleted; the top
//! role is the owners' (`CYEX_TV_OWNER_IDS`), listed to them only, never granted, edited or
//! deleted. Nobody edits a role they hold. Giving and taking roles needs `roles.manage` or the
//! narrower `roles.assign`; making and editing them needs `roles.manage`.
//!
//! Each role allows some permissions, denies some and leaves the rest to the roles below
//! (`cyex_tv_shared::permissions::resolve`): a "Muted" role above User that denies
//! `emotes.upload` stops uploads that User allows. Changing what a role says about a permission,
//! either way, needs you to hold it. `account_permissions` shows which role decided each one.
//! A role can be invisible: never shown publicly and never used to rank people.
//!
//! A role can also raise or lower the per-account limits of whoever holds it (`set_limits`,
//! `limits::AccountLimit`), under the same rules as any other edit; the User role's limits are
//! the defaults the api was started with, so they aren't set here.

use std::collections::BTreeSet;

use axum::http::StatusCode;
use cyex_tv_auth::rbac::{Actor, RuleError};
use cyex_tv_auth::{AuthError, Caller};
use cyex_tv_db::roles::{self, NewRole, RolePatch};
use cyex_tv_db::{accounts, audit, ids};
use cyex_tv_shared::error::{ApiError, ApiResult};
use cyex_tv_shared::model::{
    AuditEntry, BuiltInRole, CreateRole, PermissionDecision, PermissionEffect, Role, RoleLimit,
    RoleLimitInput, UpdateRole,
};
use cyex_tv_shared::permissions;
use cyex_tv_shared::text;
use serde_json::{Value, json};
use sqlx::{Connection, PgConnection};
use uuid::Uuid;

use crate::catalog::{db_error, forbidden, not_found, parse_id, permissions_of};
use crate::errors::{Audit, record_audit};
use crate::errors::{bad, sqlx_error};
use crate::limits::{AccountLimit, Rates, RoleOverrides, overrides_from_json, overrides_to_json};
use crate::state::AppState;
use crate::views;

/// Any constant works; it only has to be the same for every role change.
const ROLE_LOCK: i64 = 0x6379_6578_726f_6c65; // "cyexrole"

pub(crate) fn rule(err: RuleError) -> ApiError {
    let status = match err {
        RuleError::LastManager => StatusCode::CONFLICT,
        _ => StatusCode::FORBIDDEN,
    };
    ApiError::new(status, err.code(), err.message())
}

fn auth_error(err: AuthError) -> ApiError {
    err.into()
}

pub(crate) async fn actor(conn: &mut PgConnection, caller: &Caller) -> ApiResult<Actor> {
    let perms = permissions_of(conn, caller).await?;
    let held = roles::roles_of(conn, caller.account_id)
        .await
        .map_err(db_error)?;
    // The owners hold the top role.
    let highest = if caller.owner {
        Some(roles::ROOT_POSITION)
    } else {
        held.iter().map(|r| r.position).max()
    };
    Ok(Actor {
        owner: caller.owner,
        permissions: perms.into_iter().collect(),
        highest,
        held: held.into_iter().map(|r| r.id).collect(),
    })
}

/// The actor, if they hold `permission`.
async fn require(conn: &mut PgConnection, caller: &Caller, permission: &str) -> ApiResult<Actor> {
    let actor = actor(conn, caller).await?;
    if !actor.holds(permission) {
        return Err(forbidden(&format!("This needs {permission}.")));
    }
    Ok(actor)
}

/// The actor, if they may give and take roles: `roles.manage` or `roles.assign`.
async fn assigner(conn: &mut PgConnection, caller: &Caller) -> ApiResult<Actor> {
    let actor = actor(conn, caller).await?;
    if !actor.holds(permissions::ROLES_MANAGE) && !actor.holds(permissions::ROLES_ASSIGN) {
        return Err(forbidden("This needs roles.manage or roles.assign."));
    }
    Ok(actor)
}

fn built_in(role: &roles::Role) -> Option<BuiltInRole> {
    match role.system.as_deref() {
        Some(roles::SYSTEM_USER) => Some(BuiltInRole::User),
        Some(roles::SYSTEM_ADMIN) => Some(BuiltInRole::Admin),
        Some(roles::SYSTEM_ROOT) => Some(BuiltInRole::Root),
        _ => None,
    }
}

/// Custom roles sit between the User role (0) and the top role.
fn position(p: i32) -> ApiResult<i32> {
    if p <= 0 || p >= roles::ROOT_POSITION {
        return Err(bad(
            "bad_position",
            format!(
                "Positions are 1 to {}: above the User role, below the top role.",
                roles::ROOT_POSITION - 1
            ),
        ));
    }
    Ok(p)
}

/// Loads a role the caller may know about: the top role is `not_found` to everyone but its
/// holders (the owners).
async fn load_visible(
    conn: &mut PgConnection,
    caller: &Caller,
    id: Uuid,
) -> ApiResult<roles::Role> {
    let role = load(conn, id).await?;
    if role.is(roles::SYSTEM_ROOT) && !caller.owner {
        return Err(not_found());
    }
    Ok(role)
}

/// The top role is never granted, edited or deleted.
fn not_root(role: &roles::Role) -> ApiResult<()> {
    match role.is(roles::SYSTEM_ROOT) {
        true => Err(rule(RuleError::BuiltIn)),
        false => Ok(()),
    }
}

/// Neither is the User role assigned: everyone holds it.
fn assignable_kind(role: &roles::Role) -> ApiResult<()> {
    match role.is(roles::SYSTEM_ROOT) || role.is(roles::SYSTEM_USER) {
        true => Err(rule(RuleError::BuiltIn)),
        false => Ok(()),
    }
}

/// Accounts other than the owners that hold `roles.manage`.
async fn managers(state: &AppState, conn: &mut PgConnection) -> ApiResult<Vec<Uuid>> {
    let mut out = Vec::new();
    for account in roles::holders_of(conn, permissions::ROLES_MANAGE)
        .await
        .map_err(db_error)?
    {
        if !state
            .auth
            .is_owner(conn, account)
            .await
            .map_err(auth_error)?
        {
            out.push(account);
        }
    }
    Ok(out)
}

fn role_name(raw: &str) -> ApiResult<String> {
    let name = raw.trim();
    let len = name.chars().count();
    if len == 0 || len > 64 || text::has_unsafe(name, false) {
        return Err(bad("bad_name", "Role names are 1 to 64 characters."));
    }
    Ok(name.to_owned())
}

fn description(raw: &str) -> ApiResult<Option<String>> {
    let text = raw.trim();
    if text.chars().count() > 500 || text::has_unsafe(text, true) {
        return Err(bad(
            "bad_description",
            "Descriptions are at most 500 characters.",
        ));
    }
    Ok((!text.is_empty()).then(|| text.to_owned()))
}

/// A role colour: `#rrggbb`, any case, stored lower case. Anything else is `bad_color`.
fn colour(raw: &str) -> ApiResult<String> {
    let c = raw.trim();
    if c.len() == 7 && c.starts_with('#') && c[1..].bytes().all(|b| b.is_ascii_hexdigit()) {
        return Ok(c.to_ascii_lowercase());
    }
    Err(bad(
        "bad_color",
        "A role colour is a hex colour like #3b82f6.",
    ))
}

/// Known, deduplicated and sorted, or `unknown_permission`.
fn permission_list(raw: &[String]) -> ApiResult<Vec<String>> {
    let set: BTreeSet<String> = raw.iter().map(|p| p.trim().to_owned()).collect();
    if let Some(unknown) = set.iter().find(|p| !permissions::is_known(p)) {
        return Err(bad(
            "unknown_permission",
            format!("{unknown:?} is not a permission."),
        ));
    }
    Ok(set.into_iter().collect())
}

/// Allows and denies with no name in both, or `bad_permissions`.
fn disjoint(allows: &[String], denies: &[String]) -> ApiResult<()> {
    match allows.iter().find(|p| denies.contains(p)) {
        Some(both) => Err(bad(
            "bad_permissions",
            format!("{both:?} can't be allowed and denied by the same role."),
        )),
        None => Ok(()),
    }
}

/// What a role allows and denies after an update: a list given replaces its own and takes its
/// names out of the other one.
fn next_effects(
    before: (&[String], &[String]),
    allows: Option<&[String]>,
    denies: Option<&[String]>,
) -> ApiResult<(Vec<String>, Vec<String>)> {
    let (old_allows, old_denies) = before;
    let without = |list: &[String], out: &[String]| -> Vec<String> {
        list.iter().filter(|p| !out.contains(p)).cloned().collect()
    };
    Ok(match (allows, denies) {
        (Some(a), Some(d)) => {
            disjoint(a, d)?;
            (a.to_vec(), d.to_vec())
        }
        (Some(a), None) => (a.to_vec(), without(old_denies, a)),
        (None, Some(d)) => (without(old_allows, d), d.to_vec()),
        (None, None) => (old_allows.to_vec(), old_denies.to_vec()),
    })
}

/// Allow, deny or neither.
fn effect_in(allows: &[String], denies: &[String], p: &str) -> Option<PermissionEffect> {
    if allows.iter().any(|x| x == p) {
        Some(PermissionEffect::Allow)
    } else if denies.iter().any(|x| x == p) {
        Some(PermissionEffect::Deny)
    } else {
        None
    }
}

/// The names whose effect differs between two (allows, denies) pairs: each is a change the actor
/// must hold the permission for.
fn changed(before: (&[String], &[String]), after: (&[String], &[String])) -> Vec<String> {
    let names: BTreeSet<&String> = before
        .0
        .iter()
        .chain(before.1)
        .chain(after.0)
        .chain(after.1)
        .collect();
    names
        .into_iter()
        .filter(|p| effect_in(before.0, before.1, p) != effect_in(after.0, after.1, p))
        .cloned()
        .collect()
}

/// A role as `actor` sees it: with what they may do with it.
async fn view(conn: &mut PgConnection, role: &roles::Role, actor: &Actor) -> ApiResult<Role> {
    // The top role holds every permission in code (see the migration) and denies nothing.
    let (perms, denies) = if role.is(roles::SYSTEM_ROOT) {
        (cyex_tv_auth::rbac::all_permissions(), Vec::new())
    } else {
        (
            roles::permissions(conn, role.id).await.map_err(db_error)?,
            roles::denies(conn, role.id).await.map_err(db_error)?,
        )
    };
    let editable = !role.is(roles::SYSTEM_ROOT)
        && actor.holds(permissions::ROLES_MANAGE)
        && actor.may_edit(role.id, role.position).is_ok();
    let assignable = assignable_kind(role).is_ok()
        && (actor.holds(permissions::ROLES_MANAGE) || actor.holds(permissions::ROLES_ASSIGN))
        && actor.may_manage(role.position).is_ok()
        && actor.may_grant(perms.iter().map(String::as_str)).is_ok();
    Ok(Role {
        id: ids::to_text(role.id),
        name: role.name.clone(),
        description: role.description.clone(),
        position: role.position,
        permissions: perms,
        denies,
        public: role.public,
        invisible: role.invisible,
        color: role.color.clone(),
        built_in: built_in(role),
        editable,
        assignable,
    })
}

fn snapshot(role: &Role) -> Value {
    json!({
        "name": role.name,
        "description": role.description,
        "position": role.position,
        "permissions": role.permissions,
        "denies": role.denies,
        "public": role.public,
        "invisible": role.invisible,
        "color": role.color,
    })
}

async fn lock(conn: &mut PgConnection) -> ApiResult<()> {
    sqlx::query("SELECT pg_advisory_xact_lock($1)")
        .bind(ROLE_LOCK)
        .execute(conn)
        .await
        .map_err(sqlx_error)?;
    Ok(())
}

async fn load(conn: &mut PgConnection, id: Uuid) -> ApiResult<roles::Role> {
    roles::get(conn, id)
        .await
        .map_err(db_error)?
        .ok_or_else(not_found)
}

// Reads

/// Every role, most senior first. Needs `roles.manage` or `roles.assign`. The top role is listed
/// to its holders (the owners) only.
pub async fn list(state: &AppState, caller: &Caller) -> ApiResult<Vec<Role>> {
    let mut conn = state.db.acquire().await.map_err(sqlx_error)?;
    let actor = assigner(&mut conn, caller).await?;
    let mut out = Vec::new();
    for role in roles::list(&mut conn).await.map_err(db_error)? {
        if role.is(roles::SYSTEM_ROOT) && !caller.owner {
            continue;
        }
        out.push(view(&mut conn, &role, &actor).await?);
    }
    Ok(out)
}

/// An account's roles. Needs `roles.manage` or `users.view`, unless it's your own.
pub async fn account_roles(
    state: &AppState,
    caller: &Caller,
    account_id: &str,
) -> ApiResult<Vec<Role>> {
    let account_id = parse_id(account_id)?;
    let mut conn = state.db.acquire().await.map_err(sqlx_error)?;
    let actor = actor(&mut conn, caller).await?;
    if account_id != caller.account_id
        && !actor.holds(permissions::ROLES_MANAGE)
        && !actor.holds(permissions::ROLES_ASSIGN)
        && !actor.holds(permissions::USERS_VIEW)
    {
        return Err(forbidden(
            "This needs roles.manage, roles.assign or users.view.",
        ));
    }
    let mut out = Vec::new();
    // The owners hold the top role; only they see it.
    if caller.owner
        && state
            .auth
            .is_owner(&mut conn, account_id)
            .await
            .map_err(auth_error)?
        && let Some(root) = roles::system(&mut conn, roles::SYSTEM_ROOT)
            .await
            .map_err(db_error)?
    {
        out.push(view(&mut conn, &root, &actor).await?);
    }
    for role in roles::roles_of(&mut conn, account_id)
        .await
        .map_err(db_error)?
    {
        if role.is(roles::SYSTEM_ROOT) && !caller.owner {
            continue;
        }
        out.push(view(&mut conn, &role, &actor).await?);
    }
    Ok(out)
}

/// Every permission an account holds or doesn't, with the role that decided it: its most senior
/// role that allows or denies it, the User role last. Yours, or anyone's with `roles.manage`,
/// `roles.assign` or `users.view`. An owner holds every permission (`owner`); the top role is
/// named only to the owners.
pub async fn account_permissions(
    state: &AppState,
    caller: &Caller,
    account_id: &str,
) -> ApiResult<Vec<PermissionDecision>> {
    let account_id = parse_id(account_id)?;
    let mut conn = state.db.acquire().await.map_err(sqlx_error)?;
    let actor = actor(&mut conn, caller).await?;
    if account_id != caller.account_id
        && !actor.holds(permissions::ROLES_MANAGE)
        && !actor.holds(permissions::ROLES_ASSIGN)
        && !actor.holds(permissions::USERS_VIEW)
    {
        return Err(forbidden(
            "This needs roles.manage, roles.assign or users.view.",
        ));
    }
    accounts::get(&mut conn, account_id)
        .await
        .map_err(db_error)?
        .filter(|a| a.deleted_at.is_none())
        .ok_or_else(not_found)?;
    if state
        .auth
        .is_owner(&mut conn, account_id)
        .await
        .map_err(auth_error)?
    {
        let root = match caller.owner {
            true => roles::system(&mut conn, roles::SYSTEM_ROOT)
                .await
                .map_err(db_error)?,
            false => None,
        };
        return Ok(permissions::ALL
            .iter()
            .map(|p| PermissionDecision {
                permission: (*p).to_owned(),
                granted: true,
                effect: Some(PermissionEffect::Allow),
                role_id: root.as_ref().map(|r| ids::to_text(r.id)),
                role_name: root.as_ref().map(|r| r.name.clone()),
                owner: true,
            })
            .collect());
    }
    let grants = roles::grants_of(&mut conn, account_id)
        .await
        .map_err(db_error)?;
    let decided = roles::resolve_grants(&grants);
    Ok(permissions::ALL
        .iter()
        .map(|p| {
            let d = decided.get(*p);
            let by = d.map(|d| &grants[d.role]);
            PermissionDecision {
                permission: (*p).to_owned(),
                granted: d.is_some_and(|d| d.effect == permissions::Effect::Allow),
                effect: d.map(|d| match d.effect {
                    permissions::Effect::Allow => PermissionEffect::Allow,
                    permissions::Effect::Deny => PermissionEffect::Deny,
                }),
                role_id: by.map(|g| ids::to_text(g.role_id)),
                role_name: by.map(|g| g.role_name.clone()),
                owner: false,
            }
        })
        .collect())
}

/// What `audit_log` narrows by, besides paging.
#[derive(Default)]
pub struct AuditQuery<'a> {
    pub target_kind: Option<&'a str>,
    pub target_id: Option<&'a str>,
    pub actor_id: Option<&'a str>,
    pub action: Option<&'a str>,
    /// RFC 3339.
    pub since: Option<&'a str>,
    pub until: Option<&'a str>,
}

type Instant = Option<chrono::DateTime<chrono::Utc>>;

fn instant(raw: Option<&str>, what: &str) -> ApiResult<Instant> {
    raw.map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| {
            chrono::DateTime::parse_from_rfc3339(s)
                .map(|t| t.with_timezone(&chrono::Utc))
                .map_err(|_| {
                    bad(
                        "bad_time",
                        format!("{what} must be a time like 2026-11-17T00:00:00Z."),
                    )
                })
        })
        .transpose()
}

/// An `AuditQuery` read and checked: IDs parsed, times parsed, empty text dropped.
pub(crate) struct AuditParts<'a> {
    actor: Option<Uuid>,
    target_kind: Option<&'a str>,
    target_id: Option<String>,
    action: Option<&'a str>,
    since: Instant,
    until: Instant,
}

impl AuditParts<'_> {
    pub(crate) fn filter(&self) -> audit::Filter<'_> {
        audit::Filter {
            actor: self.actor,
            target_kind: self.target_kind,
            target_id: self.target_id.as_deref(),
            action: self.action,
            since: self.since,
            until: self.until,
            before_id: None,
        }
    }
}

/// The ID text an audit row names a target by: accounts, emotes and sets by their ULID (a
/// Snowflake given for one is read back to it), anything else as given.
fn target_key(kind: Option<&str>, raw: &str) -> String {
    let raw = raw.trim();
    match kind {
        Some("account" | "emote" | "set") | None => ids::from_text(raw)
            .map(ids::to_text)
            .unwrap_or_else(|| raw.to_owned()),
        _ => raw.to_owned(),
    }
}

pub(crate) fn audit_filter_parts<'a>(q: &AuditQuery<'a>) -> ApiResult<AuditParts<'a>> {
    let actor = q
        .actor_id
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(parse_id)
        .transpose()?;
    let target_kind = q.target_kind.map(str::trim).filter(|s| !s.is_empty());
    let target_id = q
        .target_id
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|t| target_key(target_kind, t));
    Ok(AuditParts {
        actor,
        target_kind,
        target_id,
        action: q.action.map(str::trim).filter(|s| !s.is_empty()),
        since: instant(q.since, "since")?,
        until: instant(q.until, "until")?,
    })
}

/// The audit log, newest first. Needs `audit.view`. `before` is an entry ID to page back from.
pub async fn audit_log(
    state: &AppState,
    caller: &Caller,
    q: &AuditQuery<'_>,
    before: Option<&str>,
    limit: Option<u32>,
) -> ApiResult<Vec<AuditEntry>> {
    let parts = audit_filter_parts(q)?;
    let mut conn = state.db.acquire().await.map_err(sqlx_error)?;
    require(&mut conn, caller, permissions::AUDIT_VIEW).await?;
    let before_id = before.map(parse_id).transpose()?;
    let limit = i64::from(crate::paging::Paged::Audit.params().size(limit));
    let entries = audit::list(
        &mut conn,
        &audit::Filter {
            before_id,
            ..parts.filter()
        },
        limit,
    )
    .await
    .map_err(db_error)?;
    Ok(entries.into_iter().map(audit_view).collect())
}

/// One audit log entry as the API shows it.
pub fn audit_view(e: audit::Entry) -> AuditEntry {
    AuditEntry {
        id: ids::to_text(e.id),
        actor_id: e.actor_account_id.map(ids::to_text),
        action: e.action,
        target_kind: e.target_kind,
        target_id: e.target_id,
        before: e.before_json.map(|v| v.to_string()),
        after: e.after_json.map(|v| v.to_string()),
        created_at: views::time(e.created_at),
    }
}

// Writes

pub async fn create(state: &AppState, caller: &Caller, input: &CreateRole) -> ApiResult<Role> {
    let name = role_name(&input.name)?;
    let description = input
        .description
        .as_deref()
        .map(description)
        .transpose()?
        .flatten();
    let perms = permission_list(&input.permissions)?;
    let denies = permission_list(input.denies.as_deref().unwrap_or_default())?;
    disjoint(&perms, &denies)?;
    let color = input.color.as_deref().map(colour).transpose()?;
    let mut conn = state.db.acquire().await.map_err(sqlx_error)?;
    let mut tx = conn.begin().await.map_err(sqlx_error)?;
    lock(&mut tx).await?;
    let actor = require(&mut tx, caller, permissions::ROLES_MANAGE).await?;
    actor.may_manage(input.position).map_err(rule)?;
    position(input.position)?;
    actor
        .may_grant(perms.iter().chain(&denies).map(String::as_str))
        .map_err(rule)?;

    let perm_refs: Vec<&str> = perms.iter().map(String::as_str).collect();
    let role = roles::create(
        &mut tx,
        &NewRole {
            name: &name,
            description: description.as_deref(),
            position: input.position,
            permissions: &perm_refs,
            created_by: Some(caller.account_id),
        },
    )
    .await
    .map_err(db_error)?;
    if !denies.is_empty() {
        let deny_refs: Vec<&str> = denies.iter().map(String::as_str).collect();
        roles::set_effects(&mut tx, role.id, &perm_refs, &deny_refs)
            .await
            .map_err(db_error)?;
    }
    let role = if input.public == Some(true) {
        roles::set_public(&mut tx, role.id, true)
            .await
            .map_err(db_error)?
    } else {
        role
    };
    let role = if input.invisible == Some(true) {
        roles::set_invisible(&mut tx, role.id, true)
            .await
            .map_err(db_error)?
    } else {
        role
    };
    let role = if color.is_some() {
        roles::update(
            &mut tx,
            role.id,
            &RolePatch {
                color: Some(color.as_deref()),
                ..RolePatch::default()
            },
        )
        .await
        .map_err(db_error)?
    } else {
        role
    };
    let out = view(&mut tx, &role, &actor).await?;
    record_audit(
        &mut tx,
        caller.account_id,
        Audit::new("role.created", "role", &out.id).change(None, Some(snapshot(&out))),
    )
    .await?;
    tx.commit().await.map_err(sqlx_error)?;
    Ok(out)
}

pub async fn update(
    state: &AppState,
    caller: &Caller,
    id: &str,
    input: &UpdateRole,
) -> ApiResult<Role> {
    let id = parse_id(id)?;
    let name = input.name.as_deref().map(role_name).transpose()?;
    let description = input.description.as_deref().map(description).transpose()?;
    let perms = input
        .permissions
        .as_deref()
        .map(permission_list)
        .transpose()?;
    let denies = input.denies.as_deref().map(permission_list).transpose()?;
    // An empty string clears the colour.
    let color = input
        .color
        .as_deref()
        .map(|c| {
            if c.trim().is_empty() {
                Ok(None)
            } else {
                colour(c).map(Some)
            }
        })
        .transpose()?;
    let mut conn = state.db.acquire().await.map_err(sqlx_error)?;
    let mut tx = conn.begin().await.map_err(sqlx_error)?;
    lock(&mut tx).await?;
    let actor = require(&mut tx, caller, permissions::ROLES_MANAGE).await?;
    let role = load_visible(&mut tx, caller, id).await?;
    not_root(&role)?;
    actor.may_edit(role.id, role.position).map_err(rule)?;
    if let Some(p) = input.position {
        actor.may_manage(p).map_err(rule)?;
        if role.is(roles::SYSTEM_USER) && p != role.position {
            return Err(rule(RuleError::BuiltIn));
        }
        if !role.is(roles::SYSTEM_USER) {
            position(p)?;
        }
    }
    let before = view(&mut tx, &role, &actor).await?;
    let effects = next_effects(
        (&before.permissions, &before.denies),
        perms.as_deref(),
        denies.as_deref(),
    )?;
    let effects_changed = perms.is_some() || denies.is_some();
    if effects_changed {
        // Changing what a role says about a permission, either way, needs you to hold it.
        let moved = changed(
            (&before.permissions, &before.denies),
            (&effects.0, &effects.1),
        );
        actor
            .may_grant(moved.iter().map(String::as_str))
            .map_err(rule)?;
        let added = effects.0.iter().filter(|p| !before.permissions.contains(p));
        // Every account holds the User role: a staff permission there is everyone's, so only
        // the owners add one (ADR 0010, amendment 2026/11/20). Taking one away is fine.
        if role.is(roles::SYSTEM_USER)
            && !caller.owner
            && added
                .into_iter()
                .any(|p| !permissions::USER_ROLE_ALLOWED.contains(&p.as_str()))
        {
            return Err(ApiError::new(
                StatusCode::FORBIDDEN,
                "owner_only",
                "Only the owner can give the User role a staff permission: every account holds it.",
            ));
        }
    }
    let managers_before = managers(state, &mut tx).await?;

    let updated = roles::update(
        &mut tx,
        id,
        &RolePatch {
            name: name.as_deref(),
            description: description.as_ref().map(|d| d.as_deref()),
            position: input.position,
            color: color.as_ref().map(Option::as_deref),
        },
    )
    .await
    .map_err(db_error)?;
    if effects_changed {
        let allows: Vec<&str> = effects.0.iter().map(String::as_str).collect();
        let denies: Vec<&str> = effects.1.iter().map(String::as_str).collect();
        roles::set_effects(&mut tx, id, &allows, &denies)
            .await
            .map_err(db_error)?;
    }
    let updated = match input.public {
        Some(public) if public != updated.public => roles::set_public(&mut tx, id, public)
            .await
            .map_err(db_error)?,
        _ => updated,
    };
    let updated = match input.invisible {
        Some(invisible) if invisible != updated.invisible => {
            roles::set_invisible(&mut tx, id, invisible)
                .await
                .map_err(db_error)?
        }
        _ => updated,
    };
    let managers_after = managers(state, &mut tx).await?;
    actor
        .keeps_a_manager(&managers_before, &managers_after)
        .map_err(rule)?;

    let after = view(&mut tx, &updated, &actor).await?;
    record_audit(
        &mut tx,
        caller.account_id,
        Audit::new("role.updated", "role", &after.id)
            .change(Some(snapshot(&before)), Some(snapshot(&after))),
    )
    .await?;
    tx.commit().await.map_err(sqlx_error)?;
    // Permissions are read on every request; what an account may count is cached, so it is read
    // again too.
    state.limits.forget_all();
    Ok(after)
}

pub async fn delete(state: &AppState, caller: &Caller, id: &str) -> ApiResult<bool> {
    let id = parse_id(id)?;
    let mut conn = state.db.acquire().await.map_err(sqlx_error)?;
    let mut tx = conn.begin().await.map_err(sqlx_error)?;
    lock(&mut tx).await?;
    let actor = require(&mut tx, caller, permissions::ROLES_MANAGE).await?;
    let Some(role) = roles::get(&mut tx, id).await.map_err(db_error)? else {
        return Ok(false);
    };
    if role.is(roles::SYSTEM_ROOT) && !caller.owner {
        return Err(not_found());
    }
    if role.system.is_some() {
        return Err(rule(RuleError::BuiltIn));
    }
    actor.may_edit(role.id, role.position).map_err(rule)?;
    let before = view(&mut tx, &role, &actor).await?;
    let members = roles::members(&mut tx, id).await.map_err(db_error)?;
    let managers_before = managers(state, &mut tx).await?;
    roles::delete(&mut tx, id).await.map_err(db_error)?;
    state.limits.forget_all();
    let managers_after = managers(state, &mut tx).await?;
    actor
        .keeps_a_manager(&managers_before, &managers_after)
        .map_err(rule)?;
    let mut snap = snapshot(&before);
    snap["members"] = json!(members.iter().map(|m| ids::to_text(*m)).collect::<Vec<_>>());
    record_audit(
        &mut tx,
        caller.account_id,
        Audit::new("role.deleted", "role", &before.id).change(Some(snap), None),
    )
    .await?;
    tx.commit().await.map_err(sqlx_error)?;
    Ok(true)
}

/// Gives an account a role. False if it had it already. You must hold every permission the role
/// grants, the role must be below your highest, and so must someone else's account. Needs `roles.manage` or `roles.assign`; the
/// User and top roles are never assigned.
pub async fn assign(
    state: &AppState,
    caller: &Caller,
    account_id: &str,
    role_id: &str,
) -> ApiResult<bool> {
    let account_id = parse_id(account_id)?;
    let role_id = parse_id(role_id)?;
    let mut conn = state.db.acquire().await.map_err(sqlx_error)?;
    let mut tx = conn.begin().await.map_err(sqlx_error)?;
    lock(&mut tx).await?;
    let actor = assigner(&mut tx, caller).await?;
    let role = load_visible(&mut tx, caller, role_id).await?;
    assignable_kind(&role)?;
    actor.may_manage(role.position).map_err(rule)?;
    let granted = roles::permissions(&mut tx, role_id)
        .await
        .map_err(db_error)?;
    actor
        .may_grant(granted.iter().map(String::as_str))
        .map_err(rule)?;
    accounts::get(&mut tx, account_id)
        .await
        .map_err(db_error)?
        .filter(|a| a.deleted_at.is_none())
        .ok_or_else(not_found)?;
    // Like taking a role away: someone else's account must be below you, so a junior manager
    // can't hand roles to a senior one.
    if account_id != caller.account_id {
        let target_highest = roles::highest_position(&mut tx, account_id)
            .await
            .map_err(db_error)?;
        actor.may_act_on(target_highest).map_err(rule)?;
    }

    let added = roles::assign(&mut tx, account_id, role_id, Some(caller.account_id))
        .await
        .map_err(db_error)?;
    state.limits.forget(account_id);
    if added {
        record_audit(
            &mut tx,
            caller.account_id,
            Audit::new("role.assigned", "account", &ids::to_text(account_id)).change(
                None,
                Some(json!({ "role_id": ids::to_text(role_id), "role": role.name })),
            ),
        )
        .await?;
    }
    tx.commit().await.map_err(sqlx_error)?;
    Ok(added)
}

/// Takes a role from an account. False if it didn't have it. The owner can't be demoted by
/// others, and the last non-owner `roles.manage` holder stays.
pub async fn unassign(
    state: &AppState,
    caller: &Caller,
    account_id: &str,
    role_id: &str,
) -> ApiResult<bool> {
    let account_id = parse_id(account_id)?;
    let role_id = parse_id(role_id)?;
    let mut conn = state.db.acquire().await.map_err(sqlx_error)?;
    let mut tx = conn.begin().await.map_err(sqlx_error)?;
    lock(&mut tx).await?;
    let actor = assigner(&mut tx, caller).await?;
    let role = load_visible(&mut tx, caller, role_id).await?;
    assignable_kind(&role)?;
    actor.may_manage(role.position).map_err(rule)?;
    let target_is_owner = state
        .auth
        .is_owner(&mut tx, account_id)
        .await
        .map_err(auth_error)?;
    actor.may_demote(target_is_owner).map_err(rule)?;
    if account_id != caller.account_id {
        let target_highest = roles::highest_position(&mut tx, account_id)
            .await
            .map_err(db_error)?;
        actor.may_act_on(target_highest).map_err(rule)?;
    }

    let managers_before = managers(state, &mut tx).await?;
    let removed = roles::unassign(&mut tx, account_id, role_id)
        .await
        .map_err(db_error)?;
    state.limits.forget(account_id);
    let managers_after = managers(state, &mut tx).await?;
    actor
        .keeps_a_manager(&managers_before, &managers_after)
        .map_err(rule)?;
    if removed {
        record_audit(
            &mut tx,
            caller.account_id,
            Audit::new("role.unassigned", "account", &ids::to_text(account_id)).change(
                Some(json!({ "role_id": ids::to_text(role_id), "role": role.name })),
                None,
            ),
        )
        .await?;
    }
    tx.commit().await.map_err(sqlx_error)?;
    Ok(removed)
}

// Limits

/// Every per-account limit with its default, and `overrides`' value where it sets one.
fn limit_view(rates: &Rates, overrides: &RoleOverrides) -> Vec<RoleLimit> {
    let int = |n: u32| i32::try_from(n).unwrap_or(i32::MAX);
    AccountLimit::ALL
        .into_iter()
        .map(|l| {
            let (default, window) = rates.default_of(l);
            RoleLimit {
                key: l.key().to_owned(),
                group: l.group(),
                default: int(default),
                max: int(l.max_value()),
                window_seconds: window.map(|w| i32::try_from(w.as_secs()).unwrap_or(i32::MAX)),
                value: overrides.get(&l).copied().map(int),
            }
        })
        .collect()
}

/// The overrides `setRoleLimits` was given: known keys, each once, 1 to the limit's most
/// (`MAX_LIMIT`, or the ceiling of a media number); a null value keeps the default. Emote limits
/// (`emote_max_bytes`, `emote_max_side`, `emote_max_frames`) aren't per role any more: the rules
/// are fixed for everyone (ADR 0031), so a request that still names one gets `bad_limit`.
pub fn parse_limits(input: &[RoleLimitInput]) -> ApiResult<RoleOverrides> {
    let mut out = RoleOverrides::new();
    let mut seen = BTreeSet::new();
    for item in input {
        let key = item.key.trim();
        if key.starts_with("emote_max_") {
            return Err(bad(
                "bad_limit",
                format!(
                    "{key} can't be set: emote limits are fixed and the same for everyone (at most 1000×1000 px, 8 MB, 1000 frames, 50 fps, 4:1)."
                ),
            ));
        }
        let Some(limit) = AccountLimit::from_key(key) else {
            return Err(bad("unknown_limit", format!("{key:?} is not a limit.")));
        };
        if !seen.insert(limit) {
            return Err(bad("bad_limit", format!("{key} is listed twice.")));
        }
        let Some(value) = item.value else { continue };
        let most = limit.max_value();
        match u32::try_from(value) {
            Ok(n) if (1..=most).contains(&n) => {
                out.insert(limit, n);
            }
            _ => {
                return Err(bad(
                    "bad_limit",
                    format!("{key} is 1 to {most}; leave it empty to keep the default."),
                ));
            }
        }
    }
    Ok(out)
}

/// The defaults every account gets without a role that sets otherwise. Needs `roles.manage`.
pub async fn limit_defaults(state: &AppState, caller: &Caller) -> ApiResult<Vec<RoleLimit>> {
    let mut conn = state.db.acquire().await.map_err(sqlx_error)?;
    require(&mut conn, caller, permissions::ROLES_MANAGE).await?;
    Ok(limit_view(&state.limits.rates, &RoleOverrides::new()))
}

/// A role's limits: each with its default and the role's override, if any. Needs `roles.manage`.
pub async fn role_limits(state: &AppState, caller: &Caller, id: &str) -> ApiResult<Vec<RoleLimit>> {
    let id = parse_id(id)?;
    let mut conn = state.db.acquire().await.map_err(sqlx_error)?;
    require(&mut conn, caller, permissions::ROLES_MANAGE).await?;
    load_visible(&mut conn, caller, id).await?;
    let stored = roles::limits(&mut conn, id).await.map_err(db_error)?;
    Ok(limit_view(
        &state.limits.rates,
        &overrides_from_json(&stored),
    ))
}

/// Replaces a role's limit overrides with `input` (what it leaves out keeps the default). Only
/// on a role you may edit: below your highest, not one you hold, not the top role. The User
/// role's limits are the defaults. Needs `roles.manage`. Audited (`role.limits_changed`).
pub async fn set_limits(
    state: &AppState,
    caller: &Caller,
    id: &str,
    input: &[RoleLimitInput],
) -> ApiResult<Vec<RoleLimit>> {
    let id = parse_id(id)?;
    let wanted = parse_limits(input)?;
    let mut conn = state.db.acquire().await.map_err(sqlx_error)?;
    let mut tx = conn.begin().await.map_err(sqlx_error)?;
    lock(&mut tx).await?;
    let actor = require(&mut tx, caller, permissions::ROLES_MANAGE).await?;
    let role = load_visible(&mut tx, caller, id).await?;
    not_root(&role)?;
    actor.may_edit(role.id, role.position).map_err(rule)?;
    if role.is(roles::SYSTEM_USER) {
        return Err(ApiError::new(
            StatusCode::FORBIDDEN,
            "built_in_role",
            "Everyone holds the User role, so its limits are the defaults the API starts with.",
        ));
    }
    let before = overrides_from_json(&roles::limits(&mut tx, id).await.map_err(db_error)?);
    if before != wanted {
        let after = overrides_to_json(&wanted);
        roles::set_limits(&mut tx, id, &after)
            .await
            .map_err(db_error)?;
        record_audit(
            &mut tx,
            caller.account_id,
            Audit::new("role.limits_changed", "role", &ids::to_text(id)).change(
                Some(json!({ "name": role.name, "limits": overrides_to_json(&before) })),
                Some(json!({ "name": role.name, "limits": after })),
            ),
        )
        .await?;
    }
    tx.commit().await.map_err(sqlx_error)?;
    // Everyone who holds it counts under the new limits from their next request on this
    // instance; others read them again within `limits::ACCOUNT_TTL`.
    state.limits.forget_all();
    Ok(limit_view(&state.limits.rates, &wanted))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(key: &str, value: Option<i32>) -> RoleLimitInput {
        RoleLimitInput {
            key: key.to_owned(),
            value,
        }
    }

    #[test]
    fn role_limits_take_known_keys_in_range() {
        let got = parse_limits(&[
            item("pending_emotes", Some(100)),
            item(" gql_account ", Some(1)),
            item("prefs", None),
        ])
        .unwrap();
        assert_eq!(got.len(), 2);
        assert_eq!(got[&AccountLimit::PendingEmotes], 100);
        assert_eq!(got[&AccountLimit::GqlAccount], 1);
        assert!(parse_limits(&[]).unwrap().is_empty());
        for (bad_input, code) in [
            (vec![item("auth", Some(5))], "unknown_limit"),
            (vec![item("nope", Some(5))], "unknown_limit"),
            (vec![item("redeem", Some(0))], "bad_limit"),
            (vec![item("redeem", Some(-4))], "bad_limit"),
            (vec![item("redeem", Some(1_000_001))], "bad_limit"),
            (
                vec![item("redeem", Some(3)), item("redeem", None)],
                "bad_limit",
            ),
        ] {
            assert_eq!(parse_limits(&bad_input).unwrap_err().code, code);
        }
    }

    #[test]
    fn media_limits_stop_at_their_ceilings_and_emote_limits_are_refused() {
        let got = parse_limits(&[
            item("badge_max_side", Some(4096)),
            item("badge_max_bytes", Some(16 * 1024 * 1024)),
            item("paint_max_bytes", Some(1)),
        ])
        .unwrap();
        assert_eq!(got[&AccountLimit::BadgeMaxSide], 4096);
        for key in [
            ("badge_max_side", 4097),
            ("badge_max_bytes", 16 * 1024 * 1024 + 1),
            ("paint_max_side", 0),
        ] {
            let err = parse_limits(&[item(key.0, Some(key.1))]).unwrap_err();
            assert_eq!(err.code, "bad_limit", "{key:?}");
        }
        // An emote key is refused whatever it says, with the reason, and never unknown.
        for (key, value) in [
            ("emote_max_side", Some(512)),
            ("emote_max_bytes", Some(1)),
            ("emote_max_frames", None),
            (" emote_max_side ", Some(5000)),
        ] {
            let err = parse_limits(&[item(key, value)]).unwrap_err();
            assert_eq!(err.code, "bad_limit", "{key}");
            assert!(err.message.contains("fixed"), "{}", err.message);
        }
        let view = limit_view(&Rates::default(), &RoleOverrides::new());
        assert!(view.iter().all(|l| !l.key.starts_with("emote_max_")));
        let max = |k: &str| view.iter().find(|l| l.key == k).unwrap().max;
        assert_eq!(max("badge_max_side"), 4096);
        assert_eq!(max("gql_account"), 1_000_000);
    }

    #[test]
    fn every_limit_shows_its_default_and_window() {
        let rates = Rates::default();
        let mut o = RoleOverrides::new();
        o.insert(AccountLimit::Checkout, 99);
        let view = limit_view(&rates, &o);
        assert_eq!(view.len(), AccountLimit::ALL.len());
        let get = |k: &str| view.iter().find(|l| l.key == k).unwrap();
        assert_eq!(get("checkout").value, Some(99));
        assert_eq!(get("checkout").default, 30);
        assert_eq!(get("checkout").window_seconds, Some(3600));
        assert_eq!(get("gql_account").window_seconds, Some(60));
        assert_eq!(get("pending_emotes").window_seconds, None);
        assert_eq!(get("pending_emotes").default, 20);
        assert_eq!(get("uploads_per_hour").default, 20);
        assert!(get("prefs").value.is_none());
    }

    #[test]
    fn inputs_are_checked() {
        assert_eq!(role_name(" Mods ").unwrap(), "Mods");
        assert!(role_name("").is_err());
        assert!(role_name(&"a".repeat(65)).is_err());
        assert_eq!(description("  ").unwrap(), None);
        assert!(description(&"a".repeat(501)).is_err());
        assert_eq!(
            permission_list(&[
                " emotes.review".into(),
                "audit.view".into(),
                "emotes.review".into()
            ])
            .unwrap(),
            ["audit.view", "emotes.review"]
        );
        assert_eq!(
            permission_list(&["emotes.delete_all".into()])
                .unwrap_err()
                .code,
            "unknown_permission"
        );
    }

    #[test]
    fn rule_errors_have_codes_and_statuses() {
        assert_eq!(rule(RuleError::LastManager).status, StatusCode::CONFLICT);
        assert_eq!(rule(RuleError::Escalation).status, StatusCode::FORBIDDEN);
        assert_eq!(rule(RuleError::NotBelowYou).code, "not_below_you");
        assert_eq!(rule(RuleError::OwnerProtected).code, "owner_protected");
        assert_eq!(rule(RuleError::OwnRole).status, StatusCode::FORBIDDEN);
        assert_eq!(rule(RuleError::BuiltIn).code, "built_in_role");
    }

    fn names(list: &[&str]) -> Vec<String> {
        list.iter().map(|p| (*p).to_owned()).collect()
    }

    #[test]
    fn a_role_never_allows_and_denies_the_same_name() {
        let a = names(&["emotes.review", "emotes.upload"]);
        let d = names(&["emotes.upload"]);
        assert_eq!(disjoint(&a, &d).unwrap_err().code, "bad_permissions");
        assert!(disjoint(&a, &names(&["audit.view"])).is_ok());
        assert_eq!(
            next_effects((&a, &[]), Some(&a), Some(&d))
                .unwrap_err()
                .code,
            "bad_permissions"
        );
    }

    #[test]
    fn one_list_given_moves_names_out_of_the_other() {
        let allows = names(&["emotes.review", "emotes.upload"]);
        let denies = names(&["audit.view"]);
        // Denying uploads takes it out of the allows, and keeps the rest.
        let (a, d) =
            next_effects((&allows, &denies), None, Some(&names(&["emotes.upload"]))).unwrap();
        assert_eq!(a, names(&["emotes.review"]));
        assert_eq!(d, names(&["emotes.upload"]));
        // Allowing audit.view takes it out of the denies.
        let (a, d) = next_effects((&allows, &denies), Some(&names(&["audit.view"])), None).unwrap();
        assert_eq!(a, names(&["audit.view"]));
        assert!(d.is_empty());
        assert_eq!(
            next_effects((&allows, &denies), None, None).unwrap(),
            (allows.clone(), denies.clone())
        );
    }

    #[test]
    fn every_effect_change_needs_the_permission() {
        let before = (names(&["emotes.review"]), names(&["audit.view"]));
        let after = (names(&["audit.view"]), names(&["emotes.upload"]));
        assert_eq!(
            changed((&before.0, &before.1), (&after.0, &after.1)),
            names(&["audit.view", "emotes.review", "emotes.upload"])
        );
        assert!(changed((&before.0, &before.1), (&before.0, &before.1)).is_empty());
    }

    #[test]
    fn custom_positions_sit_between_user_and_top() {
        assert!(position(0).is_err());
        assert!(position(-1).is_err());
        assert_eq!(position(1).unwrap(), 1);
        assert_eq!(
            position(roles::ROOT_POSITION - 1).unwrap(),
            roles::ROOT_POSITION - 1
        );
        assert_eq!(
            position(roles::ROOT_POSITION).unwrap_err().code,
            "bad_position"
        );
    }
}
