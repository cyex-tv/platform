//! The GraphQL API at `/v1/gql` (ADR 0008): accounts, channels, sets and editors, emotes and the
//! review gate, roles and the audit log, search, and the `events` subscription. Uploads are REST.
//!
//! Admin and moderation fields check their permission in `catalog`, `admin` and `moderation`,
//! each naming the one it needs. Introspection stays on, so the private parts are visible in the
//! schema but closed to anyone without the permission.

use async_graphql::http::GraphiQLSource;
use async_graphql::{Context, ErrorExtensions, MergedObject, Result, Schema, SchemaBuilder};
use axum::response::Html;
use cyex_tv_auth::{AuthError, Caller};
use cyex_tv_shared::error::ApiError;

use crate::state::AppState;

mod account;
mod activity;
mod admin;
mod audit;
mod channels;
mod cosmetics;
mod developers;
mod editors;
mod emotes;
mod entitlements;
mod extensions;
mod moderation;
mod personal;
mod profile;
mod review;
mod roles;
mod search;
mod sets;
mod shop;
mod site;
mod subscription;

use extensions::{AppScopes, RootFieldCap, SocketRunsSubscriptionsOnly};
pub use extensions::{
    MAX_ROOT_FIELDS, MAX_SOCKET_MESSAGE, MAX_SOCKET_SUBSCRIPTIONS, SOCKET_IDLE,
    SOCKET_INIT_DEADLINE, SOCKET_PING_EVERY, SocketConnection, serve_socket,
};
pub use subscription::Subscription;

// The root query: one object per area, merged in the order the schema lists the fields.
#[derive(MergedObject, Default)]
pub struct Query(
    account::AccountQuery,
    channels::ChannelQuery,
    sets::SetQuery,
    emotes::EmoteQuery,
    search::SearchQuery,
    profile::ProfileQuery,
    activity::ActivityQuery,
    personal::PersonalQuery,
    editors::EditorQuery,
    cosmetics::CosmeticQuery,
    developers::DeveloperQuery,
    site::SiteQuery,
    shop::ShopQuery,
    review::ReviewQuery,
    roles::AccountRoleQuery,
    audit::AuditQuery,
    moderation::ModerationQuery,
    admin::AdminQuery,
    entitlements::EntitlementQuery,
    roles::RoleQuery,
);

// The root mutation: one object per area, merged in the order the schema lists the fields.
#[derive(MergedObject, Default)]
pub struct Mutation(
    account::AccountMutation,
    site::SiteMutation,
    channels::ChannelMutation,
    sets::SetMutation,
    editors::EditorMutation,
    cosmetics::CosmeticMutation,
    shop::ShopMutation,
    emotes::EmoteMutation,
    review::ReviewMutation,
    moderation::ModerationMutation,
    personal::PersonalMutation,
    account::AccountSessionMutation,
    developers::DeveloperMutation,
    roles::RoleMutation,
);

pub type ApiSchema = Schema<Query, Mutation, Subscription>;

/// Depth and complexity limits apply to every request (ADR 0008), and an operation selects at
/// most `MAX_ROOT_FIELDS` top-level fields. The public reads that fan out carry their own weight
/// (`searchEmotes` and `searchAccounts` 50, the single lookups 10), so a request can't alias
/// hundreds of them past the limit.
pub fn builder() -> SchemaBuilder<Query, Mutation, Subscription> {
    Schema::build(Query::default(), Mutation::default(), Subscription)
        .limit_depth(12)
        .limit_complexity(1_000)
        .extension(SocketRunsSubscriptionsOnly)
        .extension(RootFieldCap)
        .extension(AppScopes)
        .extension(crate::public_ids::PublicIds)
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
