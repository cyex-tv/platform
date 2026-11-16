//! The cyex.tv API (ADR 0008): GraphQL at `/v1/gql` for most things, plus a small REST part.
//! The REST router and its OpenAPI document come from the same handlers, and the GraphQL SDL from
//! the schema itself, so neither committed description can drift from the code.

pub mod gql;
pub mod routes;
pub mod state;

use async_graphql_axum::{GraphQL, GraphQLSubscription};
use axum::Router;
use axum::routing::get;
use tower_http::trace::TraceLayer;
use utoipa::OpenApi;
use utoipa_axum::router::OpenApiRouter;

use crate::state::AppState;

#[derive(OpenApi)]
#[openapi(
    info(
        title = "cyex.tv REST API",
        description = "The REST part of the cyex.tv API: sign-in, uploads and the cacheable reads chat clients poll. Everything else is GraphQL at /v1/gql.",
        license(name = "AGPL-3.0-only")
    ),
    servers((url = "https://api.cyex.tv", description = "Production")),
    tags(
        (name = "system", description = "Health checks."),
        (name = "auth", description = "Signing in with Twitch or Kick."),
        (name = "emotes", description = "Uploading emotes."),
        (name = "reads", description = "Public, cacheable reads for chat clients.")
    )
)]
pub struct ApiDoc;

/// The REST routes, with their OpenAPI description attached.
pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::with_openapi(ApiDoc::openapi())
        .merge(routes::system::router())
        .merge(routes::v1::router())
}

/// The service, ready to serve: REST, GraphQL (`POST /v1/gql`, GraphiQL on `GET`) and GraphQL
/// subscriptions (`/v1/gql/ws`).
pub fn app(state: AppState) -> Router {
    let schema = gql::builder().data(state.clone()).finish();
    let graphql = GraphQL::new(schema.clone());
    let subscriptions = GraphQLSubscription::new(schema);
    let (rest_router, _spec) = router().split_for_parts();
    rest_router
        .route("/v1/gql", get(gql::graphiql).post_service(graphql))
        .route_service("/v1/gql/ws", subscriptions)
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

/// The OpenAPI document, as committed in `openapi/openapi.json`.
pub fn openapi() -> utoipa::openapi::OpenApi {
    router().split_for_parts().1
}
