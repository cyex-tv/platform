//! The REST part of v1 (ADR 0008). Everything else is GraphQL at `/v1/gql`. REST covers only:
//! - sign-in redirects (Phase 2, built);
//! - the streamed upload (Phase 3);
//! - the small, cacheable reads chat clients poll (Phase 2, built; `GET /v1/emotes/{id}` is
//!   Phase 3).
//!
//! The others answer `501 not_implemented` until their phase builds them.

use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::header::{CACHE_CONTROL, CONTENT_TYPE, ETAG, IF_NONE_MATCH, LOCATION, SET_COOKIE};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use cyex_tv_auth::AuthError;
use cyex_tv_auth::cookies;
use cyex_tv_auth::flow::Callback;
use cyex_tv_shared::error::{ApiError, ApiResult, ErrorBody};
use cyex_tv_shared::model::{
    CallbackQuery, Channel, Emote, EmoteSet, LoginQuery, PersonalLookup, PersonalLookupResult,
    Platform, UploadEmote,
};
use serde::Serialize;
use sha2::{Digest, Sha256};
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::catalog;
use crate::state::{AppState, MaybeCaller};

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

fn platform(raw: &str) -> ApiResult<Platform> {
    raw.parse()
        .map_err(|_| ApiError::from(AuthError::UnknownPlatform))
}

/// A `302 Found` that sets cookies and is never cached.
fn found(location: &str, cookies: &[String]) -> ApiResult<Response> {
    let mut response = StatusCode::FOUND.into_response();
    let headers = response.headers_mut();
    headers.insert(
        LOCATION,
        HeaderValue::from_str(location).map_err(|_| ApiError::internal())?,
    );
    headers.insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));
    for cookie in cookies {
        headers.append(
            SET_COOKIE,
            HeaderValue::from_str(cookie).map_err(|_| ApiError::internal())?,
        );
    }
    Ok(response)
}

/// Start signing in with Twitch or Kick.
///
/// Redirects to the platform's consent page and sets a short-lived cookie holding `state`. With
/// `link=true` and a signed-in session, the identity is linked to the current account instead.
#[utoipa::path(
    get,
    path = "/v1/auth/{platform}/login",
    tag = "auth",
    params(("platform" = Platform, Path, description = "`twitch` or `kick`."), LoginQuery),
    responses(
        (status = 302, description = "Redirect to the platform's sign-in page."),
        (status = 400, description = "`unknown_platform` or `bad_return_to`.", body = ErrorBody),
        (status = 401, description = "`signed_out`: `link=true` without a session.", body = ErrorBody),
        (status = 503, description = "`unavailable`: sign-in with this platform isn't set up.", body = ErrorBody)
    )
)]
async fn login(
    State(state): State<AppState>,
    Path(raw): Path<String>,
    Query(query): Query<LoginQuery>,
    MaybeCaller(caller): MaybeCaller,
) -> ApiResult<Response> {
    let platform = platform(&raw)?;
    let started = state
        .auth
        .start(
            &state.db,
            platform,
            query.return_to.as_deref(),
            query.link.unwrap_or(false),
            caller.as_ref(),
        )
        .await?;
    found(&started.location, &[started.set_cookie])
}

/// Where the platform sends the browser back to.
///
/// Checks `state` against the cookie set at login, signs in (or links), sets the session cookie
/// and redirects to `return_to`.
#[utoipa::path(
    get,
    path = "/v1/auth/{platform}/callback",
    tag = "auth",
    params(("platform" = Platform, Path, description = "`twitch` or `kick`."), CallbackQuery),
    responses(
        (status = 302, description = "Signed in. Redirect to `return_to`."),
        (status = 400, description = "`bad_state` (bad, expired or used `state`) or `declined` (the user cancelled).", body = ErrorBody),
        (status = 403, description = "`account_deleted`.", body = ErrorBody),
        (status = 409, description = "`identity_taken`: linked to another account; `platform_already_linked`.", body = ErrorBody),
        (status = 502, description = "`platform_error`: the platform didn't answer as expected.", body = ErrorBody)
    )
)]
async fn callback(
    State(state): State<AppState>,
    Path(raw): Path<String>,
    Query(query): Query<CallbackQuery>,
    headers: HeaderMap,
    MaybeCaller(caller): MaybeCaller,
) -> ApiResult<Response> {
    let platform = platform(&raw)?;
    let cookie_state = cookies::read(&headers, cookies::STATE);
    let finished = state
        .auth
        .finish(
            &state.db,
            platform,
            &Callback {
                code: query.code.as_deref(),
                state: query.state.as_deref(),
                error: query.error.as_deref(),
            },
            cookie_state.as_deref(),
            caller.as_ref(),
        )
        .await?;
    tracing::info!(
        account = %cyex_tv_db::ids::to_text(finished.account_id),
        %platform,
        created = finished.created,
        linked = finished.linked,
        "signed in"
    );
    found(&finished.location, &finished.set_cookies)
}

/// Sign out and end the session.
///
/// With the cookie, the request must come from `https://cyex.tv` (`Origin`).
#[utoipa::path(
    post,
    path = "/v1/auth/logout",
    tag = "auth",
    responses(
        (status = 204, description = "Signed out. The session cookie is cleared."),
        (status = 403, description = "`bad_origin`: the cookie was sent from another site.", body = ErrorBody)
    )
)]
async fn logout(
    State(state): State<AppState>,
    MaybeCaller(caller): MaybeCaller,
) -> ApiResult<Response> {
    let clear = state.auth.sign_out(&state.db, caller.as_ref()).await?;
    let mut response = StatusCode::NO_CONTENT.into_response();
    response.headers_mut().insert(
        SET_COOKIE,
        HeaderValue::from_str(&clear).map_err(|_| ApiError::internal())?,
    );
    Ok(response)
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

/// How long clients and the CDN may keep a read before asking again (`If-None-Match` then gets a
/// cheap 304). Live updates (Phase 5) push changes sooner.
const READ_MAX_AGE: u32 = 30;

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Answers JSON with a strong `ETag` (a hash of the body), or `304` when `If-None-Match` has it.
fn cached<T: Serialize>(headers: &HeaderMap, value: &T, max_age: u32) -> ApiResult<Response> {
    let body = serde_json::to_vec(value).map_err(|_| ApiError::internal())?;
    let etag = format!("\"{}\"", hex(&Sha256::digest(&body)[..16]));
    let cache = format!("public, max-age={max_age}, stale-while-revalidate=60");
    let fresh = headers
        .get(IF_NONE_MATCH)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| {
            v.split(',')
                .map(str::trim)
                .any(|t| t == "*" || t == etag || t.strip_prefix("W/") == Some(etag.as_str()))
        });
    let mut response = if fresh {
        StatusCode::NOT_MODIFIED.into_response()
    } else {
        ([(CONTENT_TYPE, "application/json")], body).into_response()
    };
    let out = response.headers_mut();
    out.insert(
        ETAG,
        HeaderValue::from_str(&etag).map_err(|_| ApiError::internal())?,
    );
    out.insert(
        CACHE_CONTROL,
        HeaderValue::from_str(&cache).map_err(|_| ApiError::internal())?,
    );
    Ok(response)
}

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
async fn get_channel(
    State(state): State<AppState>,
    Path((raw, id)): Path<(String, String)>,
    headers: HeaderMap,
) -> ApiResult<Response> {
    let platform = platform(&raw)?;
    let channel = catalog::channel(&state, platform, &id)
        .await?
        .ok_or_else(catalog::not_found)?;
    cached(&headers, &channel, READ_MAX_AGE)
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
async fn get_set(
    State(state): State<AppState>,
    Path(id): Path<String>,
    headers: HeaderMap,
) -> ApiResult<Response> {
    let set = catalog::set(&state, None, &id)
        .await?
        .ok_or_else(catalog::not_found)?;
    cached(&headers, &set, READ_MAX_AGE)
}

/// The cyex.tv global set, shown in every channel.
#[utoipa::path(
    get,
    path = "/v1/sets/global",
    tag = "reads",
    responses((status = 200, body = EmoteSet), (status = 304, description = "Not modified."))
)]
async fn get_global_set(State(state): State<AppState>, headers: HeaderMap) -> ApiResult<Response> {
    let set = catalog::global_set(&state, None).await?;
    cached(&headers, &set, READ_MAX_AGE)
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
    responses(
        (status = 200, body = PersonalLookupResult),
        (status = 400, description = "`too_many`: more than 100 user IDs.", body = ErrorBody)
    )
)]
async fn lookup_personal_sets(
    State(state): State<AppState>,
    Json(lookup): Json<PersonalLookup>,
) -> ApiResult<Json<PersonalLookupResult>> {
    let sets = catalog::personal_sets(&state, lookup.platform, &lookup.user_ids).await?;
    Ok(Json(PersonalLookupResult { sets }))
}
