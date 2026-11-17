//! The cyex.tv API (ADR 0008): GraphQL at `/v1/gql` for most things, plus a small REST part.
//! The REST router and its OpenAPI document come from the same handlers, and the GraphQL SDL from
//! the schema itself, so neither committed description can drift from the code.

pub mod abuse;
pub mod admin;
pub mod catalog;
pub mod cors;
pub mod cosmetics;
pub mod editors;
pub mod gql;
pub mod limits;
pub mod moderation;
pub mod routes;
pub mod search;
pub mod state;
pub mod upload;
pub mod views;

use async_graphql::http::ALL_WEBSOCKET_PROTOCOLS;
use async_graphql_axum::{GraphQLProtocol, GraphQLRequest, GraphQLResponse, GraphQLWebSocket};
use axum::Router;
use axum::extract::{Extension, State, WebSocketUpgrade};
use axum::response::IntoResponse;
use axum::routing::get;
use tower_http::trace::TraceLayer;
use utoipa::OpenApi;
use utoipa_axum::router::OpenApiRouter;

use crate::gql::ApiSchema;
use crate::state::{AppState, MaybeCaller};

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
        (name = "cosmetics", description = "cyex.tv chat badges and name paints (ADR 0015): the catalog, chatters' picks, and staff picture uploads."),
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
    let (rest_router, _spec) = router().split_for_parts();
    rest_router
        .route("/v1/gql", get(gql::graphiql).post(graphql))
        .route("/v1/gql/ws", get(graphql_ws))
        .layer(Extension(schema))
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            limits::middleware,
        ))
        // Outside the limits: preflights aren't counted, and a 429 still carries CORS headers.
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            cors::middleware,
        ))
        // Around the limits and CORS, so 429s and refused preflights are counted too. Requests
        // for no route at all are not.
        .route_layer(axum::middleware::from_fn_with_state(
            state.metrics.clone(),
            cyex_tv_shared::metrics::track,
        ))
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

/// The api's scrape-time gauges: live-update subscribers and the database pool.
pub async fn scrape_gauges(state: &AppState) -> String {
    use cyex_tv_shared::metrics::gauge;
    let mut out = String::new();
    let (subscribers, topics) = state.hub.stats();
    gauge(
        &mut out,
        "cyex_tv_hub_subscribers",
        "GraphQL subscriptions connected.",
        &[(vec![], subscribers as f64)],
    );
    gauge(
        &mut out,
        "cyex_tv_hub_topics",
        "Topics watched by at least one subscriber.",
        &[(vec![], topics as f64)],
    );
    pool_gauges(&mut out, &state.db);
    out
}

/// Pool size and idle connections.
fn pool_gauges(out: &mut String, db: &sqlx::PgPool) {
    use cyex_tv_shared::metrics::gauge;
    gauge(
        out,
        "cyex_tv_db_pool_connections",
        "Database connections open.",
        &[(vec![], f64::from(db.size()))],
    );
    gauge(
        out,
        "cyex_tv_db_pool_idle",
        "Database connections idle.",
        &[(vec![], db.num_idle() as f64)],
    );
}

/// `POST /v1/gql`, run as the caller. A cookie caller must send the site's `Origin`, since every
/// POST counts as a write (`bad_origin` otherwise).
async fn graphql(
    State(state): State<AppState>,
    Extension(schema): Extension<ApiSchema>,
    MaybeCaller(caller): MaybeCaller,
    request: GraphQLRequest,
) -> axum::response::Response {
    if let Some(caller) = &caller
        && let Err(wait) = state
            .limits
            .gql_account
            .check(&cyex_tv_db::ids::to_text(caller.account_id))
    {
        return limits::too_many(wait);
    }
    let response: GraphQLResponse = schema
        .execute(request.into_inner().data(caller))
        .await
        .into();
    response.into_response()
}

/// `/v1/gql/ws` (`graphql-transport-ws`, or the older `graphql-ws`): subscriptions only, at most
/// `gql::MAX_SOCKET_SUBSCRIPTIONS` at once. No caller is attached, so nothing on the socket runs
/// as a signed-in account.
async fn graphql_ws(
    Extension(schema): Extension<ApiSchema>,
    protocol: GraphQLProtocol,
    upgrade: WebSocketUpgrade,
) -> axum::response::Response {
    upgrade
        .protocols(ALL_WEBSOCKET_PROTOCOLS)
        .on_upgrade(move |stream| {
            let mut data = async_graphql::Data::default();
            data.insert(gql::SocketConnection::default());
            GraphQLWebSocket::new(stream, schema, protocol)
                .with_data(data)
                .serve()
        })
}

/// The OpenAPI document, as committed in `openapi/openapi.json`.
pub fn openapi() -> utoipa::openapi::OpenApi {
    router().split_for_parts().1
}
