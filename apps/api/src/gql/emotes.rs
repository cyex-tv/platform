//! GraphQL: emotes.

use async_graphql::{Context, ID, Object, Result};
use cyex_tv_shared::model::TrendingWindow;
use cyex_tv_shared::model::{
    Emote, EmoteFlag, EmoteFlagsInput, EmoteMergePreview, EmoteMergeResult, EmoteSort,
    EmoteTransferRequest, EmoteTrending, SearchResults, UpdateEmote,
};

use super::{api_error, caller, caller_opt, state};
use crate::{merge, moderation, search, transfers};

#[derive(Default)]
pub struct EmoteQuery;

#[Object]
impl EmoteQuery {
    /// An emote. Pending ones are visible to their owner and to moderators only. The ID of an
    /// emote that was merged into another (ADR 0040) answers that emote, with `mergedInto` set
    /// to its ID: go there.
    #[graphql(complexity = "10 + child_complexity")]
    async fn emote(&self, ctx: &Context<'_>, id: String) -> Result<Option<Emote>> {
        moderation::emote(state(ctx), caller_opt(ctx), &id)
            .await
            .map_err(api_error)
    }

    /// Several emotes at once, in the order asked, each once (ADR 0035): who may see what is the
    /// rule of `emote`, and unknown or hidden emotes are left out. At most 100 IDs (`too_many`);
    /// every 10 IDs cost what one `emote` lookup would.
    #[graphql(complexity = "5 + ids.len().div_ceil(10) * (10 + child_complexity)")]
    async fn emotes(&self, ctx: &Context<'_>, ids: Vec<ID>) -> Result<Vec<Emote>> {
        let ids: Vec<String> = ids.into_iter().map(|id| id.0).collect();
        moderation::emotes_by_ids(state(ctx), caller_opt(ctx), &ids)
            .await
            .map_err(api_error)
    }

    /// Approved, listed emotes by name and tag, best match first, then the most used.
    /// `perPage` is 1 to 100, default 50. `nsfw` is retired and changes nothing: NSFW emotes
    /// aren't allowed, so none is ever listed. `sort` is
    /// `RELEVANCE` (default: best match, then most used), `TOP` (most used, then name), `NEW`
    /// (newest first) or `TRENDING` (most channels added it in `window`, net of removals, then
    /// most used; only emotes with a net gain are listed). An empty `query` lists every approved,
    /// listed emote.
    /// `flags` keeps the emotes that have every flag listed (`ZERO_WIDTH`, `WIDE`; at most 3
    /// different ones). `NSFW` is retired and finds nothing. `tags` keeps
    /// the emotes that have every tag listed (exact; a leading `#` and case are ignored, at most
    /// 10). `exact: true` makes a non-empty `query` match the whole name (any case) instead of
    /// roughly. `window` is `DAY`, `WEEK` (default) or `MONTH`, and only matters for `TRENDING`.
    ///
    /// Cursor paging (ADR 0030): pass a page's `next` as `after` to read the page after it, in
    /// the same order, with nothing shown twice or skipped when emotes are added meanwhile; `page`
    /// is ignored then and there is no depth limit. `next` is null on the last page. A cursor only
    /// works with the same `query`, filters and `sort` (`bad_cursor` otherwise).
    #[allow(clippy::too_many_arguments)]
    #[graphql(complexity = "50 + child_complexity")]
    async fn search_emotes(
        &self,
        ctx: &Context<'_>,
        query: String,
        animated: Option<bool>,
        owner: Option<String>,
        #[graphql(deprecation = "NSFW emotes aren't allowed on cyex.tv; this changes nothing.")]
        nsfw: Option<bool>,
        page: Option<u32>,
        per_page: Option<u32>,
        sort: Option<EmoteSort>,
        flags: Option<Vec<EmoteFlag>>,
        tags: Option<Vec<String>>,
        exact: Option<bool>,
        window: Option<TrendingWindow>,
        after: Option<String>,
    ) -> Result<SearchResults> {
        // Retired: taken so old clients keep working, and ignored.
        let _ = nsfw;
        let flags: Vec<EmoteFlag> =
            flags
                .unwrap_or_default()
                .into_iter()
                .fold(Vec::new(), |mut all, f| {
                    if !all.contains(&f) {
                        all.push(f);
                    }
                    all
                });
        search::search(
            state(ctx),
            &search::Params {
                text: &query,
                animated,
                owner: owner.as_deref(),
                nsfw: false,
                flags: &flags,
                tags: tags.as_deref().unwrap_or_default(),
                exact: exact.unwrap_or(false),
                window: window.unwrap_or_default(),
                sort: sort.unwrap_or_default(),
                page,
                per_page,
                after: after.as_deref(),
            },
        )
        .await
        .map_err(api_error)
    }

    /// What merging emote `id` into `intoId` would do (ADR 0040), without doing it: both emotes
    /// as they read now (a target that was merged itself is followed to the final one,
    /// `followed`), how many set memberships move, and the sets that hold both. Needs
    /// `emotes.merge`; refused as `mergeEmote` would be.
    #[graphql(complexity = "20 + child_complexity")]
    async fn emote_merge_preview(
        &self,
        ctx: &Context<'_>,
        id: String,
        into_id: String,
    ) -> Result<EmoteMergePreview> {
        let caller = caller(ctx)?;
        merge::preview(state(ctx), caller, &id, &into_id)
            .await
            .map_err(api_error)
    }

    /// Where an emote stands among the 100 most added in `window` (`DAY`, `WEEK` by default or
    /// `MONTH`): its rank and the channels that added it, net of removals. Null when `id` isn't
    /// an emote ID, the emote isn't approved and listed, or it is not in the top 100.
    #[graphql(complexity = "10 + child_complexity")]
    async fn emote_trending(
        &self,
        ctx: &Context<'_>,
        id: String,
        window: Option<TrendingWindow>,
    ) -> Result<Option<EmoteTrending>> {
        search::trending(state(ctx), &id, window)
            .await
            .map_err(api_error)
    }
}

#[derive(Default)]
pub struct EmoteMutation;

#[Object]
impl EmoteMutation {
    /// Change an emote's name, tags or flags. Owner, an account editor of the owner with
    /// `EMOTE_MANAGE` on every set (audited), or `emotes.edit_any`.
    async fn update_emote(
        &self,
        ctx: &Context<'_>,
        id: String,
        input: UpdateEmote,
    ) -> Result<Emote> {
        let caller = caller(ctx)?;
        moderation::update(state(ctx), caller, &id, &input)
            .await
            .map_err(api_error)
    }

    /// Change an emote's flags after review: visibility (`UNLISTED` takes it off the listings)
    /// and zero-width. `nsfw: true` is `nsfw_not_allowed` (NSFW emotes aren't allowed); saving
    /// clears an old NSFW flag. Needs `emotes.flags` or `emotes.edit_any`. Audited
    /// (`emote.flags_changed`); the owner is told when the visibility changes (ADR 0024).
    async fn set_emote_flags(
        &self,
        ctx: &Context<'_>,
        id: String,
        input: EmoteFlagsInput,
    ) -> Result<Emote> {
        let caller = caller(ctx)?;
        moderation::set_flags(state(ctx), caller, &id, &input)
            .await
            .map_err(api_error)
    }

    /// Move an emote to another account at once (ADR 0039, `emote.transferred`). Needs
    /// `emotes.transfer`: anyone else, the owner included, gets `403 transfer_staff_only`.
    /// The receiver must be a live account (not deleted, blocked or restricted). Both the old and
    /// the new owner are notified. 60 an hour.
    async fn transfer_emote(
        &self,
        ctx: &Context<'_>,
        id: String,
        to_account_id: String,
    ) -> Result<Emote> {
        let caller = caller(ctx)?;
        transfers::transfer(state(ctx), caller, &id, &to_account_id)
            .await
            .map_err(api_error)
    }

    /// Retired with transfer requests (ADR 0039): always `410 transfer_requests_retired`.
    #[graphql(
        deprecation = "Retired (ADR 0039): ownership can't be requested. Always 410 transfer_requests_retired; staff move emotes with transferEmote."
    )]
    async fn accept_emote_transfer(&self, ctx: &Context<'_>, id: String) -> Result<Emote> {
        let caller = caller(ctx)?;
        transfers::retired_request(state(ctx), caller, &id).map_err(api_error)
    }

    /// Retired with transfer requests (ADR 0039): always `410 transfer_requests_retired`.
    #[graphql(
        deprecation = "Retired (ADR 0039): ownership can't be requested. Always 410 transfer_requests_retired; staff move emotes with transferEmote."
    )]
    async fn decline_emote_transfer(
        &self,
        ctx: &Context<'_>,
        id: String,
    ) -> Result<EmoteTransferRequest> {
        let caller = caller(ctx)?;
        transfers::retired_request(state(ctx), caller, &id).map_err(api_error)
    }

    /// Retired with transfer requests (ADR 0039): always `410 transfer_requests_retired`.
    #[graphql(
        deprecation = "Retired (ADR 0039): ownership can't be requested. Always 410 transfer_requests_retired; staff move emotes with transferEmote."
    )]
    async fn cancel_emote_transfer(
        &self,
        ctx: &Context<'_>,
        id: String,
    ) -> Result<EmoteTransferRequest> {
        let caller = caller(ctx)?;
        transfers::retired_request(state(ctx), caller, &id).map_err(api_error)
    }

    /// Merge emote `id` into `intoId` (ADR 0040). Needs `emotes.merge`. Every set membership of
    /// `id` names `intoId` instead, keeping its name in the set; a set that holds both keeps
    /// the target's membership (listed in `collapsed`). Name, tags and files are the target's.
    /// `id` is retired: gone from search and sets, and its ID answers the target (REST `301`,
    /// GraphQL `mergedInto`). A target that was merged itself is followed to the final one.
    /// Refused: into itself (`merge_into_self`), into an emote that isn't approved or is
    /// deleted (`merge_target_unavailable`), an emote merged already (`already_merged`).
    /// One transaction; every affected set's version moves once, its delta listing `id` as
    /// removed and `intoId` as added. Audited (`emote.merged`); both owners are notified.
    async fn merge_emote(
        &self,
        ctx: &Context<'_>,
        id: String,
        into_id: String,
    ) -> Result<EmoteMergeResult> {
        let caller = caller(ctx)?;
        merge::merge(state(ctx), caller, &id, &into_id)
            .await
            .map_err(api_error)
    }

    /// Delete your emote, an emote of an account you edit with `EMOTE_MANAGE`, or anyone's with
    /// `emotes.delete_any` (audited). It is removed from every set.
    async fn delete_emote(&self, ctx: &Context<'_>, id: String) -> Result<bool> {
        let caller = caller(ctx)?;
        moderation::delete(state(ctx), caller, &id)
            .await
            .map_err(api_error)
    }
}
