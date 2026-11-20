//! The account hub: your sessions (list, sign out one or all others), your notifications (unread
//! count, mark some read), a copy of everything about your account, and deleting it.
//!
//! Every answer is about the caller only: there is no account argument, so nobody can ask about
//! someone else (IDOR). Sessions are shown by a public ID, never by token or token hash.
//!
//! Deleting an account (ADR 0019, the privacy page: "its data is removed") hides it at once: its
//! profile, sets, emotes and channels read as gone, its sessions end, and it is erased for good by
//! the worker 30 days later (`privacy::erase_account`): sessions, identities, channels, sets,
//! editor grants both ways, cosmetics, roles and notifications go; reports it filed stay without
//! the reporter; its approved emotes that are in other people's sets stay, owned by nobody
//! ("deleted user"); every other emote of it is deleted. The account row stays, marked deleted,
//! so audit rows keep their actor. Signing in again before then offers to restore it
//! (`pendingDeletion`, `restoreAccount`). The owner's account and restricted or blocked accounts
//! can't be deleted this way. Audited.

use axum::http::StatusCode;
use chrono::{DateTime, Utc};
use cyex_tv_auth::{Caller, Via};
use cyex_tv_db::sessions::{self, SessionKind};
use cyex_tv_db::{
    accounts, audit, channels, editors as editors_db, ids, notifications, privacy, reads, reports,
    roles, sets,
};
use cyex_tv_shared::error::{ApiError, ApiResult};
use cyex_tv_shared::model::{AccountSession, PendingDeletion, SessionKindView};
use serde_json::{Value, json};
use sqlx::{Connection, PgConnection};
use uuid::Uuid;

use crate::catalog::{db_error, my_sets, not_found};
use crate::errors::{bad, rate_limited, sqlx_error};
use crate::state::AppState;
use crate::views;

/// A short, human name for the device a session signed in from, from its user agent. Only the
/// browser family and the system are named; versions and everything else are dropped.
pub fn device_name(kind: SessionKind, user_agent: Option<&str>) -> String {
    let Some(ua) = user_agent.map(str::trim).filter(|ua| !ua.is_empty()) else {
        return match kind {
            SessionKind::App => "cyex.tv app".to_owned(),
            SessionKind::Site => "Unknown device".to_owned(),
        };
    };
    let browser = if ua.contains("Edg/") || ua.contains("EdgA/") || ua.contains("EdgiOS/") {
        Some("Edge")
    } else if ua.contains("OPR/") || ua.contains("Opera") {
        Some("Opera")
    } else if ua.contains("Firefox/") || ua.contains("FxiOS/") {
        Some("Firefox")
    } else if ua.contains("Chrome/") || ua.contains("CriOS/") {
        Some("Chrome")
    } else if ua.contains("Safari/") {
        Some("Safari")
    } else {
        None
    };
    let system = if ua.contains("iPhone") {
        Some("iPhone")
    } else if ua.contains("iPad") {
        Some("iPad")
    } else if ua.contains("Android") {
        Some("Android")
    } else if ua.contains("CrOS") {
        Some("ChromeOS")
    } else if ua.contains("Windows") {
        Some("Windows")
    } else if ua.contains("Mac OS X") || ua.contains("Macintosh") {
        Some("macOS")
    } else if ua.contains("Linux") {
        Some("Linux")
    } else {
        None
    };
    match (browser, system, kind) {
        (Some(b), Some(s), _) => format!("{b} on {s}"),
        (Some(b), None, _) => b.to_owned(),
        (None, Some(s), SessionKind::App) => format!("App on {s}"),
        (None, Some(s), SessionKind::Site) => format!("Browser on {s}"),
        (None, None, SessionKind::App) => "cyex.tv app".to_owned(),
        (None, None, SessionKind::Site) => "Unknown device".to_owned(),
    }
}

pub(crate) fn session_view(s: &sessions::Session, caller: &Caller) -> AccountSession {
    AccountSession {
        id: ids::to_text(s.id),
        kind: match s.kind {
            SessionKind::Site => SessionKindView::Site,
            SessionKind::App => SessionKindView::App,
        },
        // An app signed in through `/v1/auth/app` (ADR 0021) is named after itself.
        device: s
            .app_client_id
            .as_deref()
            .and_then(cyex_tv_auth::app::client)
            .map_or_else(
                || device_name(s.kind, s.user_agent.as_deref()),
                |c| c.name.to_owned(),
            ),
        app_client_id: s.app_client_id.clone(),
        created_at: views::time(s.created_at),
        last_seen_at: views::time(s.last_seen_at),
        expires_at: views::time(s.expires_at),
        current: s.id_hash.as_slice() == caller.token_hash.as_slice(),
    }
}

// Sessions

/// What only the website (the cookie session) may do: an app token that leaks must not be able
/// to take a copy of everything, or sign the person out of the website and keep itself.
fn website_only(message: &str) -> ApiError {
    ApiError::new(StatusCode::FORBIDDEN, "website_only", message)
}

/// The caller's live sessions, the most recently used first.
pub async fn my_sessions(state: &AppState, caller: &Caller) -> ApiResult<Vec<AccountSession>> {
    let mut conn = state.db.acquire().await.map_err(sqlx_error)?;
    Ok(sessions::list_for_account(&mut conn, caller.account_id)
        .await
        .map_err(db_error)?
        .iter()
        .map(|s| session_view(s, caller))
        .collect())
}

/// Signs out one of the caller's sessions (the current one too). `(false, _)` when they have no
/// session with that ID; another account's ID is just that. The second value says whether it was
/// the session this request came with (the cookie then goes too).
pub async fn revoke_session(
    state: &AppState,
    caller: &Caller,
    id: &str,
) -> ApiResult<(bool, bool)> {
    let Some(id) = ids::from_text(id.trim()) else {
        return Ok((false, false));
    };
    let mut conn = state.db.acquire().await.map_err(sqlx_error)?;
    let current = sessions::list_for_account(&mut conn, caller.account_id)
        .await
        .map_err(db_error)?
        .iter()
        .any(|s| s.id == id && s.id_hash.as_slice() == caller.token_hash.as_slice());
    let done = sessions::delete_by_id(&mut conn, caller.account_id, id)
        .await
        .map_err(db_error)?;
    Ok((done, done && current))
}

/// Signs out every session of the caller, the one this request came with too (the account
/// page's Sign out). Returns how many. With an app token, only that app session ends.
pub async fn revoke_all_sessions(state: &AppState, caller: &Caller) -> ApiResult<u64> {
    let mut conn = state.db.acquire().await.map_err(sqlx_error)?;
    if caller.via != Via::Cookie {
        let done = sessions::delete(&mut conn, &caller.token_hash)
            .await
            .map_err(db_error)?;
        return Ok(u64::from(done));
    }
    sessions::delete_for_account(&mut conn, caller.account_id, None)
        .await
        .map_err(db_error)
}

/// Signs out every session of the caller but the one this request came with. Returns how many.
/// Website only (`website_only` with an app token).
pub async fn revoke_other_sessions(state: &AppState, caller: &Caller) -> ApiResult<u64> {
    if caller.via != Via::Cookie {
        return Err(website_only(
            "Sign out other sessions on cyex.tv, signed in there.",
        ));
    }
    let mut conn = state.db.acquire().await.map_err(sqlx_error)?;
    sessions::delete_for_account(&mut conn, caller.account_id, Some(&caller.token_hash))
        .await
        .map_err(db_error)
}

// Notifications

pub async fn unread_count(state: &AppState, caller: &Caller) -> ApiResult<u64> {
    let mut conn = state.db.acquire().await.map_err(sqlx_error)?;
    let n = notifications::count_unread(&mut conn, caller.account_id)
        .await
        .map_err(db_error)?;
    Ok(u64::try_from(n).unwrap_or(0))
}

/// The most IDs one `markNotificationsRead` takes.
pub const MARK_READ_MAX: usize = 100;

/// Marks the given notifications read (only the caller's), or all of them when `ids` is `None`.
pub async fn mark_read(
    state: &AppState,
    caller: &Caller,
    ids_in: Option<&[String]>,
) -> ApiResult<u64> {
    let mut conn = state.db.acquire().await.map_err(sqlx_error)?;
    let Some(raw) = ids_in else {
        return notifications::mark_read(&mut conn, caller.account_id)
            .await
            .map_err(db_error);
    };
    if raw.len() > MARK_READ_MAX {
        return Err(bad("too_many", "Mark at most 100 notifications at once."));
    }
    let wanted: Vec<Uuid> = raw
        .iter()
        .filter_map(|r| ids::from_text(r.trim()))
        .collect();
    if wanted.is_empty() {
        return Ok(0);
    }
    notifications::mark_read_ids(&mut conn, caller.account_id, &wanted)
        .await
        .map_err(db_error)
}

/// Tells an account something. Kinds and payloads are listed on `Notification.kind`.
pub(crate) async fn notify(
    conn: &mut PgConnection,
    account_id: Uuid,
    kind: &str,
    payload: &Value,
) -> ApiResult<()> {
    notifications::create(conn, account_id, kind, payload)
        .await
        .map_err(db_error)?;
    Ok(())
}

// Export

/// Format name and version of the export, so tools can tell what they read.
pub const EXPORT_FORMAT: &str = "cyex.tv account export";
pub const EXPORT_VERSION: u32 = 1;

fn to_json<T: serde::Serialize>(value: &T) -> ApiResult<Value> {
    serde_json::to_value(value).map_err(|err| {
        tracing::error!(error = %err, "export didn't serialize");
        ApiError::internal()
    })
}

/// Everything about the caller's account, as one JSON document: the account, identities,
/// sessions (no tokens), channels, emotes in every state with their tags, sets with their emotes
/// and editors, account editors both ways, cosmetics owned and shown, roles, reports filed,
/// notifications, shop orders (bought and received) and redeemed codes. Website only
/// (`website_only` with an app token). 5 an hour. Audited (`account.exported`, no content).
pub async fn export(state: &AppState, caller: &Caller) -> ApiResult<String> {
    if caller.via != Via::Cookie {
        return Err(website_only(
            "Download your data on cyex.tv, signed in there.",
        ));
    }
    state
        .limits
        .data_export
        .check(&ids::to_text(caller.account_id))
        .map_err(|_| {
            ApiError::new(
                StatusCode::TOO_MANY_REQUESTS,
                "rate_limited",
                "You asked for a copy of your data a few times already. Try again in an hour.",
            )
        })?;
    let account_id = caller.account_id;
    let mut conn = state.db.acquire().await.map_err(sqlx_error)?;
    let account = accounts::get(&mut conn, account_id)
        .await
        .map_err(db_error)?
        .ok_or_else(not_found)?;

    let identities: Vec<Value> = accounts::identities(&mut conn, account_id)
        .await
        .map_err(db_error)?
        .into_iter()
        .map(|i| {
            json!({
                "platform": i.platform,
                "platform_user_id": i.platform_user_id,
                "login": i.login,
                "display_name": i.display_name,
                "avatar": i.avatar,
                "linked_at": views::time(i.linked_at),
                "refreshed_at": views::time(i.refreshed_at),
            })
        })
        .collect();
    let sessions_out: Vec<Value> = sessions::list_for_account(&mut conn, account_id)
        .await
        .map_err(db_error)?
        .iter()
        .map(|s| to_json(&session_view(s, caller)))
        .collect::<ApiResult<_>>()?;
    let channels_out: Vec<Value> = channels::list_for_account(&mut conn, account_id)
        .await
        .map_err(db_error)?
        .into_iter()
        .map(|c| {
            json!({
                "platform": c.platform,
                "platform_channel_id": c.platform_channel_id,
                "login": c.login,
                "active_set_id": c.active_set_id.map(ids::to_text),
                "allow_personal_emotes": c.allow_personal_emotes,
                "created_at": views::time(c.created_at),
            })
        })
        .collect();

    // Emotes in every state, newest first, in pages.
    let mut emotes_out = Vec::new();
    let mut offset = 0;
    loop {
        let (rows, total) = reads::emotes_by_owner(&mut conn, account_id, 500, offset)
            .await
            .map_err(db_error)?;
        let n = rows.len() as i64;
        for e in views::emotes(&mut conn, &rows, &state.cdn_url)
            .await
            .map_err(db_error)?
        {
            emotes_out.push(to_json(&e)?);
        }
        offset += n;
        if n == 0 || offset >= total {
            break;
        }
    }

    drop(conn);
    let sets_out = to_json(&my_sets(state, caller).await?)?;
    let mut conn = state.db.acquire().await.map_err(sqlx_error)?;
    let mut set_editors = Vec::new();
    for set in sets::list_for_owner(&mut conn, account_id)
        .await
        .map_err(db_error)?
    {
        let eds: Vec<String> = sets::editors(&mut conn, set.id)
            .await
            .map_err(db_error)?
            .into_iter()
            .map(|e| ids::to_text(e.account_id))
            .collect();
        if !eds.is_empty() {
            set_editors.push(json!({ "set_id": ids::to_text(set.id), "editor_ids": eds }));
        }
    }
    let grant = |g: editors_db::Grant| {
        json!({
            "account_id": ids::to_text(g.account_id),
            "editor_id": ids::to_text(g.editor_id),
            "permissions": g.permissions,
            "granted_at": views::time(g.granted_at),
        })
    };
    let account_editors: Vec<Value> = editors_db::list_for_account(&mut conn, account_id)
        .await
        .map_err(db_error)?
        .into_iter()
        .map(grant)
        .collect();
    let editor_of: Vec<Value> = editors_db::list_for_editor(&mut conn, account_id)
        .await
        .map_err(db_error)?
        .into_iter()
        .map(grant)
        .collect();
    let role_names: Vec<String> = roles::roles_of(&mut conn, account_id)
        .await
        .map_err(db_error)?
        .into_iter()
        .map(|r| r.name)
        .collect();
    let reports_out: Vec<Value> = reports::filed_by(&mut conn, account_id)
        .await
        .map_err(db_error)?
        .into_iter()
        .map(|r| {
            json!({
                "id": ids::to_text(r.id),
                "emote_id": ids::to_text(r.emote_id),
                "reason": r.reason,
                "state": r.state,
                "created_at": views::time(r.created_at),
                "resolved_at": r.resolved_at.map(views::time),
            })
        })
        .collect();
    let notes: Vec<Value> = notifications::all_for(&mut conn, account_id)
        .await
        .map_err(db_error)?
        .into_iter()
        .map(|n| {
            json!({
                "id": ids::to_text(n.id),
                "kind": n.kind,
                "payload": n.payload,
                "created_at": views::time(n.created_at),
                "read_at": n.read_at.map(views::time),
            })
        })
        .collect();
    let mut lists = serde_json::Map::new();
    for (key, list) in [
        ("hidden_emotes", cyex_tv_db::personal::List::HiddenEmotes),
        (
            "hidden_creators",
            cyex_tv_db::personal::List::HiddenCreators,
        ),
        ("favourite_emotes", cyex_tv_db::personal::List::Favourites),
    ] {
        let entries: Vec<Value> = cyex_tv_db::personal::all(&mut conn, list, account_id)
            .await
            .map_err(db_error)?
            .into_iter()
            .map(|e| json!({ "id": ids::to_text(e.id), "added_at": views::time(e.created_at) }))
            .collect();
        lists.insert(key.to_owned(), Value::Array(entries));
    }
    let orders_out = export_orders(&mut conn, account_id).await?;
    let redemptions_out: Vec<Value> = cyex_tv_db::codes::redemptions_of(&mut conn, account_id)
        .await
        .map_err(db_error)?
        .into_iter()
        .map(|r| {
            json!({
                "code_id": ids::to_text(r.code_id),
                "batch_id": ids::to_text(r.batch_id),
                "batch_name": r.batch_name,
                "redeemed_at": views::time(r.redeemed_at),
            })
        })
        .collect();
    drop(conn);
    let cosmetics_out = to_json(&crate::cosmetics::my_cosmetics(state, caller, None).await?)?;

    let doc = json!({
        "format": EXPORT_FORMAT,
        "version": EXPORT_VERSION,
        "exported_at": views::time(chrono::Utc::now()),
        "account": {
            "id": ids::to_text(account.id),
            "created_at": views::time(account.created_at),
            "uploads_blocked": account.restricted_at.is_some(),
        },
        "identities": identities,
        "sessions": sessions_out,
        "channels": channels_out,
        "emotes": emotes_out,
        "sets": sets_out,
        "set_editors": set_editors,
        "account_editors": account_editors,
        "editor_of": editor_of,
        "cosmetics": cosmetics_out,
        "roles": role_names,
        "reports_filed": reports_out,
        "notifications": notes,
        "orders": orders_out,
        "code_redemptions": redemptions_out,
        "hidden_emotes": lists["hidden_emotes"],
        "hidden_creators": lists["hidden_creators"],
        "favourite_emotes": lists["favourite_emotes"],
    });

    let mut conn = state.db.acquire().await.map_err(sqlx_error)?;
    audit::write(
        &mut conn,
        &audit::NewEntry {
            actor: Some(account_id),
            action: "account.exported",
            target_kind: "account",
            target_id: &ids::to_text(account_id),
            before: None,
            after: None,
        },
    )
    .await
    .map_err(db_error)?;
    serde_json::to_string(&doc).map_err(|_| ApiError::internal())
}

/// The account's orders, bought and received, newest first. Paddle's transaction ID is the only
/// provider ID in them; staff refund notes aren't.
async fn export_orders(conn: &mut PgConnection, account_id: Uuid) -> ApiResult<Vec<Value>> {
    use cyex_tv_db::shop::{self as shop_db, OrderFilter};
    // As `myOrders` shows them: a gift's recipient gets it once paid, without the money.
    let filter = OrderFilter {
        party: Some(account_id),
        ..OrderFilter::default()
    };
    let mut out = Vec::new();
    let mut offset = 0;
    loop {
        let (rows, total) = shop_db::orders(conn, &filter, 100, offset)
            .await
            .map_err(db_error)?;
        let n = rows.len() as i64;
        for o in rows {
            let money = o.account_id == Some(account_id);
            out.push(json!({
                "id": ids::to_text(o.id),
                "as_buyer": o.account_id == Some(account_id),
                "as_recipient": o.recipient_id == Some(account_id),
                "buyer_id": o.account_id.map(ids::to_text),
                "recipient_id": o.recipient_id.map(ids::to_text),
                "item_id": ids::to_text(o.item_id),
                "item_name": o.item_name,
                "status": crate::shop::order_status(&o.status).as_str(),
                "gift": o.is_gift,
                "amount_cents": money.then_some(o.amount_cents),
                "currency": o.currency,
                "paid_total_cents": o.paid_total_cents.filter(|_| money),
                "paid_currency": o.paid_currency.filter(|_| money),
                "provider": o.provider,
                "provider_transaction_id": o.provider_transaction_id.filter(|_| money),
                "created_at": views::time(o.created_at),
                "paid_at": o.paid_at.map(views::time),
                "refunded_at": o.refunded_at.map(views::time),
                "failed_at": o.failed_at.map(views::time),
            }));
        }
        offset += n;
        if n == 0 || offset >= total {
            break;
        }
    }
    Ok(out)
}

// Deleting

/// Whether `confirm` names one of the account's logins (any case, a leading `@` allowed).
pub fn confirms(confirm: &str, logins: &[String]) -> bool {
    let typed = confirm.trim().trim_start_matches('@');
    !typed.is_empty() && logins.iter().any(|l| l.eq_ignore_ascii_case(typed))
}

/// Deletes the caller's account, as the module says: hidden at once, erased after 30 days unless
/// restored. `confirm` must be one of its Twitch or Kick logins, and the request must come from
/// the website (the cookie session), not an app token. The owner's account can't be deleted. 3
/// tries an hour per account (`rate_limited`). Returns when it will be erased.
pub async fn delete_account(
    state: &AppState,
    caller: &Caller,
    confirm: &str,
) -> ApiResult<DateTime<Utc>> {
    if caller.via != Via::Cookie {
        return Err(ApiError::new(
            StatusCode::FORBIDDEN,
            "website_only",
            "Delete your account on cyex.tv, signed in there.",
        ));
    }
    if caller.owner {
        return Err(ApiError::new(
            StatusCode::FORBIDDEN,
            "owner_protected",
            "The owner's account can't be deleted.",
        ));
    }
    // Counts every try, so a wrong confirmation can't be guessed at.
    state
        .limits
        .delete_account
        .check(&ids::to_text(caller.account_id))
        .map_err(|_| {
            rate_limited("That's a lot of tries to delete the account. Try again in an hour.")
        })?;
    let account_id = caller.account_id;
    let mut conn = state.db.acquire().await.map_err(sqlx_error)?;
    let mut tx = conn.begin().await.map_err(sqlx_error)?;
    privacy::lock_live(&mut tx, account_id)
        .await
        .map_err(db_error)?
        .ok_or_else(not_found)?;
    // A restricted or blocked account can't delete itself: signing in again after it is erased
    // would make a fresh, unrestricted account.
    let account = accounts::get(&mut tx, account_id)
        .await
        .map_err(db_error)?
        .ok_or_else(not_found)?;
    if account.held() {
        return Err(ApiError::new(
            StatusCode::FORBIDDEN,
            "restricted",
            "A restricted account can't be deleted. Write to us if you want it removed.",
        ));
    }
    let logins: Vec<String> = accounts::identities(&mut tx, account_id)
        .await
        .map_err(db_error)?
        .into_iter()
        .map(|i| i.login)
        .collect();
    if !confirms(confirm, &logins) {
        return Err(bad(
            "confirm_mismatch",
            "Type one of your Twitch or Kick logins to confirm.",
        ));
    }
    let erase_after = Utc::now() + chrono::Duration::days(privacy::UNDO_DAYS);
    if !privacy::schedule_erase(&mut tx, account_id, erase_after)
        .await
        .map_err(db_error)?
    {
        return Err(not_found());
    }
    audit::write(
        &mut tx,
        &audit::NewEntry {
            actor: Some(account_id),
            action: "account.deletion_scheduled",
            target_kind: "account",
            target_id: &ids::to_text(account_id),
            before: None,
            after: Some(json!({ "erase_after": views::time(erase_after) })),
        },
    )
    .await
    .map_err(db_error)?;
    tx.commit().await.map_err(sqlx_error)?;
    tracing::info!(account = %ids::to_text(account_id), "account deletion scheduled");
    Ok(erase_after)
}

/// The deletion a signed-in-to-restore session may undo (`pendingDeletion`): when it was asked
/// for, when the account will be erased, and the name it showed. `None` when there is none.
pub async fn pending_deletion(
    state: &AppState,
    pending: &Caller,
) -> ApiResult<Option<PendingDeletion>> {
    let mut conn = state.db.acquire().await.map_err(sqlx_error)?;
    let Some(account) = accounts::get(&mut conn, pending.account_id)
        .await
        .map_err(db_error)?
    else {
        return Ok(None);
    };
    if !account.restorable(Utc::now()) {
        return Ok(None);
    }
    let (Some(deleted_at), Some(erase_after)) = (account.deleted_at, account.erase_after) else {
        return Ok(None);
    };
    let identity = accounts::identities(&mut conn, account.id)
        .await
        .map_err(db_error)?
        .into_iter()
        .next();
    Ok(Some(PendingDeletion {
        requested_at: views::time(deleted_at),
        erase_after: views::time(erase_after),
        display_name: identity.as_ref().map(|i| i.display_name.clone()),
        avatar: identity.and_then(|i| i.avatar),
    }))
}

/// Restores the account a signed-in-to-restore session belongs to, within the window: it and
/// everything it had show again, and the session becomes a normal one. Audited
/// (`account.restored`). `nothing_to_restore` when the window has passed or there was nothing.
pub async fn restore_account(state: &AppState, pending: &Caller) -> ApiResult<()> {
    if pending.via != Via::Cookie {
        return Err(ApiError::new(
            StatusCode::FORBIDDEN,
            "website_only",
            "Restore your account on cyex.tv, signed in there.",
        ));
    }
    let account_id = pending.account_id;
    let mut conn = state.db.acquire().await.map_err(sqlx_error)?;
    let mut tx = conn.begin().await.map_err(sqlx_error)?;
    if !privacy::restore(&mut tx, account_id)
        .await
        .map_err(db_error)?
    {
        return Err(ApiError::new(
            StatusCode::CONFLICT,
            "nothing_to_restore",
            "There's no deleted account to restore here.",
        ));
    }
    audit::write(
        &mut tx,
        &audit::NewEntry {
            actor: Some(account_id),
            action: "account.restored",
            target_kind: "account",
            target_id: &ids::to_text(account_id),
            before: None,
            after: None,
        },
    )
    .await
    .map_err(db_error)?;
    tx.commit().await.map_err(sqlx_error)?;
    tracing::info!(account = %ids::to_text(account_id), "account restored");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn devices_are_named_without_versions() {
        let site = SessionKind::Site;
        for (ua, want) in [
            (
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36 Edg/131.0.0.0",
                "Edge on Windows",
            ),
            (
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:133.0) Gecko/20100101 Firefox/133.0",
                "Firefox on Windows",
            ),
            (
                "Mozilla/5.0 (iPhone; CPU iPhone OS 18_1 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/18.1 Mobile/15E148 Safari/604.1",
                "Safari on iPhone",
            ),
            (
                "Mozilla/5.0 (Linux; Android 14; Pixel 8) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Mobile Safari/537.36",
                "Chrome on Android",
            ),
            (
                "Mozilla/5.0 (Macintosh; Intel Mac OS X 14_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/18.1 Safari/605.1.15",
                "Safari on macOS",
            ),
            ("curl/8.9.1", "Unknown device"),
        ] {
            assert_eq!(device_name(site, Some(ua)), want, "{ua}");
        }
        assert_eq!(device_name(site, None), "Unknown device");
        assert_eq!(device_name(SessionKind::App, None), "cyex.tv app");
        assert_eq!(
            device_name(SessionKind::App, Some("cyex-deck/1.2 (Windows)")),
            "App on Windows"
        );
    }

    #[test]
    fn confirm_needs_a_login() {
        let logins = vec!["Lumen_Fox".to_owned(), "lumenkick".to_owned()];
        assert!(confirms("lumen_fox", &logins));
        assert!(confirms(" @LUMENKICK ", &logins));
        assert!(!confirms("", &logins));
        assert!(!confirms("@", &logins));
        assert!(!confirms("lumen", &logins));
    }
}
