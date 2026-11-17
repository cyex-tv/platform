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
use cyex_tv_shared::model::{
    AccountEditor, AccountResults, AccountSession, AccountStatus, AccountSummary, ActivityEntry,
    AddAccountEditor, AddSetEmote, AuditEntry, Badge, Channel, ClonedSet, CosmeticCatalog,
    CosmeticImage, CosmeticKind, CreateBadge, CreateRole, CreateSet, EditedAccount,
    EditorPermission, Emote, EmotePage, EmoteSet, EmoteSort, Entitlement, EntitlementPage, Event,
    GrantCosmetic, Identity, Me, Notification, OwnedCosmetics, Paint, PaintInput, PersonalSetEntry,
    Platform, Report, ReportAction, ReportQueue, ReviewQueue, Role, SearchResults, SetEmote,
    SetSummary, SetVisibility, UpdateBadge, UpdateChannelSettings, UpdateEmote, UpdateRole,
    UpdateSet, UpdateSetEmote, UserCosmetics,
};
use cyex_tv_shared::permissions;
use futures_util::Stream;
use sqlx::Connection;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::state::AppState;
use crate::{abuse, account, activity, admin, catalog, cosmetics, editors, moderation, search};

pub type ApiSchema = Schema<Query, Mutation, Subscription>;

/// Depth and complexity limits apply to every request (ADR 0008).
pub fn builder() -> SchemaBuilder<Query, Mutation, Subscription> {
    Schema::build(Query, Mutation, Subscription)
        .limit_depth(12)
        .limit_complexity(1_000)
        .extension(SocketRunsSubscriptionsOnly)
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
    Ok(Me {
        id: ids::to_text(account.id),
        identities,
        permissions,
        created_at: account
            .created_at
            .to_rfc3339_opts(SecondsFormat::Millis, true),
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
    /// is unknown, deleted or blocked.
    async fn account_sets(
        &self,
        ctx: &Context<'_>,
        account_id: String,
        preview: Option<u32>,
    ) -> Result<Vec<SetSummary>> {
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
    /// `RELEVANCE` (default: best match, then most used), `TOP` (most used, then name) or `NEW`
    /// (newest first). An empty `query` lists every approved, listed emote.
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
    ) -> Result<SearchResults> {
        search::search(
            state(ctx),
            &search::Params {
                text: &query,
                animated,
                owner: owner.as_deref(),
                nsfw: nsfw.unwrap_or(false),
                sort: sort.unwrap_or_default(),
                page,
                per_page,
            },
        )
        .await
        .map_err(api_error)
    }

    /// Accounts by Twitch or Kick login or display name (a leading `@` is ignored): exact login
    /// first, then login and name prefixes, then similar names. Public fields only; deleted and
    /// blocked accounts are never listed. `perPage` is 1 to 50, default 20. 60 searches a minute
    /// per account, or per IP signed out (`rate_limited`). An empty `query` finds nobody.
    async fn search_accounts(
        &self,
        ctx: &Context<'_>,
        query: String,
        page: Option<u32>,
        per_page: Option<u32>,
    ) -> Result<AccountResults> {
        let ip = ctx
            .data_opt::<crate::limits::ClientIp>()
            .map_or("", |ip| ip.0.as_str());
        search::accounts(state(ctx), caller_opt(ctx), ip, &query, page, per_page)
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
    /// account is unknown, deleted or blocked. Nothing private or from moderation shows.
    async fn account_activity(
        &self,
        ctx: &Context<'_>,
        account_id: String,
        before: Option<String>,
        limit: Option<u32>,
    ) -> Result<Vec<ActivityEntry>> {
        activity::account_activity(state(ctx), &account_id, before.as_deref(), limit)
            .await
            .map_err(api_error)
    }

    /// Channels whose active set holds the emote, most recently added first. `limit` is 1 to 50,
    /// default 8. Empty when the emote is unknown or not public.
    async fn emote_channels(
        &self,
        ctx: &Context<'_>,
        id: String,
        limit: Option<u32>,
    ) -> Result<Vec<Channel>> {
        catalog::emote_channels(state(ctx), &id, limit)
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

    /// An account's editors (ADR 0014): yours, or, with `accountId`, those of an account you edit
    /// with the `EDITORS` permission.
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

    /// Every permission name a role can hold.
    async fn permissions(&self) -> Vec<String> {
        permissions::ALL.iter().map(|p| (*p).to_owned()).collect()
    }

    /// Pending emotes, oldest first. Needs `emotes.review`. `perPage` is 1 to 100, default 50.
    async fn review_queue(
        &self,
        ctx: &Context<'_>,
        page: Option<u32>,
        per_page: Option<u32>,
    ) -> Result<ReviewQueue> {
        let caller = caller(ctx)?;
        moderation::review_queue(state(ctx), caller, page, per_page)
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

    /// Audited changes, newest first. Needs `audit.view`. Page back with `before` (an entry ID);
    /// `limit` is 1 to 100, default 50.
    async fn audit_log(
        &self,
        ctx: &Context<'_>,
        target_kind: Option<String>,
        target_id: Option<String>,
        before: Option<String>,
        limit: Option<u32>,
    ) -> Result<Vec<AuditEntry>> {
        let caller = caller(ctx)?;
        admin::audit_log(
            state(ctx),
            caller,
            target_kind.as_deref(),
            target_id.as_deref(),
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

    /// Where you are signed in: your live sessions, the most recently used first, with the one
    /// this request came with marked `current`. Never shows tokens.
    async fn my_sessions(&self, ctx: &Context<'_>) -> Result<Vec<AccountSession>> {
        let caller = caller(ctx)?;
        account::my_sessions(state(ctx), caller)
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

    /// Your notifications, newest first (the latest 100). `kind` is one of `emote.approved`,
    /// `emote.rejected`, `emote.taken_down`, `editor.added`, `editor.changed`, `editor.removed`,
    /// `cosmetic.granted` and `cosmetic.revoked`; `payload` is JSON with the IDs and names it is
    /// about (see the API reference).
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

    /// Whether an account's uploads or sign-in are blocked. Needs `users.view` or
    /// `users.restrict`.
    async fn account_status(&self, ctx: &Context<'_>, account_id: String) -> Result<AccountStatus> {
        let caller = caller(ctx)?;
        abuse::status(state(ctx), caller, &account_id)
            .await
            .map_err(api_error)
    }

    /// All roles. Needs `roles.manage`.
    async fn roles(&self, ctx: &Context<'_>) -> Result<Vec<Role>> {
        let caller = caller(ctx)?;
        admin::list(state(ctx), caller).await.map_err(api_error)
    }
}

pub struct Mutation;

#[Object]
impl Mutation {
    /// Unlink a platform from your account. The last identity can't be removed.
    async fn unlink_identity(&self, ctx: &Context<'_>, platform: Platform) -> Result<Me> {
        let caller = caller(ctx)?;
        let mut conn = state(ctx).db.acquire().await.map_err(internal)?;
        let mut tx = conn.begin().await.map_err(internal)?;
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

    /// Make someone an editor of your account, or of an account you edit with `EDITORS` (ADR
    /// 0014). Give their account ID, or a platform and login. You can only grant permissions you
    /// hold. At most 20 editors per account. Audited.
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
    /// Audited.
    async fn update_account_editor(
        &self,
        ctx: &Context<'_>,
        owner_id: Option<String>,
        account_id: String,
        permissions: Vec<EditorPermission>,
    ) -> Result<AccountEditor> {
        let caller = caller(ctx)?;
        editors::update(
            state(ctx),
            caller,
            owner_id.as_deref(),
            &account_id,
            &permissions,
        )
        .await
        .map_err(api_error)
    }

    /// Remove an account editor: the owner, an editor with `EDITORS`, or the editor themselves
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

    /// Change an emote's name, tags or flags. Owner, or `emotes.edit_any`.
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

    /// Delete your emote. It is removed from every set.
    async fn delete_emote(&self, ctx: &Context<'_>, id: String) -> Result<bool> {
        let caller = caller(ctx)?;
        moderation::delete(state(ctx), caller, &id)
            .await
            .map_err(api_error)
    }

    /// Approve a pending emote. Needs `emotes.review`. Audited.
    async fn approve_emote(&self, ctx: &Context<'_>, id: String) -> Result<Emote> {
        let caller = caller(ctx)?;
        moderation::approve(state(ctx), caller, &id)
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

    /// Block an account's uploads, or the whole account. Needs `users.restrict`; the owner is
    /// protected. Audited.
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

    /// Sign out one of your sessions, the current one too. False when you have none with that
    /// ID.
    async fn revoke_session(&self, ctx: &Context<'_>, id: String) -> Result<bool> {
        let caller = caller(ctx)?;
        account::revoke_session(state(ctx), caller, &id)
            .await
            .map_err(api_error)
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

    /// Delete your account now. `confirm` is one of your Twitch or Kick logins. Website only
    /// (`website_only` with an app token); the owner's account can't be (`owner_protected`).
    /// Your sessions, identities, channels, sets, editor grants, cosmetics and notifications go;
    /// your approved emotes that are in other people's sets stay, shown as by a deleted user;
    /// your other emotes are deleted. Signing in again later makes a new account. Audited.
    /// `confirm_mismatch` when `confirm` isn't one of your logins.
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

    /// Create a role. You can only grant permissions you hold. Needs `roles.manage`. Audited.
    async fn create_role(&self, ctx: &Context<'_>, input: CreateRole) -> Result<Role> {
        let caller = caller(ctx)?;
        admin::create(state(ctx), caller, &input)
            .await
            .map_err(api_error)
    }

    /// Change a role below your own highest one. Needs `roles.manage`. Audited.
    async fn update_role(&self, ctx: &Context<'_>, id: String, input: UpdateRole) -> Result<Role> {
        let caller = caller(ctx)?;
        admin::update(state(ctx), caller, &id, &input)
            .await
            .map_err(api_error)
    }

    /// Delete a role below your own highest one. Needs `roles.manage`. Audited.
    async fn delete_role(&self, ctx: &Context<'_>, id: String) -> Result<bool> {
        let caller = caller(ctx)?;
        admin::delete(state(ctx), caller, &id)
            .await
            .map_err(api_error)
    }

    /// Give an account a role. Needs `roles.manage`. Audited.
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
    /// Needs `roles.manage`. Audited.
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
