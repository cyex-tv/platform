//! GraphQL: personal.

use async_graphql::{Context, Object, Result};
use cyex_tv_shared::model::{
    EmotePage, EmoteSet, HiddenCreator, HiddenEmote, PersonalSetEntry, Platform,
};

use super::{api_error, caller, state};
use crate::{account, catalog, personal};

#[derive(Default)]
pub struct PersonalQuery;

#[Object]
impl PersonalQuery {
    /// One user's personal set.
    async fn personal_set(
        &self,
        ctx: &Context<'_>,
        platform: Platform,
        id: String,
    ) -> Result<Option<EmoteSet>> {
        catalog::personal_set(state(ctx), platform, &id)
            .await
            .map_err(api_error)
    }

    /// Personal sets for up to 100 chatters at once.
    async fn personal_sets(
        &self,
        ctx: &Context<'_>,
        platform: Platform,
        user_ids: Vec<String>,
    ) -> Result<Vec<PersonalSetEntry>> {
        catalog::personal_sets(state(ctx), platform, &user_ids)
            .await
            .map_err(api_error)
    }

    /// The sets you own, then the ones you edit. Pending emotes included.
    async fn my_sets(&self, ctx: &Context<'_>) -> Result<Vec<EmoteSet>> {
        let caller = caller(ctx)?;
        catalog::my_sets(state(ctx), caller)
            .await
            .map_err(api_error)
    }

    /// Your own emotes in every state, newest first. `perPage` is 1 to 100, default 50.
    /// Pass a page's `next` as `after` for the page after it (cursor paging, ADR 0030; `page` is
    /// ignored then, and nothing is shown twice or skipped when rows are added meanwhile).
    async fn my_emotes(
        &self,
        ctx: &Context<'_>,
        page: Option<u32>,
        per_page: Option<u32>,
        after: Option<String>,
    ) -> Result<EmotePage> {
        let caller = caller(ctx)?;
        catalog::my_emotes(state(ctx), caller, page, per_page, after.as_deref())
            .await
            .map_err(api_error)
    }

    /// Emotes you hid from your own view ("Hide for me"), the latest first: all of them, up to
    /// 1000. Only you can read it; the public reads stay the same for everyone, so filter with
    /// this list (and `hiddenCreators`) on your side.
    async fn hidden_emotes(&self, ctx: &Context<'_>) -> Result<Vec<HiddenEmote>> {
        let caller = caller(ctx)?;
        personal::hidden_emotes(state(ctx), caller)
            .await
            .map_err(api_error)
    }

    /// Creators whose emotes you hid from your own view, the latest first: all of them, up to 200.
    /// Only you can read it.
    async fn hidden_creators(&self, ctx: &Context<'_>) -> Result<Vec<HiddenCreator>> {
        let caller = caller(ctx)?;
        personal::hidden_creators(state(ctx), caller)
            .await
            .map_err(api_error)
    }

    /// Your favourite emotes, the latest added first. `query` keeps the ones whose name holds it
    /// (any case). `perPage` is 1 to 100, default 50. Only you can read it.
    async fn my_favourites(
        &self,
        ctx: &Context<'_>,
        query: Option<String>,
        page: Option<u32>,
        per_page: Option<u32>,
    ) -> Result<EmotePage> {
        let caller = caller(ctx)?;
        personal::my_favourites(state(ctx), caller, query.as_deref(), page, per_page)
            .await
            .map_err(api_error)
    }

    /// The IDs of all your favourite emotes (up to 1000), the latest first: for marking them.
    async fn favourite_emote_ids(&self, ctx: &Context<'_>) -> Result<Vec<String>> {
        let caller = caller(ctx)?;
        personal::favourite_ids(state(ctx), caller)
            .await
            .map_err(api_error)
    }
}

#[derive(Default)]
pub struct PersonalMutation;

#[Object]
impl PersonalMutation {
    /// Mark your notifications read: those in `ids` (up to 100; others' IDs are ignored), or all
    /// of them without `ids`. Returns how many were unread.
    async fn mark_notifications_read(
        &self,
        ctx: &Context<'_>,
        ids: Option<Vec<String>>,
    ) -> Result<u64> {
        let caller = caller(ctx)?;
        account::mark_read(state(ctx), caller, ids.as_deref())
            .await
            .map_err(api_error)
    }

    /// Hide an emote from your own view. True when newly hidden, false when it was already.
    /// `not_found` for an emote you can't see, `limit_reached` at 1000 hidden emotes.
    async fn hide_emote(&self, ctx: &Context<'_>, id: String) -> Result<bool> {
        let caller = caller(ctx)?;
        personal::hide_emote(state(ctx), caller, &id)
            .await
            .map_err(api_error)
    }

    /// Show an emote you hid again. False when it wasn't hidden.
    async fn unhide_emote(&self, ctx: &Context<'_>, id: String) -> Result<bool> {
        let caller = caller(ctx)?;
        personal::unhide_emote(state(ctx), caller, &id)
            .await
            .map_err(api_error)
    }

    /// Hide every emote by a creator from your own view. True when newly hidden. `not_found` for
    /// an unknown account, `own_account` for yourself, `limit_reached` at 200 creators.
    async fn hide_creator(&self, ctx: &Context<'_>, account_id: String) -> Result<bool> {
        let caller = caller(ctx)?;
        personal::hide_creator(state(ctx), caller, &account_id)
            .await
            .map_err(api_error)
    }

    /// Show a creator's emotes again. False when they weren't hidden.
    async fn unhide_creator(&self, ctx: &Context<'_>, account_id: String) -> Result<bool> {
        let caller = caller(ctx)?;
        personal::unhide_creator(state(ctx), caller, &account_id)
            .await
            .map_err(api_error)
    }

    /// Add an emote to your favourites. True when newly added. `not_found` for an emote you can't
    /// see, `limit_reached` at 1000 favourites.
    async fn favourite_emote(&self, ctx: &Context<'_>, id: String) -> Result<bool> {
        let caller = caller(ctx)?;
        personal::favourite_emote(state(ctx), caller, &id)
            .await
            .map_err(api_error)
    }

    /// Take an emote off your favourites. False when it wasn't one.
    async fn unfavourite_emote(&self, ctx: &Context<'_>, id: String) -> Result<bool> {
        let caller = caller(ctx)?;
        personal::unfavourite_emote(state(ctx), caller, &id)
            .await
            .map_err(api_error)
    }
}
