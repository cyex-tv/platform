//! GraphQL: search.

use async_graphql::{Context, Object, Result};
use cyex_tv_shared::model::{AccountResults, QuickSearch};

use super::{api_error, caller_opt, client_ip, state};
use crate::search;

#[derive(Default)]
pub struct SearchQuery;

#[Object]
impl SearchQuery {
    /// Accounts by Twitch or Kick login or display name (a leading `@` is ignored): exact login
    /// first, then login and name prefixes, then similar names. Public fields only; deleted and
    /// blocked accounts are never listed. `perPage` is 1 to 50, default 20. 60 searches a minute
    /// per account, or per IP signed out (`rate_limited`). A `query` under 2 characters (after the
    /// `@`) finds nobody. Matches are counted up to 1000: `totalCapped` says there are more, and
    /// the pages end there.
    async fn search_accounts(
        &self,
        ctx: &Context<'_>,
        query: String,
        page: Option<u32>,
        per_page: Option<u32>,
    ) -> Result<AccountResults> {
        let ip = client_ip(ctx);
        search::accounts(state(ctx), caller_opt(ctx), ip, &query, page, per_page)
            .await
            .map_err(api_error)
    }

    /// The header search: the top 5 emotes for `query` by the name matcher (exact name, same
    /// ignoring case, name prefix, word or initials prefix such as `point` or `pp` for
    /// `PepePoint`, substring, then similar names for 3+ characters; most used first within a
    /// tier), and the top 3 accounts (`searchAccounts` ranking, a leading `@` ignored, under 2
    /// characters finds none). Approved, listed, non-NSFW emotes only. 240 searches a minute per
    /// account, or per IP signed out (`rate_limited`).
    async fn quick_search(&self, ctx: &Context<'_>, query: String) -> Result<QuickSearch> {
        let ip = client_ip(ctx);
        search::quick(state(ctx), caller_opt(ctx), ip, &query)
            .await
            .map_err(api_error)
    }
}
