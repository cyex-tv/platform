//! `searchEmotes` (ADR 0004), through `cyex_tv_db::search::EmoteSearch`.

use std::collections::HashMap;

use cyex_tv_db::search::{EmoteSearch, PgSearch, Query};
use cyex_tv_db::{ids, reads};
use cyex_tv_shared::error::ApiResult;
use cyex_tv_shared::model::SearchResults;

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
    pub page: Option<u32>,
    pub per_page: Option<u32>,
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
