//! The account hub: your sessions (list, sign out one or all others), your notifications (unread
//! count, mark some read), a copy of everything about your account, and deleting it.
//!
//! Every answer is about the caller only: there is no account argument, so nobody can ask about
//! someone else (IDOR). Sessions are shown by a public ID, never by token or token hash.
//!
//! Deleting an account (the privacy page: "its data is removed"): sessions, identities, channels,
//! sets, editor grants both ways, cosmetics, roles and notifications go; reports it filed stay
//! without the reporter; its approved emotes that are in other people's sets stay, owned by
//! nobody ("deleted user"); every other emote of it is deleted. The account row stays, marked
//! deleted, so audit rows keep their actor. The owner's account is protected. Audited.

use axum::http::StatusCode;
use cyex_tv_auth::{Caller, Via};
use cyex_tv_db::sessions::{self, SessionKind};
use cyex_tv_db::{
    accounts, audit, channels, cosmetics as cosmetics_db, editors as editors_db, emotes, ids,
    notifications, outbox, privacy, reads, reports, roles, sets,
};
use cyex_tv_shared::error::{ApiError, ApiResult};
use cyex_tv_shared::model::{AccountSession, EventType, SessionKindView, SetKind};
use serde_json::{Value, json};
use sqlx::{Connection, PgConnection};
use uuid::Uuid;

use crate::catalog::{db_error, my_sets, not_found, publish_member_event};
use crate::state::AppState;
use crate::views;

fn sqlx_error(err: sqlx::Error) -> ApiError {
    db_error(err.into())
}

fn bad(code: &'static str, message: &str) -> ApiError {
    ApiError::new(StatusCode::BAD_REQUEST, code, message)
}

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

fn session_view(s: &sessions::Session, caller: &Caller) -> AccountSession {
    AccountSession {
        id: ids::to_text(s.id),
        kind: match s.kind {
            SessionKind::Site => SessionKindView::Site,
            SessionKind::App => SessionKindView::App,
        },
        device: device_name(s.kind, s.user_agent.as_deref()),
        created_at: views::time(s.created_at),
        last_seen_at: views::time(s.last_seen_at),
        expires_at: views::time(s.expires_at),
        current: s.id_hash.as_slice() == caller.token_hash.as_slice(),
    }
}

// Sessions

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

/// Signs out one of the caller's sessions (the current one too). False when they have no session
/// with that ID; another account's ID is just that.
pub async fn revoke_session(state: &AppState, caller: &Caller, id: &str) -> ApiResult<bool> {
    let Some(id) = ids::from_text(id.trim()) else {
        return Ok(false);
    };
    let mut conn = state.db.acquire().await.map_err(sqlx_error)?;
    sessions::delete_by_id(&mut conn, caller.account_id, id)
        .await
        .map_err(db_error)
}

/// Signs out every session of the caller but the one this request came with. Returns how many.
pub async fn revoke_other_sessions(state: &AppState, caller: &Caller) -> ApiResult<u64> {
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
/// and editors, account editors both ways, cosmetics owned and shown, roles, reports filed and
/// notifications. 5 an hour. Audited (`account.exported`, no content).
pub async fn export(state: &AppState, caller: &Caller) -> ApiResult<String> {
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
        // The shop isn't open yet; orders will be listed here when it is.
        "orders": [],
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

// Deleting

/// What deleting an account did, as counts (the audit row holds the same, and no names).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize)]
pub struct Deleted {
    pub emotes_kept: u64,
    pub emotes_deleted: u64,
    pub sets_deleted: u64,
    pub channels_deleted: u64,
}

/// Whether `confirm` names one of the account's logins (any case, a leading `@` allowed).
pub fn confirms(confirm: &str, logins: &[String]) -> bool {
    let typed = confirm.trim().trim_start_matches('@');
    !typed.is_empty() && logins.iter().any(|l| l.eq_ignore_ascii_case(typed))
}

/// Deletes the caller's account at once, as the module says. `confirm` must be one of its Twitch
/// or Kick logins, and the request must come from the website (the cookie session), not an app
/// token. The owner's account can't be deleted.
pub async fn delete_account(
    state: &AppState,
    caller: &Caller,
    confirm: &str,
) -> ApiResult<Deleted> {
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
    let account_id = caller.account_id;
    let mut conn = state.db.acquire().await.map_err(sqlx_error)?;
    let mut tx = conn.begin().await.map_err(sqlx_error)?;
    privacy::lock_live(&mut tx, account_id)
        .await
        .map_err(db_error)?
        .ok_or_else(not_found)?;
    let identities = accounts::identities(&mut tx, account_id)
        .await
        .map_err(db_error)?;
    let logins: Vec<String> = identities.iter().map(|i| i.login.clone()).collect();
    if !confirms(confirm, &logins) {
        return Err(bad(
            "confirm_mismatch",
            "Type one of your Twitch or Kick logins to confirm.",
        ));
    }
    let mut done = Deleted::default();

    // Emotes: kept ones lose their owner; the rest leave every set and are deleted.
    let keep = privacy::emotes_to_keep(&mut tx, account_id)
        .await
        .map_err(db_error)?;
    let remove = privacy::emotes_to_remove(&mut tx, account_id, &keep)
        .await
        .map_err(db_error)?;
    for id in &remove {
        for (set_id, name) in emotes::remove_from_all_sets(&mut tx, *id)
            .await
            .map_err(db_error)?
        {
            if let Some(set) = sets::get(&mut tx, set_id).await.map_err(db_error)?
                && set.owner_account_id != Some(account_id)
            {
                publish_member_event(&mut tx, &set, EventType::SetEmoteRemoved, *id, Some(&name))
                    .await?;
            }
        }
        if emotes::delete(&mut tx, *id).await.map_err(db_error)? {
            done.emotes_deleted += 1;
        }
    }
    done.emotes_kept = privacy::disown_emotes(&mut tx, &keep)
        .await
        .map_err(db_error)?;

    // Channels stop showing a set; chat clients hear it before the channel goes.
    for ch in channels::list_for_account(&mut tx, account_id)
        .await
        .map_err(db_error)?
    {
        if ch.active_set_id.is_some() {
            outbox::publish(
                &mut tx,
                &outbox::NewEvent {
                    event_type: EventType::ChannelActiveSetChanged,
                    topics: vec![outbox::channel_topic(ch.platform, &ch.platform_channel_id)],
                    set_id: None,
                    payload: json!({ "set_id": null }),
                },
            )
            .await
            .map_err(db_error)?;
        }
        done.channels_deleted += 1;
    }
    for identity in &identities {
        channels::delete_for(&mut tx, account_id, identity.platform)
            .await
            .map_err(db_error)?;
    }

    // Sets; for the personal set, chat clients drop its emotes, and cosmetics stop showing.
    let user_topics: Vec<String> = identities
        .iter()
        .map(|i| outbox::user_topic(i.platform, &i.platform_user_id))
        .collect();
    for set in sets::list_for_owner(&mut tx, account_id)
        .await
        .map_err(db_error)?
    {
        if set.kind == SetKind::Personal && !user_topics.is_empty() {
            let mut topics = user_topics.clone();
            topics.push(outbox::set_topic(set.id));
            outbox::publish(
                &mut tx,
                &outbox::NewEvent {
                    event_type: EventType::PersonalUpdated,
                    topics,
                    set_id: Some(set.id),
                    payload: json!({ "set_id": ids::to_text(set.id) }),
                },
            )
            .await
            .map_err(db_error)?;
        }
        if sets::delete(&mut tx, set.id).await.map_err(db_error)? {
            done.sets_deleted += 1;
        }
    }
    let shown = cosmetics_db::selection(&mut tx, account_id)
        .await
        .map_err(db_error)?;
    if (shown.0.is_some() || shown.1.is_some()) && !user_topics.is_empty() {
        outbox::publish(
            &mut tx,
            &outbox::NewEvent {
                event_type: EventType::UserCosmeticsUpdated,
                topics: user_topics.clone(),
                set_id: None,
                payload: json!({ "badge_id": null, "paint_id": null }),
            },
        )
        .await
        .map_err(db_error)?;
    }

    privacy::erase(&mut tx, account_id)
        .await
        .map_err(db_error)?;
    audit::write(
        &mut tx,
        &audit::NewEntry {
            actor: Some(account_id),
            action: "account.deleted",
            target_kind: "account",
            target_id: &ids::to_text(account_id),
            before: None,
            after: Some(json!(done)),
        },
    )
    .await
    .map_err(db_error)?;
    tx.commit().await.map_err(sqlx_error)?;
    tracing::info!(account = %ids::to_text(account_id), "account deleted");
    Ok(done)
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
