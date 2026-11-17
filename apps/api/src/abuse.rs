//! Reports, takedowns and account restrictions (PLAN.md Phase 7). `docs/takedown.md` describes
//! the process for people outside cyex.tv.
//!
//! - **Report:** signed-in users report an emote they can see, with a reason; at most 10 an hour
//!   and one open report per emote each. Moderators (`emotes.review` or `emotes.takedown`) see
//!   the queue and dismiss reports; taking an emote down needs `emotes.takedown`.
//! - **Takedown:** the image is taken down wherever it is used, since uploads of the same file
//!   share it: every emote made from it becomes `taken_down` and leaves every set (clients get
//!   `emote.updated` and `set.emote_removed`), its sha256 and dHash are blocked, its owners are
//!   notified, open reports on it are closed, and the worker deletes the files and purges the CDN
//!   (`files.delete`). All in one transaction, audited.
//! - **Restrict:** `users.restrict` blocks an account's uploads, or the whole account (no sign-in,
//!   sessions ended). The owner can't be restricted.

use axum::http::StatusCode;
use chrono::Utc;
use cyex_tv_auth::Caller;
use cyex_tv_db::{
    DbError, accounts, audit, blocklist, emotes, files, ids, jobs, notifications, reads, reports,
    sessions, sets,
};
use cyex_tv_shared::error::{ApiError, ApiResult};
use cyex_tv_shared::model::{
    AccountStatus, Emote, EmoteState, EventType, Notification, Report, ReportAction, ReportQueue,
    ReportState,
};
use cyex_tv_shared::permissions;
use serde_json::{Value, json};
use sqlx::{Connection, PgConnection};
use uuid::Uuid;

use crate::catalog::{
    db_error, forbidden, holds, load_set, not_found, parse_id, publish_member_event,
};
use crate::state::AppState;
use crate::views;

/// Reports one account may file per hour.
pub const REPORTS_PER_HOUR: i64 = 10;
/// The worker job that deletes a taken-down file's objects and purges the CDN.
pub const DELETE_JOB: &str = "files.delete";
const PAGE: u32 = 50;
const PAGE_MAX: u32 = 100;

fn bad(code: &'static str, message: &str) -> ApiError {
    ApiError::new(StatusCode::BAD_REQUEST, code, message)
}

fn conflict(code: &'static str, message: &str) -> ApiError {
    ApiError::new(StatusCode::CONFLICT, code, message)
}

fn sqlx_error(err: sqlx::Error) -> ApiError {
    db_error(DbError::Sqlx(err))
}

fn text(raw: &str, what: &'static str) -> ApiResult<String> {
    let t = raw.trim();
    let len = t.chars().count();
    if len == 0 || len > 500 {
        return Err(bad(
            "bad_reason",
            &format!("Give a {what} of 1 to 500 characters."),
        ));
    }
    Ok(t.to_owned())
}

fn state_of(raw: &str) -> ReportState {
    match raw {
        "dismissed" => ReportState::Dismissed,
        "actioned" => ReportState::Actioned,
        _ => ReportState::Open,
    }
}

/// A moderator: `emotes.review` or `emotes.takedown`.
async fn moderator(conn: &mut PgConnection, caller: &Caller) -> ApiResult<()> {
    if holds(conn, caller, permissions::EMOTES_TAKEDOWN).await?
        || holds(conn, caller, permissions::EMOTES_REVIEW).await?
    {
        Ok(())
    } else {
        Err(forbidden("This needs emotes.review or emotes.takedown."))
    }
}

async fn emote_view(
    state: &AppState,
    conn: &mut PgConnection,
    id: Uuid,
) -> ApiResult<Option<Emote>> {
    // Taken-down and deleted emotes included: moderators need to see what was reported.
    let row: Option<reads::EmoteRow> = sqlx::query_as(
        "SELECT e.id, e.owner_account_id AS owner, e.name, e.zero_width, e.private,
                e.nsfw, e.animated, e.state, e.created_at, f.sha256, f.variants
         FROM emotes e LEFT JOIN files f ON f.id = e.file_id WHERE e.id = $1",
    )
    .bind(id)
    .fetch_optional(&mut *conn)
    .await
    .map_err(sqlx_error)?;
    let Some(row) = row else {
        return Ok(None);
    };
    let mut built = views::emotes(conn, std::slice::from_ref(&row), &state.cdn_url)
        .await
        .map_err(db_error)?;
    Ok(built.pop())
}

async fn report_view(
    state: &AppState,
    conn: &mut PgConnection,
    r: &reports::Report,
) -> ApiResult<Report> {
    Ok(Report {
        id: ids::to_text(r.id),
        emote: emote_view(state, conn, r.emote_id).await?,
        reporter_id: r.reporter_id.map(ids::to_text),
        reason: r.reason.clone(),
        state: state_of(&r.state),
        resolution: r.resolution.clone(),
        created_at: views::time(r.created_at),
        resolved_at: r.resolved_at.map(views::time),
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

// Reports

pub async fn report(
    state: &AppState,
    caller: &Caller,
    emote_id: &str,
    reason: &str,
) -> ApiResult<Report> {
    let reason = text(reason, "reason")?;
    // Only what the caller can see can be reported.
    if crate::moderation::emote(state, Some(caller), emote_id)
        .await?
        .is_none()
    {
        return Err(not_found());
    }
    let emote_id = parse_id(emote_id)?;
    let mut conn = state.db.acquire().await.map_err(sqlx_error)?;
    let recent = reports::count_by_since(
        &mut conn,
        caller.account_id,
        Utc::now() - chrono::Duration::hours(1),
    )
    .await
    .map_err(db_error)?;
    if recent >= REPORTS_PER_HOUR {
        return Err(ApiError::new(
            StatusCode::TOO_MANY_REQUESTS,
            "rate_limited",
            "That's a lot of reports. Try again in an hour.",
        ));
    }
    let created = match reports::create(&mut conn, emote_id, caller.account_id, &reason).await {
        Ok(r) => r,
        Err(DbError::AlreadyInSet) => {
            return Err(conflict(
                "already_reported",
                "You have reported this emote already; a moderator will look at it.",
            ));
        }
        Err(err) => return Err(db_error(err)),
    };
    report_view(state, &mut conn, &created).await
}

pub async fn queue(
    state: &AppState,
    caller: &Caller,
    page: Option<u32>,
    per_page: Option<u32>,
) -> ApiResult<ReportQueue> {
    let mut conn = state.db.acquire().await.map_err(sqlx_error)?;
    moderator(&mut conn, caller).await?;
    let per_page = per_page.unwrap_or(PAGE).clamp(1, PAGE_MAX);
    let page = page.unwrap_or(1).max(1);
    let (rows, total) = reports::open(
        &mut conn,
        i64::from(per_page),
        i64::from(page - 1) * i64::from(per_page),
    )
    .await
    .map_err(db_error)?;
    let mut items = Vec::with_capacity(rows.len());
    for r in &rows {
        items.push(report_view(state, &mut conn, r).await?);
    }
    Ok(ReportQueue {
        items,
        total: u64::try_from(total).unwrap_or(0),
    })
}

pub async fn resolve(
    state: &AppState,
    caller: &Caller,
    id: &str,
    action: ReportAction,
    note: Option<&str>,
) -> ApiResult<Report> {
    let id = parse_id(id)?;
    let note = note.map(|n| text(n, "note")).transpose()?;
    let mut conn = state.db.acquire().await.map_err(sqlx_error)?;
    moderator(&mut conn, caller).await?;
    let found = reports::get(&mut conn, id)
        .await
        .map_err(db_error)?
        .ok_or_else(not_found)?;
    if found.state != "open" {
        return Err(conflict("not_open", "That report was already resolved."));
    }
    drop(conn);
    match action {
        ReportAction::Dismiss => {
            let mut conn = state.db.acquire().await.map_err(sqlx_error)?;
            let mut tx = conn.begin().await.map_err(sqlx_error)?;
            let done =
                reports::resolve(&mut tx, id, "dismissed", caller.account_id, note.as_deref())
                    .await
                    .map_err(db_error)?
                    .ok_or_else(|| conflict("not_open", "That report was already resolved."))?;
            write_audit(
                &mut tx,
                caller,
                "report.dismissed",
                "report",
                &ids::to_text(id),
                None,
                Some(json!({ "emote_id": ids::to_text(found.emote_id), "note": note })),
            )
            .await?;
            tx.commit().await.map_err(sqlx_error)?;
            report_view(state, &mut conn, &done).await
        }
        ReportAction::TakeDown => {
            let reason = note.unwrap_or_else(|| found.reason.clone());
            takedown(state, caller, &ids::to_text(found.emote_id), &reason).await?;
            let mut conn = state.db.acquire().await.map_err(sqlx_error)?;
            let done = reports::get(&mut conn, id)
                .await
                .map_err(db_error)?
                .ok_or_else(not_found)?;
            report_view(state, &mut conn, &done).await
        }
    }
}

// Takedown

/// Takes an emote's image down everywhere. Needs `emotes.takedown`.
pub async fn takedown(
    state: &AppState,
    caller: &Caller,
    id: &str,
    reason: &str,
) -> ApiResult<Emote> {
    let id = parse_id(id)?;
    let reason = text(reason, "reason")?;
    let mut conn = state.db.acquire().await.map_err(sqlx_error)?;
    let mut tx = conn.begin().await.map_err(sqlx_error)?;
    if !holds(&mut tx, caller, permissions::EMOTES_TAKEDOWN).await? {
        return Err(forbidden("This needs emotes.takedown."));
    }
    let target = emotes::lock(&mut tx, id)
        .await
        .map_err(db_error)?
        .ok_or_else(not_found)?;
    if target.state == EmoteState::TakenDown {
        return Err(conflict(
            "already_taken_down",
            "That emote is taken down already.",
        ));
    }
    // Everyone's uploads of the same image go too.
    let affected = match target.file_id {
        Some(file_id) => emotes::by_file(&mut tx, file_id).await.map_err(db_error)?,
        None => vec![target.clone()],
    };
    let affected_ids: Vec<Uuid> = affected.iter().map(|e| e.id).collect();

    for emote in &affected {
        // Clients showing it hear first that it changed, then that it left each set.
        for set_id in emotes::sets_containing(&mut tx, emote.id)
            .await
            .map_err(db_error)?
        {
            let set = load_set(&mut tx, set_id).await?;
            publish_member_event(&mut tx, &set, EventType::EmoteUpdated, emote.id, None).await?;
        }
        for (set_id, name) in emotes::remove_from_all_sets(&mut tx, emote.id)
            .await
            .map_err(db_error)?
        {
            let set = sets::get(&mut tx, set_id)
                .await
                .map_err(db_error)?
                .ok_or_else(not_found)?;
            publish_member_event(
                &mut tx,
                &set,
                EventType::SetEmoteRemoved,
                emote.id,
                Some(&name),
            )
            .await?;
        }
        emotes::set_state(&mut tx, emote.id, EmoteState::TakenDown)
            .await
            .map_err(db_error)?;
        if let Some(owner) = emote.owner_account_id {
            notifications::create(
                &mut tx,
                owner,
                "emote.taken_down",
                &json!({ "emote_id": ids::to_text(emote.id), "name": emote.name, "reason": reason }),
            )
            .await
            .map_err(db_error)?;
        }
    }

    // Block the image, so it can't come back.
    if let Some(file_id) = target.file_id {
        let file = files::get(&mut tx, file_id)
            .await
            .map_err(db_error)?
            .ok_or_else(not_found)?;
        let sha: [u8; 32] = file
            .sha256
            .as_slice()
            .try_into()
            .map_err(|_| ApiError::internal())?;
        if blocklist::check(&mut tx, &sha, file.dhash.unwrap_or(0))
            .await
            .map_err(db_error)?
            .is_none()
        {
            blocklist::add(
                &mut tx,
                Some(&sha),
                file.dhash,
                &format!("takedown: {reason}"),
                Some(caller.account_id),
            )
            .await
            .map_err(db_error)?;
        }
        jobs::enqueue(
            &mut tx,
            DELETE_JOB,
            &json!({ "file_id": ids::to_text(file_id) }),
            10,
        )
        .await
        .map_err(db_error)?;
    }
    reports::action_all(&mut tx, &affected_ids, caller.account_id, &reason)
        .await
        .map_err(db_error)?;
    write_audit(
        &mut tx,
        caller,
        "emote.taken_down",
        "emote",
        &ids::to_text(id),
        Some(json!({ "state": target.state })),
        Some(json!({
            "state": EmoteState::TakenDown,
            "reason": reason,
            "emotes": affected_ids.iter().map(|e| ids::to_text(*e)).collect::<Vec<_>>(),
        })),
    )
    .await?;
    tx.commit().await.map_err(sqlx_error)?;
    emote_view(state, &mut conn, id)
        .await?
        .ok_or_else(not_found)
}

// Restrictions

pub async fn restrict(
    state: &AppState,
    caller: &Caller,
    account_id: &str,
    uploads_blocked: Option<bool>,
    blocked: Option<bool>,
) -> ApiResult<AccountStatus> {
    let account_id = parse_id(account_id)?;
    let mut conn = state.db.acquire().await.map_err(sqlx_error)?;
    let mut tx = conn.begin().await.map_err(sqlx_error)?;
    if !holds(&mut tx, caller, permissions::USERS_RESTRICT).await? {
        return Err(forbidden("This needs users.restrict."));
    }
    let before = accounts::get(&mut tx, account_id)
        .await
        .map_err(db_error)?
        .ok_or_else(not_found)?;
    if state
        .auth
        .is_owner(&mut tx, account_id)
        .await
        .map_err(ApiError::from)?
    {
        return Err(ApiError::new(
            StatusCode::FORBIDDEN,
            "owner_protected",
            "The owner's account can't be restricted.",
        ));
    }
    let mut after = before.clone();
    if let Some(u) = uploads_blocked {
        after = accounts::set_restricted(&mut tx, account_id, u)
            .await
            .map_err(db_error)?;
    }
    if let Some(b) = blocked {
        after = accounts::set_blocked(&mut tx, account_id, b)
            .await
            .map_err(db_error)?;
        if b {
            sessions::delete_for_account(&mut tx, account_id, None)
                .await
                .map_err(db_error)?;
        }
    }
    write_audit(
        &mut tx,
        caller,
        "account.restricted",
        "account",
        &ids::to_text(account_id),
        Some(json!({ "uploads_blocked": before.restricted_at.is_some(), "blocked": before.blocked_at.is_some() })),
        Some(json!({ "uploads_blocked": after.restricted_at.is_some(), "blocked": after.blocked_at.is_some() })),
    )
    .await?;
    tx.commit().await.map_err(sqlx_error)?;
    Ok(AccountStatus {
        id: ids::to_text(account_id),
        uploads_blocked: after.restricted_at.is_some(),
        blocked: after.blocked_at.is_some(),
    })
}

// Notifications

pub async fn my_notifications(
    state: &AppState,
    caller: &Caller,
    unread_only: bool,
) -> ApiResult<Vec<Notification>> {
    let mut conn = state.db.acquire().await.map_err(sqlx_error)?;
    Ok(
        notifications::list(&mut conn, caller.account_id, unread_only)
            .await
            .map_err(db_error)?
            .into_iter()
            .map(|n| Notification {
                id: ids::to_text(n.id),
                kind: n.kind,
                payload: n.payload.to_string(),
                created_at: views::time(n.created_at),
                read: n.read_at.is_some(),
            })
            .collect(),
    )
}

pub async fn mark_read(state: &AppState, caller: &Caller) -> ApiResult<u64> {
    let mut conn = state.db.acquire().await.map_err(sqlx_error)?;
    notifications::mark_read(&mut conn, caller.account_id)
        .await
        .map_err(db_error)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reasons_are_bounded() {
        assert_eq!(text("  spam ", "reason").unwrap(), "spam");
        assert!(text("", "reason").is_err());
        assert!(text(&"x".repeat(501), "note").is_err());
    }

    #[test]
    fn report_states_read_back() {
        assert_eq!(state_of("open"), ReportState::Open);
        assert_eq!(state_of("dismissed"), ReportState::Dismissed);
        assert_eq!(state_of("actioned"), ReportState::Actioned);
    }
}
