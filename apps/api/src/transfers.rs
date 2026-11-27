//! Moving an emote to another account (ADR 0014; ADR 0039).
//!
//! - **Who may move it:** staff with `emotes.transfer`, directly, to any live account. Nobody can
//!   ask for an emote, and its owner can't give it away either (`transfer_staff_only`).
//! - **Requests made before:** `withdraw_open` (run at start-up) cancels every open one, audited
//!   as `emote.transfer_cancelled` with `"reason": "retired"`, and tells both sides. Accepting,
//!   declining and cancelling answer `transfer_requests_retired`.
//! - Every move is audited (`emote.transferred`), with the old owner as subject, and both owners
//!   are notified.

use axum::http::StatusCode;
use cyex_tv_auth::Caller;
use cyex_tv_db::transfers::{self, Request};
use cyex_tv_db::{accounts, audit, emotes, ids, reads};
use cyex_tv_shared::error::{ApiError, ApiResult};
use cyex_tv_shared::model::{EditorPermission, Emote, EmoteState, EmoteTransferRequest, EventType};
use cyex_tv_shared::permissions;
use serde_json::{Value, json};
use sqlx::{Connection, PgConnection};
use uuid::Uuid;

use crate::catalog::{db_error, holds, not_found, parse_id};
use crate::errors::{bad, limited_as, sqlx_error};
use crate::limits::AccountLimit;
use crate::moderation::{announce, build, restricted, row};
use crate::state::AppState;
use crate::views;

/// What anyone without `emotes.transfer` hears from `transferEmote` (ADR 0039).
pub const STAFF_ONLY: &str = "Only cyex.tv staff can change who owns an emote. Ownership can't be requested or given away; contact support if an emote should move.";
/// What the retired request mutations answer (ADR 0039).
pub const REQUESTS_RETIRED: &str = "Transfer requests are gone: ownership can't be requested. Only staff can move an emote to another account.";

fn rate_limited(state: &AppState, caller: &Caller) -> ApiResult<()> {
    limited_as(
        state,
        AccountLimit::EmoteTransfers,
        &ids::to_text(caller.account_id),
        caller.account_id,
        "That's a lot of transfers. Try again in a while.",
    )
}

#[allow(clippy::too_many_arguments)]
async fn write(
    conn: &mut PgConnection,
    actor: Option<Uuid>,
    action: &str,
    emote: Uuid,
    subject: Uuid,
    before: Option<Value>,
    after: Option<Value>,
) -> ApiResult<()> {
    audit::write_for(
        conn,
        &audit::NewEntry {
            actor,
            action,
            target_kind: "emote",
            target_id: &ids::to_text(emote),
            before,
            after,
        },
        Some(subject),
    )
    .await
    .map_err(db_error)?;
    Ok(())
}

fn request_json(r: &Request) -> Value {
    json!({
        "request_id": ids::to_text(r.id),
        "from_id": ids::to_text(r.from_account_id),
        "to_id": ids::to_text(r.to_account_id),
        "requested_by": ids::to_text(r.requested_by),
        "expires_at": views::time(r.expires_at),
    })
}

/// The notification payload: the emote, both accounts and the one who acted, names as they read now.
async fn payload(
    conn: &mut PgConnection,
    role: &str,
    emote: Uuid,
    name: &str,
    r: (Uuid, Uuid, Uuid),
    request: Option<Uuid>,
) -> ApiResult<Value> {
    let (from, to, actor) = r;
    let names = reads::summaries(conn, &[from, to, actor])
        .await
        .map_err(db_error)?;
    let n = |a: Uuid| names.get(&a).map(|s| s.display_name.clone());
    Ok(json!({
        "role": role,
        "emote_id": ids::to_text(emote),
        "name": name,
        "request_id": request.map(ids::to_text),
        "from_id": ids::to_text(from),
        "from_name": n(from),
        "to_id": ids::to_text(to),
        "to_name": n(to),
        "actor_id": ids::to_text(actor),
        "actor_name": n(actor),
    }))
}

/// The receiver must be a live account that may hold emotes: not deleted, blocked or restricted.
async fn receiver_checks(conn: &mut PgConnection, from: Uuid, to: Uuid) -> ApiResult<()> {
    if to == from {
        return Err(bad("same_owner", "That account owns it already."));
    }
    let receiver = accounts::get(conn, to)
        .await
        .map_err(db_error)?
        .filter(|a| a.deleted_at.is_none() && a.blocked_at.is_none())
        .ok_or_else(not_found)?;
    if receiver.restricted_at.is_some() {
        return Err(restricted("That account can't take emotes right now."));
    }
    Ok(())
}

/// Moves the emote, audits `emote.transferred` (`"via": "emotes.transfer"`) and tells the old and
/// the new owner (unless one of them did it).
async fn move_emote(
    conn: &mut PgConnection,
    caller: &Caller,
    emote: &reads::EmoteRow,
    from: Uuid,
    to: Uuid,
) -> ApiResult<()> {
    if !emotes::transfer(conn, emote.id, to)
        .await
        .map_err(db_error)?
    {
        return Err(not_found());
    }
    // Whatever was still waiting for this emote is moot now.
    for r in transfers::close_open_for_emote(conn, emote.id, Some(caller.account_id))
        .await
        .map_err(db_error)?
    {
        write(
            conn,
            Some(caller.account_id),
            "emote.transfer_cancelled",
            emote.id,
            r.from_account_id,
            Some(request_json(&r)),
            Some(json!({ "reason": "moved" })),
        )
        .await?;
    }
    write(
        conn,
        Some(caller.account_id),
        "emote.transferred",
        emote.id,
        from,
        Some(json!({ "owner_id": ids::to_text(from), "name": emote.name })),
        Some(json!({ "owner_id": ids::to_text(to), "via": permissions::EMOTES_TRANSFER })),
    )
    .await?;
    for (who, role) in [(from, "from"), (to, "to")] {
        if who == caller.account_id {
            continue;
        }
        let p = payload(
            conn,
            role,
            emote.id,
            &emote.name,
            (from, to, caller.account_id),
            None,
        )
        .await?;
        crate::account::notify(conn, who, "emote.transferred", &p).await?;
    }
    if emote.state == EmoteState::Approved {
        announce(conn, emote.id, EventType::EmoteUpdated).await?;
    }
    Ok(())
}

/// `403 transfer_staff_only`: nobody but staff with `emotes.transfer` moves an emote (ADR 0039).
pub fn staff_only() -> ApiError {
    ApiError::new(StatusCode::FORBIDDEN, "transfer_staff_only", STAFF_ONLY)
}

/// `410 transfer_requests_retired`: the request flow is gone (ADR 0039).
pub fn requests_retired() -> ApiError {
    crate::meta::note(crate::meta::EMOTE_TRANSFER_REQUESTS);
    ApiError::new(
        StatusCode::GONE,
        "transfer_requests_retired",
        REQUESTS_RETIRED,
    )
}

/// `transferEmote`: staff with `emotes.transfer` give any emote to another account, at once
/// (ADR 0039). Anyone else, the owner included, is refused (`transfer_staff_only`); an emote they
/// can't see answers `not_found`.
pub async fn transfer(
    state: &AppState,
    caller: &Caller,
    id: &str,
    to_account_id: &str,
) -> ApiResult<Emote> {
    let id = parse_id(id)?;
    let to = parse_id(to_account_id)?;
    rate_limited(state, caller)?;
    let mut conn = state.db.acquire().await.map_err(sqlx_error)?;
    let mut tx = conn.begin().await.map_err(sqlx_error)?;
    let lock = emotes::lock(&mut tx, id).await.map_err(db_error)?;
    let before = row(&mut tx, id).await?.ok_or_else(not_found)?;
    let (Some(_), Some(from)) = (lock, before.owner) else {
        return Err(not_found());
    };
    if !holds(&mut tx, caller, permissions::EMOTES_TRANSFER).await? {
        // An emote others can't see yet answers as if it weren't there.
        let sees = before.state == EmoteState::Approved
            || from == caller.account_id
            || crate::editors::has_emote_flag(
                &mut tx,
                from,
                caller.account_id,
                EditorPermission::EmoteManage,
            )
            .await?;
        return Err(if sees { staff_only() } else { not_found() });
    }
    receiver_checks(&mut tx, from, to).await?;
    move_emote(&mut tx, caller, &before, from, to).await?;
    tx.commit().await.map_err(sqlx_error)?;
    let after = row(&mut conn, id).await?.ok_or_else(not_found)?;
    build(state, &mut conn, &after).await
}

/// Cancels every open transfer request (editors can't ask any more; ADR 0014, amendment
/// 2026/11/22). Each is audited as `emote.transfer_cancelled` (no actor, `"reason": "retired"`)
/// and both the owner and the editor who asked get `emote.transfer_cancelled`. Run at start-up;
/// running it again finds nothing. Returns how many it closed.
pub async fn withdraw_open(state: &AppState) -> ApiResult<usize> {
    let mut conn = state.db.acquire().await.map_err(sqlx_error)?;
    let mut tx = conn.begin().await.map_err(sqlx_error)?;
    let closed = transfers::withdraw_all(&mut tx).await.map_err(db_error)?;
    let emote_ids: Vec<Uuid> = closed.iter().map(|r| r.emote_id).collect();
    let rows = reads::emotes(&mut tx, &emote_ids).await.map_err(db_error)?;
    for r in &closed {
        write(
            &mut tx,
            None,
            "emote.transfer_cancelled",
            r.emote_id,
            r.from_account_id,
            Some(request_json(r)),
            Some(json!({ "state": "cancelled", "reason": "retired" })),
        )
        .await?;
        let name = rows
            .iter()
            .find(|e| e.id == r.emote_id)
            .map(|e| e.name.clone())
            .unwrap_or_default();
        for (who, role) in [(r.from_account_id, "owner"), (r.requested_by, "editor")] {
            let mut p = payload(
                &mut tx,
                role,
                r.emote_id,
                &name,
                (r.from_account_id, r.to_account_id, r.requested_by),
                Some(r.id),
            )
            .await?;
            p["reason"] = json!("retired");
            crate::account::notify(&mut tx, who, "emote.transfer_cancelled", &p).await?;
        }
    }
    tx.commit().await.map_err(sqlx_error)?;
    Ok(closed.len())
}

/// `emoteTransferRequests`: always empty. Transfer requests are retired (ADR 0039); the history
/// stays in the audit log.
pub fn open_requests(
    _caller: &Caller,
    emote_id: Option<&str>,
) -> ApiResult<Vec<EmoteTransferRequest>> {
    emote_id
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(parse_id)
        .transpose()?;
    crate::meta::note(crate::meta::EMOTE_TRANSFER_REQUESTS);
    Ok(Vec::new())
}

/// `acceptEmoteTransfer`, `declineEmoteTransfer` and `cancelEmoteTransfer`: retired (ADR 0039).
/// The same `410 transfer_requests_retired` for every ID, so nothing can be probed.
pub fn retired_request<T>(state: &AppState, caller: &Caller, id: &str) -> ApiResult<T> {
    parse_id(id)?;
    rate_limited(state, caller)?;
    Err(requests_retired())
}
