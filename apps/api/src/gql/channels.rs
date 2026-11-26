//! GraphQL: channels.

use async_graphql::{Context, Object, Result};
use cyex_tv_db::{DbError, accounts};
use cyex_tv_shared::model::{Channel, Me, Platform, UpdateChannelSettings};

use super::account::me_of;
use super::{api_error, caller, coded, internal, state};
use crate::catalog;

#[derive(Default)]
pub struct ChannelQuery;

#[Object]
impl ChannelQuery {
    /// A channel by its platform ID.
    #[graphql(complexity = "10 + child_complexity")]
    async fn channel(
        &self,
        ctx: &Context<'_>,
        platform: Platform,
        id: String,
    ) -> Result<Option<Channel>> {
        catalog::channel(state(ctx), platform, &id)
            .await
            .map_err(api_error)
    }

    /// Several channels of one platform at once, by their platform IDs, in the order asked,
    /// each once (ADR 0032). Channels not on cyex.tv are left out. At most 100 IDs
    /// (`too_many`); every 10 IDs cost what one `channel` lookup would.
    #[graphql(complexity = "5 + ids.len().div_ceil(10) * (10 + child_complexity)")]
    async fn channels(
        &self,
        ctx: &Context<'_>,
        platform: Platform,
        ids: Vec<String>,
    ) -> Result<Vec<Channel>> {
        catalog::channels_by_ids(state(ctx), platform, &ids)
            .await
            .map_err(api_error)
    }

    /// A channel by its login or slug. Logins change; store the ID, not the login.
    #[graphql(complexity = "10 + child_complexity")]
    async fn channel_by_login(
        &self,
        ctx: &Context<'_>,
        platform: Platform,
        login: String,
    ) -> Result<Option<Channel>> {
        catalog::channel_by_login(state(ctx), platform, &login)
            .await
            .map_err(api_error)
    }
}

#[derive(Default)]
pub struct ChannelMutation;

#[Object]
impl ChannelMutation {
    /// Go by the name and picture of your identity on `platform` (when more than one is linked,
    /// pick which login you show as). Every linked login still finds you in search.
    /// `not_found` when that platform isn't linked. Unlinking it goes back to the default.
    async fn set_display_platform(&self, ctx: &Context<'_>, platform: Platform) -> Result<Me> {
        let caller = caller(ctx)?;
        let mut conn = state(ctx).db.acquire().await.map_err(internal)?;
        match accounts::set_display_platform(&mut conn, caller.account_id, platform).await {
            Ok(()) => {}
            Err(DbError::NotFound) => {
                return Err(coded(
                    "not_found",
                    "That platform isn't linked to your account.",
                ));
            }
            Err(err) => return Err(internal(err)),
        }
        drop(conn);
        me_of(ctx, caller).await
    }

    /// Change a channel's active set and settings. The channel's owner, or their account editors
    /// with `SETS`.
    async fn update_channel_settings(
        &self,
        ctx: &Context<'_>,
        platform: Platform,
        id: String,
        input: UpdateChannelSettings,
    ) -> Result<Channel> {
        let caller = caller(ctx)?;
        catalog::update_channel_settings(state(ctx), caller, platform, &id, &input)
            .await
            .map_err(api_error)
    }
}
