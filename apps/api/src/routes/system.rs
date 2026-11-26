//! Health checks for Docker and the tunnel.

use axum::Json;
use axum::extract::State;
use cyex_tv_shared::error::{ApiError, ApiResult, ErrorBody};
use cyex_tv_shared::model::Health;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::state::AppState;

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(health))
        .routes(routes!(ready))
        .routes(routes!(get_meta))
}

/// The process is up.
#[utoipa::path(
    get,
    path = "/health",
    tag = "system",
    responses((status = 200, description = "The service is running.", body = Health))
)]
async fn health() -> Json<Health> {
    Json(Health {
        status: "ok".to_owned(),
    })
}

/// The service can reach its database.
#[utoipa::path(
    get,
    path = "/health/ready",
    tag = "system",
    responses(
        (status = 200, description = "Ready to serve.", body = Health),
        (status = 503, description = "The database isn't reachable.", body = ErrorBody)
    )
)]
async fn ready(State(state): State<AppState>) -> ApiResult<Json<Health>> {
    cyex_tv_db::ping(&state.db).await.map_err(|err| {
        tracing::warn!(error = %err, "database not ready");
        ApiError::unavailable("The database isn't reachable.")
    })?;
    Ok(Json(Health {
        status: "ok".to_owned(),
    }))
}

/// The API's version, build, where its limits are published, and what is deprecated with the
/// sunset date of each (ADR 0027 for the first one). The GraphQL `meta` query answers the same.
/// Every answer of the API also carries `X-Cyex-Api-Version`.
#[utoipa::path(
    get,
    path = "/v1/meta",
    tag = "system",
    responses((status = 200, description = "Version, build, limits link and deprecations.", body = crate::meta::ApiMeta))
)]
async fn get_meta() -> (
    [(axum::http::HeaderName, &'static str); 1],
    Json<crate::meta::ApiMeta>,
) {
    (
        [(axum::http::header::CACHE_CONTROL, "public, max-age=60")],
        Json(crate::meta::meta()),
    )
}
