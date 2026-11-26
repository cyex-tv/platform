//! GraphQL: activity.

use async_graphql::{Context, Object, Result};
use cyex_tv_shared::model::{ActivityEntry, ActivityPage, Channel};

use super::{api_error, caller_opt, heavy_read, state};
use crate::{activity, catalog};

#[derive(Default)]
pub struct ActivityQuery;

#[Object]
impl ActivityQuery {
    /// An account's public activity, newest first, like 7TV's "Activity" tab: sets and emotes the
    /// owner or their editors changed, the active set, badge and paint picks, approved uploads.
    /// Page back with `before` (an entry ID); `limit` is 1 to 100, default 30. Empty when the
    /// account is unknown, deleted or blocked. Nothing private or from moderation shows. A page
    /// can be shorter than `limit` and still not the last: use `accountActivityPage` to know.
    /// Anyone but the owner and staff pages back through the newest 100 lines only.
    /// A heavy read: 120 a minute per account, or per IP signed out (`rate_limited`).
    #[graphql(complexity = "100 + child_complexity")]
    async fn account_activity(
        &self,
        ctx: &Context<'_>,
        account_id: String,
        before: Option<String>,
        limit: Option<u32>,
    ) -> Result<Vec<ActivityEntry>> {
        heavy_read(ctx)?;
        activity::account_activity(
            state(ctx),
            caller_opt(ctx),
            &account_id,
            before.as_deref(),
            limit,
        )
        .await
        .map_err(api_error)
    }

    /// `accountActivity` with a cursor: `next` is the `before` for the following page, null when
    /// there is none. A page can hold fewer than `limit` entries and still have a `next`, because
    /// the feed skips what isn't public.
    ///
    /// `filter` narrows it: `groups` (emotes, sets, cosmetics, editors; editor lines only show
    /// when asked for), `actorId` (only lines by the owner or that editor), and `since` /
    /// `until` (RFC 3339; `bad_time` otherwise).
    ///
    /// Only the owner and staff (`users.view`) see the whole feed and may narrow it by time. For
    /// everyone else it is the newest 100 lines, all time: `since` and `until` are ignored and
    /// paging ends at the 100th line (`next` is null there; a cursor past it gets no entries).
    #[graphql(complexity = "100 + child_complexity")]
    async fn account_activity_page(
        &self,
        ctx: &Context<'_>,
        account_id: String,
        before: Option<String>,
        limit: Option<u32>,
        filter: Option<cyex_tv_shared::model::ActivityFilter>,
    ) -> Result<ActivityPage> {
        heavy_read(ctx)?;
        activity::account_activity_where(
            state(ctx),
            caller_opt(ctx),
            &account_id,
            before.as_deref(),
            limit,
            filter.as_ref(),
        )
        .await
        .map_err(api_error)
    }

    /// An emote's public activity, newest first: `EMOTE_UPLOADED` (when it was approved, shown
    /// as by its owner) and `SET_EMOTE_ADDED`, `SET_EMOTE_REMOVED`, `SET_EMOTE_RENAMED` in listed
    /// sets, by their owners and account editors. Page back with `before` (an entry ID); `limit`
    /// is 1 to 100, default 30. Empty when the emote is unknown or not public. Nothing private or
    /// from moderation shows. A page can be shorter than `limit` and still not the last: use
    /// `emoteActivityPage` to know. A heavy read: 120 a minute per account, or per IP signed out
    /// (`rate_limited`).
    #[graphql(complexity = "100 + child_complexity")]
    async fn emote_activity(
        &self,
        ctx: &Context<'_>,
        emote_id: String,
        before: Option<String>,
        limit: Option<u32>,
    ) -> Result<Vec<ActivityEntry>> {
        heavy_read(ctx)?;
        activity::emote_activity(state(ctx), &emote_id, before.as_deref(), limit)
            .await
            .map_err(api_error)
    }

    /// `emoteActivity` with a cursor: `next` is the `before` for the following page, null when
    /// there is none. A page can hold fewer than `limit` entries and still have a `next`.
    /// `actorId` keeps only the lines shown as by that account (an upload is its owner's).
    #[graphql(complexity = "100 + child_complexity")]
    async fn emote_activity_page(
        &self,
        ctx: &Context<'_>,
        emote_id: String,
        before: Option<String>,
        limit: Option<u32>,
        actor_id: Option<String>,
    ) -> Result<ActivityPage> {
        heavy_read(ctx)?;
        activity::emote_activity_where(
            state(ctx),
            &emote_id,
            before.as_deref(),
            limit,
            actor_id.as_deref(),
        )
        .await
        .map_err(api_error)
    }

    /// Who an account's (`accountId`) or an emote's (`emoteId`; exactly one, `bad_scope`
    /// otherwise) public activity shows most often as by, most lines first (the newest 2000
    /// lines are counted): the people a "By" filter offers before anything is typed. For an
    /// account, `filter`'s groups and time range apply (its `actorId` doesn't). Empty when the
    /// feed is. `limit` 1 to 10, default 5. A heavy read, as the feed.
    #[graphql(complexity = "100 + child_complexity")]
    async fn activity_top_actors(
        &self,
        ctx: &Context<'_>,
        account_id: Option<String>,
        emote_id: Option<String>,
        filter: Option<cyex_tv_shared::model::ActivityFilter>,
        limit: Option<u32>,
    ) -> Result<Vec<cyex_tv_shared::model::PersonCount>> {
        heavy_read(ctx)?;
        crate::top_people::activity_top_actors(
            state(ctx),
            account_id.as_deref(),
            emote_id.as_deref(),
            filter.as_ref(),
            limit,
        )
        .await
        .map_err(api_error)
    }

    /// Channels whose active set holds the emote, most recently added first. `limit` is 1 to 50,
    /// default 8; `offset` skips that many (0 to 10000, default 0) for paging. Empty when the
    /// emote is unknown or not public.
    async fn emote_channels(
        &self,
        ctx: &Context<'_>,
        id: String,
        limit: Option<u32>,
        offset: Option<u32>,
    ) -> Result<Vec<Channel>> {
        catalog::emote_channels(state(ctx), &id, limit, offset)
            .await
            .map_err(api_error)
    }
}
