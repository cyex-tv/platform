//! The cyex.tv HTTP API. The router and the OpenAPI document come from the same handlers, so the
//! committed spec can't drift from the code (ADR 0008).

pub mod routes;
pub mod state;

use axum::Router;
use tower_http::trace::TraceLayer;
use utoipa::OpenApi;
use utoipa_axum::router::OpenApiRouter;

use crate::state::AppState;

#[derive(OpenApi)]
#[openapi(
    info(
        title = "cyex.tv API",
        description = "Emotes for Twitch and Kick. Public reads need no auth; writes need a cyex.tv session.",
        license(name = "AGPL-3.0-only")
    ),
    servers((url = "https://api.cyex.tv", description = "Production")),
    paths(routes::v1::events),
    tags(
        (name = "system", description = "Health checks."),
        (name = "auth", description = "Signing in with Twitch or Kick."),
        (name = "accounts", description = "The signed-in account and its linked identities."),
        (name = "channels", description = "Twitch and Kick channels and their active set."),
        (name = "sets", description = "Emote sets: channel, personal and global."),
        (name = "emotes", description = "Uploading and managing emotes."),
        (name = "search", description = "Finding approved emotes."),
        (name = "moderation", description = "The review queue. Needs `emotes.review`."),
        (name = "roles", description = "Roles and permissions. Needs `roles.manage`."),
        (name = "events", description = "Live updates over SSE, served by the events service.")
    )
)]
pub struct ApiDoc;

/// Every route, with its OpenAPI description attached.
pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::with_openapi(ApiDoc::openapi())
        .merge(routes::system::router())
        .merge(routes::v1::router())
}

/// The service, ready to serve.
pub fn app(state: AppState) -> Router {
    let (router, _spec) = router().split_for_parts();
    router.layer(TraceLayer::new_for_http()).with_state(state)
}

/// The OpenAPI document, as committed in `openapi/openapi.json`.
pub fn openapi() -> utoipa::openapi::OpenApi {
    router().split_for_parts().1
}
