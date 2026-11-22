//! Giving an emote to another account (ADR 0014; owner decisions of 2026/11/19 and 2026/11/22).
//!
//! - **Who may give it:** its owner, nobody else. Account editors could once ask for a transfer
//!   the owner then accepted; the owner retired that (2026/11/22): nobody can ask to have someone
//!   else's emote given away. `transferEmote` by anyone but the owner is `forbidden`, and
//!   `acceptEmoteTransfer` is too. The `emote_transfer` editor flag grants nothing.
//! - **Who may receive it:** the owner, or an account where the owner holds `emote_upload` on
//!   every set (it already trusts them with uploads). Blocked, deleted and restricted accounts
//!   (either side) are refused.
//! - **Requests made before:** `withdraw_open` (run at start-up) cancels every open one, audited as
//!   `emote.transfer_cancelled` with `"reason": "retired"`, and tells both sides. Declining and
//!   cancelling still answer, for a client that shows an old request.
//! - Every move is audited (`emote.transferred`), with the owner as subject, and the receiver is
//!   notified.

use chrono::Utc;
use cyex_tv_auth::Caller;
use cyex_tv_db::transfers::{self, Request, TransferState};
use cyex_tv_db::{accounts, audit, emotes, ids, reads};
use cyex_tv_shared::error::ApiResult;
use cyex_tv_shared::model::{
    AccountSummary, EditorPermission, Emote, EmoteState, EmoteTransferRequest, EventType,
};
use serde_json::{Value, json};
use sqlx::{Connection, PgConnection};
use uuid::Uuid;

use crate::catalog::{db_error, forbidden, not_found, parse_id};
use crate::errors::{bad, conflict, limited_as, sqlx_error};
use crate::limits::AccountLimit;
use crate::moderation::{
    OWNER_RESTRICTED, YOU_RESTRICTED, announce, build, not_restricted, restricted, row,
};
use crate::state::AppState;
use crate::views;

/// What anyone but an emote's owner hears from `transferEmote` and `acceptEmoteTransfer`.
pub const OWNER_ONLY: &str = "Only an emote's owner can give it away. Nobody can ask for someone else's emote to be transferred.";

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

/// Moves the emote, audits `emote.transferred` and tells the receiver (unless it's the owner).
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
        Some(json!({ "owner_id": ids::to_text(to) })),
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

/// `transferEmote`: the owner gives the emote away, at once. Nobody else may, editors included.
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
    if from != caller.account_id {
        // An emote others can't see yet answers as if it weren't there.
        if before.state != EmoteState::Approved
            && !crate::editors::has_emote_flag(
                &mut tx,
                from,
                caller.account_id,
                EditorPermission::EmoteManage,
            )
            .await?
        {
            return Err(not_found());
        }
        return Err(forbidden(OWNER_ONLY));
    }
    receiver_checks(&mut tx, caller.account_id, from, to).await?;
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
            state: r.state.into(),
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
    if r.state == TransferState::Pending && r.expires_at <= Utc::now() {
        return Err(conflict(
            "expired",
            "That transfer request expired. The editor can ask again.",
        ));
    }
    if r.state != TransferState::Pending {
        return Err(conflict(
            "not_pending",
            "That transfer request isn't open any more.",
        ));
    }
    Ok(r)
}

/// `acceptEmoteTransfer`: retired with editor transfer requests (ADR 0014, amendment 2026/11/22).
/// Every request was withdrawn and no new one can be made, so there is nothing to accept: the same
/// `forbidden` for every ID, which tells nobody whether a request exists.
pub async fn accept(state: &AppState, caller: &Caller, id: &str) -> ApiResult<Emote> {
    parse_id(id)?;
    rate_limited(state, caller)?;
    Err(forbidden(OWNER_ONLY))
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
            TransferState::Declined,
            "emote.transfer_declined",
            r.requested_by,
            "emote.transfer_declined",
        )
    } else {
        (
            r.requested_by == caller.account_id,
            TransferState::Cancelled,
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
        Some(json!({ "state": to_state.as_str() })),
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
