//! `searchEmotes` (ADR 0004), through `cyex_tv_db::search::EmoteSearch`, and `searchAccounts`.

use std::collections::HashMap;

use cyex_tv_auth::Caller;
use cyex_tv_db::search::{ACCOUNT_QUERY_MIN, EmoteSearch, PgSearch, Query};
use cyex_tv_db::{ids, reads};
use cyex_tv_shared::error::ApiResult;
use cyex_tv_shared::model::{
    AccountHit, AccountResults, AccountSummary, EmoteFlag, EmoteSort, EmoteTrending,
    PublicIdentity, SearchResults, TrendingWindow,
};

use crate::catalog::db_error;
use crate::errors::sqlx_error;
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
    let per_page = per_page
        .unwrap_or(ACCOUNTS_PER_PAGE)
        .clamp(1, ACCOUNTS_PER_PAGE_MAX);
    // A page past the last one comes back empty (with the real total), not as the last page.
    let page = page.unwrap_or(1).max(1);
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
    let hits = cyex_tv_db::search::accounts(
        &mut conn,
        text,
        i64::from(per_page),
        i64::from(page - 1) * i64::from(per_page),
    )
    .await
    .map_err(db_error)?;
    let summaries = reads::summaries(&mut conn, &hits.ids)
        .await
        .map_err(db_error)?;
    let mut identities = cyex_tv_db::accounts::identities_many(&mut conn, &hits.ids)
        .await
        .map_err(db_error)?;
    let items = hits
        .ids
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
        .collect();
    Ok(AccountResults {
        items,
        total: u64::try_from(hits.total).unwrap_or(0),
        total_capped: hits.capped,
        page,
        per_page,
    })
}

pub async fn search(state: &AppState, p: &Params<'_>) -> ApiResult<SearchResults> {
    let per_page = p.per_page.unwrap_or(PER_PAGE).clamp(1, PER_PAGE_MAX);
    let page = p.page.unwrap_or(1).max(1);
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
            limit: i64::from(per_page),
            offset: i64::from(page - 1) * i64::from(per_page),
        })
        .await
        .map_err(db_error)?;

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
