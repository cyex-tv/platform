//! The cyex.tv API (ADR 0008): GraphQL at `/v1/gql` for most things, plus a small REST part.
//! The REST router and its OpenAPI document come from the same handlers, and the GraphQL SDL from
//! the schema itself, so neither committed description can drift from the code.

pub mod abuse;
pub mod account;
pub mod activity;
pub mod admin;
pub mod catalog;
pub mod codes;
pub mod console_token;
pub mod cors;
pub mod cosmetics;
pub mod developers;
pub mod editors;
pub mod errors;
pub mod gql;
pub mod idempotency;
pub mod limits;
pub mod meta;
pub mod moderation;
pub mod paddle;
pub mod personal;
pub mod public_ids;
pub mod public_limits;
pub mod read_cache;
pub mod review;
pub mod routes;
pub mod search;
pub mod shop;
pub mod site;
pub mod staff;
pub mod state;
pub mod top_people;
pub mod transfers;
pub mod upload;
pub mod views;
pub mod webhooks;

use async_graphql::http::ALL_WEBSOCKET_PROTOCOLS;
use async_graphql_axum::{GraphQLProtocol, GraphQLRequest, GraphQLResponse};
use axum::Router;
use axum::extract::{Extension, State, WebSocketUpgrade};
use axum::response::IntoResponse;
use axum::routing::get;
use tower_http::trace::TraceLayer;
use utoipa::OpenApi;
use utoipa_axum::router::OpenApiRouter;

use crate::gql::ApiSchema;
use crate::state::{AppState, MaybeCaller};

/// Adds `components.schemas.ErrorCode`: every stable error code (`cyex_tv_shared::error_codes`),
/// as the enum of codes and, under `x-codes`, each one's group, meaning and when it happens.
struct ErrorCodes;

impl utoipa::Modify for ErrorCodes {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        use cyex_tv_shared::error_codes::ERROR_CODES;
        use utoipa::openapi::extensions::ExtensionsBuilder;
        use utoipa::openapi::schema::{ObjectBuilder, Type};

        let table: Vec<serde_json::Value> = ERROR_CODES
            .iter()
            .map(|e| {
                serde_json::json!({
                    "code": e.code,
                    "group": e.group,
                    "meaning": e.meaning,
                    "when": e.when,
                })
            })
            .collect();
        let schema = ObjectBuilder::new()
            .schema_type(Type::String)
            .description(Some(
                "A stable error code, the `error` of an error answer (and `extensions.code` in                  GraphQL). Match on it; `message` is for people. Codes are never renamed or reused,                  and new ones may appear, so treat an unknown code like its HTTP status. `x-codes`                  says what each means and when it comes.",
            ))
            .enum_values(Some(ERROR_CODES.iter().map(|e| e.code)))
            .extensions(Some(ExtensionsBuilder::new().add("x-codes", table).build()))
            .build();
        openapi
            .components
            .get_or_insert_with(Default::default)
            .schemas
            .insert("ErrorCode".to_owned(), schema.into());
    }
}

#[derive(OpenApi)]
#[openapi(
    modifiers(&ErrorCodes),
    info(
        title = "cyex.tv REST API",
        description = "The REST part of the cyex.tv API: sign-in, uploads and the cacheable reads chat clients poll. Everything else is GraphQL at /v1/gql.",
        license(name = "AGPL-3.0-only")
    ),
    servers((url = "https://api.cyex.tv", description = "Production")),
    tags(
        (name = "system", description = "Health checks."),
        (name = "auth", description = "Signing in with Twitch or Kick, and app sign-in for desktop apps (authorization code with PKCE and a loopback redirect, ADR 0021)."),
        (name = "emotes", description = "Uploading emotes."),
        (name = "cosmetics", description = "cyex.tv chat badges and name paints (ADR 0015): the catalog, chatters' picks, and staff picture uploads."),
        (name = "shop", description = "The cosmetics shop (ADR 0016): Paddle's webhook. Items, checkout, orders and codes are GraphQL."),
        (name = "reads", description = "Public, cacheable reads for chat clients. Every answer carries `RateLimit-Limit`, `RateLimit-Remaining` and `RateLimit-Reset` (seconds until the allowance is full again); a `429` also has `Retry-After`. Bots can send a developer API key as `Authorization: Bearer cyex_live_...` (made on your cyex.tv profile, ADR 0033): it reads the same data under a limit of its own, which the headers then describe.")
    )
)]
pub struct ApiDoc;

/// The REST routes, with their OpenAPI description attached.
pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::with_openapi(ApiDoc::openapi())
        .merge(routes::system::router())
        .merge(routes::v1::router())
        .merge(routes::app_auth::router())
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
        // Outside the limits and CORS: the version header (and the deprecation headers) go on
        // 429s too.
        .layer(axum::middleware::from_fn(meta::middleware))
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
// Each argument is an axum extractor; grouping them would only hide what the handler reads.
#[allow(clippy::too_many_arguments)]
async fn graphql(
    State(state): State<AppState>,
    Extension(schema): Extension<ApiSchema>,
    ip: Option<Extension<limits::ClientIp>>,
    key: Option<Extension<developers::KeyUse>>,
    method: axum::http::Method,
    headers: axum::http::HeaderMap,
    MaybeCaller(caller): MaybeCaller,
    request: GraphQLRequest,
) -> axum::response::Response {
    // Without a live session, the cookie may still belong to an account whose deletion can be
    // undone (ADR 0019): `pendingDeletion` and `restoreAccount` use it, nothing else does.
    let restorable = match &caller {
        Some(_) => None,
        None => match state
            .auth
            .restorable_caller(&state.db, &method, &headers)
            .await
        {
            Ok(found) => found,
            Err(err) => return cyex_tv_shared::error::ApiError::from(err).into_response(),
        },
    };
    // A signed-in caller counts under the per-account limit, which is the one the
    // `RateLimit-*` headers describe (over the per-IP one the middleware sets).
    let mut quota = None;
    if let Some(caller) = &caller {
        match limits::take_quota(
            &state.limits,
            limits::AccountLimit::GqlAccount,
            &cyex_tv_db::ids::to_text(caller.account_id),
            caller.account_id,
        ) {
            Ok(q) => quota = Some(q),
            Err((wait, q)) => return limits::too_many_quota(wait, q),
        }
    }
    let ip = ip.map_or_else(|| limits::ClientIp(String::new()), |Extension(ip)| ip);
    let response: GraphQLResponse = schema
        .execute(
            request
                .into_inner()
                .data(caller)
                .data(gql::Restorable(restorable))
                .data(key.map(|Extension(k)| k))
                .data(ip),
        )
        .await
        .into();
    let mut response = response.into_response();
    if let Some(q) = quota {
        limits::set_quota(response.headers_mut(), q);
    }
    response
}

/// `/v1/gql/ws` (`graphql-transport-ws`, or the older `graphql-ws`): subscriptions only, at most
/// `gql::MAX_SOCKET_SUBSCRIPTIONS` at once. No caller is attached, so nothing on the socket runs
/// as a signed-in account. Messages are at most `gql::MAX_SOCKET_MESSAGE`, `connection_init` must
/// come within `gql::SOCKET_INIT_DEADLINE`, a socket silent for `gql::SOCKET_IDLE` is closed,
/// and one IP holds at most `Rates::sockets_per_ip` open at once (429 past that).
async fn graphql_ws(
    State(state): State<AppState>,
    Extension(schema): Extension<ApiSchema>,
    ip: Option<Extension<limits::ClientIp>>,
    protocol: GraphQLProtocol,
    upgrade: WebSocketUpgrade,
) -> axum::response::Response {
    let ip = ip.map_or_else(String::new, |Extension(ip)| ip.0);
    let Some(slot) = state.limits.sockets.try_take(&ip) else {
        tracing::info!(%ip, "too many open sockets");
        return limits::too_many_open();
    };
    upgrade
        .protocols(ALL_WEBSOCKET_PROTOCOLS)
        .max_message_size(gql::MAX_SOCKET_MESSAGE)
        .max_frame_size(gql::MAX_SOCKET_MESSAGE)
        .on_upgrade(move |stream| async move {
            gql::serve_socket(stream, schema, protocol).await;
            drop(slot);
        })
}

/// The OpenAPI document, as committed in `openapi/openapi.json`.
pub fn openapi() -> utoipa::openapi::OpenApi {
    router().split_for_parts().1
}
