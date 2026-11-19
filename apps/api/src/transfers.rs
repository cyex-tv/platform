//! Giving an emote to another account (ADR 0014; owner decisions of 2026/11/19).
//!
//! - **Who may give it:** its owner, and an account editor of the owner with `emote_transfer` on
//!   every set.
//! - **Who may receive it:** the caller, or an account where the caller holds `emote_upload` on
//!   every set (it already trusts them with uploads). Blocked, deleted and restricted accounts
//!   (either side, and the caller) are refused.
//! - **Owner must accept:** the owner's own transfer is immediate. An editor's makes a pending
//!   request instead; the owner accepts (the emote moves) or declines, the editor may cancel, and
//!   one left alone expires after 7 days. One open request per emote. Accepting checks the rules
//!   again, as they stand then.
//! - Every step is audited (`emote.transfer_requested`, `_accepted`, `_declined`, `_cancelled`,
//!   `_expired`, and `emote.transferred` when it moves), with the owner as subject, and the other
//!   side is notified.

use axum::http::StatusCode;
use chrono::{Duration, Utc};
use cyex_tv_auth::Caller;
use cyex_tv_db::transfers::{self, Request};
use cyex_tv_db::{DbError, accounts, audit, emotes, ids, reads};
use cyex_tv_shared::error::{ApiError, ApiResult};
use cyex_tv_shared::model::{
    AccountSummary, EditorPermission, Emote, EmoteState, EmoteTransferRequest, EventType,
    TransferRequestState,
};
use serde_json::{Value, json};
use sqlx::{Connection, PgConnection};
use uuid::Uuid;

use crate::catalog::{db_error, forbidden, not_found, parse_id};
use crate::moderation::{
    OWNER_RESTRICTED, YOU_RESTRICTED, announce, build, editor_of_owner, not_restricted, restricted,
    row,
};
use crate::state::AppState;
use crate::views;

/// How long a request waits for the owner.
pub const REQUEST_DAYS: i64 = 7;

fn sqlx_error(err: sqlx::Error) -> ApiError {
    db_error(DbError::Sqlx(err))
}

fn bad(code: &'static str, message: &str) -> ApiError {
    ApiError::new(StatusCode::BAD_REQUEST, code, message)
}

fn conflict(code: &'static str, message: &str) -> ApiError {
    ApiError::new(StatusCode::CONFLICT, code, message)
}

fn rate_limited(state: &AppState, caller: &Caller) -> ApiResult<()> {
    state
        .limits
        .emote_transfers
        .check(&ids::to_text(caller.account_id))
        .map_err(|_| {
            ApiError::new(
                StatusCode::TOO_MANY_REQUESTS,
                "rate_limited",
                "That's a lot of transfers. Try again in a while.",
            )
        })
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

/// Marks due requests as expired, each audited (no actor: nobody did it).
async fn expire_due(conn: &mut PgConnection, emote: Option<Uuid>) -> ApiResult<()> {
    for r in transfers::expire_due(conn, emote).await.map_err(db_error)? {
        write(
            conn,
            None,
            "emote.transfer_expired",
            r.emote_id,
            r.from_account_id,
            None,
            Some(request_json(&r)),
        )
        .await?;
    }
    Ok(())
}

/// Expires every due request and keeps that, whatever the caller's own change does next.
async fn expire_now(conn: &mut PgConnection) -> ApiResult<()> {
    let mut tx = conn.begin().await.map_err(sqlx_error)?;
    expire_due(&mut tx, None).await?;
    tx.commit().await.map_err(sqlx_error)
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

/// The receiver rule, and that nobody involved is held: the caller, the owner, the receiver (who
/// must also be live).
async fn receiver_checks(
    conn: &mut PgConnection,
    giver: Uuid,
    from: Uuid,
    to: Uuid,
) -> ApiResult<()> {
    if to == from {
        return Err(bad("same_owner", "That account owns it already."));
    }
    let trusted = to == giver
        || crate::editors::has_emote_flag(conn, to, giver, EditorPermission::EmoteUpload).await?;
    if !trusted {
        return Err(forbidden(
            "It can go to the one giving it, or to an account they edit with the upload permission on every set.",
        ));
    }
    let receiver = accounts::get(conn, to)
        .await
        .map_err(db_error)?
        .filter(|a| a.deleted_at.is_none() && a.blocked_at.is_none())
        .ok_or_else(not_found)?;
    not_restricted(conn, giver, YOU_RESTRICTED).await?;
    not_restricted(conn, from, OWNER_RESTRICTED).await?;
    if receiver.restricted_at.is_some() {
        return Err(restricted("That account can't take emotes right now."));
    }
    Ok(())
}

/// Moves the emote, audits `emote.transferred` and tells both sides (unless they did it).
#[allow(clippy::too_many_arguments)]
async fn move_emote(
    conn: &mut PgConnection,
    caller: &Caller,
    emote: &reads::EmoteRow,
    from: Uuid,
    to: Uuid,
    via_editor: bool,
    request: Option<&Request>,
) -> ApiResult<()> {
    if !emotes::transfer(conn, emote.id, to)
        .await
        .map_err(db_error)?
    {
        return Err(not_found());
    }
    // Whatever else was waiting for this emote is moot now.
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
    let mut after = json!({ "owner_id": ids::to_text(to) });
    if via_editor {
        after["via"] = json!("account_editor");
    }
    if let Some(r) = request {
        after["request_id"] = json!(ids::to_text(r.id));
        after["requested_by"] = json!(ids::to_text(r.requested_by));
    }
    write(
        conn,
        Some(caller.account_id),
        "emote.transferred",
        emote.id,
        from,
        Some(json!({ "owner_id": ids::to_text(from), "name": emote.name })),
        Some(after),
    )
    .await?;
    let mut tell = vec![(from, "from"), (to, "to")];
    if let Some(r) = request {
        tell.push((r.requested_by, "editor"));
    }
    let mut told = Vec::new();
    for (who, role) in tell {
        if who == caller.account_id || told.contains(&who) {
            continue;
        }
        told.push(who);
        let p = payload(
            conn,
            role,
            emote.id,
            &emote.name,
            (from, to, caller.account_id),
            request.map(|r| r.id),
        )
        .await?;
        crate::account::notify(conn, who, "emote.transferred", &p).await?;
    }
    if emote.state == EmoteState::Approved {
        announce(conn, emote.id, EventType::EmoteUpdated).await?;
    }
    Ok(())
}

/// `transferEmote`: the owner's moves the emote at once; an editor's makes a request the owner
/// accepts or declines (the emote comes back unchanged).
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
    let owner = from == caller.account_id;
    let editor = !owner
        && editor_of_owner(&mut tx, caller, Some(from), EditorPermission::EmoteTransfer).await?;
    if !owner && !editor {
        // An emote others can't see yet answers as if it weren't there.
        if before.state != EmoteState::Approved {
            return Err(not_found());
        }
        return Err(forbidden(
            "Only the emote's owner and its editors with the transfer permission can give it away.",
        ));
    }
    receiver_checks(&mut tx, caller.account_id, from, to).await?;
    if owner {
        move_emote(&mut tx, caller, &before, from, to, false, None).await?;
    } else {
        expire_due(&mut tx, Some(id)).await?;
        let expires = Utc::now() + Duration::days(REQUEST_DAYS);
        let Some(r) = transfers::create(&mut tx, id, from, to, caller.account_id, expires)
            .await
            .map_err(db_error)?
        else {
            return Err(conflict(
                "transfer_pending",
                "A transfer of this emote is already waiting for its owner. Cancel that one first.",
            ));
        };
        let mut after = request_json(&r);
        after["via"] = json!("account_editor");
        write(
            &mut tx,
            Some(caller.account_id),
            "emote.transfer_requested",
            id,
            from,
            None,
            Some(after),
        )
        .await?;
        let p = payload(
            &mut tx,
            "owner",
            id,
            &before.name,
            (from, to, caller.account_id),
            Some(r.id),
        )
        .await?;
        crate::account::notify(&mut tx, from, "emote.transfer_requested", &p).await?;
    }
    tx.commit().await.map_err(sqlx_error)?;
    let after = row(&mut conn, id).await?.ok_or_else(not_found)?;
    build(state, &mut conn, &after).await
}

fn summary(names: &std::collections::HashMap<Uuid, reads::Summary>, id: Uuid) -> AccountSummary {
    names
        .get(&id)
        .map(AccountSummary::from)
        .unwrap_or_else(|| AccountSummary {
            id: ids::to_text(id),
            display_name: String::new(),
            avatar: None,
        })
}

async fn views_of(
    conn: &mut PgConnection,
    list: &[Request],
) -> ApiResult<Vec<EmoteTransferRequest>> {
    let mut people: Vec<Uuid> = Vec::new();
    for r in list {
        people.extend([r.from_account_id, r.to_account_id, r.requested_by]);
    }
    let names = reads::summaries(conn, &people).await.map_err(db_error)?;
    let emote_ids: Vec<Uuid> = list.iter().map(|r| r.emote_id).collect();
    let rows = reads::emotes(conn, &emote_ids).await.map_err(db_error)?;
    Ok(list
        .iter()
        .map(|r| EmoteTransferRequest {
            id: ids::to_text(r.id),
            emote_id: ids::to_text(r.emote_id),
            emote_name: rows
                .iter()
                .find(|e| e.id == r.emote_id)
                .map(|e| e.name.clone())
                .unwrap_or_default(),
            from: summary(&names, r.from_account_id),
            to: summary(&names, r.to_account_id),
            requested_by: summary(&names, r.requested_by),
            state: TransferRequestState::parse(&r.state).unwrap_or(TransferRequestState::Pending),
            created_at: views::time(r.created_at),
            expires_at: views::time(r.expires_at),
            resolved_at: r.resolved_at.map(views::time),
        })
        .collect())
}

/// `emoteTransferRequests`: the open requests on your emotes (to answer) and the ones you made,
/// newest first; of one emote when given.
pub async fn open_requests(
    state: &AppState,
    caller: &Caller,
    emote_id: Option<&str>,
) -> ApiResult<Vec<EmoteTransferRequest>> {
    let emote = emote_id
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(parse_id)
        .transpose()?;
    let mut conn = state.db.acquire().await.map_err(sqlx_error)?;
    let mut tx = conn.begin().await.map_err(sqlx_error)?;
    expire_due(&mut tx, emote).await?;
    let list = transfers::open_for(&mut tx, caller.account_id, emote)
        .await
        .map_err(db_error)?;
    let out = views_of(&mut tx, &list).await?;
    tx.commit().await.map_err(sqlx_error)?;
    Ok(out)
}

/// The request, still open (callers expire due requests first, in their own transaction). Anyone
/// but its owner and the editor who asked is told it doesn't exist, so request IDs can't be
/// probed.
async fn open_request(conn: &mut PgConnection, caller: &Caller, id: Uuid) -> ApiResult<Request> {
    let r = transfers::lock(conn, id)
        .await
        .map_err(db_error)?
        .filter(|r| caller.account_id == r.from_account_id || caller.account_id == r.requested_by)
        .ok_or_else(not_found)?;
    if r.state == "pending" && r.expires_at <= Utc::now() {
        return Err(conflict(
            "expired",
            "That transfer request expired. The editor can ask again.",
        ));
    }
    if r.state != "pending" {
        return Err(conflict(
            "not_pending",
            "That transfer request isn't open any more.",
        ));
    }
    Ok(r)
}

/// Closes a request that can't go ahead any more (`reason`: `moved`, `editor_permission`),
/// audited as cancelled.
async fn cancel_moot(
    conn: &mut PgConnection,
    caller: &Caller,
    r: &Request,
    reason: &str,
) -> ApiResult<()> {
    transfers::resolve(conn, r.id, "cancelled", Some(caller.account_id))
        .await
        .map_err(db_error)?;
    write(
        conn,
        Some(caller.account_id),
        "emote.transfer_cancelled",
        r.emote_id,
        r.from_account_id,
        Some(request_json(r)),
        Some(json!({ "reason": reason })),
    )
    .await
}

/// `acceptEmoteTransfer`: the owner agrees and the emote moves, if the rules still allow it.
pub async fn accept(state: &AppState, caller: &Caller, id: &str) -> ApiResult<Emote> {
    let id = parse_id(id)?;
    rate_limited(state, caller)?;
    let mut conn = state.db.acquire().await.map_err(sqlx_error)?;
    expire_now(&mut conn).await?;
    let mut tx = conn.begin().await.map_err(sqlx_error)?;
    let r = open_request(&mut tx, caller, id).await?;
    if r.from_account_id != caller.account_id {
        return Err(forbidden("Only the emote's owner can accept its transfer."));
    }
    emotes::lock(&mut tx, r.emote_id).await.map_err(db_error)?;
    let emote = row(&mut tx, r.emote_id).await?;
    let Some(emote) = emote.filter(|e| e.owner == Some(r.from_account_id)) else {
        // It moved or went away since: the request is moot.
        cancel_moot(&mut tx, caller, &r, "moved").await?;
        tx.commit().await.map_err(sqlx_error)?;
        return Err(conflict(
            "not_pending",
            "That emote isn't yours any more, so the request is closed.",
        ));
    };
    // The editor who asked must still hold `emote_transfer` on every set: a request outlives
    // neither their grant nor that flag.
    if !crate::editors::has_emote_flag(
        &mut tx,
        r.from_account_id,
        r.requested_by,
        EditorPermission::EmoteTransfer,
    )
    .await?
    {
        cancel_moot(&mut tx, caller, &r, "editor_permission").await?;
        tx.commit().await.map_err(sqlx_error)?;
        return Err(conflict(
            "not_pending",
            "The editor who asked can't transfer your emotes any more, so the request is closed.",
        ));
    }
    // As they stand now: the editor still may give it there, and nobody is held.
    receiver_checks(&mut tx, r.requested_by, r.from_account_id, r.to_account_id).await?;
    transfers::resolve(&mut tx, r.id, "accepted", Some(caller.account_id))
        .await
        .map_err(db_error)?;
    write(
        &mut tx,
        Some(caller.account_id),
        "emote.transfer_accepted",
        r.emote_id,
        r.from_account_id,
        Some(request_json(&r)),
        Some(json!({ "state": "accepted" })),
    )
    .await?;
    move_emote(
        &mut tx,
        caller,
        &emote,
        r.from_account_id,
        r.to_account_id,
        false,
        Some(&r),
    )
    .await?;
    tx.commit().await.map_err(sqlx_error)?;
    let after = row(&mut conn, r.emote_id).await?.ok_or_else(not_found)?;
    build(state, &mut conn, &after).await
}

/// `declineEmoteTransfer` (the owner) and `cancelEmoteTransfer` (the editor who asked).
async fn close(
    state: &AppState,
    caller: &Caller,
    id: &str,
    as_owner: bool,
) -> ApiResult<EmoteTransferRequest> {
    let id = parse_id(id)?;
    rate_limited(state, caller)?;
    let mut conn = state.db.acquire().await.map_err(sqlx_error)?;
    expire_now(&mut conn).await?;
    let mut tx = conn.begin().await.map_err(sqlx_error)?;
    let r = open_request(&mut tx, caller, id).await?;
    let (allowed, to_state, action, tell, kind) = if as_owner {
        (
            r.from_account_id == caller.account_id,
            "declined",
            "emote.transfer_declined",
            r.requested_by,
            "emote.transfer_declined",
        )
    } else {
        (
            r.requested_by == caller.account_id,
            "cancelled",
            "emote.transfer_cancelled",
            r.from_account_id,
            "emote.transfer_cancelled",
        )
    };
    if !allowed {
        return Err(forbidden(if as_owner {
            "Only the emote's owner can decline its transfer."
        } else {
            "Only the editor who asked can cancel the request."
        }));
    }
    let done = transfers::resolve(&mut tx, r.id, to_state, Some(caller.account_id))
        .await
        .map_err(db_error)?
        .ok_or_else(|| conflict("not_pending", "That transfer request isn't open any more."))?;
    write(
        &mut tx,
        Some(caller.account_id),
        action,
        r.emote_id,
        r.from_account_id,
        Some(request_json(&r)),
        Some(json!({ "state": to_state })),
    )
    .await?;
    let name = row(&mut tx, r.emote_id)
        .await?
        .map(|e| e.name)
        .unwrap_or_default();
    if tell != caller.account_id {
        let p = payload(
            &mut tx,
            if as_owner { "editor" } else { "owner" },
            r.emote_id,
            &name,
            (r.from_account_id, r.to_account_id, caller.account_id),
            Some(r.id),
        )
        .await?;
        crate::account::notify(&mut tx, tell, kind, &p).await?;
    }
    let out = views_of(&mut tx, std::slice::from_ref(&done))
        .await?
        .remove(0);
    tx.commit().await.map_err(sqlx_error)?;
    Ok(out)
}

pub async fn decline(
    state: &AppState,
    caller: &Caller,
    id: &str,
) -> ApiResult<EmoteTransferRequest> {
    close(state, caller, id, true).await
}

pub async fn cancel(
    state: &AppState,
    caller: &Caller,
    id: &str,
) -> ApiResult<EmoteTransferRequest> {
    close(state, caller, id, false).await
}
