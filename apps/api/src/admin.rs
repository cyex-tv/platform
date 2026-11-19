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

use std::collections::BTreeSet;

use axum::http::StatusCode;
use cyex_tv_auth::rbac::{Actor, RuleError};
use cyex_tv_auth::{AuthError, Caller};
use cyex_tv_db::roles::{self, NewRole, RolePatch};
use cyex_tv_db::{DbError, accounts, audit, ids};
use cyex_tv_shared::error::{ApiError, ApiResult};
use cyex_tv_shared::model::{AuditEntry, BuiltInRole, CreateRole, Role, UpdateRole};
use cyex_tv_shared::permissions;
use cyex_tv_shared::text;
use serde_json::{Value, json};
use sqlx::{Connection, PgConnection};
use uuid::Uuid;

use crate::catalog::{db_error, forbidden, not_found, parse_id, permissions_of};
use crate::state::AppState;
use crate::views;

/// Any constant works; it only has to be the same for every role change.
const ROLE_LOCK: i64 = 0x6379_6578_726f_6c65; // "cyexrole"

/// The most audit entries one page returns.
pub const AUDIT_PAGE_MAX: i64 = 100;

pub(crate) fn rule(err: RuleError) -> ApiError {
    let status = match err {
        RuleError::LastManager => StatusCode::CONFLICT,
        _ => StatusCode::FORBIDDEN,
    };
    ApiError::new(status, err.code(), err.message())
}

fn bad(code: &'static str, message: &str) -> ApiError {
    ApiError::new(StatusCode::BAD_REQUEST, code, message)
}

fn auth_error(err: AuthError) -> ApiError {
    err.into()
}

fn sqlx_error(err: sqlx::Error) -> ApiError {
    db_error(DbError::Sqlx(err))
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
            &format!(
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

/// Known, deduplicated and sorted, or `unknown_permission`.
fn permission_list(raw: &[String]) -> ApiResult<Vec<String>> {
    let set: BTreeSet<String> = raw.iter().map(|p| p.trim().to_owned()).collect();
    if let Some(unknown) = set.iter().find(|p| !permissions::is_known(p)) {
        return Err(bad(
            "unknown_permission",
            &format!("{unknown:?} is not a permission."),
        ));
    }
    Ok(set.into_iter().collect())
}

/// A role as `actor` sees it: with what they may do with it.
async fn view(conn: &mut PgConnection, role: &roles::Role, actor: &Actor) -> ApiResult<Role> {
    // The top role holds every permission in code (see the migration).
    let perms = if role.is(roles::SYSTEM_ROOT) {
        cyex_tv_auth::rbac::all_permissions()
    } else {
        roles::permissions(conn, role.id).await.map_err(db_error)?
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
        public: role.public,
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
        "public": role.public,
    })
}

async fn write_audit(
    conn: &mut PgConnection,
    caller: &Caller,
    action: &str,
    target_kind: &str,
    target_id: &str,
    before: Option<Value>,
    after: Option<Value>,
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
                        &format!("{what} must be a time like 2026-11-17T00:00:00Z."),
                    )
                })
        })
        .transpose()
}

/// The audit log, newest first. Needs `audit.view`. `before` is an entry ID to page back from.
pub async fn audit_log(
    state: &AppState,
    caller: &Caller,
    q: &AuditQuery<'_>,
    before: Option<&str>,
    limit: Option<u32>,
) -> ApiResult<Vec<AuditEntry>> {
    let actor_id = q
        .actor_id
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(parse_id)
        .transpose()?;
    let since = instant(q.since, "since")?;
    let until = instant(q.until, "until")?;
    let (target_kind, target_id) = (q.target_kind, q.target_id);
    let action = q.action.map(str::trim).filter(|s| !s.is_empty());
    let mut conn = state.db.acquire().await.map_err(sqlx_error)?;
    require(&mut conn, caller, permissions::AUDIT_VIEW).await?;
    let before_id = before.map(parse_id).transpose()?;
    let limit = i64::from(limit.unwrap_or(50)).clamp(1, AUDIT_PAGE_MAX);
    let entries = audit::list(
        &mut conn,
        &audit::Filter {
            actor: actor_id,
            target_kind,
            target_id,
            action,
            since,
            until,
            before_id,
        },
        limit,
    )
    .await
    .map_err(db_error)?;
    Ok(entries
        .into_iter()
        .map(|e| AuditEntry {
            id: ids::to_text(e.id),
            actor_id: e.actor_account_id.map(ids::to_text),
            action: e.action,
            target_kind: e.target_kind,
            target_id: e.target_id,
            before: e.before_json.map(|v| v.to_string()),
            after: e.after_json.map(|v| v.to_string()),
            created_at: views::time(e.created_at),
        })
        .collect())
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
    let mut conn = state.db.acquire().await.map_err(sqlx_error)?;
    let mut tx = conn.begin().await.map_err(sqlx_error)?;
    lock(&mut tx).await?;
    let actor = require(&mut tx, caller, permissions::ROLES_MANAGE).await?;
    actor.may_manage(input.position).map_err(rule)?;
    position(input.position)?;
    actor
        .may_grant(perms.iter().map(String::as_str))
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
    let role = if input.public == Some(true) {
        roles::set_public(&mut tx, role.id, true)
            .await
            .map_err(db_error)?
    } else {
        role
    };
    let out = view(&mut tx, &role, &actor).await?;
    write_audit(
        &mut tx,
        caller,
        "role.created",
        "role",
        &out.id,
        None,
        Some(snapshot(&out)),
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
    if let Some(perms) = &perms {
        let added = perms.iter().filter(|p| !before.permissions.contains(p));
        actor.may_grant(added.map(String::as_str)).map_err(rule)?;
    }
    let managers_before = managers(state, &mut tx).await?;

    let updated = roles::update(
        &mut tx,
        id,
        &RolePatch {
            name: name.as_deref(),
            description: description.as_ref().map(|d| d.as_deref()),
            position: input.position,
        },
    )
    .await
    .map_err(db_error)?;
    if let Some(perms) = &perms {
        let refs: Vec<&str> = perms.iter().map(String::as_str).collect();
        roles::set_permissions(&mut tx, id, &refs)
            .await
            .map_err(db_error)?;
    }
    let updated = match input.public {
        Some(public) if public != updated.public => roles::set_public(&mut tx, id, public)
            .await
            .map_err(db_error)?,
        _ => updated,
    };
    let managers_after = managers(state, &mut tx).await?;
    actor
        .keeps_a_manager(&managers_before, &managers_after)
        .map_err(rule)?;

    let after = view(&mut tx, &updated, &actor).await?;
    write_audit(
        &mut tx,
        caller,
        "role.updated",
        "role",
        &after.id,
        Some(snapshot(&before)),
        Some(snapshot(&after)),
    )
    .await?;
    tx.commit().await.map_err(sqlx_error)?;
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
    let managers_after = managers(state, &mut tx).await?;
    actor
        .keeps_a_manager(&managers_before, &managers_after)
        .map_err(rule)?;
    let mut snap = snapshot(&before);
    snap["members"] = json!(members.iter().map(|m| ids::to_text(*m)).collect::<Vec<_>>());
    write_audit(
        &mut tx,
        caller,
        "role.deleted",
        "role",
        &before.id,
        Some(snap),
        None,
    )
    .await?;
    tx.commit().await.map_err(sqlx_error)?;
    Ok(true)
}

/// Gives an account a role. False if it had it already. You must hold every permission the role
/// grants, and the role must be below your highest. Needs `roles.manage` or `roles.assign`; the
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

    let added = roles::assign(&mut tx, account_id, role_id, Some(caller.account_id))
        .await
        .map_err(db_error)?;
    if added {
        write_audit(
            &mut tx,
            caller,
            "role.assigned",
            "account",
            &ids::to_text(account_id),
            None,
            Some(json!({ "role_id": ids::to_text(role_id), "role": role.name })),
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
    let managers_after = managers(state, &mut tx).await?;
    actor
        .keeps_a_manager(&managers_before, &managers_after)
        .map_err(rule)?;
    if removed {
        write_audit(
            &mut tx,
            caller,
            "role.unassigned",
            "account",
            &ids::to_text(account_id),
            Some(json!({ "role_id": ids::to_text(role_id), "role": role.name })),
            None,
        )
        .await?;
    }
    tx.commit().await.map_err(sqlx_error)?;
    Ok(removed)
}

#[cfg(test)]
mod tests {
    use super::*;

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
