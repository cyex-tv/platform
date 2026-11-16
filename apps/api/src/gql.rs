//! The GraphQL API at `/v1/gql` (ADR 0008): accounts, channels, sets and editors, emotes and the
//! review gate, roles and the audit log, search, and the `events` subscription. Uploads are REST.
//!
//! Admin and moderation fields check their permission in `catalog`, `admin` and `moderation`,
//! each naming the one it needs. Introspection stays on, so the private parts are visible in the
//! schema but closed to anyone without the permission.

// The stubs take the arguments the real resolvers will use; they go away as each one lands.
#![allow(unused_variables)]

use async_graphql::http::GraphiQLSource;
use async_graphql::{Context, ErrorExtensions, Object, Result, Schema, SchemaBuilder};
use axum::response::Html;
use chrono::SecondsFormat;
use cyex_tv_auth::{AuthError, Caller};
use cyex_tv_db::{DbError, accounts, channels, ids};
use cyex_tv_hub::Message as HubMessage;
use cyex_tv_hub::topics as hub_topics;
use cyex_tv_shared::error::ApiError;
use cyex_tv_shared::model::{
    AddSetEmote, AuditEntry, Channel, CreateRole, CreateSet, Emote, EmoteSet, Event, Identity, Me,
    PersonalSetEntry, Platform, ReviewQueue, Role, SearchResults, SetEmote, UpdateChannelSettings,
    UpdateEmote, UpdateRole, UpdateSet, UpdateSetEmote,
};
use cyex_tv_shared::permissions;
use futures_util::Stream;
use sqlx::Connection;

use crate::state::AppState;
use crate::{admin, catalog, moderation, search};

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
    async fn emote(&self, ctx: &Context<'_>, id: String) -> Result<Option<Emote>> {
        moderation::emote(state(ctx), caller_opt(ctx), &id)
            .await
            .map_err(api_error)
    }

    /// Approved, listed emotes by name and tag, best match first, then the most used.
    /// `perPage` is 1 to 100, default 50. NSFW emotes only with `nsfw: true`.
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
    ) -> Result<SearchResults> {
        search::search(
            state(ctx),
            &search::Params {
                text: &query,
                animated,
                owner: owner.as_deref(),
                nsfw: nsfw.unwrap_or(false),
                page,
                per_page,
            },
        )
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
    /// `user:<platform>:<id>` (ADR 0005). The same events as the event WebSocket.
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
        let sub = state(ctx).hub.subscriber();
        sub.subscribe(topics.iter().map(String::as_str));
        Ok(futures_util::stream::unfold(sub, |mut sub| async move {
            let event = match sub.recv().await? {
                HubMessage::Event(d) => cyex_tv_hub::model_event(&d.event, &d.topic),
                HubMessage::Lagged => cyex_tv_hub::resync(""),
            };
            Some((event, sub))
        }))
    }
}
