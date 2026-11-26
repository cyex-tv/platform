//! Personal lists: "Hide for me" (by emote or by creator) and favourite emotes.
//!
//! Private to the caller: no field takes an account argument, so nobody reads or changes someone
//! else's list (IDOR). The public reads (search, trending, sets, the header search) stay the same
//! for everyone and stay cacheable; the website takes the caller's list and filters with it.
//!
//! Caps: 1000 hidden emotes, 200 hidden creators, 1000 favourites (`limit_reached`). Changes count
//! against a per-account rate (`rate_limited`). Hiding or favouriting needs an emote the caller may
//! see (`not_found` otherwise); a creator must be a live account other than the caller.

use cyex_tv_auth::Caller;
use cyex_tv_db::personal::{self, Added, List};
use cyex_tv_db::{ids, reads};
use cyex_tv_shared::error::{ApiError, ApiResult};
use cyex_tv_shared::model::{AccountSummary, EmotePage, EmoteState, HiddenCreator, HiddenEmote};
use sqlx::Connection;
use std::collections::HashMap;
use uuid::Uuid;

use crate::errors::{bad, conn, db_error, limited_as, not_found, parse_id, sqlx_error};
use crate::limits::AccountLimit;
use crate::paging::Paged;
use crate::state::AppState;
use crate::views;

pub const HIDDEN_EMOTES_MAX: i64 = 1_000;
pub const HIDDEN_CREATORS_MAX: i64 = 200;
pub const FAVOURITES_MAX: i64 = 1_000;

fn limit_reached(message: &str, cap: i64) -> ApiError {
    let cap = cap.unsigned_abs();
    ApiError::limit("limit_reached", message, cap, cap)
}

fn take_rate(state: &AppState, caller: &Caller) -> ApiResult<()> {
    limited_as(
        state,
        AccountLimit::PersonalLists,
        &ids::to_text(caller.account_id),
        caller.account_id,
        "",
    )
}

/// The emote, when the caller may see it: approved, or their own.
async fn visible_emote(state: &AppState, caller: &Caller, id: &str) -> ApiResult<Uuid> {
    crate::moderation::emote(state, Some(caller), id)
        .await?
        .and_then(|e| ids::from_text(&e.id))
        .ok_or_else(not_found)
}

async fn add(
    state: &AppState,
    caller: &Caller,
    list: List,
    id: Uuid,
    cap: i64,
    full: &str,
) -> ApiResult<bool> {
    let mut conn = conn(state).await?;
    let mut tx = conn.begin().await.map_err(sqlx_error)?;
    let done = personal::add(&mut tx, list, caller.account_id, id, cap)
        .await
        .map_err(db_error)?;
    tx.commit().await.map_err(sqlx_error)?;
    match done {
        Added::Added => Ok(true),
        Added::Already => Ok(false),
        Added::Full => Err(limit_reached(full, cap)),
    }
}

async fn remove(state: &AppState, caller: &Caller, list: List, raw: &str) -> ApiResult<bool> {
    take_rate(state, caller)?;
    let Some(id) = ids::from_text(raw.trim()) else {
        return Ok(false);
    };
    let mut conn = conn(state).await?;
    personal::remove(&mut conn, list, caller.account_id, id)
        .await
        .map_err(db_error)
}

/// Emote views by ID, for the ones the caller may still see (approved, or their own).
async fn emote_views(
    conn: &mut sqlx::PgConnection,
    state: &AppState,
    caller: &Caller,
    ids_wanted: &[Uuid],
) -> ApiResult<HashMap<Uuid, cyex_tv_shared::model::Emote>> {
    let rows: Vec<reads::EmoteRow> = reads::emotes(conn, ids_wanted)
        .await
        .map_err(db_error)?
        .into_iter()
        .filter(|r| r.state == EmoteState::Approved || r.owner == Some(caller.account_id))
        .collect();
    let built = views::emotes(conn, &rows, &state.cdn_url)
        .await
        .map_err(db_error)?;
    Ok(rows.iter().map(|r| r.id).zip(built).collect())
}

// Hide for me

/// Hide an emote from your own view. True when newly hidden.
pub async fn hide_emote(state: &AppState, caller: &Caller, id: &str) -> ApiResult<bool> {
    take_rate(state, caller)?;
    let id = visible_emote(state, caller, id).await?;
    add(
        state,
        caller,
        List::HiddenEmotes,
        id,
        HIDDEN_EMOTES_MAX,
        "You've hidden 1.000 emotes, the most there can be. Unhide some first.",
    )
    .await
}

pub async fn unhide_emote(state: &AppState, caller: &Caller, id: &str) -> ApiResult<bool> {
    remove(state, caller, List::HiddenEmotes, id).await
}

/// Hide every emote by a creator from your own view. True when newly hidden.
pub async fn hide_creator(state: &AppState, caller: &Caller, account_id: &str) -> ApiResult<bool> {
    take_rate(state, caller)?;
    let id = parse_id(account_id)?;
    if id == caller.account_id {
        return Err(bad("own_account", "You can't hide your own emotes."));
    }
    if crate::catalog::account(state, account_id).await?.is_none() {
        return Err(not_found());
    }
    add(
        state,
        caller,
        List::HiddenCreators,
        id,
        HIDDEN_CREATORS_MAX,
        "You've hidden 200 creators, the most there can be. Unhide some first.",
    )
    .await
}

pub async fn unhide_creator(
    state: &AppState,
    caller: &Caller,
    account_id: &str,
) -> ApiResult<bool> {
    remove(state, caller, List::HiddenCreators, account_id).await
}

/// Your hidden emotes, the latest first (all of them, up to 1000).
pub async fn hidden_emotes(state: &AppState, caller: &Caller) -> ApiResult<Vec<HiddenEmote>> {
    let mut conn = conn(state).await?;
    let entries = personal::all(&mut conn, List::HiddenEmotes, caller.account_id)
        .await
        .map_err(db_error)?;
    let wanted: Vec<Uuid> = entries.iter().map(|e| e.id).collect();
    let mut found = emote_views(&mut conn, state, caller, &wanted).await?;
    Ok(entries
        .into_iter()
        .map(|e| HiddenEmote {
            emote_id: ids::to_text(e.id),
            emote: found.remove(&e.id),
            hidden_at: views::time(e.created_at),
        })
        .collect())
}

/// Your hidden creators, the latest first (all of them, up to 200).
pub async fn hidden_creators(state: &AppState, caller: &Caller) -> ApiResult<Vec<HiddenCreator>> {
    let mut conn = conn(state).await?;
    let entries = personal::all(&mut conn, List::HiddenCreators, caller.account_id)
        .await
        .map_err(db_error)?;
    let wanted: Vec<Uuid> = entries.iter().map(|e| e.id).collect();
    let summaries = reads::summaries(&mut conn, &wanted)
        .await
        .map_err(db_error)?;
    Ok(entries
        .into_iter()
        .map(|e| HiddenCreator {
            account_id: ids::to_text(e.id),
            account: summaries.get(&e.id).map(AccountSummary::from),
            hidden_at: views::time(e.created_at),
        })
        .collect())
}

// Favourites

/// Mark an emote as a favourite. True when newly added.
pub async fn favourite_emote(state: &AppState, caller: &Caller, id: &str) -> ApiResult<bool> {
    take_rate(state, caller)?;
    let id = visible_emote(state, caller, id).await?;
    add(
        state,
        caller,
        List::Favourites,
        id,
        FAVOURITES_MAX,
        "You have 1.000 favourites, the most there can be. Remove some first.",
    )
    .await
}

pub async fn unfavourite_emote(state: &AppState, caller: &Caller, id: &str) -> ApiResult<bool> {
    remove(state, caller, List::Favourites, id).await
}

/// The IDs of all your favourites, the latest first.
pub async fn favourite_ids(state: &AppState, caller: &Caller) -> ApiResult<Vec<String>> {
    let mut conn = conn(state).await?;
    Ok(
        personal::all(&mut conn, List::Favourites, caller.account_id)
            .await
            .map_err(db_error)?
            .into_iter()
            .map(|e| ids::to_text(e.id))
            .collect(),
    )
}

/// A page of your favourites, the latest first; `query` keeps names holding it.
pub async fn my_favourites(
    state: &AppState,
    caller: &Caller,
    query: Option<&str>,
    page: Option<u32>,
    per_page: Option<u32>,
) -> ApiResult<EmotePage> {
    let paged = Paged::MyFavourites.paging(page, per_page);
    let mut conn = conn(state).await?;
    let (wanted, total) = personal::favourites_page(
        &mut conn,
        caller.account_id,
        query,
        paged.limit,
        paged.offset,
    )
    .await
    .map_err(db_error)?;
    let mut found = emote_views(&mut conn, state, caller, &wanted).await?;
    Ok(EmotePage {
        items: wanted.iter().filter_map(|id| found.remove(id)).collect(),
        total: u64::try_from(total).unwrap_or(0),
        next: None,
    })
}
