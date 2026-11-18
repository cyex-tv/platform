//! `searchEmotes` (ADR 0004), through `cyex_tv_db::search::EmoteSearch`, and `searchAccounts`.

use std::collections::HashMap;

use axum::http::StatusCode;
use cyex_tv_auth::Caller;
use cyex_tv_db::search::{EmoteSearch, PgSearch, Query};
use cyex_tv_db::{ids, reads};
use cyex_tv_shared::error::{ApiError, ApiResult};
use cyex_tv_shared::model::{
    AccountHit, AccountResults, AccountSummary, EmoteSort, PublicIdentity, SearchResults,
};

use crate::catalog::db_error;
use crate::state::AppState;
use crate::views;

pub const PER_PAGE: u32 = 50;
pub const PER_PAGE_MAX: u32 = 100;

#[derive(Clone, Debug, Default)]
pub struct Params<'a> {
    pub text: &'a str,
    pub animated: Option<bool>,
    pub owner: Option<&'a str>,
    pub nsfw: bool,
    pub sort: EmoteSort,
    pub page: Option<u32>,
    pub per_page: Option<u32>,
}

pub const ACCOUNTS_PER_PAGE: u32 = 20;
pub const ACCOUNTS_PER_PAGE_MAX: u32 = 50;

/// `searchAccounts`: live accounts by Twitch or Kick login or display name, public fields only.
/// A leading `@` is ignored. Limited per account, or per IP when signed out.
pub async fn accounts(
    state: &AppState,
    caller: Option<&Caller>,
    ip: &str,
    text: &str,
    page: Option<u32>,
    per_page: Option<u32>,
) -> ApiResult<AccountResults> {
    let key = match caller {
        Some(c) => format!("a:{}", ids::to_text(c.account_id)),
        None => format!("ip:{ip}"),
    };
    state.limits.account_search.check(&key).map_err(|_| {
        ApiError::new(
            StatusCode::TOO_MANY_REQUESTS,
            "rate_limited",
            "That's a lot of searches. Try again in a minute.",
        )
    })?;
    let per_page = per_page
        .unwrap_or(ACCOUNTS_PER_PAGE)
        .clamp(1, ACCOUNTS_PER_PAGE_MAX);
    let page = page.unwrap_or(1).clamp(1, 1000);
    let text = text.trim().trim_start_matches('@');
    let mut conn = state.db.acquire().await.map_err(|e| db_error(e.into()))?;
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
    let mut items = Vec::with_capacity(hits.ids.len());
    for id in &hits.ids {
        let Some(s) = summaries.get(id) else { continue };
        let identities = cyex_tv_db::accounts::identities(&mut conn, *id)
            .await
            .map_err(db_error)?
            .into_iter()
            .map(|i| PublicIdentity {
                platform: i.platform,
                login: i.login,
                display_name: i.display_name,
            })
            .collect();
        items.push(AccountHit {
            account: AccountSummary {
                id: ids::to_text(s.account_id),
                display_name: s.display_name.clone(),
                avatar: s.avatar.clone(),
            },
            identities,
        });
    }
    Ok(AccountResults {
        items,
        total: u64::try_from(hits.total).unwrap_or(0),
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
