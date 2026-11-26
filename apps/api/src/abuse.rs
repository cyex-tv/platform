//! Reports, takedowns and account restrictions. `docs/takedown.md` describes
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
//! - **Restrict:** staff stop an account from uploading (`users.restrict_uploads`), from filing
//!   reports and tickets (`users.restrict_tickets`), or block it outright (`users.block`: no
//!   sign-in, sessions ended); `users.restrict` does all three. Each restriction has a reason and
//!   lasts for good or until a time; the API treats one whose time has passed as gone. Every one
//!   placed or lifted is kept in `account_restrictions` and audited. The owner can't be
//!   restricted, and nobody restricts an account whose highest role is at or above their own.

use std::collections::HashMap;

use axum::http::StatusCode;
use chrono::Utc;
use cyex_tv_auth::Caller;
use cyex_tv_db::accounts::Hold;
use cyex_tv_db::{
    DbError, accounts, blocklist, emotes, files, ids, jobs, notifications, reads, reports,
    sessions, sets,
};
use cyex_tv_shared::error::{ApiError, ApiResult};
use cyex_tv_shared::model::{
    AccountReports, AccountStatus, AdminAccount, BatchOutcome, Emote, EmoteState, EventType,
    Notification, Report, ReportAction, ReportQueue, ReportState, Restriction, RestrictionInput,
    RestrictionKind, TransferRequestState,
};
use cyex_tv_shared::permissions;
use cyex_tv_shared::text;
use serde_json::{Value, json};
use sqlx::{Connection, PgConnection};
use uuid::Uuid;

use crate::admin::{actor, rule};
use crate::catalog::require_any;
use crate::catalog::{db_error, forbidden, load_set, not_found, parse_id, publish_member_event};
use crate::errors::{Audit, record_audit};
use crate::errors::{bad, conflict, paging, sqlx_error};
use crate::state::AppState;
use crate::views;

/// Reports one account may file per hour.
pub const REPORTS_PER_HOUR: i64 = 10;
/// The worker job that deletes a taken-down file's objects and purges the CDN.
pub const DELETE_JOB: &str = "files.delete";
const PAGE: u32 = 50;
const PAGE_MAX: u32 = 100;

fn text(raw: &str, what: &'static str) -> ApiResult<String> {
    let t = raw.trim();
    let len = t.chars().count();
    if len == 0 || len > 500 || text::has_unsafe(t, true) {
        return Err(bad(
            "bad_reason",
            format!("Give a {what} of 1 to 500 characters."),
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

/// A moderator: `emotes.review`, `emotes.takedown` or `reports.manage`.
async fn moderator(conn: &mut PgConnection, caller: &Caller) -> ApiResult<()> {
    require_any(
        conn,
        caller,
        &[
            permissions::EMOTES_REVIEW,
            permissions::EMOTES_TAKEDOWN,
            permissions::REPORTS_MANAGE,
        ],
    )
    .await
}

/// The emotes reported, by ID, in one query. Taken-down and deleted emotes included: moderators
/// need to see what was reported.
async fn emote_views(
    state: &AppState,
    conn: &mut PgConnection,
    ids: &[Uuid],
) -> ApiResult<HashMap<Uuid, Emote>> {
    if ids.is_empty() {
        return Ok(HashMap::new());
    }
    let rows: Vec<reads::EmoteRow> = sqlx::query_as(
        "SELECT e.id, e.owner_account_id AS owner, e.name, e.zero_width, e.private,
                e.unlisted, e.nsfw, e.animated, e.state, e.created_at, f.sha256, f.variants,
                (f.processing_failed_at IS NOT NULL) AS processing_failed
         FROM emotes e LEFT JOIN files f ON f.id = e.file_id WHERE e.id = ANY($1)",
    )
    .bind(ids)
    .fetch_all(&mut *conn)
    .await
    .map_err(sqlx_error)?;
    let built = views::emotes(conn, &rows, &state.cdn_url)
        .await
        .map_err(db_error)?;
    Ok(rows.iter().map(|r| r.id).zip(built).collect())
}

async fn emote_view(
    state: &AppState,
    conn: &mut PgConnection,
    id: Uuid,
) -> ApiResult<Option<Emote>> {
    Ok(emote_views(state, conn, &[id]).await?.remove(&id))
}

async fn report_views(
    state: &AppState,
    conn: &mut PgConnection,
    rows: &[reports::Report],
) -> ApiResult<Vec<Report>> {
    let emote_ids: Vec<Uuid> = rows.iter().map(|r| r.emote_id).collect();
    let emotes = emote_views(state, conn, &emote_ids).await?;
    Ok(rows
        .iter()
        .map(|r| Report {
            id: ids::to_text(r.id),
            emote: emotes.get(&r.emote_id).cloned(),
            reporter_id: r.reporter_id.map(ids::to_text),
            reason: r.reason.clone(),
            state: state_of(&r.state),
            resolution: r.resolution.clone(),
            created_at: views::time(r.created_at),
            resolved_at: r.resolved_at.map(views::time),
        })
        .collect())
}

async fn report_view(
    state: &AppState,
    conn: &mut PgConnection,
    r: &reports::Report,
) -> ApiResult<Report> {
    let mut built = report_views(state, conn, std::slice::from_ref(r)).await?;
    built.pop().ok_or_else(ApiError::internal)
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
    let me = accounts::get(&mut conn, caller.account_id)
        .await
        .map_err(db_error)?
        .ok_or_else(not_found)?;
    if me.tickets_blocked_at.is_some() {
        let message = match me.tickets_blocked_until {
            Some(until) => format!(
                "Your account can't file reports until {}.",
                until.format("%Y/%m/%d %H:%M UTC")
            ),
            None => "Your account can't file reports.".to_owned(),
        };
        return Err(ApiError::new(
            StatusCode::FORBIDDEN,
            "tickets_blocked",
            &message,
        ));
    }
    // Everyone may report through the User role, unless a more senior role denies it.
    if !crate::catalog::holds(&mut conn, caller, permissions::REPORTS_FILE).await? {
        return Err(ApiError::new(
            StatusCode::FORBIDDEN,
            "forbidden",
            "Your roles don't allow filing reports.",
        ));
    }
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
        )
        .retry_after(3600));
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
    let paged = paging(page, per_page, PAGE, PAGE_MAX);
    let (rows, total) = reports::open(&mut conn, paged.limit, paged.offset)
        .await
        .map_err(db_error)?;
    let items = report_views(state, &mut conn, &rows).await?;
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
            record_audit(
                &mut tx,
                caller.account_id,
                Audit::new("report.dismissed", "report", &ids::to_text(id)).change(
                    None,
                    Some(json!({ "emote_id": ids::to_text(found.emote_id), "note": note })),
                ),
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

/// Dismisses up to 100 open reports with one note, in one transaction, each with its own
/// `report.dismissed` audit row as a single dismissal has. Reports already resolved (or unknown)
/// are left alone and listed in `failed`. Takedowns stay one at a time. Needs `emotes.review`,
/// `emotes.takedown` or `reports.manage`.
pub async fn dismiss_many(
    state: &AppState,
    caller: &Caller,
    raw_ids: &[String],
    note: Option<&str>,
) -> ApiResult<BatchOutcome> {
    let wanted = crate::moderation::batch_ids(raw_ids)?;
    let note = note
        .map(str::trim)
        .filter(|n| !n.is_empty())
        .map(|n| text(n, "note"))
        .transpose()?;
    let mut conn = state.db.acquire().await.map_err(sqlx_error)?;
    let mut tx = conn.begin().await.map_err(sqlx_error)?;
    moderator(&mut tx, caller).await?;
    let mut out = BatchOutcome::default();
    for (raw, id) in wanted {
        let Some(id) = id else {
            out.failed
                .push(crate::moderation::batch_failure(&raw, &not_found()));
            continue;
        };
        let Some(done) =
            reports::resolve(&mut tx, id, "dismissed", caller.account_id, note.as_deref())
                .await
                .map_err(db_error)?
        else {
            let err = match reports::get(&mut tx, id).await.map_err(db_error)? {
                Some(_) => conflict("not_open", "That report was already resolved."),
                None => not_found(),
            };
            out.failed
                .push(crate::moderation::batch_failure(&raw, &err));
            continue;
        };
        record_audit(
            &mut tx,
            caller.account_id,
            Audit::new("report.dismissed", "report", &ids::to_text(id)).change(
                None,
                Some(
                    json!({ "emote_id": ids::to_text(done.emote_id), "note": note, "batch": true }),
                ),
            ),
        )
        .await?;
        out.done.push(ids::to_text(id));
    }
    tx.commit().await.map_err(sqlx_error)?;
    Ok(out)
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
    require_any(&mut tx, caller, &[permissions::EMOTES_TAKEDOWN]).await?;
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
    record_audit(
        &mut tx,
        caller.account_id,
        Audit::new("emote.taken_down", "emote", &ids::to_text(id)).change(
            Some(json!({ "state": target.state })),
            Some(json!({
                "state": EmoteState::TakenDown,
                "reason": reason,
                "emotes": affected_ids.iter().map(|e| ids::to_text(*e)).collect::<Vec<_>>(),
            })),
        ),
    )
    .await?;
    tx.commit().await.map_err(sqlx_error)?;
    emote_view(state, &mut conn, id)
        .await?
        .ok_or_else(not_found)
}

// Restrictions

/// The longest a timed restriction may run.
pub const MAX_RESTRICTION_DAYS: i64 = 3650;

fn hold_of(kind: RestrictionKind) -> Hold {
    match kind {
        RestrictionKind::Uploads => Hold::Uploads,
        RestrictionKind::Tickets => Hold::Tickets,
        RestrictionKind::Account => Hold::Account,
    }
}

fn kind_of(hold: Hold) -> RestrictionKind {
    match hold {
        Hold::Uploads => RestrictionKind::Uploads,
        Hold::Tickets => RestrictionKind::Tickets,
        Hold::Account => RestrictionKind::Account,
    }
}

/// The permission that places or lifts `hold`, besides `users.restrict` (which does all three).
fn permission_for(hold: Hold) -> &'static str {
    match hold {
        Hold::Uploads => permissions::USERS_RESTRICT_UPLOADS,
        Hold::Tickets => permissions::USERS_RESTRICT_TICKETS,
        Hold::Account => permissions::USERS_BLOCK,
    }
}

/// Every permission that can see an account's restrictions.
const SEE_RESTRICTIONS: [&str; 5] = [
    permissions::USERS_VIEW,
    permissions::USERS_RESTRICT,
    permissions::USERS_RESTRICT_UPLOADS,
    permissions::USERS_RESTRICT_TICKETS,
    permissions::USERS_BLOCK,
];

async fn sees_restrictions(conn: &mut PgConnection, caller: &Caller) -> ApiResult<bool> {
    let held = crate::catalog::permissions_of(conn, caller).await?;
    Ok(caller.owner || SEE_RESTRICTIONS.iter().any(|p| held.iter().any(|h| h == p)))
}

pub(crate) fn status_of(account: &accounts::Account) -> AccountStatus {
    AccountStatus {
        id: ids::to_text(account.id),
        uploads_blocked: account.restricted_at.is_some(),
        blocked: account.blocked_at.is_some(),
        tickets_blocked: account.tickets_blocked_at.is_some(),
        uploads_blocked_until: account.restricted_until.map(views::time),
        blocked_until: account.blocked_until.map(views::time),
        tickets_blocked_until: account.tickets_blocked_until.map(views::time),
    }
}

/// A restriction's end: in the future and at most ten years out, or `None` for good.
fn until(raw: Option<&str>) -> ApiResult<Option<chrono::DateTime<Utc>>> {
    let Some(raw) = raw.map(str::trim).filter(|s| !s.is_empty()) else {
        return Ok(None);
    };
    let at = chrono::DateTime::parse_from_rfc3339(raw)
        .map(|t| t.with_timezone(&Utc))
        .map_err(|_| {
            bad(
                "bad_time",
                "until must be a time like 2026-11-17T00:00:00Z.",
            )
        })?;
    let now = Utc::now();
    if at <= now {
        return Err(bad("bad_time", "The end has to be in the future."));
    }
    if at > now + chrono::Duration::days(MAX_RESTRICTION_DAYS) {
        return Err(bad(
            "bad_time",
            "At most ten years; leave the end empty for good.",
        ));
    }
    Ok(Some(at))
}

/// Who may restrict `account_id` for `holds`: the owner can't be, nobody acts on their own account
/// or one at or above their own highest role, and each hold needs its permission (or
/// `users.restrict`).
async fn may_restrict(
    state: &AppState,
    conn: &mut PgConnection,
    caller: &Caller,
    account_id: Uuid,
    holds_wanted: &[Hold],
) -> ApiResult<accounts::Account> {
    let held = crate::catalog::permissions_of(conn, caller).await?;
    let has = |p: &str| caller.owner || held.iter().any(|h| h == p);
    if holds_wanted.is_empty() && !has(permissions::USERS_RESTRICT) {
        return Err(forbidden("This needs users.restrict."));
    }
    for hold in holds_wanted {
        if !has(permissions::USERS_RESTRICT) && !has(permission_for(*hold)) {
            return Err(forbidden(&format!(
                "This needs users.restrict or {}.",
                permission_for(*hold)
            )));
        }
    }
    let before = accounts::get(conn, account_id)
        .await
        .map_err(db_error)?
        .filter(|a| a.deleted_at.is_none())
        .ok_or_else(not_found)?;
    if state
        .auth
        .is_owner(conn, account_id)
        .await
        .map_err(ApiError::from)?
    {
        return Err(ApiError::new(
            StatusCode::FORBIDDEN,
            "owner_protected",
            "The owner's account can't be restricted.",
        ));
    }
    // Nobody places, changes or lifts their own holds: lifting one would undo what someone
    // more senior decided.
    if account_id == caller.account_id {
        return Err(ApiError::new(
            StatusCode::FORBIDDEN,
            "own_account",
            "You can't change restrictions on your own account.",
        ));
    }
    let target_highest = cyex_tv_db::roles::highest_position(conn, account_id)
        .await
        .map_err(db_error)?;
    actor(conn, caller)
        .await?
        .may_act_on(target_highest)
        .map_err(rule)?;
    Ok(before)
}

fn snapshot(a: &accounts::Account) -> Value {
    json!({
        "uploads_blocked": a.restricted_at.is_some(),
        "uploads_blocked_until": a.restricted_until.map(views::time),
        "tickets_blocked": a.tickets_blocked_at.is_some(),
        "tickets_blocked_until": a.tickets_blocked_until.map(views::time),
        "blocked": a.blocked_at.is_some(),
        "blocked_until": a.blocked_until.map(views::time),
    })
}

/// The older switch: block or unblock uploads and the whole account, for good and without a
/// reason. Kept as it was; each change is also recorded in the account's restrictions.
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
    let wanted: Vec<Hold> = [
        uploads_blocked.map(|_| Hold::Uploads),
        blocked.map(|_| Hold::Account),
    ]
    .into_iter()
    .flatten()
    .collect();
    let before = may_restrict(state, &mut tx, caller, account_id, &wanted).await?;
    let mut after = before.clone();
    for (hold, on) in [(Hold::Uploads, uploads_blocked), (Hold::Account, blocked)] {
        let Some(on) = on else { continue };
        after = accounts::set_hold(&mut tx, account_id, hold, on, None)
            .await
            .map_err(db_error)?;
        if on {
            accounts::record_hold(
                &mut tx,
                account_id,
                hold,
                None,
                None,
                Some(caller.account_id),
            )
            .await
            .map_err(db_error)?;
        } else {
            accounts::end_records(&mut tx, account_id, hold, Some(caller.account_id))
                .await
                .map_err(db_error)?;
        }
        if hold == Hold::Account && on {
            sessions::delete_for_account(&mut tx, account_id, None)
                .await
                .map_err(db_error)?;
        }
    }
    record_audit(&mut tx, caller.account_id, Audit::new("account.restricted", "account", &ids::to_text(account_id)).change(Some(json!({ "uploads_blocked": before.restricted_at.is_some(), "blocked": before.blocked_at.is_some() })), Some(json!({ "uploads_blocked": after.restricted_at.is_some(), "blocked": after.blocked_at.is_some() }))))
    .await?;
    tx.commit().await.map_err(sqlx_error)?;
    Ok(status_of(&after))
}

/// Places a restriction with a reason, for good or until a time. One of the same kind that is
/// running is replaced (its end changes; its record is marked lifted). Audited
/// (`account.restriction_placed`).
pub async fn place(
    state: &AppState,
    caller: &Caller,
    account_id: &str,
    input: &RestrictionInput,
) -> ApiResult<AccountStatus> {
    let account_id = parse_id(account_id)?;
    let reason = text(&input.reason, "reason")?;
    let ends = until(input.until.as_deref())?;
    let hold = hold_of(input.kind);
    let mut conn = state.db.acquire().await.map_err(sqlx_error)?;
    let mut tx = conn.begin().await.map_err(sqlx_error)?;
    let before = may_restrict(state, &mut tx, caller, account_id, &[hold]).await?;
    let after = accounts::set_hold(&mut tx, account_id, hold, true, ends)
        .await
        .map_err(db_error)?;
    let record = accounts::record_hold(
        &mut tx,
        account_id,
        hold,
        Some(&reason),
        ends,
        Some(caller.account_id),
    )
    .await
    .map_err(db_error)?;
    if hold == Hold::Account {
        sessions::delete_for_account(&mut tx, account_id, None)
            .await
            .map_err(db_error)?;
    }
    let mut after_snap = snapshot(&after);
    after_snap["kind"] = json!(hold.kind());
    after_snap["reason"] = json!(reason);
    after_snap["until"] = json!(ends.map(views::time));
    after_snap["restriction_id"] = json!(ids::to_text(record.id));
    record_audit(
        &mut tx,
        caller.account_id,
        Audit::new(
            "account.restriction_placed",
            "account",
            &ids::to_text(account_id),
        )
        .change(Some(snapshot(&before)), Some(after_snap)),
    )
    .await?;
    tx.commit().await.map_err(sqlx_error)?;
    Ok(status_of(&after))
}

/// Lifts a restriction early. Lifting one that isn't on changes nothing (and isn't audited).
pub async fn lift(
    state: &AppState,
    caller: &Caller,
    account_id: &str,
    kind: RestrictionKind,
    reason: Option<&str>,
) -> ApiResult<AccountStatus> {
    let account_id = parse_id(account_id)?;
    let reason = reason
        .map(str::trim)
        .filter(|r| !r.is_empty())
        .map(|r| text(r, "reason"))
        .transpose()?;
    let hold = hold_of(kind);
    let mut conn = state.db.acquire().await.map_err(sqlx_error)?;
    let mut tx = conn.begin().await.map_err(sqlx_error)?;
    let before = may_restrict(state, &mut tx, caller, account_id, &[hold]).await?;
    if hold.of(&before).0.is_none() {
        return Ok(status_of(&before));
    }
    let after = accounts::set_hold(&mut tx, account_id, hold, false, None)
        .await
        .map_err(db_error)?;
    accounts::end_records(&mut tx, account_id, hold, Some(caller.account_id))
        .await
        .map_err(db_error)?;
    let mut after_snap = snapshot(&after);
    after_snap["kind"] = json!(hold.kind());
    after_snap["reason"] = json!(reason);
    record_audit(
        &mut tx,
        caller.account_id,
        Audit::new(
            "account.restriction_lifted",
            "account",
            &ids::to_text(account_id),
        )
        .change(Some(snapshot(&before)), Some(after_snap)),
    )
    .await?;
    tx.commit().await.map_err(sqlx_error)?;
    Ok(status_of(&after))
}

/// Every restriction placed on an account, newest first (the latest 100), running or over.
/// Needs `users.view`, `users.restrict` or one of the narrower restrict permissions.
pub async fn restrictions(
    state: &AppState,
    caller: &Caller,
    account_id: &str,
) -> ApiResult<Vec<Restriction>> {
    let account_id = parse_id(account_id)?;
    let mut conn = state.db.acquire().await.map_err(sqlx_error)?;
    if !sees_restrictions(&mut conn, caller).await? {
        return Err(forbidden(
            "This needs users.view or a users.restrict permission.",
        ));
    }
    let account = accounts::get(&mut conn, account_id)
        .await
        .map_err(db_error)?
        .ok_or_else(not_found)?;
    let rows = accounts::restrictions(&mut conn, account_id, 100)
        .await
        .map_err(db_error)?;
    let now = Utc::now();
    Ok(rows
        .into_iter()
        .filter_map(|r| {
            let hold = Hold::from_kind(&r.kind)?;
            // Running: not lifted, not past its end, and the account still holds that kind (the
            // record of an older switch-off may predate this table).
            let active = r.lifted_at.is_none()
                && r.ends_at.is_none_or(|e| e > now)
                && hold.of(&account).0.is_some();
            Some(Restriction {
                id: ids::to_text(r.id),
                kind: kind_of(hold),
                reason: r.reason,
                ends_at: r.ends_at.map(views::time),
                created_by: r.created_by.map(ids::to_text),
                created_at: views::time(r.created_at),
                lifted_at: r.lifted_at.map(views::time),
                lifted_by: r.lifted_by.map(ids::to_text),
                active,
            })
        })
        .collect())
}

/// What an account may do now. Needs `users.view`, `users.restrict` or one of the narrower
/// restrict permissions.
pub async fn status(
    state: &AppState,
    caller: &Caller,
    account_id: &str,
) -> ApiResult<AccountStatus> {
    let account_id = parse_id(account_id)?;
    let mut conn = state.db.acquire().await.map_err(sqlx_error)?;
    if !sees_restrictions(&mut conn, caller).await? {
        return Err(forbidden("This needs users.view or users.restrict."));
    }
    let account = accounts::get(&mut conn, account_id)
        .await
        .map_err(db_error)?
        .filter(|a| a.deleted_at.is_none())
        .ok_or_else(not_found)?;
    Ok(status_of(&account))
}

/// Reports about an account's emotes and reports it filed. For moderators.
pub async fn account_reports(
    state: &AppState,
    caller: &Caller,
    account_id: &str,
) -> ApiResult<AccountReports> {
    let account_id = parse_id(account_id)?;
    let mut conn = state.db.acquire().await.map_err(sqlx_error)?;
    moderator(&mut conn, caller).await?;
    let about = reports::about_owner(&mut conn, account_id, 50)
        .await
        .map_err(db_error)?;
    let by = reports::by_reporter(&mut conn, account_id, 50)
        .await
        .map_err(db_error)?;
    Ok(AccountReports {
        about: report_views(state, &mut conn, &about).await?,
        by: report_views(state, &mut conn, &by).await?,
    })
}

/// An account with its identities and restrictions. Needs `users.view` or `users.restrict`.
pub async fn admin_account(
    state: &AppState,
    caller: &Caller,
    account_id: &str,
) -> ApiResult<AdminAccount> {
    let account_id = parse_id(account_id)?;
    let mut conn = state.db.acquire().await.map_err(sqlx_error)?;
    if !sees_restrictions(&mut conn, caller).await? {
        return Err(forbidden("This needs users.view or users.restrict."));
    }
    let account = accounts::get(&mut conn, account_id)
        .await
        .map_err(db_error)?
        .filter(|a| a.deleted_at.is_none())
        .ok_or_else(not_found)?;
    let identities: Vec<_> = accounts::identities(&mut conn, account_id)
        .await
        .map_err(db_error)?
        .into_iter()
        .map(|i| cyex_tv_shared::model::Identity {
            platform: i.platform,
            platform_user_id: i.platform_user_id,
            login: i.login,
            display_name: i.display_name,
            avatar: i.avatar,
        })
        .collect();
    Ok(AdminAccount {
        id: ids::to_text(account_id),
        display_name: identities
            .first()
            .map(|i| i.display_name.clone())
            .unwrap_or_default(),
        avatar: identities.iter().find_map(|i| i.avatar.clone()),
        identities,
        created_at: views::time(account.created_at),
        uploads_blocked: account.restricted_at.is_some(),
        blocked: account.blocked_at.is_some(),
        tickets_blocked: account.tickets_blocked_at.is_some(),
        uploads_blocked_until: account.restricted_until.map(views::time),
        blocked_until: account.blocked_until.map(views::time),
        tickets_blocked_until: account.tickets_blocked_until.map(views::time),
    })
}

// Notifications

pub async fn my_notifications(
    state: &AppState,
    caller: &Caller,
    unread_only: bool,
) -> ApiResult<Vec<Notification>> {
    let mut conn = state.db.acquire().await.map_err(sqlx_error)?;
    let rows = notifications::list(&mut conn, caller.account_id, unread_only)
        .await
        .map_err(db_error)?;
    let request_of = |n: &notifications::Notification| {
        (n.kind == "emote.transfer_requested")
            .then(|| {
                n.payload
                    .get("request_id")?
                    .as_str()
                    .and_then(ids::from_text)
            })
            .flatten()
    };
    let wanted: Vec<Uuid> = rows.iter().filter_map(request_of).collect();
    let states: std::collections::HashMap<Uuid, TransferRequestState> = if wanted.is_empty() {
        std::collections::HashMap::new()
    } else {
        cyex_tv_db::transfers::states_of(&mut conn, &wanted)
            .await
            .map_err(db_error)?
            .into_iter()
            .map(|(id, s)| (id, TransferRequestState::from(s)))
            .collect()
    };
    Ok(rows
        .into_iter()
        .map(|n| Notification {
            transfer_state: request_of(&n).and_then(|id| states.get(&id).copied()),
            id: ids::to_text(n.id),
            kind: n.kind,
            payload: n.payload.to_string(),
            created_at: views::time(n.created_at),
            read: n.read_at.is_some(),
        })
        .collect())
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
    fn restriction_ends_are_bounded() {
        assert_eq!(until(None).unwrap(), None);
        assert_eq!(until(Some("  ")).unwrap(), None);
        assert_eq!(
            until(Some("2001-01-01T00:00:00Z")).unwrap_err().code,
            "bad_time"
        );
        assert_eq!(until(Some("tomorrow")).unwrap_err().code, "bad_time");
        let soon = (Utc::now() + chrono::Duration::hours(1)).to_rfc3339();
        assert!(until(Some(&soon)).unwrap().is_some());
        let far = (Utc::now() + chrono::Duration::days(MAX_RESTRICTION_DAYS + 1)).to_rfc3339();
        assert!(until(Some(&far)).is_err());
    }

    #[test]
    fn each_hold_has_its_permission_and_kind() {
        for hold in Hold::ALL {
            assert_eq!(hold_of(kind_of(hold)), hold);
            assert!(permissions::is_known(permission_for(hold)));
        }
    }

    #[test]
    fn report_states_read_back() {
        assert_eq!(state_of("open"), ReportState::Open);
        assert_eq!(state_of("dismissed"), ReportState::Dismissed);
        assert_eq!(state_of("actioned"), ReportState::Actioned);
    }
}
