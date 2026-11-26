//! GraphQL: sets.

use async_graphql::{Context, Object, Result};
use cyex_tv_shared::model::{
    AddSetEmote, ClonedSet, CreateSet, EmoteSet, SetEmote, SetEmotePage, SetSummary, SetVisibility,
    UpdateSet, UpdateSetEmote,
};

use super::{api_error, caller, caller_opt, heavy_read, state};
use crate::catalog;

#[derive(Default)]
pub struct SetQuery;

#[Object]
impl SetQuery {
    /// A set with its emotes. Null when there is none, or when it is `PRIVATE` and you are not
    /// its owner, an editor or staff (a channel's active set is always public).
    async fn set(&self, ctx: &Context<'_>, id: String) -> Result<Option<EmoteSet>> {
        catalog::set(state(ctx), caller_opt(ctx), &id)
            .await
            .map_err(api_error)
    }

    /// What you may set a set's capacity to (`updateSet`): from the emotes it holds (at least 1)
    /// to the owner's `set_capacity_max` role limit (1.000 by default), or 1.000.000 with
    /// `sets.edit_any`. Null when you can't change its capacity. Signed in.
    async fn set_capacity_range(
        &self,
        ctx: &Context<'_>,
        id: String,
    ) -> Result<Option<cyex_tv_shared::model::SetCapacityRange>> {
        let caller = caller(ctx)?;
        catalog::set_capacity_range(state(ctx), caller, &id)
            .await
            .map_err(api_error)
    }

    /// A page of a set's emotes, oldest first, for sets too big to read whole. Who may see what is
    /// the rule of `set`; an unknown or hidden set is an empty page. `query` keeps the emotes whose
    /// name in the set or own name contains it (case-insensitive). `perPage` is 1 to 200, default
    /// 120; `total` counts every match.
    async fn set_emotes(
        &self,
        ctx: &Context<'_>,
        id: String,
        query: Option<String>,
        page: Option<u32>,
        per_page: Option<u32>,
    ) -> Result<SetEmotePage> {
        catalog::set_emotes(
            state(ctx),
            caller_opt(ctx),
            &id,
            query.as_deref(),
            page,
            per_page,
        )
        .await
        .map_err(api_error)
    }

    /// An account's `LISTED` sets as profile cards, oldest first, at most 50. `preview` is how
    /// many of each set's newest emotes to include, 0 to 50, default 12. Empty when the account
    /// is unknown, deleted or blocked. Counts as a heavy read: 120 a minute per account, or per IP
    /// signed out (`rate_limited`), and a high query cost.
    #[graphql(complexity = "100 + child_complexity")]
    async fn account_sets(
        &self,
        ctx: &Context<'_>,
        account_id: String,
        preview: Option<u32>,
    ) -> Result<Vec<SetSummary>> {
        heavy_read(ctx)?;
        catalog::account_sets(state(ctx), caller_opt(ctx), &account_id, preview)
            .await
            .map_err(api_error)
    }

    /// The cyex.tv global set, shown in every channel.
    async fn global_set(&self, ctx: &Context<'_>) -> Result<EmoteSet> {
        catalog::global_set(state(ctx), caller_opt(ctx))
            .await
            .map_err(api_error)
    }
}

#[derive(Default)]
pub struct SetMutation;

#[Object]
impl SetMutation {
    /// Create a channel set, or your personal set (one per account). With `ownerId`, for an
    /// account you edit with `SETS`.
    async fn create_set(&self, ctx: &Context<'_>, input: CreateSet) -> Result<EmoteSet> {
        let caller = caller(ctx)?;
        catalog::create_set(state(ctx), caller, &input)
            .await
            .map_err(api_error)
    }

    /// Copy a set you can open into a new channel set: yours, or with `ownerId` one for an account
    /// you edit with `SETS`. Name defaults to the source's; the copy is public (`UNLISTED` and `PRIVATE` need `sets.edit_any`). Emotes go in
    /// as `addSetEmote` would put them, in order, keeping their names: up to the capacity,
    /// approved ones only (your own pending ones too), never other people's private emotes.
    /// 30 copies an hour. `rate_limited`, `not_found`, `forbidden`, `bad_name`.
    async fn clone_set(
        &self,
        ctx: &Context<'_>,
        id: String,
        name: Option<String>,
        owner_id: Option<String>,
        visibility: Option<SetVisibility>,
    ) -> Result<ClonedSet> {
        let caller = caller(ctx)?;
        catalog::clone_set(
            state(ctx),
            caller,
            &id,
            name.as_deref(),
            owner_id.as_deref(),
            visibility,
        )
        .await
        .map_err(api_error)
    }

    /// Rename a set or change its capacity (owner, editors, account editors with `SETS`). Capacity
    /// is 1 to the owner's `set_capacity_max` role limit (1.000 by default; `capacity_over_limit`
    /// above it), up to 1.000.000 with `sets.edit_any`, and never below the emotes the set holds
    /// (`capacity_too_low`); see `setCapacityRange`. Every set is public: `visibility` other than
    /// what it is needs `sets.edit_any` (`bad_visibility`).
    async fn update_set(
        &self,
        ctx: &Context<'_>,
        id: String,
        input: UpdateSet,
    ) -> Result<EmoteSet> {
        let caller = caller(ctx)?;
        catalog::update_set(state(ctx), caller, &id, &input)
            .await
            .map_err(api_error)
    }

    /// Delete a set. Its owner, or their account editors with `SETS`. A channel's active set
    /// can't be deleted.
    async fn delete_set(&self, ctx: &Context<'_>, id: String) -> Result<bool> {
        let caller = caller(ctx)?;
        catalog::delete_set(state(ctx), caller, &id)
            .await
            .map_err(api_error)
    }

    /// Add an emote to a set. Personal sets take approved emotes only.
    async fn add_set_emote(
        &self,
        ctx: &Context<'_>,
        set_id: String,
        emote_id: String,
        input: Option<AddSetEmote>,
    ) -> Result<SetEmote> {
        let caller = caller(ctx)?;
        catalog::add_set_emote(
            state(ctx),
            caller,
            &set_id,
            &emote_id,
            &input.unwrap_or_default(),
        )
        .await
        .map_err(api_error)
    }

    /// Rename an emote in a set, or change its zero-width flag there.
    async fn update_set_emote(
        &self,
        ctx: &Context<'_>,
        set_id: String,
        emote_id: String,
        input: UpdateSetEmote,
    ) -> Result<SetEmote> {
        let caller = caller(ctx)?;
        catalog::update_set_emote(state(ctx), caller, &set_id, &emote_id, &input)
            .await
            .map_err(api_error)
    }

    /// Remove an emote from a set.
    async fn remove_set_emote(
        &self,
        ctx: &Context<'_>,
        set_id: String,
        emote_id: String,
    ) -> Result<bool> {
        let caller = caller(ctx)?;
        catalog::remove_set_emote(state(ctx), caller, &set_id, &emote_id)
            .await
            .map_err(api_error)
    }

    /// Make someone an editor of a set. Its owner, or their account editors with `EDITORS` and
    /// `EMOTES`.
    async fn add_set_editor(
        &self,
        ctx: &Context<'_>,
        set_id: String,
        account_id: String,
    ) -> Result<bool> {
        let caller = caller(ctx)?;
        catalog::add_set_editor(state(ctx), caller, &set_id, &account_id)
            .await
            .map_err(api_error)
    }

    /// Remove an editor. Owner, or the editor themselves.
    async fn remove_set_editor(
        &self,
        ctx: &Context<'_>,
        set_id: String,
        account_id: String,
    ) -> Result<bool> {
        let caller = caller(ctx)?;
        catalog::remove_set_editor(state(ctx), caller, &set_id, &account_id)
            .await
            .map_err(api_error)
    }
}
