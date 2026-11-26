//! GraphQL: profile.

use async_graphql::{Context, Object, Result};
use cyex_tv_shared::model::AccountSummary;

use super::{api_error, state};
use crate::{catalog, cosmetics};

#[derive(Default)]
pub struct ProfileQuery;

#[Object]
impl ProfileQuery {
    /// An account's public profile: linked Twitch and Kick logins (no IDs), roles marked public,
    /// and each channel's active set ("Active emotes"). Listed sets: `accountSets`. Null when the
    /// account is unknown, deleted or blocked.
    async fn account_profile(
        &self,
        ctx: &Context<'_>,
        account_id: String,
    ) -> Result<Option<cyex_tv_shared::model::AccountProfile>> {
        catalog::account_profile(state(ctx), &account_id)
            .await
            .map_err(api_error)
    }

    /// The badge and paint up to 100 accounts show, with the badges and paints themselves, by
    /// cyex.tv account ID (for names on profiles, tiles and lists). Only accounts showing
    /// something are listed; blocked and deleted ones never. `too_many` over 100.
    async fn account_cosmetics(
        &self,
        ctx: &Context<'_>,
        account_ids: Vec<String>,
    ) -> Result<Vec<cyex_tv_shared::model::AccountCosmetics>> {
        cosmetics::account_cosmetics(state(ctx), &account_ids)
            .await
            .map_err(api_error)
    }

    /// An account's public summary. Null when the account is unknown, deleted or blocked.
    async fn account(&self, ctx: &Context<'_>, id: String) -> Result<Option<AccountSummary>> {
        catalog::account(state(ctx), &id).await.map_err(api_error)
    }
}
