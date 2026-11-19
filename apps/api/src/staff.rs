//! The staff account dashboard's reads and its notes (ADR 0010 amendment 2026/11/19):
//!
//! - **Notes:** staff with `users.notes` keep short notes on an account, with author and time.
//!   Adding and deleting one is audited; a deleted note stays in the database for the trail.
//! - **Sessions:** where an account is signed in (`users.sessions`). Never tokens.
//! - **Subscriptions:** none exist yet; the read is there so pages and clients can be built
//!   against it, and returns nothing until they do (`billing.view`, `subscriptions.manage` or
//!   `shop.orders`).
//! - **Entitlement graph:** for an account, every badge, paint and role it holds or held and where
//!   each came from (a staff grant, an order, a redeem code, an event, the User role everyone
//!   holds, the owner list), who gave it, when, and when and by whom it was taken away; and the
//!   reverse for one badge, paint or role. Built from the entitlement tables, the shop's order
//!   entitlements, code redemptions, role assignments and the audit log. Needs
//!   `entitlements.view` or `cosmetics.grant`.

use std::collections::HashMap;

use axum::http::StatusCode;
use cyex_tv_auth::Caller;
use cyex_tv_db::{DbError, accounts, audit, ids, roles, sessions, staff};
use cyex_tv_shared::error::{ApiError, ApiResult};
use cyex_tv_shared::model::{
    AccountSession, AccountSubscription, EntitlementGrant, GrantKind, GrantSource, StaffNote,
};
use cyex_tv_shared::permissions;
use cyex_tv_shared::text;
use serde_json::{Value, json};
use sqlx::{Connection, PgConnection};
use uuid::Uuid;

use crate::catalog::{db_error, forbidden, not_found, parse_id, permissions_of};
use crate::state::AppState;
use crate::views;

/// The longest note.
pub const MAX_NOTE_LEN: usize = 2000;
/// Notes, sessions and grants one read returns at most.
const LIMIT: i64 = 200;

fn sqlx_error(err: sqlx::Error) -> ApiError {
    db_error(DbError::Sqlx(err))
}

async fn require_any(conn: &mut PgConnection, caller: &Caller, any: &[&str]) -> ApiResult<()> {
    if caller.owner {
        return Ok(());
    }
    let held = permissions_of(conn, caller).await?;
    if any.iter().any(|p| held.iter().any(|h| h == p)) {
        return Ok(());
    }
    Err(forbidden(&format!("This needs {}.", any.join(" or "))))
}

async fn live_account(conn: &mut PgConnection, id: Uuid) -> ApiResult<accounts::Account> {
    accounts::get(conn, id)
        .await
        .map_err(db_error)?
        .filter(|a| a.deleted_at.is_none())
        .ok_or_else(not_found)
}

async fn write_audit(
    conn: &mut PgConnection,
    caller: &Caller,
    action: &str,
    target_id: &str,
    before: Option<Value>,
    after: Option<Value>,
) -> ApiResult<()> {
    audit::write(
        conn,
        &audit::NewEntry {
            actor: Some(caller.account_id),
            action,
            target_kind: "account",
            target_id,
            before,
            after,
        },
    )
    .await
    .map_err(db_error)?;
    Ok(())
}

// Notes

fn note_body(raw: &str) -> ApiResult<String> {
    let body = raw.trim();
    let len = body.chars().count();
    if len == 0 || len > MAX_NOTE_LEN || text::has_unsafe(body, true) {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "bad_note",
            "Notes are 1 to 2000 characters.",
        ));
    }
    Ok(body.to_owned())
}

fn note_view(n: &staff::Note) -> StaffNote {
    StaffNote {
        id: ids::to_text(n.id),
        account_id: ids::to_text(n.account_id),
        author_id: n.author_id.map(ids::to_text),
        body: n.body.clone(),
        created_at: views::time(n.created_at),
    }
}

pub async fn notes(
    state: &AppState,
    caller: &Caller,
    account_id: &str,
) -> ApiResult<Vec<StaffNote>> {
    let account_id = parse_id(account_id)?;
    let mut conn = state.db.acquire().await.map_err(sqlx_error)?;
    require_any(&mut conn, caller, &[permissions::USERS_NOTES]).await?;
    Ok(staff::notes(&mut conn, account_id, LIMIT)
        .await
        .map_err(db_error)?
        .iter()
        .map(note_view)
        .collect())
}

pub async fn add_note(
    state: &AppState,
    caller: &Caller,
    account_id: &str,
    body: &str,
) -> ApiResult<StaffNote> {
    let account_id = parse_id(account_id)?;
    let body = note_body(body)?;
    let mut conn = state.db.acquire().await.map_err(sqlx_error)?;
    let mut tx = conn.begin().await.map_err(sqlx_error)?;
    require_any(&mut tx, caller, &[permissions::USERS_NOTES]).await?;
    live_account(&mut tx, account_id).await?;
    let note = staff::add_note(&mut tx, account_id, caller.account_id, &body)
        .await
        .map_err(db_error)?;
    write_audit(
        &mut tx,
        caller,
        "account.note_added",
        &ids::to_text(account_id),
        None,
        Some(json!({ "note_id": ids::to_text(note.id), "body": body })),
    )
    .await?;
    tx.commit().await.map_err(sqlx_error)?;
    Ok(note_view(&note))
}

/// Deletes a note: its author may, and so may the owner. False if it was gone.
pub async fn delete_note(state: &AppState, caller: &Caller, id: &str) -> ApiResult<bool> {
    let id = parse_id(id)?;
    let mut conn = state.db.acquire().await.map_err(sqlx_error)?;
    let mut tx = conn.begin().await.map_err(sqlx_error)?;
    require_any(&mut tx, caller, &[permissions::USERS_NOTES]).await?;
    let Some(note) = staff::note(&mut tx, id).await.map_err(db_error)? else {
        return Ok(false);
    };
    if note.author_id != Some(caller.account_id) && !caller.owner {
        return Err(forbidden("Only the note's author can delete it."));
    }
    let done = staff::delete_note(&mut tx, id, caller.account_id)
        .await
        .map_err(db_error)?;
    if done {
        write_audit(
            &mut tx,
            caller,
            "account.note_deleted",
            &ids::to_text(note.account_id),
            Some(json!({ "note_id": ids::to_text(note.id), "body": note.body })),
            None,
        )
        .await?;
    }
    tx.commit().await.map_err(sqlx_error)?;
    Ok(done)
}

// Sessions and subscriptions

pub async fn account_sessions(
    state: &AppState,
    caller: &Caller,
    account_id: &str,
) -> ApiResult<Vec<AccountSession>> {
    let account_id = parse_id(account_id)?;
    let mut conn = state.db.acquire().await.map_err(sqlx_error)?;
    require_any(&mut conn, caller, &[permissions::USERS_SESSIONS]).await?;
    live_account(&mut conn, account_id).await?;
    Ok(sessions::list_for_account(&mut conn, account_id)
        .await
        .map_err(db_error)?
        .iter()
        .map(|s| crate::account::session_view(s, caller))
        .collect())
}

/// An account's subscriptions, past, current and to come. None exist yet: always empty.
pub async fn account_subscriptions(
    state: &AppState,
    caller: &Caller,
    account_id: &str,
) -> ApiResult<Vec<AccountSubscription>> {
    let account_id = parse_id(account_id)?;
    let mut conn = state.db.acquire().await.map_err(sqlx_error)?;
    require_any(
        &mut conn,
        caller,
        &[
            permissions::BILLING_VIEW,
            permissions::SUBSCRIPTIONS_MANAGE,
            permissions::SHOP_ORDERS,
        ],
    )
    .await?;
    live_account(&mut conn, account_id).await?;
    Ok(Vec::new())
}

// The entitlement graph

const GRAPH_PERMISSIONS: [&str; 2] = [permissions::ENTITLEMENTS_VIEW, permissions::COSMETICS_GRANT];

fn cosmetic_kind(raw: &str) -> GrantKind {
    match raw {
        "paint" => GrantKind::Paint,
        _ => GrantKind::Badge,
    }
}

fn source_of(raw: &str) -> GrantSource {
    match raw {
        "purchase" => GrantSource::Purchase,
        "code" => GrantSource::Code,
        "event" => GrantSource::Event,
        _ => GrantSource::Staff,
    }
}

fn s(v: &Option<Value>, key: &str) -> Option<String> {
    v.as_ref()
        .and_then(|j| j.get(key))
        .and_then(Value::as_str)
        .map(str::to_owned)
}

/// Builds the grants for an account (`account`) or one item (`item`), current and past.
async fn grants(
    conn: &mut PgConnection,
    caller: &Caller,
    owner_target: bool,
    account: Option<Uuid>,
    item: Option<(GrantKind, Uuid)>,
) -> ApiResult<Vec<EntitlementGrant>> {
    let mut out: Vec<EntitlementGrant> = Vec::new();
    let cosmetic = match item {
        Some((GrantKind::Badge, id)) => Some(("badge", id)),
        Some((GrantKind::Paint, id)) => Some(("paint", id)),
        _ => None,
    };
    let want_cosmetics = item.is_none() || cosmetic.is_some();
    let want_roles = item.is_none() || matches!(item, Some((GrantKind::Role, _)));

    // Held now.
    if want_cosmetics {
        for g in staff::cosmetic_grants(
            conn,
            account,
            cosmetic.map(|c| c.0),
            cosmetic.map(|c| c.1),
            LIMIT,
        )
        .await
        .map_err(db_error)?
        {
            out.push(EntitlementGrant {
                kind: cosmetic_kind(&g.kind),
                item_id: ids::to_text(g.item_id),
                item_name: g.item_name,
                account_id: ids::to_text(g.account_id),
                source: source_of(&g.source),
                reason: g.reason,
                granted_by: g.granted_by.map(ids::to_text),
                granted_at: Some(views::time(g.granted_at)),
                order_id: g.order_id.map(ids::to_text),
                order_name: g.order_name,
                order_status: g.order_status,
                code_batch_id: g.batch_id.map(ids::to_text),
                code_batch_name: g.batch_name,
                revoked_at: None,
                revoked_by: None,
                revoke_reason: None,
                active: true,
            });
        }
        // Taken back by a refund.
        for g in staff::refunded_grants(
            conn,
            account,
            cosmetic.map(|c| c.0),
            cosmetic.map(|c| c.1),
            LIMIT,
        )
        .await
        .map_err(db_error)?
        {
            out.push(EntitlementGrant {
                kind: cosmetic_kind(&g.kind),
                item_id: ids::to_text(g.item_id),
                item_name: g.item_name.unwrap_or_default(),
                account_id: ids::to_text(g.account_id),
                source: GrantSource::Purchase,
                reason: None,
                granted_by: None,
                granted_at: g.paid_at.map(views::time),
                order_id: Some(ids::to_text(g.order_id)),
                order_name: Some(g.order_name),
                order_status: Some("refunded".to_owned()),
                code_batch_id: None,
                code_batch_name: None,
                revoked_at: g.refunded_at.map(views::time),
                revoked_by: None,
                revoke_reason: Some("Order refunded".to_owned()),
                active: false,
            });
        }
    }
    if want_roles {
        let role = match item {
            Some((GrantKind::Role, id)) => Some(id),
            _ => None,
        };
        for g in staff::role_grants(conn, account, role, LIMIT)
            .await
            .map_err(db_error)?
        {
            if g.system.as_deref() == Some(roles::SYSTEM_ROOT) && !caller.owner {
                continue;
            }
            out.push(EntitlementGrant {
                kind: GrantKind::Role,
                item_id: ids::to_text(g.role_id),
                item_name: g.role_name,
                account_id: ids::to_text(g.account_id),
                source: GrantSource::Staff,
                reason: None,
                granted_by: g.granted_by.map(ids::to_text),
                granted_at: Some(views::time(g.granted_at)),
                order_id: None,
                order_name: None,
                order_status: None,
                code_batch_id: None,
                code_batch_name: None,
                revoked_at: None,
                revoked_by: None,
                revoke_reason: None,
                active: true,
            });
        }
        // The built-in ones nobody assigns: User for everyone, the top role for the owners.
        if let Some(account) = account {
            if let Some(user) = roles::system(conn, roles::SYSTEM_USER)
                .await
                .map_err(db_error)?
            {
                out.push(implicit(&user, account, GrantSource::Default));
            }
            if owner_target
                && caller.owner
                && let Some(root) = roles::system(conn, roles::SYSTEM_ROOT)
                    .await
                    .map_err(db_error)?
            {
                out.push(implicit(&root, account, GrantSource::Owner));
            }
        }
    }

    // Taken away by staff, from the audit log. A grant event before it says when it was given.
    let account_text = account.map(ids::to_text);
    let item_text = item.map(|(_, id)| ids::to_text(id));
    // The latest events, replayed oldest first.
    let mut events = staff::grant_events(conn, account_text.as_deref(), item_text.as_deref(), 2000)
        .await
        .map_err(db_error)?;
    events.reverse();
    // (account, item) -> (granted at, granted by) of the latest grant seen so far.
    let mut given: HashMap<(String, String), (String, Option<String>)> = HashMap::new();
    for e in &events {
        let actor = e.actor.map(ids::to_text);
        let at = views::time(e.created_at);
        match e.action.as_str() {
            "cosmetic.granted" => {
                if let Some(c) = s(&e.after_json, "cosmetic_id") {
                    given.insert((e.target_id.clone(), c), (at, actor));
                }
            }
            "role.assigned" => {
                if let Some(r) = s(&e.after_json, "role_id") {
                    given.insert((e.target_id.clone(), r), (at, actor));
                }
            }
            "cosmetic.revoked" => {
                let Some(c) = s(&e.before_json, "cosmetic_id") else {
                    continue;
                };
                if cosmetic.is_none() && item.is_some() {
                    continue;
                }
                let kind = s(&e.before_json, "kind").unwrap_or_default();
                let from = given.remove(&(e.target_id.clone(), c.clone()));
                out.push(EntitlementGrant {
                    kind: cosmetic_kind(&kind),
                    item_id: c,
                    item_name: String::new(),
                    account_id: e.target_id.clone(),
                    source: source_of(&s(&e.before_json, "source").unwrap_or_default()),
                    reason: s(&e.before_json, "reason"),
                    granted_by: from.as_ref().and_then(|f| f.1.clone()),
                    granted_at: from.map(|f| f.0),
                    order_id: None,
                    order_name: None,
                    order_status: None,
                    code_batch_id: None,
                    code_batch_name: None,
                    revoked_at: Some(at),
                    revoked_by: actor,
                    revoke_reason: s(&e.after_json, "reason"),
                    active: false,
                });
            }
            "role.unassigned" => {
                if !want_roles {
                    continue;
                }
                let Some(r) = s(&e.before_json, "role_id") else {
                    continue;
                };
                let from = given.remove(&(e.target_id.clone(), r.clone()));
                out.push(EntitlementGrant {
                    kind: GrantKind::Role,
                    item_name: s(&e.before_json, "role").unwrap_or_default(),
                    item_id: r,
                    account_id: e.target_id.clone(),
                    source: GrantSource::Staff,
                    reason: None,
                    granted_by: from.as_ref().and_then(|f| f.1.clone()),
                    granted_at: from.map(|f| f.0),
                    order_id: None,
                    order_name: None,
                    order_status: None,
                    code_batch_id: None,
                    code_batch_name: None,
                    revoked_at: Some(at),
                    revoked_by: actor,
                    revoke_reason: None,
                    active: false,
                });
            }
            "role.deleted" => {
                if !want_roles {
                    continue;
                }
                let name = s(&e.before_json, "name").unwrap_or_default();
                let members: Vec<String> = e
                    .before_json
                    .as_ref()
                    .and_then(|j| j.get("members"))
                    .and_then(Value::as_array)
                    .map(|m| {
                        m.iter()
                            .filter_map(|v| v.as_str().map(str::to_owned))
                            .collect()
                    })
                    .unwrap_or_default();
                for m in members {
                    if account_text.as_ref().is_some_and(|a| *a != m) {
                        continue;
                    }
                    let from = given.remove(&(m.clone(), e.target_id.clone()));
                    out.push(EntitlementGrant {
                        kind: GrantKind::Role,
                        item_id: e.target_id.clone(),
                        item_name: name.clone(),
                        account_id: m,
                        source: GrantSource::Staff,
                        reason: None,
                        granted_by: from.as_ref().and_then(|f| f.1.clone()),
                        granted_at: from.map(|f| f.0),
                        order_id: None,
                        order_name: None,
                        order_status: None,
                        code_batch_id: None,
                        code_batch_name: None,
                        revoked_at: Some(at.clone()),
                        revoked_by: actor.clone(),
                        revoke_reason: Some("Role deleted".to_owned()),
                        active: false,
                    });
                }
            }
            _ => {}
        }
    }
    // Names for what was taken away, from what is still known (a deleted cosmetic stays unnamed).
    let names: HashMap<String, String> = out
        .iter()
        .filter(|g| !g.item_name.is_empty())
        .map(|g| (g.item_id.clone(), g.item_name.clone()))
        .collect();
    for g in out.iter_mut().filter(|g| g.item_name.is_empty()) {
        if let Some(n) = names.get(&g.item_id) {
            g.item_name = n.clone();
        } else if let Some(id) = ids::from_text(&g.item_id) {
            g.item_name = cosmetic_name(conn, g.kind, id).await?.unwrap_or_default();
        }
    }
    // Held first, then newest first.
    out.sort_by(|a, b| {
        b.active
            .cmp(&a.active)
            .then_with(|| b.revoked_at.cmp(&a.revoked_at))
            .then_with(|| b.granted_at.cmp(&a.granted_at))
    });
    Ok(out)
}

fn implicit(role: &roles::Role, account: Uuid, source: GrantSource) -> EntitlementGrant {
    EntitlementGrant {
        kind: GrantKind::Role,
        item_id: ids::to_text(role.id),
        item_name: role.name.clone(),
        account_id: ids::to_text(account),
        source,
        reason: None,
        granted_by: None,
        granted_at: None,
        order_id: None,
        order_name: None,
        order_status: None,
        code_batch_id: None,
        code_batch_name: None,
        revoked_at: None,
        revoked_by: None,
        revoke_reason: None,
        active: true,
    }
}

async fn cosmetic_name(
    conn: &mut PgConnection,
    kind: GrantKind,
    id: Uuid,
) -> ApiResult<Option<String>> {
    let table = match kind {
        GrantKind::Badge => "badges",
        GrantKind::Paint => "paints",
        GrantKind::Role => "roles",
    };
    sqlx::query_scalar(&format!("SELECT name FROM {table} WHERE id = $1"))
        .bind(id)
        .fetch_optional(conn)
        .await
        .map_err(sqlx_error)
}

/// Everything an account holds or held, and where each came from.
pub async fn graph(
    state: &AppState,
    caller: &Caller,
    account_id: &str,
) -> ApiResult<Vec<EntitlementGrant>> {
    let account_id = parse_id(account_id)?;
    let mut conn = state.db.acquire().await.map_err(sqlx_error)?;
    require_any(&mut conn, caller, &GRAPH_PERMISSIONS).await?;
    live_account(&mut conn, account_id).await?;
    let owner_target = state
        .auth
        .is_owner(&mut conn, account_id)
        .await
        .map_err(ApiError::from)?;
    grants(&mut conn, caller, owner_target, Some(account_id), None).await
}

/// Who holds or held one badge, paint or role, and how each got it (the latest 200).
pub async fn holders(
    state: &AppState,
    caller: &Caller,
    kind: GrantKind,
    id: &str,
) -> ApiResult<Vec<EntitlementGrant>> {
    let id = parse_id(id)?;
    let mut conn = state.db.acquire().await.map_err(sqlx_error)?;
    require_any(&mut conn, caller, &GRAPH_PERMISSIONS).await?;
    if kind == GrantKind::Role {
        let role = roles::get(&mut conn, id)
            .await
            .map_err(db_error)?
            .ok_or_else(not_found)?;
        if role.is(roles::SYSTEM_ROOT) && !caller.owner {
            return Err(not_found());
        }
        if role.is(roles::SYSTEM_USER) || role.is(roles::SYSTEM_ROOT) {
            return Err(ApiError::new(
                StatusCode::BAD_REQUEST,
                "implicit_role",
                "Nobody is assigned this role: every account holds User, and the owners hold the top role.",
            ));
        }
    }
    grants(&mut conn, caller, false, None, Some((kind, id))).await
}
