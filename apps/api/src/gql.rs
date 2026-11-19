//! The GraphQL API at `/v1/gql` (ADR 0008): accounts, channels, sets and editors, emotes and the
//! review gate, roles and the audit log, search, and the `events` subscription. Uploads are REST.
//!
//! Admin and moderation fields check their permission in `catalog`, `admin` and `moderation`,
//! each naming the one it needs. Introspection stays on, so the private parts are visible in the
//! schema but closed to anyone without the permission.

// The stubs take the arguments the real resolvers will use; they go away as each one lands.
#![allow(unused_variables)]

use async_graphql::http::GraphiQLSource;
use async_graphql::{
    Context, ErrorExtensions, MaybeUndefined, Object, Result, Schema, SchemaBuilder,
};
use axum::response::Html;
use chrono::SecondsFormat;
use cyex_tv_auth::{AuthError, Caller};
use cyex_tv_db::{DbError, accounts, channels, ids};
use cyex_tv_hub::Message as HubMessage;
use cyex_tv_hub::topics as hub_topics;
use cyex_tv_shared::error::ApiError;
use cyex_tv_shared::model::TrendingWindow;
use cyex_tv_shared::model::{
    AccountEditor, AccountReports, AccountResults, AccountSession, AccountStatus, AccountSummary,
    ActivityEntry, ActivityPage, AddAccountEditor, AddSetEmote, AdminAccount, AuditEntry, Badge,
    Channel, ClonedSet, CosmeticCatalog, CosmeticImage, CosmeticKind, CreateBadge, CreateRole,
    CreateSet, EditableChannel, EditedAccount, EditorPermission, Emote, EmoteFlag, EmoteNote,
    EmotePage, EmoteSet, EmoteSort, EmoteTransferRequest, EmoteTrending, EmoteVisibility,
    Entitlement, EntitlementPage, Event, GrantCosmetic, Identity, Me, Notification, OwnedCosmetics,
    Paint, PaintInput, PendingDeletion, PersonalSetEntry, Platform, QuickSearch, Report,
    ReportAction, ReportQueue, ReviewQueue, ReviewQueueFilter, Role, SearchResults, SetEmote,
    SetSummary, SetVisibility, UpdateBadge, UpdateChannelSettings, UpdateEmote, UpdateRole,
    UpdateSet, UpdateSetEmote, UserCosmetics,
};
use cyex_tv_shared::model::{
    Checkout, CodeBatch, CodeBatchInput, CodeBatchPage, CreatedCodeBatch, Order, OrderPage,
    OrderStatus, Recipient, RedeemResult, ShopItem, ShopItemInput,
};
use cyex_tv_shared::permissions;
use futures_util::Stream;
use sqlx::Connection;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::state::AppState;
use crate::{
    abuse, account, activity, admin, catalog, codes, cosmetics, editors, moderation, search, shop,
    staff, transfers,
};

pub type ApiSchema = Schema<Query, Mutation, Subscription>;

/// Depth and complexity limits apply to every request (ADR 0008).
pub fn builder() -> SchemaBuilder<Query, Mutation, Subscription> {
    Schema::build(Query, Mutation, Subscription)
        .limit_depth(12)
        .limit_complexity(1_000)
        .extension(SocketRunsSubscriptionsOnly)
        .extension(AppScopes)
}

/// App sign-in sessions (ADR 0021) run only the mutations their scopes name; every query stays
/// open to them. Checked per resolved field, so aliases and fragments change nothing.
struct AppScopes;

impl async_graphql::extensions::ExtensionFactory for AppScopes {
    fn create(&self) -> Arc<dyn async_graphql::extensions::Extension> {
        Arc::new(AppScopes)
    }
}

#[async_trait::async_trait]
impl async_graphql::extensions::Extension for AppScopes {
    async fn resolve(
        &self,
        ctx: &async_graphql::extensions::ExtensionContext<'_>,
        info: async_graphql::extensions::ResolveInfo<'_>,
        next: async_graphql::extensions::NextResolve<'_>,
    ) -> async_graphql::ServerResult<Option<async_graphql::Value>> {
        if info.parent_type == "Mutation"
            && let Some(client) = ctx
                .data_opt::<Option<Caller>>()
                .and_then(Option::as_ref)
                .and_then(|c| c.app_client.as_deref())
            && !cyex_tv_auth::app::allows_mutation(client, info.name)
        {
            return Err(coded(
                "insufficient_scope",
                "This app may not do that. Use cyex.tv for it.",
            )
            .into_server_error(async_graphql::Pos::default()));
        }
        next.run(ctx, info).await
    }
}

/// Subscription streams one `/v1/gql/ws` connection may hold at once.
pub const MAX_SOCKET_SUBSCRIPTIONS: usize = 8;

/// Connection data for one `/v1/gql/ws` socket: marks requests as coming over the socket and
/// counts the subscription streams it holds.
#[derive(Default)]
pub struct SocketConnection {
    streams: Arc<AtomicUsize>,
}

impl SocketConnection {
    /// Take a stream slot, or `None` when the connection holds the most it may.
    fn take_slot(&self) -> Option<StreamSlot> {
        let slot = StreamSlot(self.streams.clone());
        // Over the limit, the slot is given straight back as it drops.
        (self.streams.fetch_add(1, Ordering::AcqRel) < MAX_SOCKET_SUBSCRIPTIONS).then_some(slot)
    }
}

/// One subscription stream's place in its connection's count, given back when the stream ends.
struct StreamSlot(Arc<AtomicUsize>);

impl Drop for StreamSlot {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::AcqRel);
    }
}

/// The socket runs subscriptions only. async-graphql would otherwise run a query or mutation sent
/// as a `subscribe` message once, outside the `POST /v1/gql` rate limits. A document sent over the
/// socket may hold subscription operations only; over HTTP there is no [`SocketConnection`], so
/// everything passes.
struct SocketRunsSubscriptionsOnly;

impl async_graphql::extensions::ExtensionFactory for SocketRunsSubscriptionsOnly {
    fn create(&self) -> Arc<dyn async_graphql::extensions::Extension> {
        Arc::new(SocketRunsSubscriptionsOnly)
    }
}

#[async_trait::async_trait]
impl async_graphql::extensions::Extension for SocketRunsSubscriptionsOnly {
    async fn parse_query(
        &self,
        ctx: &async_graphql::extensions::ExtensionContext<'_>,
        query: &str,
        variables: &async_graphql::Variables,
        next: async_graphql::extensions::NextParseQuery<'_>,
    ) -> async_graphql::ServerResult<async_graphql::parser::types::ExecutableDocument> {
        let document = next.run(ctx, query, variables).await?;
        if ctx.data_opt::<SocketConnection>().is_some()
            && let Some((_, op)) = document.operations.iter().find(|(_, op)| {
                op.node.ty != async_graphql::parser::types::OperationType::Subscription
            })
        {
            return Err(coded(
                "subscriptions_only",
                "The socket runs subscriptions only. Send queries and mutations to POST /v1/gql.",
            )
            .into_server_error(op.pos));
        }
        Ok(document)
    }
}

/// The SDL, as committed in `schema/schema.graphql`.
pub fn sdl() -> String {
    builder().finish().sdl()
}

/// `GET /v1/gql` in a browser: GraphiQL against this API.
pub async fn graphiql() -> Html<String> {
    Html(
        GraphiQLSource::build()
            .endpoint("/v1/gql")
            .subscription_endpoint("/v1/gql/ws")
            .finish(),
    )
}

fn coded(code: &'static str, message: &'static str) -> async_graphql::Error {
    async_graphql::Error::new(message).extend_with(|_, e| e.set("code", code))
}

fn internal(err: impl std::fmt::Display) -> async_graphql::Error {
    tracing::error!(error = %err, "graphql resolver failed");
    coded("internal", "Something went wrong on our side.")
}

/// An API error as a GraphQL error, keeping its code.
fn api_error(err: ApiError) -> async_graphql::Error {
    async_graphql::Error::new(err.message).extend_with(move |_, e| e.set("code", err.code))
}

/// The client's IP as the limits saw it; empty when unknown (a test without the middleware).
fn client_ip<'a>(ctx: &Context<'a>) -> &'a str {
    ctx.data_opt::<crate::limits::ClientIp>()
        .map_or("", |ip| ip.0.as_str())
}

/// Takes one token of the heavy-reads limit for the public read being resolved.
fn heavy_read(ctx: &Context<'_>) -> Result<()> {
    crate::limits::heavy_read(state(ctx), caller_opt(ctx), client_ip(ctx)).map_err(api_error)
}

fn caller_opt<'a>(ctx: &Context<'a>) -> Option<&'a Caller> {
    ctx.data_opt::<Option<Caller>>().and_then(Option::as_ref)
}

fn auth_error(err: AuthError) -> async_graphql::Error {
    if let AuthError::Db(inner) = &err {
        return internal(inner);
    }
    async_graphql::Error::new(err.message()).extend_with(|_, e| e.set("code", err.code()))
}

fn state<'a>(ctx: &Context<'a>) -> &'a AppState {
    // `app()` puts the state into the schema; a schema without it is a wiring bug.
    ctx.data_unchecked::<AppState>()
}

/// The cookie session of an account whose deletion can still be undone (ADR 0019), when the
/// request has no live session. Only `pendingDeletion` and `restoreAccount` read it.
pub struct Restorable(pub Option<Caller>);

/// The restorable session, or `signed_out`.
fn restorable<'a>(ctx: &Context<'a>) -> Result<&'a Caller> {
    ctx.data_opt::<Restorable>()
        .and_then(|r| r.0.as_ref())
        .ok_or_else(|| {
            coded(
                "signed_out",
                "Sign in to the deleted account to restore it.",
            )
        })
}

/// The signed-in caller, or `signed_out`.
fn caller<'a>(ctx: &Context<'a>) -> Result<&'a Caller> {
    ctx.data_opt::<Option<Caller>>()
        .and_then(Option::as_ref)
        .ok_or_else(|| coded("signed_out", "Sign in to use this."))
}

/// The account behind `caller`, as `Me`.
async fn me_of(ctx: &Context<'_>, caller: &Caller) -> Result<Me> {
    let mut conn = state(ctx).db.acquire().await.map_err(internal)?;
    let account = accounts::get(&mut conn, caller.account_id)
        .await
        .map_err(internal)?
        .ok_or_else(|| coded("signed_out", "Sign in to use this."))?;
    let identities = accounts::identities(&mut conn, account.id)
        .await
        .map_err(internal)?
        .into_iter()
        .map(|i| Identity {
            platform: i.platform,
            platform_user_id: i.platform_user_id,
            login: i.login,
            display_name: i.display_name,
            avatar: i.avatar,
        })
        .collect();
    let permissions = catalog::permissions_of(&mut conn, caller)
        .await
        .map_err(api_error)?;
    let display_platform = accounts::display_platform(&mut conn, account.id)
        .await
        .map_err(internal)?;
    let roles = cyex_tv_db::roles::roles_of(&mut conn, account.id)
        .await
        .map_err(internal)?
        .into_iter()
        .map(|r| cyex_tv_shared::model::OwnRole {
            id: ids::to_text(r.id),
            name: r.name,
            public: r.public,
        })
        .collect();
    Ok(Me {
        id: ids::to_text(account.id),
        identities,
        permissions,
        roles,
        created_at: account
            .created_at
            .to_rfc3339_opts(SecondsFormat::Millis, true),
        display_platform,
    })
}

pub struct Query;

#[Object]
impl Query {
    /// The signed-in account.
    async fn me(&self, ctx: &Context<'_>) -> Result<Me> {
        let caller = caller(ctx)?;
        me_of(ctx, caller).await
    }

    /// The deletion you can still undo: set only for the cookie session that signing in to a
    /// deleted account within its 30 days gave (`restoreAccount` undoes it). Null otherwise,
    /// including for every normal session.
    async fn pending_deletion(&self, ctx: &Context<'_>) -> Result<Option<PendingDeletion>> {
        let Some(pending) = ctx.data_opt::<Restorable>().and_then(|r| r.0.as_ref()) else {
            return Ok(None);
        };
        account::pending_deletion(state(ctx), pending)
            .await
            .map_err(api_error)
    }

    /// A channel by its platform ID.
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

    /// A channel by its login or slug. Logins change; store the ID, not the login.
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

    /// A set with its emotes. Null when there is none, or when it is `PRIVATE` and you are not
    /// its owner, an editor or staff (a channel's active set is always public).
    async fn set(&self, ctx: &Context<'_>, id: String) -> Result<Option<EmoteSet>> {
        catalog::set(state(ctx), caller_opt(ctx), &id)
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

    /// An emote. Pending ones are visible to their owner and to moderators only.
    async fn emote(&self, ctx: &Context<'_>, id: String) -> Result<Option<Emote>> {
        moderation::emote(state(ctx), caller_opt(ctx), &id)
            .await
            .map_err(api_error)
    }

    /// Approved, listed emotes by name and tag, best match first, then the most used.
    /// `perPage` is 1 to 100, default 50. NSFW emotes only with `nsfw: true`. `sort` is
    /// `RELEVANCE` (default: best match, then most used), `TOP` (most used, then name), `NEW`
    /// (newest first) or `TRENDING` (most channels added it in `window`, net of removals, then
    /// most used; only emotes with a net gain are listed). An empty `query` lists every approved,
    /// listed emote.
    /// `flags` keeps the emotes that have every flag listed (`ZERO_WIDTH`, `WIDE`, `NSFW`; at most
    /// 3 different ones). `NSFW` lists only NSFW emotes, with or without `nsfw: true`. `tags` keeps
    /// the emotes that have every tag listed (exact; a leading `#` and case are ignored, at most
    /// 10). `exact: true` makes a non-empty `query` match the whole name (any case) instead of
    /// roughly. `window` is `DAY`, `WEEK` (default) or `MONTH`, and only matters for `TRENDING`.
    #[allow(clippy::too_many_arguments)]
    async fn search_emotes(
        &self,
        ctx: &Context<'_>,
        query: String,
        animated: Option<bool>,
        owner: Option<String>,
        nsfw: Option<bool>,
        page: Option<u32>,
        per_page: Option<u32>,
        sort: Option<EmoteSort>,
        flags: Option<Vec<EmoteFlag>>,
        tags: Option<Vec<String>>,
        exact: Option<bool>,
        window: Option<TrendingWindow>,
    ) -> Result<SearchResults> {
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
                nsfw: nsfw.unwrap_or(false),
                flags: &flags,
                tags: tags.as_deref().unwrap_or_default(),
                exact: exact.unwrap_or(false),
                window: window.unwrap_or_default(),
                sort: sort.unwrap_or_default(),
                page,
                per_page,
            },
        )
        .await
        .map_err(api_error)
    }

    /// Where an emote stands among the 100 most added in `window` (`DAY`, `WEEK` by default or
    /// `MONTH`): its rank and the channels that added it, net of removals. Null when `id` isn't
    /// an emote ID, the emote isn't approved and listed, or it is not in the top 100.
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

    /// An account's public activity, newest first, like 7TV's "Activity" tab: sets and emotes the
    /// owner or their editors changed, the active set, badge and paint picks, approved uploads.
    /// Page back with `before` (an entry ID); `limit` is 1 to 100, default 30. Empty when the
    /// account is unknown, deleted or blocked. Nothing private or from moderation shows. A page
    /// can be shorter than `limit` and still not the last: use `accountActivityPage` to know.
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
        activity::account_activity(state(ctx), &account_id, before.as_deref(), limit)
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
    #[graphql(complexity = "100 + child_complexity")]
    async fn emote_activity_page(
        &self,
        ctx: &Context<'_>,
        emote_id: String,
        before: Option<String>,
        limit: Option<u32>,
    ) -> Result<ActivityPage> {
        heavy_read(ctx)?;
        activity::emote_activity_page(state(ctx), &emote_id, before.as_deref(), limit)
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
    async fn my_emotes(
        &self,
        ctx: &Context<'_>,
        page: Option<u32>,
        per_page: Option<u32>,
    ) -> Result<EmotePage> {
        let caller = caller(ctx)?;
        catalog::my_emotes(state(ctx), caller, page, per_page)
            .await
            .map_err(api_error)
    }

    /// A set's editors. For its owner, its editors and `sets.edit_any`.
    async fn set_editors(&self, ctx: &Context<'_>, set_id: String) -> Result<Vec<AccountSummary>> {
        let caller = caller(ctx)?;
        catalog::set_editors(state(ctx), caller, &set_id)
            .await
            .map_err(api_error)
    }

    /// Your channels: one per linked platform you have signed in with.
    async fn my_channels(&self, ctx: &Context<'_>) -> Result<Vec<Channel>> {
        let caller = caller(ctx)?;
        catalog::my_channels(state(ctx), caller)
            .await
            .map_err(api_error)
    }

    /// Channels you can manage: yours (`OWNER`), those of accounts you are an account editor of
    /// (`ACCOUNT_EDITOR`), and those of accounts where you edit at least one set (`SET_EDITOR`).
    /// Each channel once, under your highest role; sorted by role (`OWNER`, `ACCOUNT_EDITOR`,
    /// `SET_EDITOR`), then login (ignoring case), then platform. Signed-in only.
    async fn my_editable_channels(&self, ctx: &Context<'_>) -> Result<Vec<EditableChannel>> {
        let caller = caller(ctx)?;
        editors::my_editable_channels(state(ctx), caller)
            .await
            .map_err(api_error)
    }

    /// A channel, by its platform ID (`id`) or its login (`login`; one of the two), with the sets
    /// of its account you may add, rename and remove emotes in, your role on it and whether its
    /// active set is among them. The same rights check as `addSetEmote`, `updateSetEmote` and
    /// `removeSetEmote` (owner, account editors with `EMOTES` covering the set, set editors,
    /// `sets.edit_any`). Null when the channel is unknown. Signed-in only.
    async fn channel_editable_sets(
        &self,
        ctx: &Context<'_>,
        platform: Platform,
        id: Option<String>,
        login: Option<String>,
    ) -> Result<Option<cyex_tv_shared::model::ChannelEditableSets>> {
        let caller = caller(ctx)?;
        catalog::channel_editable_sets(
            state(ctx),
            caller,
            platform,
            id.as_deref(),
            login.as_deref(),
        )
        .await
        .map_err(api_error)
    }

    /// An account's editors (ADR 0014): yours, or, with `accountId`, those of an account you edit
    /// with the `EDITORS` permission, or anyone's with `editors.manage_any`.
    async fn account_editors(
        &self,
        ctx: &Context<'_>,
        account_id: Option<String>,
    ) -> Result<Vec<AccountEditor>> {
        let caller = caller(ctx)?;
        editors::account_editors(state(ctx), caller, account_id.as_deref())
            .await
            .map_err(api_error)
    }

    /// Open emote transfer requests you take part in (ADR 0014): those waiting for you to accept
    /// or decline (your emotes), and those you made as an editor; of one emote when given.
    async fn emote_transfer_requests(
        &self,
        ctx: &Context<'_>,
        emote_id: Option<String>,
    ) -> Result<Vec<EmoteTransferRequest>> {
        let caller = caller(ctx)?;
        transfers::open_requests(state(ctx), caller, emote_id.as_deref())
            .await
            .map_err(api_error)
    }

    /// The accounts you edit, with what you may do on each.
    async fn editor_of(&self, ctx: &Context<'_>) -> Result<Vec<EditedAccount>> {
        let caller = caller(ctx)?;
        editors::editor_of(state(ctx), caller)
            .await
            .map_err(api_error)
    }

    /// The sets of the accounts you edit. Each set names its owner.
    async fn editor_sets(&self, ctx: &Context<'_>) -> Result<Vec<EmoteSet>> {
        let caller = caller(ctx)?;
        editors::editor_sets(state(ctx), caller)
            .await
            .map_err(api_error)
    }

    /// The channels of the accounts you edit.
    async fn editor_channels(&self, ctx: &Context<'_>) -> Result<Vec<Channel>> {
        let caller = caller(ctx)?;
        editors::editor_channels(state(ctx), caller)
            .await
            .map_err(api_error)
    }

    /// Every cyex.tv chat badge and name paint (ADR 0015). Public; clients cache it
    /// (`GET /v1/cosmetics` has an ETag) and follow the `cosmetics` topic.
    async fn cosmetics(&self, ctx: &Context<'_>) -> Result<CosmeticCatalog> {
        cosmetics::catalog(state(ctx)).await.map_err(api_error)
    }

    /// The badge and paint up to 100 chatters show, as catalog IDs. Only chatters showing
    /// something are listed.
    async fn user_cosmetics(
        &self,
        ctx: &Context<'_>,
        platform: Platform,
        user_ids: Vec<String>,
    ) -> Result<Vec<UserCosmetics>> {
        cosmetics::user_cosmetics(state(ctx), platform, &user_ids)
            .await
            .map_err(api_error)
    }

    /// The cosmetics you own and show; with `accountId`, those of an account you edit with
    /// `PROFILE`.
    async fn my_cosmetics(
        &self,
        ctx: &Context<'_>,
        account_id: Option<String>,
    ) -> Result<OwnedCosmetics> {
        let caller = caller(ctx)?;
        cosmetics::my_cosmetics(state(ctx), caller, account_id.as_deref())
            .await
            .map_err(api_error)
    }

    /// A badge or paint picture, to see whether its files are made (`ready`). Needs
    /// `cosmetics.manage`.
    async fn cosmetic_image(&self, ctx: &Context<'_>, id: String) -> Result<Option<CosmeticImage>> {
        let caller = caller(ctx)?;
        cosmetics::image(state(ctx), caller, &id)
            .await
            .map_err(api_error)
    }

    /// Who owns which badges or paints, newest first; narrow it with `accountId` and/or
    /// `cosmeticId`. `perPage` is 1 to 100, default 50. Needs `cosmetics.grant`.
    #[allow(clippy::too_many_arguments)]
    async fn cosmetic_entitlements(
        &self,
        ctx: &Context<'_>,
        kind: CosmeticKind,
        account_id: Option<String>,
        cosmetic_id: Option<String>,
        page: Option<u32>,
        per_page: Option<u32>,
    ) -> Result<EntitlementPage> {
        let caller = caller(ctx)?;
        cosmetics::entitlements(
            state(ctx),
            caller,
            kind,
            account_id.as_deref(),
            cosmetic_id.as_deref(),
            page,
            per_page,
        )
        .await
        .map_err(api_error)
    }

    /// Shop items on sale now, by sort order (ADR 0016). Public; signed in, `owned` says whether
    /// you own everything in each. With `all: true`, every item that isn't deleted, hidden and
    /// scheduled ones too (needs `shop.manage`).
    async fn shop_items(&self, ctx: &Context<'_>, all: Option<bool>) -> Result<Vec<ShopItem>> {
        shop::items(state(ctx), caller_opt(ctx), all.unwrap_or(false))
            .await
            .map_err(api_error)
    }

    /// One shop item: on sale for everyone; any, deleted ones too, with `shop.manage`.
    async fn shop_item(&self, ctx: &Context<'_>, id: String) -> Result<Option<ShopItem>> {
        shop::item(state(ctx), caller_opt(ctx), &id)
            .await
            .map_err(api_error)
    }

    /// Orders you bought, and gifts you got once paid (without what the buyer paid), newest
    /// first. `perPage` 1 to 100, default 50.
    async fn my_orders(
        &self,
        ctx: &Context<'_>,
        page: Option<u32>,
        per_page: Option<u32>,
    ) -> Result<OrderPage> {
        let caller = caller(ctx)?;
        shop::my_orders(state(ctx), caller, page, per_page)
            .await
            .map_err(api_error)
    }

    /// One order, for its buyer, its recipient (once paid, without what the buyer paid), or
    /// `shop.orders`. Poll it after `startCheckout` until it is `PAID`.
    async fn order(&self, ctx: &Context<'_>, id: String) -> Result<Option<Order>> {
        let caller = caller(ctx)?;
        shop::order(state(ctx), caller, &id)
            .await
            .map_err(api_error)
    }

    /// Every order, newest first; narrow by status, account (bought or received) and item.
    /// Needs `shop.orders`, or `billing.view` narrowed to one account.
    #[allow(clippy::too_many_arguments)]
    async fn orders(
        &self,
        ctx: &Context<'_>,
        status: Option<OrderStatus>,
        account_id: Option<String>,
        item_id: Option<String>,
        page: Option<u32>,
        per_page: Option<u32>,
    ) -> Result<OrderPage> {
        let caller = caller(ctx)?;
        shop::orders(
            state(ctx),
            caller,
            status,
            account_id.as_deref(),
            item_id.as_deref(),
            page,
            per_page,
        )
        .await
        .map_err(api_error)
    }

    /// Redeem code batches, newest first. Needs `codes.manage`. The codes themselves are shown
    /// only once, by `createCodeBatch`.
    async fn code_batches(
        &self,
        ctx: &Context<'_>,
        page: Option<u32>,
        per_page: Option<u32>,
    ) -> Result<CodeBatchPage> {
        let caller = caller(ctx)?;
        codes::batches(state(ctx), caller, page, per_page)
            .await
            .map_err(api_error)
    }

    /// Every permission name a role can hold.
    async fn permissions(&self) -> Vec<String> {
        permissions::ALL.iter().map(|p| (*p).to_owned()).collect()
    }

    /// Pending emotes, oldest first. Needs `emotes.review`. `perPage` is 1 to 100, default 50.
    /// With `filter`: one tab (public, unlisted and private, or resolved), a search by name or
    /// uploader, and an order.
    async fn review_queue(
        &self,
        ctx: &Context<'_>,
        page: Option<u32>,
        per_page: Option<u32>,
        filter: Option<ReviewQueueFilter>,
    ) -> Result<ReviewQueue> {
        let caller = caller(ctx)?;
        moderation::review_queue(state(ctx), caller, page, per_page, filter.as_ref())
            .await
            .map_err(api_error)
    }

    /// Reviewers' notes on an emote, newest first (the latest 200). Needs `emotes.review`.
    async fn emote_notes(&self, ctx: &Context<'_>, emote_id: String) -> Result<Vec<EmoteNote>> {
        let caller = caller(ctx)?;
        moderation::notes(state(ctx), caller, &emote_id)
            .await
            .map_err(api_error)
    }

    /// What happened to one emote, newest first: the latest 100 audit entries about it
    /// (upload, edits, reviews, notes, takedowns). Needs `emotes.review`.
    async fn emote_history(&self, ctx: &Context<'_>, emote_id: String) -> Result<Vec<AuditEntry>> {
        let caller = caller(ctx)?;
        moderation::history(state(ctx), caller, &emote_id)
            .await
            .map_err(api_error)
    }

    /// An account's roles. Yours, or anyone's with `roles.manage` or `users.view`.
    async fn account_roles(&self, ctx: &Context<'_>, account_id: String) -> Result<Vec<Role>> {
        let caller = caller(ctx)?;
        admin::account_roles(state(ctx), caller, &account_id)
            .await
            .map_err(api_error)
    }

    /// Audited changes, newest first. Needs `audit.view`. Narrow by target, `actorId`, an exact
    /// `action`, and `since` (inclusive) / `until` (exclusive) as RFC 3339 times. Page back with
    /// `before` (an entry ID); `limit` is 1 to 100, default 50.
    #[allow(clippy::too_many_arguments)]
    async fn audit_log(
        &self,
        ctx: &Context<'_>,
        target_kind: Option<String>,
        target_id: Option<String>,
        actor_id: Option<String>,
        action: Option<String>,
        since: Option<String>,
        until: Option<String>,
        before: Option<String>,
        limit: Option<u32>,
    ) -> Result<Vec<AuditEntry>> {
        let caller = caller(ctx)?;
        admin::audit_log(
            state(ctx),
            caller,
            &admin::AuditQuery {
                target_kind: target_kind.as_deref(),
                target_id: target_id.as_deref(),
                actor_id: actor_id.as_deref(),
                action: action.as_deref(),
                since: since.as_deref(),
                until: until.as_deref(),
            },
            before.as_deref(),
            limit,
        )
        .await
        .map_err(api_error)
    }

    /// Open reports, oldest first. Needs `emotes.review` or `emotes.takedown`.
    async fn report_queue(
        &self,
        ctx: &Context<'_>,
        page: Option<u32>,
        per_page: Option<u32>,
    ) -> Result<ReportQueue> {
        let caller = caller(ctx)?;
        abuse::queue(state(ctx), caller, page, per_page)
            .await
            .map_err(api_error)
    }

    /// Reports about an account's emotes and the ones it filed, newest first, at most 50 each.
    /// Needs `emotes.review` or `emotes.takedown`.
    async fn account_reports(
        &self,
        ctx: &Context<'_>,
        account_id: String,
    ) -> Result<AccountReports> {
        let caller = caller(ctx)?;
        abuse::account_reports(state(ctx), caller, &account_id)
            .await
            .map_err(api_error)
    }

    /// Where you are signed in: your live sessions, the most recently used first, with the one
    /// this request came with marked `current`. Never shows tokens.
    async fn my_sessions(&self, ctx: &Context<'_>) -> Result<Vec<AccountSession>> {
        let caller = caller(ctx)?;
        account::my_sessions(state(ctx), caller)
            .await
            .map_err(api_error)
    }

    /// An account as staff see it: identities, created time and restrictions. Needs
    /// `users.view` or `users.restrict`.
    async fn admin_account(&self, ctx: &Context<'_>, account_id: String) -> Result<AdminAccount> {
        let caller = caller(ctx)?;
        abuse::admin_account(state(ctx), caller, &account_id)
            .await
            .map_err(api_error)
    }

    /// How many of your notifications are unread.
    async fn unread_notification_count(&self, ctx: &Context<'_>) -> Result<u64> {
        let caller = caller(ctx)?;
        account::unread_count(state(ctx), caller)
            .await
            .map_err(api_error)
    }

    /// An account's emotes in every state, newest first. Needs `users.view`, `emotes.review`,
    /// `emotes.takedown`, `emotes.edit_any` or `emotes.delete_any`, or to be the account or its
    /// editor with `EMOTE_MANAGE` on every set. `perPage` is 1 to 100, default 50.
    async fn account_emotes(
        &self,
        ctx: &Context<'_>,
        account_id: String,
        page: Option<u32>,
        per_page: Option<u32>,
    ) -> Result<EmotePage> {
        let caller = caller(ctx)?;
        moderation::account_emotes(state(ctx), caller, &account_id, page, per_page)
            .await
            .map_err(api_error)
    }

    /// Your notifications, newest first (the latest 100). `kind` is one of `emote.approved`,
    /// `emote.rejected`, `emote.taken_down`, `editor.added`, `editor.changed`, `editor.removed`,
    /// `cosmetic.granted` and `cosmetic.revoked`; `payload` is JSON with the IDs and names it is
    /// about (see the API reference). Cosmetics from the shop and from codes carry `source`
    /// (`purchase` or `code`), and from orders `order_id`.
    async fn my_notifications(
        &self,
        ctx: &Context<'_>,
        unread_only: Option<bool>,
    ) -> Result<Vec<Notification>> {
        let caller = caller(ctx)?;
        abuse::my_notifications(state(ctx), caller, unread_only.unwrap_or(false))
            .await
            .map_err(api_error)
    }

    /// Every restriction placed on an account, newest first (the latest 100), running or over,
    /// with its reason and end. Needs `users.view`, `users.restrict` or a narrower restrict
    /// permission.
    async fn account_restrictions(
        &self,
        ctx: &Context<'_>,
        account_id: String,
    ) -> Result<Vec<cyex_tv_shared::model::Restriction>> {
        let caller = caller(ctx)?;
        abuse::restrictions(state(ctx), caller, &account_id)
            .await
            .map_err(api_error)
    }

    /// What an account may do now: uploads, reports and sign-in, each with its end when timed.
    /// Needs `users.view`, `users.restrict` or a narrower restrict permission.
    async fn account_status(&self, ctx: &Context<'_>, account_id: String) -> Result<AccountStatus> {
        let caller = caller(ctx)?;
        abuse::status(state(ctx), caller, &account_id)
            .await
            .map_err(api_error)
    }

    /// Staff notes on an account, newest first (the latest 200). Needs `users.notes`.
    async fn staff_notes(
        &self,
        ctx: &Context<'_>,
        account_id: String,
    ) -> Result<Vec<cyex_tv_shared::model::StaffNote>> {
        let caller = caller(ctx)?;
        staff::notes(state(ctx), caller, &account_id)
            .await
            .map_err(api_error)
    }

    /// Where an account is signed in, the most recently used first. Never tokens. Needs
    /// `users.sessions`.
    async fn account_sessions(
        &self,
        ctx: &Context<'_>,
        account_id: String,
    ) -> Result<Vec<AccountSession>> {
        let caller = caller(ctx)?;
        staff::account_sessions(state(ctx), caller, &account_id)
            .await
            .map_err(api_error)
    }

    /// An account's subscriptions: past, current and to come. There are no subscriptions yet,
    /// so this is empty until there are. Needs `billing.view`, `subscriptions.manage` or
    /// `shop.orders`.
    async fn account_subscriptions(
        &self,
        ctx: &Context<'_>,
        account_id: String,
    ) -> Result<Vec<cyex_tv_shared::model::AccountSubscription>> {
        let caller = caller(ctx)?;
        staff::account_subscriptions(state(ctx), caller, &account_id)
            .await
            .map_err(api_error)
    }

    /// The entitlement graph of an account: every badge, paint and role it holds or held, where
    /// each came from (staff, an order, a redeem code, an event, the User role, the owner list),
    /// who gave it and when, and when and by whom it was taken away. Held ones first. Needs
    /// `entitlements.view` or `cosmetics.grant`.
    async fn entitlement_graph(
        &self,
        ctx: &Context<'_>,
        account_id: String,
    ) -> Result<Vec<cyex_tv_shared::model::EntitlementGrant>> {
        let caller = caller(ctx)?;
        staff::graph(state(ctx), caller, &account_id)
            .await
            .map_err(api_error)
    }

    /// The reverse: who holds or held one badge, paint or role, and how each got it (the latest
    /// 200 of each kind of record). Not for the User and top roles (`implicit_role`). Needs
    /// `entitlements.view` or `cosmetics.grant`.
    async fn entitlement_holders(
        &self,
        ctx: &Context<'_>,
        kind: cyex_tv_shared::model::GrantKind,
        id: String,
    ) -> Result<Vec<cyex_tv_shared::model::EntitlementGrant>> {
        let caller = caller(ctx)?;
        staff::holders(state(ctx), caller, kind, &id)
            .await
            .map_err(api_error)
    }

    /// All roles, most senior first, each with whether you may edit or assign it. Needs
    /// `roles.manage` or `roles.assign`. The top role is listed to the owners only.
    async fn roles(&self, ctx: &Context<'_>) -> Result<Vec<Role>> {
        let caller = caller(ctx)?;
        admin::list(state(ctx), caller).await.map_err(api_error)
    }
}

pub struct Mutation;

#[Object]
impl Mutation {
    /// Unlink a platform from your account. The last identity can't be removed, and a
    /// restricted or blocked account can't unlink (`restricted`).
    async fn unlink_identity(&self, ctx: &Context<'_>, platform: Platform) -> Result<Me> {
        let caller = caller(ctx)?;
        let mut conn = state(ctx).db.acquire().await.map_err(internal)?;
        let mut tx = conn.begin().await.map_err(internal)?;
        // Locked, so a restriction landing at the same time is seen. Signing in again with the
        // dropped identity would otherwise make a fresh, unrestricted account.
        let restricted: Option<bool> = sqlx::query_scalar(
            "SELECT (restricted_at IS NOT NULL AND (restricted_until IS NULL OR restricted_until > now()))
                 OR (blocked_at IS NOT NULL AND (blocked_until IS NULL OR blocked_until > now()))
                 OR (tickets_blocked_at IS NOT NULL
                     AND (tickets_blocked_until IS NULL OR tickets_blocked_until > now()))
               FROM accounts WHERE id = $1 FOR SHARE",
        )
        .bind(caller.account_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(internal)?;
        if restricted == Some(true) {
            return Err(api_error(ApiError::new(
                axum::http::StatusCode::FORBIDDEN,
                "restricted",
                "A restricted account can't unlink a platform. Write to us if you need it changed.",
            )));
        }
        match accounts::unlink(&mut tx, caller.account_id, platform).await {
            Ok(_) => {}
            Err(DbError::NotFound) => {
                return Err(coded(
                    "not_found",
                    "That platform isn't linked to your account.",
                ));
            }
            Err(err) => return Err(auth_error(err.into())),
        }
        // The account no longer proves it owns that platform's channel.
        channels::delete_for(&mut tx, caller.account_id, platform)
            .await
            .map_err(internal)?;
        tx.commit().await.map_err(internal)?;
        drop(conn);
        me_of(ctx, caller).await
    }

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

    /// Create a channel set, or your personal set (one per account). With `ownerId`, for an
    /// account you edit with `SETS`.
    async fn create_set(&self, ctx: &Context<'_>, input: CreateSet) -> Result<EmoteSet> {
        let caller = caller(ctx)?;
        catalog::create_set(state(ctx), caller, &input)
            .await
            .map_err(api_error)
    }

    /// Copy a set you can open into a new channel set: yours, or with `ownerId` one for an account
    /// you edit with `SETS`. Name defaults to the source's; visibility to `LISTED`. Emotes go in
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

    /// Rename a set, change who sees it (`visibility`: owner or account editors with `SETS`), or
    /// change its capacity. Owner or editor; capacity needs `sets.edit_any`.
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

    /// Make someone an editor of your account, of an account you edit with `EDITORS` (ADR
    /// 0014), or of anyone's with `editors.manage_any` (audited with `via`). Give their account ID, or a platform and login. You can only grant permissions you
    /// hold. `setIds` limits them to some of the account's sets; leave it out for every set, new
    /// ones included. At most 50 editors per account (`too_many_editors`). Audited.
    async fn add_account_editor(
        &self,
        ctx: &Context<'_>,
        input: AddAccountEditor,
    ) -> Result<AccountEditor> {
        let caller = caller(ctx)?;
        editors::add(state(ctx), caller, &input)
            .await
            .map_err(api_error)
    }

    /// Change an account editor's permissions. `ownerId` is the account edited; empty for yours.
    /// The owner, an editor with `EDITORS`, or `editors.manage_any`.
    /// `setIds` changes which sets they work on: empty for every set, left out to keep them.
    /// Audited.
    async fn update_account_editor(
        &self,
        ctx: &Context<'_>,
        owner_id: Option<String>,
        account_id: String,
        permissions: Vec<EditorPermission>,
        set_ids: Option<Vec<String>>,
    ) -> Result<AccountEditor> {
        let caller = caller(ctx)?;
        editors::update(
            state(ctx),
            caller,
            owner_id.as_deref(),
            &account_id,
            &permissions,
            set_ids.as_deref(),
        )
        .await
        .map_err(api_error)
    }

    /// Remove an account editor: the owner, an editor with `EDITORS`, `editors.manage_any`, or the editor themselves
    /// (with `ownerId` set to the account they leave). False when they weren't one. Audited.
    async fn remove_account_editor(
        &self,
        ctx: &Context<'_>,
        owner_id: Option<String>,
        account_id: String,
    ) -> Result<bool> {
        let caller = caller(ctx)?;
        editors::remove(state(ctx), caller, owner_id.as_deref(), &account_id)
            .await
            .map_err(api_error)
    }

    /// Pick the badge and paint you show, from what you own (ADR 0015). Leave a field out to keep
    /// it; send null or an empty string to show none. With `ownerId`, for an account you edit
    /// with `PROFILE`.
    async fn select_cosmetics(
        &self,
        ctx: &Context<'_>,
        badge_id: MaybeUndefined<String>,
        paint_id: MaybeUndefined<String>,
        owner_id: Option<String>,
    ) -> Result<OwnedCosmetics> {
        let caller = caller(ctx)?;
        let pick = |m: MaybeUndefined<String>| match m {
            MaybeUndefined::Undefined => cosmetics::Pick::Keep,
            MaybeUndefined::Null => cosmetics::Pick::Clear,
            MaybeUndefined::Value(v) => cosmetics::Pick::Set(v),
        };
        cosmetics::select(
            state(ctx),
            caller,
            owner_id.as_deref(),
            &pick(badge_id),
            &pick(paint_id),
        )
        .await
        .map_err(api_error)
    }

    /// Make a badge from a ready badge picture (`POST /v1/cosmetics/images`). Needs
    /// `cosmetics.manage`. Audited; publishes `cosmetic.created`.
    async fn create_badge(&self, ctx: &Context<'_>, input: CreateBadge) -> Result<Badge> {
        let caller = caller(ctx)?;
        cosmetics::create_badge(state(ctx), caller, &input)
            .await
            .map_err(api_error)
    }

    /// Change a badge's name, tooltip or picture. Needs `cosmetics.manage`. Audited.
    async fn update_badge(
        &self,
        ctx: &Context<'_>,
        id: String,
        input: UpdateBadge,
    ) -> Result<Badge> {
        let caller = caller(ctx)?;
        cosmetics::update_badge(state(ctx), caller, &id, &input)
            .await
            .map_err(api_error)
    }

    /// Delete a badge: everyone loses it and stops showing it. False when there was none. Needs
    /// `cosmetics.manage`. Audited.
    async fn delete_badge(&self, ctx: &Context<'_>, id: String) -> Result<bool> {
        let caller = caller(ctx)?;
        cosmetics::delete_badge(state(ctx), caller, &id)
            .await
            .map_err(api_error)
    }

    /// Make a paint. Needs `cosmetics.manage`. Audited; publishes `cosmetic.created`.
    async fn create_paint(&self, ctx: &Context<'_>, input: PaintInput) -> Result<Paint> {
        let caller = caller(ctx)?;
        cosmetics::create_paint(state(ctx), caller, &input)
            .await
            .map_err(api_error)
    }

    /// Replace every field of a paint. Needs `cosmetics.manage`. Audited.
    async fn update_paint(
        &self,
        ctx: &Context<'_>,
        id: String,
        input: PaintInput,
    ) -> Result<Paint> {
        let caller = caller(ctx)?;
        cosmetics::update_paint(state(ctx), caller, &id, &input)
            .await
            .map_err(api_error)
    }

    /// Delete a paint: everyone loses it and stops showing it. False when there was none. Needs
    /// `cosmetics.manage`. Audited.
    async fn delete_paint(&self, ctx: &Context<'_>, id: String) -> Result<bool> {
        let caller = caller(ctx)?;
        cosmetics::delete_paint(state(ctx), caller, &id)
            .await
            .map_err(api_error)
    }

    /// Give an account a badge or paint, with a reason. Needs `cosmetics.grant`. Audited.
    async fn grant_cosmetic(&self, ctx: &Context<'_>, input: GrantCosmetic) -> Result<Entitlement> {
        let caller = caller(ctx)?;
        cosmetics::grant(state(ctx), caller, &input)
            .await
            .map_err(api_error)
    }

    /// Take a badge or paint away; if they showed it, it stops. False when they didn't own it.
    /// Needs `cosmetics.grant`. Audited.
    async fn revoke_cosmetic(
        &self,
        ctx: &Context<'_>,
        account_id: String,
        kind: CosmeticKind,
        cosmetic_id: String,
        reason: Option<String>,
    ) -> Result<bool> {
        let caller = caller(ctx)?;
        cosmetics::revoke(
            state(ctx),
            caller,
            &account_id,
            kind,
            &cosmetic_id,
            reason.as_deref(),
        )
        .await
        .map_err(api_error)
    }

    /// Make a shop item: a bundle of 1 to 10 badges and paints. Needs `shop.manage`. Audited.
    async fn create_shop_item(&self, ctx: &Context<'_>, input: ShopItemInput) -> Result<ShopItem> {
        let caller = caller(ctx)?;
        shop::create_item(state(ctx), caller, &input)
            .await
            .map_err(api_error)
    }

    /// Replace every field of a shop item. Orders made already keep their price. Needs
    /// `shop.manage`. Audited.
    async fn update_shop_item(
        &self,
        ctx: &Context<'_>,
        id: String,
        input: ShopItemInput,
    ) -> Result<ShopItem> {
        let caller = caller(ctx)?;
        shop::update_item(state(ctx), caller, &id, &input)
            .await
            .map_err(api_error)
    }

    /// Delete a shop item: never listed or sold again; its orders stay. False when there was
    /// none. Needs `shop.manage`. Audited.
    async fn delete_shop_item(&self, ctx: &Context<'_>, id: String) -> Result<bool> {
        let caller = caller(ctx)?;
        shop::delete_item(state(ctx), caller, &id)
            .await
            .map_err(api_error)
    }

    /// Start buying an item for yourself, or as a gift with `recipient`. Answers the Paddle
    /// transaction to open in Paddle's checkout; the order turns `PAID` once Paddle confirms.
    /// `checkout_unavailable` while Paddle isn't set up or the item has no Paddle price. Asking
    /// again for the same item and recipient within the hour hands out the same transaction
    /// (one pending order each); `checkout_busy` while another request is starting it.
    async fn start_checkout(
        &self,
        ctx: &Context<'_>,
        item_id: String,
        recipient: Option<Recipient>,
    ) -> Result<Checkout> {
        let caller = caller(ctx)?;
        shop::start_checkout(state(ctx), caller, &item_id, recipient.as_ref())
            .await
            .map_err(api_error)
    }

    /// Refund a paid order: asks Paddle for a full refund when it is set up, and takes the
    /// cosmetics back at once (only those no other paid order, code or staff grant covers).
    /// `already_refunded` when Paddle says it was refunded already; `refund_in_progress` while
    /// another refund of it runs (it may be tried again after 10 minutes); `refund_unavailable`
    /// when the order has a Paddle transaction and Paddle isn't set up. Needs `shop.orders`.
    /// Audited.
    async fn refund_order(&self, ctx: &Context<'_>, id: String, reason: String) -> Result<Order> {
        let caller = caller(ctx)?;
        shop::refund_order(state(ctx), caller, &id, &reason)
            .await
            .map_err(api_error)
    }

    /// Redeem a code (case, spaces and dashes don't matter). Wrong codes lock you out for a
    /// while after a few tries (`locked_out`). Each cosmetic it gives sends a `cosmetic.granted`
    /// notification (`source` `code`).
    async fn redeem_code(&self, ctx: &Context<'_>, code: String) -> Result<RedeemResult> {
        let caller = caller(ctx)?;
        let ip = ctx
            .data_opt::<crate::limits::ClientIp>()
            .map(|i| i.0.as_str());
        codes::redeem(state(ctx), caller, ip, &code)
            .await
            .map_err(api_error)
    }

    /// Make a batch of 1 to 1000 redeem codes. The answer holds the codes in plain text; this is
    /// the only time they are shown. Needs `codes.manage`. Audited.
    async fn create_code_batch(
        &self,
        ctx: &Context<'_>,
        input: CodeBatchInput,
    ) -> Result<CreatedCodeBatch> {
        let caller = caller(ctx)?;
        codes::create_batch(state(ctx), caller, &input)
            .await
            .map_err(api_error)
    }

    /// Turn off every code of a batch. Needs `codes.manage`. Audited.
    async fn disable_code_batch(&self, ctx: &Context<'_>, id: String) -> Result<CodeBatch> {
        let caller = caller(ctx)?;
        codes::disable_batch(state(ctx), caller, &id)
            .await
            .map_err(api_error)
    }

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

    /// Give an emote to another account (ADR 0014). Its owner, or an account editor of the owner
    /// with `EMOTE_TRANSFER` on every set. The receiver is you, or an account you edit with
    /// `EMOTE_UPLOAD` on every set. The owner's transfer is immediate (`emote.transferred`); an
    /// editor's makes a request the owner accepts or declines within 7 days, and the emote comes
    /// back unchanged (`emoteTransferRequests`; one open request per emote, `transfer_pending`).
    /// 60 an hour, with accepting, declining and cancelling.
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

    /// Accept an editor's transfer request of your emote: it moves if the rules still allow it.
    async fn accept_emote_transfer(&self, ctx: &Context<'_>, id: String) -> Result<Emote> {
        let caller = caller(ctx)?;
        transfers::accept(state(ctx), caller, &id)
            .await
            .map_err(api_error)
    }

    /// Decline an editor's transfer request of your emote.
    async fn decline_emote_transfer(
        &self,
        ctx: &Context<'_>,
        id: String,
    ) -> Result<EmoteTransferRequest> {
        let caller = caller(ctx)?;
        transfers::decline(state(ctx), caller, &id)
            .await
            .map_err(api_error)
    }

    /// Withdraw a transfer request you made.
    async fn cancel_emote_transfer(
        &self,
        ctx: &Context<'_>,
        id: String,
    ) -> Result<EmoteTransferRequest> {
        let caller = caller(ctx)?;
        transfers::cancel(state(ctx), caller, &id)
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

    /// Approve a pending emote. Needs `emotes.review`. Audited. With `visibility`, it is also
    /// made public, unlisted or private in the same step (for example, approved for personal use
    /// only).
    async fn approve_emote(
        &self,
        ctx: &Context<'_>,
        id: String,
        visibility: Option<EmoteVisibility>,
    ) -> Result<Emote> {
        let caller = caller(ctx)?;
        moderation::approve(state(ctx), caller, &id, visibility)
            .await
            .map_err(api_error)
    }

    /// Add a reviewers' note to an emote (1 to 2000 characters). Needs `emotes.review`. Audited.
    async fn add_emote_note(
        &self,
        ctx: &Context<'_>,
        emote_id: String,
        body: String,
    ) -> Result<EmoteNote> {
        let caller = caller(ctx)?;
        moderation::add_note(state(ctx), caller, &emote_id, &body)
            .await
            .map_err(api_error)
    }

    /// Delete an emote note: its author may, and the owner. False if it was gone. Audited.
    async fn delete_emote_note(&self, ctx: &Context<'_>, id: String) -> Result<bool> {
        let caller = caller(ctx)?;
        moderation::delete_note(state(ctx), caller, &id)
            .await
            .map_err(api_error)
    }

    /// Reject a pending emote with a reason. Needs `emotes.review`. Audited.
    async fn reject_emote(&self, ctx: &Context<'_>, id: String, reason: String) -> Result<Emote> {
        let caller = caller(ctx)?;
        moderation::reject(state(ctx), caller, &id, &reason)
            .await
            .map_err(api_error)
    }

    /// Report an emote you can see, with a reason. At most 10 an hour, one open report per emote.
    async fn report_emote(&self, ctx: &Context<'_>, id: String, reason: String) -> Result<Report> {
        let caller = caller(ctx)?;
        abuse::report(state(ctx), caller, &id, &reason)
            .await
            .map_err(api_error)
    }

    /// Dismiss a report, or take its emote down (`emotes.takedown`). Needs `emotes.review` or
    /// `emotes.takedown`. Audited.
    async fn resolve_report(
        &self,
        ctx: &Context<'_>,
        id: String,
        action: ReportAction,
        note: Option<String>,
    ) -> Result<Report> {
        let caller = caller(ctx)?;
        abuse::resolve(state(ctx), caller, &id, action, note.as_deref())
            .await
            .map_err(api_error)
    }

    /// Take an emote's image down everywhere: out of every set, its file blocked and deleted,
    /// its owners told. Needs `emotes.takedown`. Audited.
    async fn takedown_emote(&self, ctx: &Context<'_>, id: String, reason: String) -> Result<Emote> {
        let caller = caller(ctx)?;
        abuse::takedown(state(ctx), caller, &id, &reason)
            .await
            .map_err(api_error)
    }

    /// Add a staff note to an account (1 to 2000 characters). Needs `users.notes`. Audited.
    async fn add_staff_note(
        &self,
        ctx: &Context<'_>,
        account_id: String,
        body: String,
    ) -> Result<cyex_tv_shared::model::StaffNote> {
        let caller = caller(ctx)?;
        staff::add_note(state(ctx), caller, &account_id, &body)
            .await
            .map_err(api_error)
    }

    /// Delete a staff note: its author may, and the owner. False if it was gone. Audited.
    async fn delete_staff_note(&self, ctx: &Context<'_>, id: String) -> Result<bool> {
        let caller = caller(ctx)?;
        staff::delete_note(state(ctx), caller, &id)
            .await
            .map_err(api_error)
    }

    /// Restrict an account with a reason, for good or `until` a time: no uploads
    /// (`users.restrict_uploads`), no reports or tickets (`users.restrict_tickets`), or a full
    /// block that ends its sessions (`users.block`); `users.restrict` does all three. A running
    /// restriction of the same kind is replaced. The owner, and accounts whose highest role is at
    /// or above yours, are protected. Audited.
    async fn place_restriction(
        &self,
        ctx: &Context<'_>,
        account_id: String,
        input: cyex_tv_shared::model::RestrictionInput,
    ) -> Result<AccountStatus> {
        let caller = caller(ctx)?;
        abuse::place(state(ctx), caller, &account_id, &input)
            .await
            .map_err(api_error)
    }

    /// Lift a restriction early, with an optional reason. Needs the same permission as placing
    /// it. Audited.
    async fn lift_restriction(
        &self,
        ctx: &Context<'_>,
        account_id: String,
        kind: cyex_tv_shared::model::RestrictionKind,
        reason: Option<String>,
    ) -> Result<AccountStatus> {
        let caller = caller(ctx)?;
        abuse::lift(state(ctx), caller, &account_id, kind, reason.as_deref())
            .await
            .map_err(api_error)
    }

    /// Block an account's uploads, or the whole account, for good and without a reason (see
    /// `placeRestriction` for timed ones). Needs `users.restrict`, or `users.restrict_uploads` /
    /// `users.block` for the one you change; the owner is protected. Audited.
    async fn restrict_account(
        &self,
        ctx: &Context<'_>,
        account_id: String,
        uploads_blocked: Option<bool>,
        blocked: Option<bool>,
    ) -> Result<AccountStatus> {
        let caller = caller(ctx)?;
        abuse::restrict(state(ctx), caller, &account_id, uploads_blocked, blocked)
            .await
            .map_err(api_error)
    }

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

    /// Sign out one of your sessions, the current one too (then the cookie is cleared). False
    /// when you have none with that ID.
    async fn revoke_session(&self, ctx: &Context<'_>, id: String) -> Result<bool> {
        let caller = caller(ctx)?;
        let (done, current) = account::revoke_session(state(ctx), caller, &id)
            .await
            .map_err(api_error)?;
        if current && caller.via == cyex_tv_auth::Via::Cookie {
            ctx.insert_http_header(
                axum::http::header::SET_COOKIE,
                cyex_tv_auth::cookies::clear_session(state(ctx).auth.config()),
            );
        }
        Ok(done)
    }

    /// Sign out everywhere, here too: every site and app session ends and the cookie is cleared.
    /// Returns how many sessions ended.
    async fn sign_out_everywhere(&self, ctx: &Context<'_>) -> Result<u64> {
        let caller = caller(ctx)?;
        let n = account::revoke_all_sessions(state(ctx), caller)
            .await
            .map_err(api_error)?;
        if caller.via == cyex_tv_auth::Via::Cookie {
            ctx.insert_http_header(
                axum::http::header::SET_COOKIE,
                cyex_tv_auth::cookies::clear_session(state(ctx).auth.config()),
            );
        }
        Ok(n)
    }

    /// Sign out everywhere but here. Returns how many sessions ended.
    async fn revoke_other_sessions(&self, ctx: &Context<'_>) -> Result<u64> {
        let caller = caller(ctx)?;
        account::revoke_other_sessions(state(ctx), caller)
            .await
            .map_err(api_error)
    }

    /// A copy of everything about your account, as JSON text (`format`, `version`, `account`,
    /// `identities`, `sessions`, `channels`, `emotes`, `sets`, `set_editors`, `account_editors`,
    /// `editor_of`, `cosmetics`, `roles`, `reports_filed`, `notifications`, `orders`). 5 an hour
    /// (`rate_limited`). Audited.
    async fn export_my_data(&self, ctx: &Context<'_>) -> Result<String> {
        let caller = caller(ctx)?;
        account::export(state(ctx), caller).await.map_err(api_error)
    }

    /// Delete your account. `confirm` is one of your Twitch or Kick logins. Website only
    /// (`website_only` with an app token); the owner's account can't be (`owner_protected`).
    /// The account is hidden at once (profile, sets, emotes and channels read as gone, every
    /// session ends, the cookie is cleared) and erased for good 30 days later: then your
    /// identities, channels, sets, editor grants, cosmetics and notifications go, your approved
    /// emotes that are in other people's sets stay, shown as by a deleted user, and your other
    /// emotes are deleted. Signing in again before then offers to undo it (`pendingDeletion`,
    /// `restoreAccount`); after it, signing in makes a new account. Audited.
    /// `confirm_mismatch` when `confirm` isn't one of your logins; `restricted` while the account's
    /// uploads or sign-in are blocked. 3 tries an hour (`rate_limited`).
    async fn delete_account(&self, ctx: &Context<'_>, confirm: String) -> Result<bool> {
        let caller = caller(ctx)?;
        account::delete_account(state(ctx), caller, &confirm)
            .await
            .map_err(api_error)?;
        ctx.insert_http_header(
            axum::http::header::SET_COOKIE,
            cyex_tv_auth::cookies::clear_session(state(ctx).auth.config()),
        );
        Ok(true)
    }

    /// Undo `deleteAccount` within 30 days, signed in with the cookie that signing in to the
    /// deleted account gave (the website's restore page). Everything shows again and the session
    /// works as before. `signed_out` without such a session; `nothing_to_restore` when the window
    /// has passed. Audited.
    async fn restore_account(&self, ctx: &Context<'_>) -> Result<bool> {
        let pending = restorable(ctx)?;
        account::restore_account(state(ctx), pending)
            .await
            .map_err(api_error)?;
        Ok(true)
    }

    /// Create a role. You can only grant permissions you hold. Needs `roles.manage`. Audited.
    async fn create_role(&self, ctx: &Context<'_>, input: CreateRole) -> Result<Role> {
        let caller = caller(ctx)?;
        admin::create(state(ctx), caller, &input)
            .await
            .map_err(api_error)
    }

    /// Change a role below your own highest one that you don't hold. The top role can't be
    /// changed; the User role keeps position 0. Needs `roles.manage`. Audited.
    async fn update_role(&self, ctx: &Context<'_>, id: String, input: UpdateRole) -> Result<Role> {
        let caller = caller(ctx)?;
        admin::update(state(ctx), caller, &id, &input)
            .await
            .map_err(api_error)
    }

    /// Delete a role below your own highest one that you don't hold; built-in roles stay.
    /// Needs `roles.manage`. Audited.
    async fn delete_role(&self, ctx: &Context<'_>, id: String) -> Result<bool> {
        let caller = caller(ctx)?;
        admin::delete(state(ctx), caller, &id)
            .await
            .map_err(api_error)
    }

    /// Give an account a role below your highest one that grants only what you hold. Needs
    /// `roles.manage` or `roles.assign`; the User and top roles can't be given. Audited.
    async fn assign_role(
        &self,
        ctx: &Context<'_>,
        account_id: String,
        role_id: String,
    ) -> Result<bool> {
        let caller = caller(ctx)?;
        admin::assign(state(ctx), caller, &account_id, &role_id)
            .await
            .map_err(api_error)
    }

    /// Take a role from an account. The owner and the last `roles.manage` holder are protected.
    /// Needs `roles.manage` or `roles.assign`. Audited.
    async fn unassign_role(
        &self,
        ctx: &Context<'_>,
        account_id: String,
        role_id: String,
    ) -> Result<bool> {
        let caller = caller(ctx)?;
        admin::unassign(state(ctx), caller, &account_id, &role_id)
            .await
            .map_err(api_error)
    }
}

pub struct Subscription;

#[async_graphql::Subscription]
impl Subscription {
    /// Live updates for up to 500 topics: `channel:<platform>:<id>`, `set:<id>`,
    /// `user:<platform>:<id>` (ADR 0005), and `cosmetics` (ADR 0015). The same events as the event WebSocket.
    async fn events(
        &self,
        ctx: &Context<'_>,
        topics: Vec<String>,
    ) -> Result<impl Stream<Item = Event> + use<>> {
        if topics.len() > hub_topics::MAX_TOPICS {
            return Err(coded("too_many_topics", "At most 500 topics."));
        }
        if let Some(bad) = topics.iter().find(|t| !hub_topics::is_valid(t)) {
            return Err(
                async_graphql::Error::new(format!("{bad:?} is not a topic."))
                    .extend_with(|_, e| e.set("code", "invalid_topic")),
            );
        }
        let slot = match ctx.data_opt::<SocketConnection>() {
            Some(connection) => Some(connection.take_slot().ok_or_else(|| {
                coded(
                    "too_many_subscriptions",
                    "At most 8 subscriptions on one socket.",
                )
            })?),
            None => None,
        };
        // One socket holds at most `MAX_SOCKET_SUBSCRIPTIONS` streams (ADR 0005).
        let sub = state(ctx).hub.subscriber();
        sub.subscribe(topics.iter().map(String::as_str));
        // The slot lives as long as the stream, so ending it frees the connection's place.
        Ok(futures_util::stream::unfold(
            (sub, slot),
            |(mut sub, slot)| async move {
                let event = match sub.recv().await? {
                    HubMessage::Event(d) => cyex_tv_hub::model_event(&d.event, &d.topic),
                    HubMessage::Lagged => cyex_tv_hub::resync(""),
                };
                Some((event, (sub, slot)))
            },
        ))
    }
}
