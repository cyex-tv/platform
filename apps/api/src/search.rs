//! `searchEmotes` (ADR 0004), through `cyex_tv_db::search::EmoteSearch`, `searchAccounts` and
//! `quickSearch`.

use std::collections::HashMap;

use cyex_tv_auth::Caller;
use cyex_tv_db::search::{ACCOUNT_QUERY_MIN, EmoteSearch, PgSearch, Query};
use cyex_tv_db::{ids, reads};
use cyex_tv_shared::error::ApiResult;
use cyex_tv_shared::model::{
    AccountHit, AccountResults, AccountSummary, EmoteFlag, EmoteSort, EmoteTrending,
    PublicIdentity, QuickSearch, SearchResults, TrendingWindow,
};

use sqlx::PgConnection;
use uuid::Uuid;

use crate::catalog::db_error;
use crate::errors::{paging, sqlx_error};
use crate::limits;
use crate::state::AppState;
use crate::views;

pub const PER_PAGE: u32 = 50;
pub const PER_PAGE_MAX: u32 = 100;
/// `emoteTrending` ranks the top this many emotes.
pub const TRENDING_TOP: i64 = 100;

#[derive(Clone, Debug, Default)]
pub struct Params<'a> {
    pub text: &'a str,
    pub animated: Option<bool>,
    pub owner: Option<&'a str>,
    pub nsfw: bool,
    pub flags: &'a [EmoteFlag],
    pub tags: &'a [String],
    pub exact: bool,
    pub window: TrendingWindow,
    pub sort: EmoteSort,
    pub page: Option<u32>,
    pub per_page: Option<u32>,
}

pub const ACCOUNTS_PER_PAGE: u32 = 20;
pub const ACCOUNTS_PER_PAGE_MAX: u32 = 50;

/// `searchAccounts`: live accounts by Twitch or Kick login or display name, public fields only.
/// A leading `@` is ignored; a query under two characters finds nobody. Limited per account, or
/// per IP when signed out. Matches are counted up to 1000 (`total_capped`), so the pages end there; a page past the last is empty.
pub async fn accounts(
    state: &AppState,
    caller: Option<&Caller>,
    ip: &str,
    text: &str,
    page: Option<u32>,
    per_page: Option<u32>,
) -> ApiResult<AccountResults> {
    let paged = paging(page, per_page, ACCOUNTS_PER_PAGE, ACCOUNTS_PER_PAGE_MAX);
    // A page past the last one comes back empty (with the real total), not as the last page.
    let (page, per_page) = (paged.page, paged.per_page);
    let empty = || AccountResults {
        items: Vec::new(),
        total: 0,
        total_capped: false,
        page,
        per_page,
    };
    let text = text.trim().trim_start_matches('@').trim();
    if text.chars().count() < ACCOUNT_QUERY_MIN {
        return Ok(empty());
    }
    limits::account_search(state, caller, ip)?;
    let mut conn = state.db.acquire().await.map_err(sqlx_error)?;
    // Past `MAX_PAGE`, one row is read for the total, and dropped.
    let (limit, offset) = if paged.past_end() {
        (1, 0)
    } else {
        (paged.limit, paged.offset)
    };
    let hits = cyex_tv_db::search::accounts(&mut conn, text, limit, offset)
        .await
        .map_err(db_error)?;
    let items = if paged.past_end() {
        Vec::new()
    } else {
        account_hits(&mut conn, &hits.ids).await?
    };
    Ok(AccountResults {
        items,
        total: u64::try_from(hits.total).unwrap_or(0),
        total_capped: hits.capped,
        page,
        per_page,
    })
}

/// The public view of the accounts `ids`, in that order. Accounts that vanished are skipped.
pub(crate) async fn account_hits(
    conn: &mut PgConnection,
    ids: &[Uuid],
) -> ApiResult<Vec<AccountHit>> {
    let summaries = reads::summaries(conn, ids).await.map_err(db_error)?;
    let mut identities = cyex_tv_db::accounts::identities_many(conn, ids)
        .await
        .map_err(db_error)?;
    Ok(ids
        .iter()
        .filter_map(|id| {
            let s = summaries.get(id)?;
            Some(AccountHit {
                account: AccountSummary::from(s),
                identities: identities
                    .remove(id)
                    .unwrap_or_default()
                    .into_iter()
                    .map(|i| PublicIdentity {
                        platform: i.platform,
                        login: i.login,
                        display_name: i.display_name,
                    })
                    .collect(),
            })
        })
        .collect())
}

/// Emotes shown by `quickSearch`.
pub const QUICK_EMOTES: i64 = 5;
/// Accounts shown by `quickSearch`.
pub const QUICK_USERS: i64 = 3;

/// `quickSearch`: the top 5 emotes (`cyex_tv_db::search::quick`) and the top 3 accounts
/// (`searchAccounts` ranking; a leading `@` is ignored, under two characters finds no one).
/// Public. Limited per account, or per IP when signed out.
pub async fn quick(
    state: &AppState,
    caller: Option<&Caller>,
    ip: &str,
    text: &str,
) -> ApiResult<QuickSearch> {
    limits::quick_search(state, caller, ip)?;
    let mut conn = state.db.acquire().await.map_err(sqlx_error)?;
    let ids = cyex_tv_db::search::quick(&mut conn, text, QUICK_EMOTES)
        .await
        .map_err(db_error)?;
    let rows = reads::emotes(&mut conn, &ids).await.map_err(db_error)?;
    let mut by_id: HashMap<_, _> = rows.into_iter().map(|r| (r.id, r)).collect();
    let ordered: Vec<reads::EmoteRow> = ids.iter().filter_map(|id| by_id.remove(id)).collect();
    let emotes = views::emotes(&mut conn, &ordered, &state.cdn_url)
        .await
        .map_err(db_error)?;
    let name = text.trim().trim_start_matches('@').trim();
    let users = if name.chars().count() < ACCOUNT_QUERY_MIN {
        Vec::new()
    } else {
        let hits = cyex_tv_db::search::accounts(&mut conn, name, QUICK_USERS, 0)
            .await
            .map_err(db_error)?;
        account_hits(&mut conn, &hits.ids).await?
    };
    Ok(QuickSearch { emotes, users })
}

pub async fn search(state: &AppState, p: &Params<'_>) -> ApiResult<SearchResults> {
    let paged = paging(p.page, p.per_page, PER_PAGE, PER_PAGE_MAX);
    let (page, per_page) = (paged.page, paged.per_page);
    let empty = || SearchResults {
        items: Vec::new(),
        total: 0,
        page,
        per_page,
    };
    // An owner ID that isn't one matches nothing.
    let owner = match p.owner.map(str::trim).filter(|o| !o.is_empty()) {
        Some(raw) => match ids::from_text(raw) {
            Some(id) => Some(id),
            None => return Ok(empty()),
        },
        None => None,
    };
    let engine = PgSearch {
        pool: state.db.clone(),
    };
    let hits = engine
        .search(&Query {
            text: p.text,
            animated: p.animated,
            owner,
            nsfw: p.nsfw,
            flags: p.flags,
            tags: p.tags,
            exact: p.exact,
            window: p.window,
            sort: p.sort,
            // Past `MAX_PAGE`, one row is read for the total, and dropped.
            limit: if paged.past_end() { 1 } else { paged.limit },
            offset: paged.offset,
        })
        .await
        .map_err(db_error)?;
    if paged.past_end() {
        return Ok(SearchResults {
            total: u64::try_from(hits.total).unwrap_or(0),
            ..empty()
        });
    }

    let mut conn = state.db.acquire().await.map_err(|e| db_error(e.into()))?;
    let rows = reads::emotes(&mut conn, &hits.ids)
        .await
        .map_err(db_error)?;
    // Back into rank order.
    let mut by_id: HashMap<_, _> = rows.into_iter().map(|r| (r.id, r)).collect();
    let ordered: Vec<reads::EmoteRow> = hits.ids.iter().filter_map(|id| by_id.remove(id)).collect();
    Ok(SearchResults {
        items: views::emotes(&mut conn, &ordered, &state.cdn_url)
            .await
            .map_err(db_error)?,
        total: u64::try_from(hits.total).unwrap_or(0),
        page,
        per_page,
    })
}

/// `emoteTrending`: where the emote stands among the top 100 of the `TRENDING` sort in `window`.
/// Null when the ID isn't one, the emote isn't approved and listed, or it is not in the top 100.
pub async fn trending(
    state: &AppState,
    id: &str,
    window: Option<TrendingWindow>,
) -> ApiResult<Option<EmoteTrending>> {
    let window = window.unwrap_or_default();
    let Some(id) = ids::from_text(id.trim()) else {
        return Ok(None);
    };
    let mut conn = state.db.acquire().await.map_err(sqlx_error)?;
    let found = cyex_tv_db::search::trending_rank(&mut conn, id, window.days(), TRENDING_TOP)
        .await
        .map_err(db_error)?;
    Ok(found.map(|(rank, net)| EmoteTrending {
        rank: u32::try_from(rank).unwrap_or(u32::MAX),
        channels: u32::try_from(net).unwrap_or(u32::MAX),
        window,
    }))
}
