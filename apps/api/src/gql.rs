//! The GraphQL API at `/v1/gql` (ADR 0008). Resolvers answer `not_implemented` until their phase
//! builds them:
//! - Phase 2: accounts, channels, sets and editors (built), roles
//! - Phase 3: emotes and moderation
//! - Phase 5: the `events` subscription
//! - Phase 6: search
//!
//! Admin and moderation fields check a permission first, through [`require`]. Introspection stays
//! on, so the private parts are visible in the schema but closed to anyone without the permission.

// The stubs take the arguments the real resolvers will use; they go away as each one lands.
#![allow(unused_variables)]

use async_graphql::http::GraphiQLSource;
use async_graphql::{Context, ErrorExtensions, Object, Result, Schema, SchemaBuilder};
use axum::response::Html;
use chrono::SecondsFormat;
use cyex_tv_auth::{AuthError, Caller};
use cyex_tv_db::{DbError, accounts, channels, ids};
use cyex_tv_shared::error::ApiError;
use cyex_tv_shared::model::{
    AddSetEmote, Channel, CreateRole, CreateSet, Emote, EmoteSet, Event, Identity, Me,
    PersonalSetEntry, Platform, ReviewQueue, Role, SearchResults, SetEmote, UpdateChannelSettings,
    UpdateEmote, UpdateRole, UpdateSet, UpdateSetEmote,
};
use cyex_tv_shared::permissions;
use futures_util::stream::Empty;
use sqlx::Connection;

use crate::catalog;
use crate::state::AppState;

pub type ApiSchema = Schema<Query, Mutation, Subscription>;

/// Depth and complexity limits apply to every request (ADR 0008).
pub fn builder() -> SchemaBuilder<Query, Mutation, Subscription> {
    Schema::build(Query, Mutation, Subscription)
        .limit_depth(12)
        .limit_complexity(1_000)
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

fn not_implemented() -> async_graphql::Error {
    coded(
        "not_implemented",
        "This field is part of the v1 API but isn't built yet.",
    )
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

/// Checks that the caller holds `permission` through their roles (ADR 0010). The owner rule and
/// the escalation guards come with the role mutations (Phase 2 PR 4).
async fn require<'a>(ctx: &Context<'a>, permission: &'static str) -> Result<&'a Caller> {
    let caller = caller(ctx)?;
    let mut conn = state(ctx).db.acquire().await.map_err(internal)?;
    let held = catalog::permissions_of(&mut conn, caller.account_id)
        .await
        .map_err(api_error)?;
    if held.iter().any(|p| p == permission) {
        Ok(caller)
    } else {
        Err(coded("forbidden", "You don't have permission to do this."))
    }
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
    let permissions = catalog::permissions_of(&mut conn, account.id)
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

    /// A set with its emotes.
    async fn set(&self, ctx: &Context<'_>, id: String) -> Result<Option<EmoteSet>> {
        catalog::set(state(ctx), caller_opt(ctx), &id)
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
    async fn emote(&self, id: String) -> Result<Option<Emote>> {
        Err(not_implemented())
    }

    /// Approved, listed emotes by name and tag. `perPage` is 1 to 100, default 50.
    async fn search_emotes(
        &self,
        query: String,
        animated: Option<bool>,
        owner: Option<String>,
        page: Option<u32>,
        per_page: Option<u32>,
    ) -> Result<SearchResults> {
        Err(not_implemented())
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

    /// Your channels: one per linked platform you have signed in with.
    async fn my_channels(&self, ctx: &Context<'_>) -> Result<Vec<Channel>> {
        let caller = caller(ctx)?;
        catalog::my_channels(state(ctx), caller)
            .await
            .map_err(api_error)
    }

    /// Every permission name a role can hold.
    async fn permissions(&self) -> Vec<String> {
        permissions::ALL.iter().map(|p| (*p).to_owned()).collect()
    }

    /// Pending emotes, oldest first. Needs `emotes.review`.
    async fn review_queue(&self, ctx: &Context<'_>) -> Result<ReviewQueue> {
        require(ctx, permissions::EMOTES_REVIEW).await?;
        Err(not_implemented())
    }

    /// All roles. Needs `roles.manage`.
    async fn roles(&self, ctx: &Context<'_>) -> Result<Vec<Role>> {
        require(ctx, permissions::ROLES_MANAGE).await?;
        Err(not_implemented())
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

    /// Change a channel's active set and settings. Only the channel's owner.
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

    /// Create a channel set, or your personal set (one per account).
    async fn create_set(&self, ctx: &Context<'_>, input: CreateSet) -> Result<EmoteSet> {
        let caller = caller(ctx)?;
        catalog::create_set(state(ctx), caller, &input)
            .await
            .map_err(api_error)
    }

    /// Rename a set or change its capacity. Owner or editor; capacity needs `sets.edit_any`.
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

    /// Delete a set. Owner only. A channel's active set can't be deleted.
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

    /// Make someone an editor of a set. Owner only.
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

    /// Change an emote's name, tags or flags. Owner, or `emotes.edit_any`.
    async fn update_emote(
        &self,
        ctx: &Context<'_>,
        id: String,
        input: UpdateEmote,
    ) -> Result<Emote> {
        Err(not_implemented())
    }

    /// Delete your emote. It is removed from every set.
    async fn delete_emote(&self, ctx: &Context<'_>, id: String) -> Result<bool> {
        Err(not_implemented())
    }

    /// Approve a pending emote. Needs `emotes.review`. Audited.
    async fn approve_emote(&self, ctx: &Context<'_>, id: String) -> Result<Emote> {
        require(ctx, permissions::EMOTES_REVIEW).await?;
        Err(not_implemented())
    }

    /// Reject a pending emote with a reason. Needs `emotes.review`. Audited.
    async fn reject_emote(&self, ctx: &Context<'_>, id: String, reason: String) -> Result<Emote> {
        require(ctx, permissions::EMOTES_REVIEW).await?;
        Err(not_implemented())
    }

    /// Create a role. You can only grant permissions you hold. Needs `roles.manage`. Audited.
    async fn create_role(&self, ctx: &Context<'_>, input: CreateRole) -> Result<Role> {
        require(ctx, permissions::ROLES_MANAGE).await?;
        Err(not_implemented())
    }

    /// Change a role below your own highest one. Needs `roles.manage`. Audited.
    async fn update_role(&self, ctx: &Context<'_>, id: String, input: UpdateRole) -> Result<Role> {
        require(ctx, permissions::ROLES_MANAGE).await?;
        Err(not_implemented())
    }

    /// Delete a role below your own highest one. Needs `roles.manage`. Audited.
    async fn delete_role(&self, ctx: &Context<'_>, id: String) -> Result<bool> {
        require(ctx, permissions::ROLES_MANAGE).await?;
        Err(not_implemented())
    }

    /// Give an account a role. Needs `roles.manage`. Audited.
    async fn assign_role(
        &self,
        ctx: &Context<'_>,
        account_id: String,
        role_id: String,
    ) -> Result<bool> {
        require(ctx, permissions::ROLES_MANAGE).await?;
        Err(not_implemented())
    }

    /// Take a role from an account. The owner and the last `roles.manage` holder are protected.
    /// Needs `roles.manage`. Audited.
    async fn unassign_role(
        &self,
        ctx: &Context<'_>,
        account_id: String,
        role_id: String,
    ) -> Result<bool> {
        require(ctx, permissions::ROLES_MANAGE).await?;
        Err(not_implemented())
    }
}

pub struct Subscription;

#[async_graphql::Subscription]
impl Subscription {
    /// Live updates for up to 500 topics: `channel:<platform>:<id>`, `set:<id>`,
    /// `user:<platform>:<id>` (ADR 0005). The same events as the event WebSocket.
    async fn events(&self, topics: Vec<String>) -> Result<Empty<Event>> {
        Err(not_implemented())
    }
}
