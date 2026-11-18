//! Emotes after upload: reading one, the owner's edits, deleting, and the review gate (D10).
//!
//! Who sees an emote that isn't approved: its owner and `emotes.review` holders. Deleted emotes
//! are gone for everyone. Approving or rejecting needs `emotes.review` and works only on pending
//! emotes; both are audited, and so is any edit made through `emotes.edit_any`.

use std::collections::BTreeSet;

use axum::http::StatusCode;
use cyex_tv_auth::Caller;
use cyex_tv_db::emotes::{self, EmotePatch};
use cyex_tv_db::{DbError, accounts, audit, ids, reads, sets};
use cyex_tv_shared::error::{ApiError, ApiResult};
use cyex_tv_shared::model::{Emote, EmotePage, EmoteState, EventType, ReviewQueue, UpdateEmote};
use cyex_tv_shared::permissions;
use cyex_tv_shared::text;
use serde_json::{Value, json};
use sqlx::{Connection, PgConnection};
use uuid::Uuid;

use crate::catalog::{
    db_error, forbidden, holds, load_set, member_name, not_found, parse_id, publish_member_event,
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

async fn row(conn: &mut PgConnection, id: Uuid) -> ApiResult<Option<reads::EmoteRow>> {
    Ok(reads::emotes(conn, &[id])
        .await
        .map_err(db_error)?
        .into_iter()
        .next())
}

async fn build(
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
async fn announce(conn: &mut PgConnection, emote_id: Uuid, event_type: EventType) -> ApiResult<()> {
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

// Reads

/// An emote as `caller` may see it.
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
            Some(c) => holds(&mut conn, c, permissions::EMOTES_REVIEW).await?,
        };
        if !allowed {
            return Ok(None);
        }
    }
    build(state, &mut conn, &found).await.map(Some)
}

/// Pending emotes, oldest first. Needs `emotes.review`.
pub async fn review_queue(
    state: &AppState,
    caller: &Caller,
    page: Option<u32>,
    per_page: Option<u32>,
) -> ApiResult<ReviewQueue> {
    let mut conn = state.db.acquire().await.map_err(sqlx_error)?;
    if !holds(&mut conn, caller, permissions::EMOTES_REVIEW).await? {
        return Err(forbidden("This needs emotes.review."));
    }
    let per_page = per_page.unwrap_or(QUEUE_PAGE).clamp(1, QUEUE_PAGE_MAX);
    let page = page.unwrap_or(1).max(1);
    let offset = i64::from(page - 1) * i64::from(per_page);
    let (rows, total) =
        reads::emotes_in_state(&mut conn, EmoteState::Pending, i64::from(per_page), offset)
            .await
            .map_err(db_error)?;
    Ok(ReviewQueue {
        items: views::emotes(&mut conn, &rows, &state.cdn_url)
            .await
            .map_err(db_error)?,
        total: u64::try_from(total).unwrap_or(0),
    })
}

/// An account's emotes in every state, newest first. For `users.view` and moderators.
pub async fn account_emotes(
    state: &AppState,
    caller: &Caller,
    account_id: &str,
    page: Option<u32>,
    per_page: Option<u32>,
) -> ApiResult<EmotePage> {
    let owner = parse_id(account_id)?;
    let mut conn = state.db.acquire().await.map_err(sqlx_error)?;
    if !holds(&mut conn, caller, permissions::USERS_VIEW).await?
        && !holds(&mut conn, caller, permissions::EMOTES_REVIEW).await?
        && !holds(&mut conn, caller, permissions::EMOTES_TAKEDOWN).await?
    {
        return Err(forbidden("This needs users.view or emotes.review."));
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
            before: Some(json!({ "state": before.state })),
            after: Some(json!({ "state": to, "reason": why })),
        },
        before.owner.filter(|_| to == EmoteState::Approved),
    )
    .await
    .map_err(db_error)?;
    if let Some(owner) = before.owner {
        let (kind, payload) = match to {
            EmoteState::Approved => (
                "emote.approved",
                json!({ "emote_id": ids::to_text(id), "name": before.name }),
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

pub async fn approve(state: &AppState, caller: &Caller, id: &str) -> ApiResult<Emote> {
    review(state, caller, id, EmoteState::Approved, None).await
}

pub async fn reject(state: &AppState, caller: &Caller, id: &str, why: &str) -> ApiResult<Emote> {
    review(state, caller, id, EmoteState::Rejected, Some(why)).await
}

/// The owner, or `emotes.edit_any` (audited). A restricted owner can't edit. When an owner
/// without `emotes.review` or `emotes.edit_any` changes the name or tags of an approved emote, it
/// goes back to review (audited as `emote.resubmitted`); flags alone don't.
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
    if !owner && !edit_any {
        return Err(forbidden("Only the emote's owner can change it."));
    }
    if owner {
        let account = accounts::get(&mut tx, caller.account_id)
            .await
            .map_err(db_error)?
            .ok_or_else(not_found)?;
        if account.restricted_at.is_some() {
            return Err(ApiError::new(
                StatusCode::FORBIDDEN,
                "restricted",
                "Your account can't change emotes right now.",
            ));
        }
    }
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
            private: input.private,
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
    if !owner {
        let snap = |r: &reads::EmoteRow, tags: &[String]| json!({ "name": r.name, "zero_width": r.zero_width, "private": r.private, "nsfw": r.nsfw, "tags": tags });
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

/// The owner deletes their emote: it leaves every set at once. False if there was none.
pub async fn delete(state: &AppState, caller: &Caller, id: &str) -> ApiResult<bool> {
    let id = parse_id(id)?;
    let mut conn = state.db.acquire().await.map_err(sqlx_error)?;
    let mut tx = conn.begin().await.map_err(sqlx_error)?;
    let Some(found) = row(&mut tx, id).await? else {
        return Ok(false);
    };
    if found.owner != Some(caller.account_id) {
        return Err(forbidden("Only the emote's owner can delete it."));
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
    fn reasons_are_required_and_bounded() {
        assert_eq!(reason("  blurry  ").unwrap(), "blurry");
        assert!(reason(" ").is_err());
        assert!(reason(&"a".repeat(501)).is_err());
    }
}
