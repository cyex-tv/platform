//! The REST part of v1 (ADR 0008). Everything else is GraphQL at `/v1/gql`. REST covers only:
//! - sign-in redirects (Phase 2);
//! - the streamed upload (Phase 3);
//! - the small, cacheable reads chat clients poll (Phase 2).
//!
//! Every handler here answers `501 not_implemented` until its phase builds it.

use axum::Json;
use axum::http::StatusCode;
use axum::response::Redirect;
use cyex_tv_shared::error::{ApiError, ApiResult, ErrorBody};
use cyex_tv_shared::model::{
    CallbackQuery, Channel, Emote, EmoteSet, LoginQuery, PersonalLookup, PersonalLookupResult,
    Platform, UploadEmote,
};
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::state::AppState;

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(login))
        .routes(routes!(callback))
        .routes(routes!(logout))
        .routes(routes!(upload_emote))
        .routes(routes!(get_channel))
        .routes(routes!(get_set))
        .routes(routes!(get_global_set))
        .routes(routes!(get_emote))
        .routes(routes!(lookup_personal_sets))
}

// Auth

/// Start signing in with Twitch or Kick.
///
/// Redirects to the platform's consent page. With `link=true` and a signed-in session, the
/// identity is linked to the current account instead.
#[utoipa::path(
    get,
    path = "/v1/auth/{platform}/login",
    tag = "auth",
    params(("platform" = Platform, Path, description = "`twitch` or `kick`."), LoginQuery),
    responses(
        (status = 302, description = "Redirect to the platform's sign-in page."),
        (status = 400, description = "Unknown platform or bad `return_to`.", body = ErrorBody)
    )
)]
async fn login() -> ApiResult<Redirect> {
    Err(ApiError::not_implemented())
}

/// Where the platform sends the browser back to.
///
/// Sets the session cookie and redirects to `return_to`.
#[utoipa::path(
    get,
    path = "/v1/auth/{platform}/callback",
    tag = "auth",
    params(("platform" = Platform, Path, description = "`twitch` or `kick`."), CallbackQuery),
    responses(
        (status = 302, description = "Signed in. Redirect to `return_to`."),
        (status = 400, description = "Bad or expired `state`, or the user declined.", body = ErrorBody),
        (status = 409, description = "`identity_taken`: linked to another account.", body = ErrorBody)
    )
)]
async fn callback() -> ApiResult<Redirect> {
    Err(ApiError::not_implemented())
}

/// Sign out and end the session.
#[utoipa::path(
    post,
    path = "/v1/auth/logout",
    tag = "auth",
    responses((status = 204, description = "Signed out."))
)]
async fn logout() -> ApiResult<StatusCode> {
    Err(ApiError::not_implemented())
}

// Upload

/// Upload an emote. It starts `pending`: usable in your own sets, hidden from everyone else until a
/// moderator approves it. Manage it afterwards through GraphQL.
///
/// PNG, APNG, GIF, WebP or AVIF; at most 5 MB, 1000×1000 px and 1000 frames.
#[utoipa::path(
    post,
    path = "/v1/emotes",
    tag = "emotes",
    request_body(content = UploadEmote, content_type = "multipart/form-data"),
    responses(
        (status = 202, description = "Accepted; processing.", body = Emote),
        (status = 413, description = "`too_large`.", body = ErrorBody),
        (status = 415, description = "`unsupported_format`.", body = ErrorBody),
        (status = 422, description = "`invalid_image` or `blocked`.", body = ErrorBody)
    )
)]
async fn upload_emote() -> ApiResult<Json<Emote>> {
    Err(ApiError::not_implemented())
}

// Chat-client reads: public, no auth, `ETag` and `Cache-Control`.

/// A channel and its active set ID. Poll this, then fetch the set.
#[utoipa::path(
    get,
    path = "/v1/channels/{platform}/{id}",
    tag = "reads",
    params(
        ("platform" = Platform, Path, description = "`twitch` or `kick`."),
        ("id" = String, Path, description = "The broadcaster's platform user ID.")
    ),
    responses(
        (status = 200, body = Channel),
        (status = 304, description = "Not modified."),
        (status = 404, description = "`not_found`: no cyex.tv account has this channel.", body = ErrorBody)
    )
)]
async fn get_channel() -> ApiResult<Json<Channel>> {
    Err(ApiError::not_implemented())
}

/// A set with its emotes.
#[utoipa::path(
    get,
    path = "/v1/sets/{id}",
    tag = "reads",
    params(("id" = String, Path, description = "Set ID.")),
    responses(
        (status = 200, body = EmoteSet),
        (status = 304, description = "Not modified."),
        (status = 404, body = ErrorBody)
    )
)]
async fn get_set() -> ApiResult<Json<EmoteSet>> {
    Err(ApiError::not_implemented())
}

/// The cyex.tv global set, shown in every channel.
#[utoipa::path(
    get,
    path = "/v1/sets/global",
    tag = "reads",
    responses((status = 200, body = EmoteSet), (status = 304, description = "Not modified."))
)]
async fn get_global_set() -> ApiResult<Json<EmoteSet>> {
    Err(ApiError::not_implemented())
}

/// An approved emote.
#[utoipa::path(
    get,
    path = "/v1/emotes/{id}",
    tag = "reads",
    params(("id" = String, Path, description = "Emote ID.")),
    responses((status = 200, body = Emote), (status = 404, body = ErrorBody))
)]
async fn get_emote() -> ApiResult<Json<Emote>> {
    Err(ApiError::not_implemented())
}

/// Personal sets for up to 100 chatters at once, for clients rendering a chat.
#[utoipa::path(
    post,
    path = "/v1/personal-sets/lookup",
    tag = "reads",
    request_body = PersonalLookup,
    responses((status = 200, body = PersonalLookupResult))
)]
async fn lookup_personal_sets() -> ApiResult<Json<PersonalLookupResult>> {
    Err(ApiError::not_implemented())
}
