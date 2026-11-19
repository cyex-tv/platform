//! Emotes after upload: reading one, the owner's edits, deleting, and the review gate (D10).
//!
//! Who sees an emote that isn't approved: its owner and `emotes.review` holders. Deleted emotes
//! are gone for everyone. Approving or rejecting needs `emotes.review` and works only on pending
//! emotes; both are audited, and so is any edit made through `emotes.edit_any`.
//!
//! Account editors (ADR 0014) with `emote_manage` on every set see, change and delete the owner's
//! emotes as the owner does; with `emote_transfer` they give them to another account
//! (`transfer`). Each such change is audited with the editor as actor, `"via": "account_editor"`
//! and the owner as subject. A hold on uploads of the editor or the owner refuses them.

use std::collections::BTreeSet;

use axum::http::StatusCode;
use cyex_tv_auth::Caller;
use cyex_tv_db::emotes::{self, EmotePatch};
use cyex_tv_db::{DbError, accounts, audit, ids, reads, sets, staff};
use cyex_tv_shared::error::{ApiError, ApiResult};
use cyex_tv_shared::model::{
    AuditEntry, EditorPermission, Emote, EmoteNote, EmotePage, EmoteState, EmoteVisibility,
    EventType, ReviewCounts, ReviewQueue, ReviewQueueFilter, ReviewSort, ReviewTab, UpdateEmote,
};
use cyex_tv_shared::permissions;
use cyex_tv_shared::text;
use serde_json::{Value, json};
use sqlx::{Connection, PgConnection};
use uuid::Uuid;

use crate::catalog::{
    db_error, forbidden, holds, load_set, member_name, not_found, parse_id, permissions_of,
    publish_member_event,
};
use crate::state::AppState;
use crate::views;

/// Tags per emote, and their length.
pub const MAX_TAGS: usize = 10;
pub const MAX_TAG_LEN: usize = 32;
pub const MAX_REASON_LEN: usize = 500;
/// Review queue page size: default and most.
pub const QUEUE_PAGE: u32 = 50;
pub const QUEUE_PAGE_MAX: u32 = 100;
/// The longest review queue search, and the longest emote note.
pub const MAX_QUEUE_SEARCH: usize = 100;
pub const MAX_NOTE_LEN: usize = 2000;
/// Notes and history entries one read returns at most.
const NOTES_LIMIT: i64 = 200;
const HISTORY_LIMIT: i64 = 100;

fn bad(code: &'static str, message: &str) -> ApiError {
    ApiError::new(StatusCode::BAD_REQUEST, code, message)
}

fn sqlx_error(err: sqlx::Error) -> ApiError {
    db_error(DbError::Sqlx(err))
}

/// Tags: lowercase, trimmed, 1 to 32 letters, digits, `-` or `_`; at most 10; duplicates dropped.
pub fn clean_tags(raw: &[String]) -> ApiResult<Vec<String>> {
    let mut out = BTreeSet::new();
    for tag in raw {
        let tag = tag.trim().to_lowercase();
        let len = tag.chars().count();
        if len == 0
            || len > MAX_TAG_LEN
            || !tag
                .chars()
                .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
        {
            return Err(bad("bad_tag", "Tags are 1 to 32 letters, digits, - or _."));
        }
        out.insert(tag);
    }
    if out.len() > MAX_TAGS {
        return Err(bad("too_many_tags", "At most 10 tags."));
    }
    Ok(out.into_iter().collect())
}

fn reason(raw: &str) -> ApiResult<String> {
    let text = raw.trim();
    let len = text.chars().count();
    if len == 0 || len > MAX_REASON_LEN || text::has_unsafe(text, true) {
        return Err(bad(
            "bad_reason",
            "Give a reason of 1 to 500 characters; the uploader sees it.",
        ));
    }
    Ok(text.to_owned())
}

pub(crate) async fn row(conn: &mut PgConnection, id: Uuid) -> ApiResult<Option<reads::EmoteRow>> {
    Ok(reads::emotes(conn, &[id])
        .await
        .map_err(db_error)?
        .into_iter()
        .next())
}

pub(crate) async fn build(
    state: &AppState,
    conn: &mut PgConnection,
    row: &reads::EmoteRow,
) -> ApiResult<Emote> {
    let mut built = views::emotes(conn, std::slice::from_ref(row), &state.cdn_url)
        .await
        .map_err(db_error)?;
    Ok(built.remove(0))
}

/// Tells every set holding the emote (and the channels and users showing those sets).
pub(crate) async fn announce(
    conn: &mut PgConnection,
    emote_id: Uuid,
    event_type: EventType,
) -> ApiResult<()> {
    for set_id in emotes::sets_containing(conn, emote_id)
        .await
        .map_err(db_error)?
    {
        let set = load_set(conn, set_id).await?;
        publish_member_event(conn, &set, event_type, emote_id, None).await?;
    }
    Ok(())
}

async fn write_audit(
    conn: &mut PgConnection,
    caller: &Caller,
    action: &str,
    emote_id: Uuid,
    before: Option<Value>,
    after: Option<Value>,
) -> ApiResult<()> {
    audit::write(
        conn,
        &audit::NewEntry {
            actor: Some(caller.account_id),
            action,
            target_kind: "emote",
            target_id: &ids::to_text(emote_id),
            before,
            after,
        },
    )
    .await
    .map_err(db_error)?;
    Ok(())
}

/// `write_audit` for a change an account editor made on the owner's emote: marked
/// `"via": "account_editor"` and shown to the owner as theirs (the subject).
async fn write_editor_audit(
    conn: &mut PgConnection,
    caller: &Caller,
    action: &str,
    emote_id: Uuid,
    owner: Uuid,
    before: Option<Value>,
    after: Option<Value>,
) -> ApiResult<()> {
    let mut after = after.unwrap_or_else(|| json!({}));
    after["via"] = json!("account_editor");
    audit::write_for(
        conn,
        &audit::NewEntry {
            actor: Some(caller.account_id),
            action,
            target_kind: "emote",
            target_id: &ids::to_text(emote_id),
            before,
            after: Some(after),
        },
        Some(owner),
    )
    .await
    .map_err(db_error)?;
    Ok(())
}

/// True if the caller is an account editor of the emote's owner with `flag` on every set.
pub(crate) async fn editor_of_owner(
    conn: &mut PgConnection,
    caller: &Caller,
    owner: Option<Uuid>,
    flag: EditorPermission,
) -> ApiResult<bool> {
    match owner {
        Some(owner) if owner != caller.account_id => {
            crate::editors::has_emote_flag(conn, owner, caller.account_id, flag).await
        }
        _ => Ok(false),
    }
}

pub(crate) fn restricted(message: &str) -> ApiError {
    ApiError::new(StatusCode::FORBIDDEN, "restricted", message)
}

/// Refuses while uploads of `account` are held (#87): the account can't change emotes, and
/// nobody can change them for it.
pub(crate) async fn not_restricted(
    conn: &mut PgConnection,
    account: Uuid,
    message: &str,
) -> ApiResult<()> {
    let held = accounts::get(conn, account)
        .await
        .map_err(db_error)?
        .is_none_or(|a| a.restricted_at.is_some());
    if held {
        Err(restricted(message))
    } else {
        Ok(())
    }
}

pub(crate) const YOU_RESTRICTED: &str = "Your account can't change emotes right now.";
pub(crate) const OWNER_RESTRICTED: &str =
    "That account can't change emotes right now, so nobody can change them for it.";

// Reads

/// Whether the emote's owner is an account whose deletion is pending (ADR 0019): its emotes are
/// hidden with it. Emotes of erased accounts have no owner, so this is only ever the pending ones.
pub(crate) async fn owner_hidden(conn: &mut PgConnection, owner: Option<Uuid>) -> ApiResult<bool> {
    let Some(owner) = owner else {
        return Ok(false);
    };
    Ok(accounts::get(conn, owner)
        .await
        .map_err(db_error)?
        .is_some_and(|a| a.deleted_at.is_some()))
}

/// An emote as `caller` may see it. An emote of an account whose deletion is pending is hidden
/// from everyone but moderators.
pub async fn emote(
    state: &AppState,
    caller: Option<&Caller>,
    id: &str,
) -> ApiResult<Option<Emote>> {
    let Some(id) = ids::from_text(id.trim()) else {
        return Ok(None);
    };
    let mut conn = state.db.acquire().await.map_err(sqlx_error)?;
    let Some(found) = row(&mut conn, id).await? else {
        return Ok(None);
    };
    if found.state != EmoteState::Approved {
        let allowed = match caller {
            None => false,
            Some(c) if found.owner == Some(c.account_id) => true,
            Some(c) => {
                holds(&mut conn, c, permissions::EMOTES_REVIEW).await?
                    || editor_of_owner(&mut conn, c, found.owner, EditorPermission::EmoteManage)
                        .await?
            }
        };
        if !allowed {
            return Ok(None);
        }
    }
    if owner_hidden(&mut conn, found.owner).await? {
        let staff = match caller {
            Some(c) => holds(&mut conn, c, permissions::EMOTES_REVIEW).await?,
            None => false,
        };
        if !staff {
            return Ok(None);
        }
    }
    build(state, &mut conn, &found).await.map(Some)
}

/// The review queue: pending emotes (all, public or unlisted and private) or the resolved ones,
/// searched by name or uploader and sorted. Needs `emotes.review`.
pub async fn review_queue(
    state: &AppState,
    caller: &Caller,
    page: Option<u32>,
    per_page: Option<u32>,
    filter: Option<&ReviewQueueFilter>,
) -> ApiResult<ReviewQueue> {
    let mut conn = state.db.acquire().await.map_err(sqlx_error)?;
    if !holds(&mut conn, caller, permissions::EMOTES_REVIEW).await? {
        return Err(forbidden("This needs emotes.review."));
    }
    let per_page = per_page.unwrap_or(QUEUE_PAGE).clamp(1, QUEUE_PAGE_MAX);
    let page = page.unwrap_or(1).max(1);
    let offset = i64::from(page - 1) * i64::from(per_page);
    let query = QueueRead::from_filter(filter.cloned().unwrap_or_default())?;
    let (rows, total) = reads::review_queue(
        &mut conn,
        &reads::QueueQuery {
            tab: query.tab,
            search: query.search.as_deref(),
            sort: query.sort,
            descending: query.descending,
        },
        i64::from(per_page),
        offset,
    )
    .await
    .map_err(db_error)?;
    let counts = reads::queue_counts(&mut conn).await.map_err(db_error)?;
    Ok(ReviewQueue {
        items: views::emotes(&mut conn, &rows, &state.cdn_url)
            .await
            .map_err(db_error)?,
        total: u64::try_from(total).unwrap_or(0),
        counts: ReviewCounts {
            public: u64::try_from(counts.public).unwrap_or(0),
            limited: u64::try_from(counts.limited).unwrap_or(0),
        },
    })
}

/// A review queue read with the defaults filled in.
#[derive(Debug, PartialEq, Eq)]
struct QueueRead {
    tab: reads::QueueTab,
    search: Option<String>,
    sort: reads::QueueSort,
    descending: bool,
}

impl QueueRead {
    fn from_filter(f: ReviewQueueFilter) -> ApiResult<Self> {
        let tab = f.tab.unwrap_or_default();
        let search = f
            .search
            .map(|s| s.trim().to_owned())
            .filter(|s| !s.is_empty());
        if search
            .as_deref()
            .is_some_and(|s| s.chars().count() > MAX_QUEUE_SEARCH || text::has_unsafe(s, false))
        {
            return Err(bad("bad_search", "Search for at most 100 characters."));
        }
        let resolved = tab == ReviewTab::Resolved;
        let sort = f.sort.unwrap_or(if resolved {
            ReviewSort::Reviewed
        } else {
            ReviewSort::Uploaded
        });
        Ok(Self {
            tab: match tab {
                ReviewTab::Pending => reads::QueueTab::Pending,
                ReviewTab::Public => reads::QueueTab::Public,
                ReviewTab::Limited => reads::QueueTab::Limited,
                ReviewTab::Resolved => reads::QueueTab::Resolved,
            },
            search,
            sort: match sort {
                ReviewSort::Uploaded => reads::QueueSort::Uploaded,
                ReviewSort::Reviewed => reads::QueueSort::Reviewed,
                ReviewSort::Uploader => reads::QueueSort::Uploader,
                ReviewSort::Name => reads::QueueSort::Name,
            },
            descending: f.descending.unwrap_or(resolved),
        })
    }
}

/// An account's emotes in every state, newest first. For `users.view`, moderators, the account
/// itself, and its editors with `emote_manage` on every set.
pub async fn account_emotes(
    state: &AppState,
    caller: &Caller,
    account_id: &str,
    page: Option<u32>,
    per_page: Option<u32>,
) -> ApiResult<EmotePage> {
    let owner = parse_id(account_id)?;
    let mut conn = state.db.acquire().await.map_err(sqlx_error)?;
    let held = permissions_of(&mut conn, caller).await?;
    let may = [
        permissions::USERS_VIEW,
        permissions::EMOTES_REVIEW,
        permissions::EMOTES_TAKEDOWN,
        permissions::EMOTES_EDIT_ANY,
        permissions::EMOTES_DELETE_ANY,
    ];
    if !held.iter().any(|p| may.contains(&p.as_str()))
        && owner != caller.account_id
        && !crate::editors::has_emote_flag(
            &mut conn,
            owner,
            caller.account_id,
            EditorPermission::EmoteManage,
        )
        .await?
    {
        return Err(forbidden(
            "This needs users.view or emotes.review, or the account's emote manage permission.",
        ));
    }
    let per_page = per_page.unwrap_or(QUEUE_PAGE).clamp(1, QUEUE_PAGE_MAX);
    let page = page.unwrap_or(1).max(1);
    let offset = i64::from(page - 1) * i64::from(per_page);
    let (rows, total) = reads::emotes_by_owner(&mut conn, owner, i64::from(per_page), offset)
        .await
        .map_err(db_error)?;
    Ok(EmotePage {
        items: views::emotes(&mut conn, &rows, &state.cdn_url)
            .await
            .map_err(db_error)?,
        total: u64::try_from(total).unwrap_or(0),
    })
}

// Writes

async fn review(
    state: &AppState,
    caller: &Caller,
    id: &str,
    to: EmoteState,
    why: Option<&str>,
    visibility: Option<EmoteVisibility>,
) -> ApiResult<Emote> {
    let id = parse_id(id)?;
    let why = why.map(reason).transpose()?;
    let mut conn = state.db.acquire().await.map_err(sqlx_error)?;
    let mut tx = conn.begin().await.map_err(sqlx_error)?;
    if !holds(&mut tx, caller, permissions::EMOTES_REVIEW).await? {
        return Err(forbidden("This needs emotes.review."));
    }
    let before = row(&mut tx, id).await?.ok_or_else(not_found)?;
    if emotes::review(&mut tx, id, to, caller.account_id, why.as_deref())
        .await
        .map_err(db_error)?
        .is_none()
    {
        return Err(ApiError::new(
            StatusCode::CONFLICT,
            "not_pending",
            "Only pending emotes can be approved or rejected.",
        ));
    }
    // Approving as unlisted or private (or public again) sets the visibility in the same step.
    let was = EmoteVisibility::of(before.private, before.unlisted);
    let now = visibility.unwrap_or(was);
    if now != was {
        let (private, unlisted) = now.flags();
        emotes::update(
            &mut tx,
            id,
            &EmotePatch {
                private: Some(private),
                unlisted: Some(unlisted),
                ..EmotePatch::default()
            },
        )
        .await
        .map_err(db_error)?;
    }
    let action = match to {
        EmoteState::Approved => "emote.approved",
        _ => "emote.rejected",
    };
    // An approval is on the owner's public activity feed (as "uploaded"); the reviewer and the
    // reason are not.
    audit::write_for(
        &mut tx,
        &audit::NewEntry {
            actor: Some(caller.account_id),
            action,
            target_kind: "emote",
            target_id: &ids::to_text(id),
            before: Some(json!({ "state": before.state, "visibility": was })),
            after: Some(json!({ "state": to, "reason": why, "visibility": now })),
        },
        before.owner.filter(|_| to == EmoteState::Approved),
    )
    .await
    .map_err(db_error)?;
    if let Some(owner) = before.owner {
        let (kind, payload) = match to {
            EmoteState::Approved => (
                "emote.approved",
                json!({ "emote_id": ids::to_text(id), "name": before.name, "visibility": now }),
            ),
            _ => (
                "emote.rejected",
                json!({ "emote_id": ids::to_text(id), "name": before.name, "reason": why }),
            ),
        };
        crate::account::notify(&mut tx, owner, kind, &payload).await?;
    }
    // Sets that held it pending show it now; a rejected one stays hidden, so nothing to tell.
    if to == EmoteState::Approved {
        announce(&mut tx, id, EventType::EmoteUpdated).await?;
    }
    tx.commit().await.map_err(sqlx_error)?;

    let after = row(&mut conn, id).await?.ok_or_else(not_found)?;
    build(state, &mut conn, &after).await
}

/// Approves a pending emote; with `visibility`, also makes it public, unlisted or private.
pub async fn approve(
    state: &AppState,
    caller: &Caller,
    id: &str,
    visibility: Option<EmoteVisibility>,
) -> ApiResult<Emote> {
    review(state, caller, id, EmoteState::Approved, None, visibility).await
}

pub async fn reject(state: &AppState, caller: &Caller, id: &str, why: &str) -> ApiResult<Emote> {
    review(state, caller, id, EmoteState::Rejected, Some(why), None).await
}

// Review notes and history

fn note_body(raw: &str) -> ApiResult<String> {
    let body = raw.trim();
    let len = body.chars().count();
    if len == 0 || len > MAX_NOTE_LEN || text::has_unsafe(body, true) {
        return Err(bad("bad_note", "Notes are 1 to 2000 characters."));
    }
    Ok(body.to_owned())
}

fn note_view(n: &staff::EmoteNote) -> EmoteNote {
    EmoteNote {
        id: ids::to_text(n.id),
        emote_id: ids::to_text(n.emote_id),
        author_id: n.author_id.map(ids::to_text),
        body: n.body.clone(),
        created_at: views::time(n.created_at),
    }
}

async fn require_review(conn: &mut PgConnection, caller: &Caller) -> ApiResult<()> {
    if holds(conn, caller, permissions::EMOTES_REVIEW).await? {
        Ok(())
    } else {
        Err(forbidden("This needs emotes.review."))
    }
}

/// Reviewers' notes on an emote, newest first (the latest 200). Needs `emotes.review`.
pub async fn notes(state: &AppState, caller: &Caller, emote_id: &str) -> ApiResult<Vec<EmoteNote>> {
    let emote_id = parse_id(emote_id)?;
    let mut conn = state.db.acquire().await.map_err(sqlx_error)?;
    require_review(&mut conn, caller).await?;
    Ok(staff::emote_notes(&mut conn, emote_id, NOTES_LIMIT)
        .await
        .map_err(db_error)?
        .iter()
        .map(note_view)
        .collect())
}

/// Adds a note to an emote. Needs `emotes.review`. Audited.
pub async fn add_note(
    state: &AppState,
    caller: &Caller,
    emote_id: &str,
    body: &str,
) -> ApiResult<EmoteNote> {
    let emote_id = parse_id(emote_id)?;
    let body = note_body(body)?;
    let mut conn = state.db.acquire().await.map_err(sqlx_error)?;
    let mut tx = conn.begin().await.map_err(sqlx_error)?;
    require_review(&mut tx, caller).await?;
    row(&mut tx, emote_id).await?.ok_or_else(not_found)?;
    let note = staff::add_emote_note(&mut tx, emote_id, caller.account_id, &body)
        .await
        .map_err(db_error)?;
    write_audit(
        &mut tx,
        caller,
        "emote.note_added",
        emote_id,
        None,
        Some(json!({ "note_id": ids::to_text(note.id), "body": body })),
    )
    .await?;
    tx.commit().await.map_err(sqlx_error)?;
    Ok(note_view(&note))
}

/// Deletes an emote note: its author may, and the owner. False if it was gone. Audited.
pub async fn delete_note(state: &AppState, caller: &Caller, id: &str) -> ApiResult<bool> {
    let id = parse_id(id)?;
    let mut conn = state.db.acquire().await.map_err(sqlx_error)?;
    let mut tx = conn.begin().await.map_err(sqlx_error)?;
    require_review(&mut tx, caller).await?;
    let Some(note) = staff::emote_note(&mut tx, id).await.map_err(db_error)? else {
        return Ok(false);
    };
    if note.author_id != Some(caller.account_id) && !caller.owner {
        return Err(forbidden("Only the note's author can delete it."));
    }
    let done = staff::delete_emote_note(&mut tx, id, caller.account_id)
        .await
        .map_err(db_error)?;
    if done {
        write_audit(
            &mut tx,
            caller,
            "emote.note_deleted",
            note.emote_id,
            Some(json!({ "note_id": ids::to_text(note.id), "body": note.body })),
            None,
        )
        .await?;
    }
    tx.commit().await.map_err(sqlx_error)?;
    Ok(done)
}

/// What happened to one emote, newest first (the latest 100 audit entries: upload, edits,
/// reviews, notes, takedowns). Needs `emotes.review`.
pub async fn history(
    state: &AppState,
    caller: &Caller,
    emote_id: &str,
) -> ApiResult<Vec<AuditEntry>> {
    let emote_id = parse_id(emote_id)?;
    let key = ids::to_text(emote_id);
    let mut conn = state.db.acquire().await.map_err(sqlx_error)?;
    require_review(&mut conn, caller).await?;
    let entries = audit::list(
        &mut conn,
        &audit::Filter {
            actor: None,
            target_kind: Some("emote"),
            target_id: Some(&key),
            action: None,
            since: None,
            until: None,
            before_id: None,
        },
        HISTORY_LIMIT,
    )
    .await
    .map_err(db_error)?;
    Ok(entries.into_iter().map(crate::admin::audit_view).collect())
}

/// The owner, an account editor with `emote_manage` (audited, `"via": "account_editor"`), or
/// `emotes.edit_any` (audited). A restricted owner can't edit, nor can a restricted editor, nor
/// anyone for a restricted owner. When an owner or editor without `emotes.review` or
/// `emotes.edit_any` changes the name or tags of an approved emote, it goes back to review
/// (audited as `emote.resubmitted`); flags alone don't.
pub async fn update(
    state: &AppState,
    caller: &Caller,
    id: &str,
    input: &UpdateEmote,
) -> ApiResult<Emote> {
    let id = parse_id(id)?;
    let name = input.name.as_deref().map(member_name).transpose()?;
    let tags = input.tags.as_deref().map(clean_tags).transpose()?;
    let mut conn = state.db.acquire().await.map_err(sqlx_error)?;
    let mut tx = conn.begin().await.map_err(sqlx_error)?;
    let before = row(&mut tx, id).await?.ok_or_else(not_found)?;
    let owner = before.owner == Some(caller.account_id);
    let edit_any = holds(&mut tx, caller, permissions::EMOTES_EDIT_ANY).await?;
    // Staff with `emotes.edit_any` act as staff even when they also edit the account.
    let editor = !owner
        && !edit_any
        && editor_of_owner(&mut tx, caller, before.owner, EditorPermission::EmoteManage).await?;
    if !owner && !edit_any && !editor {
        return Err(forbidden("Only the emote's owner can change it."));
    }
    if owner || editor {
        not_restricted(&mut tx, caller.account_id, YOU_RESTRICTED).await?;
    }
    if editor && let Some(o) = before.owner {
        not_restricted(&mut tx, o, OWNER_RESTRICTED).await?;
    }
    let current = EmoteVisibility::of(before.private, before.unlisted);
    let visibility =
        EmoteVisibility::after(current, input.visibility, input.private).ok_or_else(|| {
            bad(
                "bad_visibility",
                "`visibility` and `private` say different things.",
            )
        })?;
    let (private, unlisted) = visibility.flags();
    let old_tags = emotes::tags(&mut tx, id).await.map_err(db_error)?;
    let renamed = name.is_some_and(|n| n != before.name);
    let retagged = tags.as_ref().is_some_and(|t| {
        t.iter().collect::<BTreeSet<_>>() != old_tags.iter().collect::<BTreeSet<_>>()
    });
    let resubmit = before.state == EmoteState::Approved
        && (renamed || retagged)
        && !edit_any
        && !holds(&mut tx, caller, permissions::EMOTES_REVIEW).await?;
    emotes::update(
        &mut tx,
        id,
        &EmotePatch {
            name,
            zero_width: input.zero_width,
            private: Some(private),
            unlisted: Some(unlisted),
            nsfw: input.nsfw,
        },
    )
    .await
    .map_err(db_error)?;
    if let Some(tags) = &tags {
        emotes::set_tags(&mut tx, id, tags)
            .await
            .map_err(db_error)?;
    }
    if resubmit {
        emotes::resubmit(&mut tx, id).await.map_err(db_error)?;
    }
    let after = row(&mut tx, id).await?.ok_or_else(not_found)?;
    let new_tags = tags.clone().unwrap_or_else(|| old_tags.clone());
    if resubmit {
        write_audit(
            &mut tx,
            caller,
            "emote.resubmitted",
            id,
            Some(json!({ "state": before.state, "name": before.name, "tags": old_tags })),
            Some(json!({ "state": after.state, "name": after.name, "tags": new_tags })),
        )
        .await?;
    }
    let snap = |r: &reads::EmoteRow, tags: &[String]| json!({ "name": r.name, "zero_width": r.zero_width, "private": r.private, "unlisted": r.unlisted, "nsfw": r.nsfw, "tags": tags });
    if editor && let Some(o) = before.owner {
        write_editor_audit(
            &mut tx,
            caller,
            "emote.updated",
            id,
            o,
            Some(snap(&before, &old_tags)),
            Some(snap(&after, &new_tags)),
        )
        .await?;
    } else if !owner {
        write_audit(
            &mut tx,
            caller,
            "emote.updated",
            id,
            Some(snap(&before, &old_tags)),
            Some(snap(&after, &new_tags)),
        )
        .await?;
    }
    // Only emotes the public could see (before or after) are announced: one that went back to
    // review is dropped by clients when they refetch.
    if before.state == EmoteState::Approved || after.state == EmoteState::Approved {
        announce(&mut tx, id, EventType::EmoteUpdated).await?;
    }
    tx.commit().await.map_err(sqlx_error)?;
    build(state, &mut conn, &after).await
}

/// The owner, an account editor with `emote_manage` (audited as `emote.deleted`, `"via":
/// "account_editor"`), or `emotes.delete_any` (audited as `emote.deleted`) deletes an emote: it
/// leaves every set at once. A restricted editor can't, nor anyone for a restricted owner. False
/// if there was none.
pub async fn delete(state: &AppState, caller: &Caller, id: &str) -> ApiResult<bool> {
    let id = parse_id(id)?;
    let mut conn = state.db.acquire().await.map_err(sqlx_error)?;
    let mut tx = conn.begin().await.map_err(sqlx_error)?;
    let Some(found) = row(&mut tx, id).await? else {
        return Ok(false);
    };
    let owner = found.owner == Some(caller.account_id);
    let delete_any = !owner && holds(&mut tx, caller, permissions::EMOTES_DELETE_ANY).await?;
    let editor = !owner
        && !delete_any
        && editor_of_owner(&mut tx, caller, found.owner, EditorPermission::EmoteManage).await?;
    if !owner && !delete_any && !editor {
        return Err(forbidden("Only the emote's owner can delete it."));
    }
    if !owner {
        let tags = emotes::tags(&mut tx, id).await.map_err(db_error)?;
        let snapshot = json!({
            "name": found.name,
            "owner_id": found.owner.map(ids::to_text),
            "state": found.state,
            "private": found.private,
            "unlisted": found.unlisted,
            "tags": tags,
        });
        if editor && let Some(o) = found.owner {
            not_restricted(&mut tx, caller.account_id, YOU_RESTRICTED).await?;
            not_restricted(&mut tx, o, OWNER_RESTRICTED).await?;
            write_editor_audit(
                &mut tx,
                caller,
                "emote.deleted",
                id,
                o,
                Some(snapshot),
                None,
            )
            .await?;
        } else {
            write_audit(
                &mut tx,
                caller,
                "emote.deleted",
                id,
                Some(snapshot),
                Some(json!({ "via": permissions::EMOTES_DELETE_ANY })),
            )
            .await?;
        }
    }
    for (set_id, name) in emotes::remove_from_all_sets(&mut tx, id)
        .await
        .map_err(db_error)?
    {
        let set = sets::get(&mut tx, set_id)
            .await
            .map_err(db_error)?
            .ok_or_else(not_found)?;
        publish_member_event(&mut tx, &set, EventType::SetEmoteRemoved, id, Some(&name)).await?;
    }
    let deleted = emotes::delete(&mut tx, id).await.map_err(db_error)?;
    tx.commit().await.map_err(sqlx_error)?;
    Ok(deleted)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tags_are_cleaned() {
        assert_eq!(
            clean_tags(&[" Pog ".into(), "pog".into(), "café".into(), "a-b_c".into()]).unwrap(),
            ["a-b_c", "café", "pog"]
        );
        assert_eq!(clean_tags(&[]).unwrap(), Vec::<String>::new());
        for bad in ["", "two words", "emoji😀", &"a".repeat(33)] {
            assert!(clean_tags(&[bad.to_owned()]).is_err(), "{bad:?}");
        }
        let many: Vec<String> = (0..11).map(|i| format!("t{i}")).collect();
        assert_eq!(clean_tags(&many).unwrap_err().code, "too_many_tags");
    }

    #[test]
    fn queue_reads_fill_in_their_defaults() {
        use cyex_tv_shared::model::{ReviewQueueFilter, ReviewSort, ReviewTab};
        let plain = QueueRead::from_filter(ReviewQueueFilter::default()).unwrap();
        assert_eq!(plain.tab, reads::QueueTab::Pending);
        assert_eq!(plain.sort, reads::QueueSort::Uploaded);
        assert!(!plain.descending && plain.search.is_none());
        let resolved = QueueRead::from_filter(ReviewQueueFilter {
            tab: Some(ReviewTab::Resolved),
            search: Some("  ab  ".into()),
            ..ReviewQueueFilter::default()
        })
        .unwrap();
        assert_eq!(resolved.sort, reads::QueueSort::Reviewed);
        assert!(resolved.descending);
        assert_eq!(resolved.search.as_deref(), Some("ab"));
        let named = QueueRead::from_filter(ReviewQueueFilter {
            tab: Some(ReviewTab::Resolved),
            sort: Some(ReviewSort::Name),
            descending: Some(false),
            search: Some("   ".into()),
        })
        .unwrap();
        assert_eq!(named.sort, reads::QueueSort::Name);
        assert!(!named.descending && named.search.is_none());
        let long = ReviewQueueFilter {
            search: Some("x".repeat(101)),
            ..ReviewQueueFilter::default()
        };
        assert_eq!(QueueRead::from_filter(long).unwrap_err().code, "bad_search");
    }

    #[test]
    fn reasons_are_required_and_bounded() {
        assert_eq!(reason("  blurry  ").unwrap(), "blurry");
        assert!(reason(" ").is_err());
        assert!(reason(&"a".repeat(501)).is_err());
    }
}
