//! The GraphQL API at `/v1/gql` (ADR 0008), drafted in Phase 1 for review. Resolvers answer
//! `not_implemented` until their phase builds them:
//! - Phase 2: accounts, channels, sets, roles
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
use cyex_tv_shared::model::{
    AddSetEmote, Channel, CreateRole, CreateSet, Emote, EmoteSet, Event, Me, PersonalSetEntry,
    Platform, ReviewQueue, Role, SearchResults, SetEmote, UpdateChannelSettings, UpdateEmote,
    UpdateRole, UpdateSet, UpdateSetEmote,
};
use cyex_tv_shared::permissions;
use futures_util::stream::Empty;

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
    async_graphql::Error::new(message)
        .extend_with(|_, e| e.set("code", code))
}

fn not_implemented() -> async_graphql::Error {
    coded(
        "not_implemented",
        "This field is part of the v1 API but isn't built yet.",
    )
}

/// Checks that the caller holds `permission` (ADR 0010). There are no sessions before Phase 2,
/// so for now nobody does.
fn require(ctx: &Context<'_>, permission: &'static str) -> Result<()> {
    Err(coded("signed_out", "Sign in to use this."))
}

pub struct Query;

#[Object]
impl Query {
    /// The signed-in account.
    async fn me(&self, ctx: &Context<'_>) -> Result<Me> {
        Err(not_implemented())
    }

    /// A channel by its platform ID.
    async fn channel(&self, platform: Platform, id: String) -> Result<Option<Channel>> {
        Err(not_implemented())
    }

    /// A channel by its login or slug. Logins change; store the ID, not the login.
    async fn channel_by_login(&self, platform: Platform, login: String) -> Result<Option<Channel>> {
        Err(not_implemented())
    }

    /// A set with its emotes.
    async fn set(&self, id: String) -> Result<Option<EmoteSet>> {
        Err(not_implemented())
    }

    /// The cyex.tv global set, shown in every channel.
    async fn global_set(&self) -> Result<EmoteSet> {
        Err(not_implemented())
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
    async fn personal_set(&self, platform: Platform, id: String) -> Result<Option<EmoteSet>> {
        Err(not_implemented())
    }

    /// Personal sets for up to 100 chatters at once.
    async fn personal_sets(
        &self,
        platform: Platform,
        user_ids: Vec<String>,
    ) -> Result<Vec<PersonalSetEntry>> {
        Err(not_implemented())
    }

    /// Every permission name a role can hold.
    async fn permissions(&self) -> Vec<String> {
        permissions::ALL.iter().map(|p| (*p).to_owned()).collect()
    }

    /// Pending emotes, oldest first. Needs `emotes.review`.
    async fn review_queue(&self, ctx: &Context<'_>) -> Result<ReviewQueue> {
        require(ctx, permissions::EMOTES_REVIEW)?;
        Err(not_implemented())
    }

    /// All roles. Needs `roles.manage`.
    async fn roles(&self, ctx: &Context<'_>) -> Result<Vec<Role>> {
        require(ctx, permissions::ROLES_MANAGE)?;
        Err(not_implemented())
    }
}

pub struct Mutation;

#[Object]
impl Mutation {
    /// Unlink a platform from your account. The last identity can't be removed.
    async fn unlink_identity(&self, ctx: &Context<'_>, platform: Platform) -> Result<Me> {
        Err(not_implemented())
    }

    /// Change a channel's active set and settings. Only the channel's owner.
    async fn update_channel_settings(
        &self,
        ctx: &Context<'_>,
        platform: Platform,
        id: String,
        input: UpdateChannelSettings,
    ) -> Result<Channel> {
        Err(not_implemented())
    }

    /// Create a channel set, or your personal set (one per account).
    async fn create_set(&self, ctx: &Context<'_>, input: CreateSet) -> Result<EmoteSet> {
        Err(not_implemented())
    }

    /// Rename a set or change its capacity. Owner or editor; capacity needs `sets.edit_any`.
    async fn update_set(
        &self,
        ctx: &Context<'_>,
        id: String,
        input: UpdateSet,
    ) -> Result<EmoteSet> {
        Err(not_implemented())
    }

    /// Delete a set. Owner only. A channel's active set can't be deleted.
    async fn delete_set(&self, ctx: &Context<'_>, id: String) -> Result<bool> {
        Err(not_implemented())
    }

    /// Add an emote to a set. Personal sets take approved emotes only.
    async fn add_set_emote(
        &self,
        ctx: &Context<'_>,
        set_id: String,
        emote_id: String,
        input: Option<AddSetEmote>,
    ) -> Result<SetEmote> {
        Err(not_implemented())
    }

    /// Rename an emote in a set, or change its zero-width flag there.
    async fn update_set_emote(
        &self,
        ctx: &Context<'_>,
        set_id: String,
        emote_id: String,
        input: UpdateSetEmote,
    ) -> Result<SetEmote> {
        Err(not_implemented())
    }

    /// Remove an emote from a set.
    async fn remove_set_emote(
        &self,
        ctx: &Context<'_>,
        set_id: String,
        emote_id: String,
    ) -> Result<bool> {
        Err(not_implemented())
    }

    /// Make someone an editor of a set. Owner only.
    async fn add_set_editor(
        &self,
        ctx: &Context<'_>,
        set_id: String,
        account_id: String,
    ) -> Result<bool> {
        Err(not_implemented())
    }

    /// Remove an editor. Owner, or the editor themselves.
    async fn remove_set_editor(
        &self,
        ctx: &Context<'_>,
        set_id: String,
        account_id: String,
    ) -> Result<bool> {
        Err(not_implemented())
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
        require(ctx, permissions::EMOTES_REVIEW)?;
        Err(not_implemented())
    }

    /// Reject a pending emote with a reason. Needs `emotes.review`. Audited.
    async fn reject_emote(&self, ctx: &Context<'_>, id: String, reason: String) -> Result<Emote> {
        require(ctx, permissions::EMOTES_REVIEW)?;
        Err(not_implemented())
    }

    /// Create a role. You can only grant permissions you hold. Needs `roles.manage`. Audited.
    async fn create_role(&self, ctx: &Context<'_>, input: CreateRole) -> Result<Role> {
        require(ctx, permissions::ROLES_MANAGE)?;
        Err(not_implemented())
    }

    /// Change a role below your own highest one. Needs `roles.manage`. Audited.
    async fn update_role(&self, ctx: &Context<'_>, id: String, input: UpdateRole) -> Result<Role> {
        require(ctx, permissions::ROLES_MANAGE)?;
        Err(not_implemented())
    }

    /// Delete a role below your own highest one. Needs `roles.manage`. Audited.
    async fn delete_role(&self, ctx: &Context<'_>, id: String) -> Result<bool> {
        require(ctx, permissions::ROLES_MANAGE)?;
        Err(not_implemented())
    }

    /// Give an account a role. Needs `roles.manage`. Audited.
    async fn assign_role(
        &self,
        ctx: &Context<'_>,
        account_id: String,
        role_id: String,
    ) -> Result<bool> {
        require(ctx, permissions::ROLES_MANAGE)?;
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
        require(ctx, permissions::ROLES_MANAGE)?;
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
