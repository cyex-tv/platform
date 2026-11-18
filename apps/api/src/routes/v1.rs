//! The REST part of v1 (ADR 0008). Everything else is GraphQL at `/v1/gql`. REST covers only:
//! - sign-in redirects (Phase 2, built);
//! - the upload (Phase 3, built; transcoding is the worker's);
//! - the small, cacheable reads chat clients poll (built).
//!
//! The others answer `501 not_implemented` until their phase builds them.

use axum::Json;
use axum::extract::{DefaultBodyLimit, Multipart, Path, Query, State};
use axum::http::header::{CACHE_CONTROL, CONTENT_TYPE, ETAG, IF_NONE_MATCH, LOCATION, SET_COOKIE};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use cyex_tv_auth::AuthError;
use cyex_tv_auth::cookies;
use cyex_tv_auth::flow::Callback;
use cyex_tv_shared::error::{ApiError, ApiResult, ErrorBody};
use cyex_tv_shared::model::{
    CallbackQuery, Channel, CosmeticCatalog, CosmeticImage, CosmeticKind, Emote, EmoteSet,
    LoginQuery, PersonalLookup, PersonalLookupResult, Platform, UploadCosmeticImage, UploadEmote,
    UserCosmeticsQuery, UserCosmeticsResult,
};
use serde::Serialize;
use sha2::{Digest, Sha256};
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::state::{AppState, MaybeCaller};
use crate::{catalog, cosmetics, moderation, upload};

/// The upload body may be a little over the 5 MB file limit (multipart framing, the name and
/// tags); anything beyond this is cut off before it is read.
const UPLOAD_BODY_LIMIT: usize = 5 * 1024 * 1024 + 64 * 1024;

pub fn router() -> OpenApiRouter<AppState> {
    let upload = OpenApiRouter::new()
        .routes(routes!(upload_emote))
        .routes(routes!(upload_cosmetic_image))
        .layer(DefaultBodyLimit::max(UPLOAD_BODY_LIMIT));
    OpenApiRouter::new()
        .merge(upload)
        .routes(routes!(login))
        .routes(routes!(callback))
        .routes(routes!(logout))
        .routes(routes!(get_channel))
        .routes(routes!(get_set))
        .routes(routes!(get_global_set))
        .routes(routes!(get_emote))
        .routes(routes!(lookup_personal_sets))
        .routes(routes!(get_cosmetics))
        .routes(routes!(get_user_cosmetics))
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
/// PNG, APNG, GIF or WebP; at most 5 MB, 1000×1000 px and 1000 frames. Sign in first (with the
/// cookie, from `https://cyex.tv`). At most 20 uploads an hour. The flags `zero_width`, `private`
/// and `nsfw` may be set at once (each `true` or `false`, default `false`); `updateEmote` changes
/// them later.
#[utoipa::path(
    post,
    path = "/v1/emotes",
    tag = "emotes",
    request_body(content = UploadEmote, content_type = "multipart/form-data"),
    responses(
        (status = 202, description = "Accepted; the files are being made. The emote is `pending`.", body = Emote),
        (status = 400, description = "`bad_name`, `bad_tag`, `too_many_tags` or `bad_form`.", body = ErrorBody),
        (status = 401, description = "`signed_out`.", body = ErrorBody),
        (status = 403, description = "`restricted`, or `bad_origin` for a cookie from another site.", body = ErrorBody),
        (status = 413, description = "`too_large`: over 5 MB, 1000×1000 px or 1000 frames.", body = ErrorBody),
        (status = 415, description = "`unsupported_format`: not PNG, APNG, GIF or WebP.", body = ErrorBody),
        (status = 422, description = "`invalid_image` (doesn't decode) or `blocked`.", body = ErrorBody),
        (status = 429, description = "`rate_limited`.", body = ErrorBody),
        (status = 503, description = "`unavailable`: storage isn't reachable.", body = ErrorBody)
    )
)]
async fn upload_emote(
    State(state): State<AppState>,
    MaybeCaller(caller): MaybeCaller,
    mut form: Multipart,
) -> ApiResult<(StatusCode, Json<Emote>)> {
    let caller = caller.ok_or_else(|| ApiError::from(AuthError::SignedOut))?;
    let bad_form = |message: &str| ApiError::new(StatusCode::BAD_REQUEST, "bad_form", message);
    let mut name = None;
    let mut tags = None;
    let mut flags = upload::Flags::default();
    let mut file: Option<Vec<u8>> = None;
    while let Some(mut field) = form
        .next_field()
        .await
        .map_err(|_| bad_form("The form couldn't be read."))?
    {
        match field.name() {
            Some("file") => {
                let mut bytes = Vec::new();
                while let Some(chunk) = field.chunk().await.map_err(|err| {
                    if err.status() == StatusCode::PAYLOAD_TOO_LARGE {
                        upload::too_large()
                    } else {
                        bad_form("The file couldn't be read.")
                    }
                })? {
                    if bytes.len() + chunk.len() > cyex_tv_media::Limits::default().max_bytes {
                        return Err(upload::too_large());
                    }
                    bytes.extend_from_slice(&chunk);
                }
                file = Some(bytes);
            }
            Some(key @ ("name" | "tags")) => {
                let key = key.to_owned();
                let text = field
                    .text()
                    .await
                    .map_err(|_| bad_form("A form field couldn't be read."))?;
                if text.len() > 1000 {
                    return Err(bad_form("A form field is too long."));
                }
                if key == "name" {
                    name = Some(text);
                } else {
                    tags = Some(text);
                }
            }
            Some(key @ ("zero_width" | "private" | "nsfw")) => {
                let key = key.to_owned();
                let text = field
                    .text()
                    .await
                    .map_err(|_| bad_form("A form field couldn't be read."))?;
                let on = upload::parse_flag(&text)
                    .ok_or_else(|| bad_form("A flag is true or false."))?;
                match key.as_str() {
                    "zero_width" => flags.zero_width = on,
                    "private" => flags.private = on,
                    _ => flags.nsfw = on,
                }
            }
            _ => {
                return Err(bad_form(
                    "Unknown form field. Send name, tags, zero_width, private, nsfw and file.",
                ));
            }
        }
    }
    let name = name.ok_or_else(|| bad_form("Give the emote a name."))?;
    let file = file.ok_or_else(|| bad_form("Attach the image as `file`."))?;
    let emote = upload::upload(
        &state,
        &caller,
        upload::Upload {
            name,
            tags: upload::split_tags(tags.as_deref()),
            flags,
            file: file.into(),
        },
    )
    .await?;
    Ok((StatusCode::ACCEPTED, Json(emote)))
}

/// Reads a multipart file field under the 5 MB limit.
async fn file_field(
    field: &mut axum::extract::multipart::Field<'_>,
    bad_form: impl Fn(&str) -> ApiError,
) -> ApiResult<Vec<u8>> {
    let mut bytes = Vec::new();
    while let Some(chunk) = field.chunk().await.map_err(|err| {
        if err.status() == StatusCode::PAYLOAD_TOO_LARGE {
            upload::too_large()
        } else {
            bad_form("The file couldn't be read.")
        }
    })? {
        if bytes.len() + chunk.len() > cyex_tv_media::Limits::default().max_bytes {
            return Err(upload::too_large());
        }
        bytes.extend_from_slice(&chunk);
    }
    Ok(bytes)
}

/// Upload a badge or paint picture (staff, `cosmetics.manage`).
///
/// The worker makes its files (WebP and AVIF at 1x, 2x and 4x) under `cosmetics/`; poll
/// `cosmeticImage(id)` until `ready`, then use the ID in `createBadge` or `createPaint`. Badges:
/// square, at least 72×72 px. Paints: at least 64 px tall. PNG, APNG, GIF or WebP, at most 5 MB.
/// The same bytes give the same picture again.
#[utoipa::path(
    post,
    path = "/v1/cosmetics/images",
    tag = "cosmetics",
    request_body(content = UploadCosmeticImage, content_type = "multipart/form-data"),
    responses(
        (status = 202, description = "Accepted; the files are being made.", body = CosmeticImage),
        (status = 400, description = "`bad_form` or `bad_kind`.", body = ErrorBody),
        (status = 401, description = "`signed_out`.", body = ErrorBody),
        (status = 403, description = "`forbidden` (needs `cosmetics.manage`), or `bad_origin`.", body = ErrorBody),
        (status = 413, description = "`too_large`.", body = ErrorBody),
        (status = 415, description = "`unsupported_format`.", body = ErrorBody),
        (status = 422, description = "`invalid_image` or `bad_dimensions`.", body = ErrorBody),
        (status = 429, description = "`rate_limited`.", body = ErrorBody),
        (status = 503, description = "`unavailable`: storage isn't reachable.", body = ErrorBody)
    )
)]
async fn upload_cosmetic_image(
    State(state): State<AppState>,
    MaybeCaller(caller): MaybeCaller,
    mut form: Multipart,
) -> ApiResult<(StatusCode, Json<CosmeticImage>)> {
    let caller = caller.ok_or_else(|| ApiError::from(AuthError::SignedOut))?;
    let bad_form = |message: &str| ApiError::new(StatusCode::BAD_REQUEST, "bad_form", message);
    let mut kind = None;
    let mut file: Option<Vec<u8>> = None;
    while let Some(mut field) = form
        .next_field()
        .await
        .map_err(|_| bad_form("The form couldn't be read."))?
    {
        match field.name() {
            Some("file") if file.is_none() => file = Some(file_field(&mut field, bad_form).await?),
            Some("kind") if kind.is_none() => {
                let text = field
                    .text()
                    .await
                    .map_err(|_| bad_form("A form field couldn't be read."))?;
                kind = Some(CosmeticKind::parse(text.trim()).ok_or_else(|| {
                    ApiError::new(
                        StatusCode::BAD_REQUEST,
                        "bad_kind",
                        "kind is badge or paint.",
                    )
                })?);
            }
            _ => {
                return Err(bad_form(
                    "Unknown or repeated form field. Send kind and file.",
                ));
            }
        }
    }
    let kind = kind.ok_or_else(|| bad_form("Say what it is for: kind=badge or kind=paint."))?;
    let file = file.ok_or_else(|| bad_form("Attach the image as `file`."))?;
    let image = cosmetics::upload_image(&state, &caller, kind, file.into()).await?;
    Ok((StatusCode::ACCEPTED, Json(image)))
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
    responses(
        (status = 200, body = Emote),
        (status = 304, description = "Not modified."),
        (status = 404, description = "`not_found`: no such approved emote.", body = ErrorBody)
    )
)]
async fn get_emote(
    State(state): State<AppState>,
    Path(id): Path<String>,
    headers: HeaderMap,
) -> ApiResult<Response> {
    let emote = moderation::emote(&state, None, &id)
        .await?
        .ok_or_else(catalog::not_found)?;
    cached(&headers, &emote, READ_MAX_AGE)
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

/// Every cyex.tv chat badge and name paint (ADR 0015). Cache it with `If-None-Match`; live
/// changes come as `cosmetic.*` events on the `cosmetics` topic.
#[utoipa::path(
    get,
    path = "/v1/cosmetics",
    tag = "cosmetics",
    responses((status = 200, body = CosmeticCatalog), (status = 304, description = "Not modified."))
)]
async fn get_cosmetics(State(state): State<AppState>, headers: HeaderMap) -> ApiResult<Response> {
    let catalog = cosmetics::catalog(&state).await?;
    cached(&headers, &catalog, READ_MAX_AGE)
}

/// The badge and paint up to 100 chatters show, as catalog IDs. Only chatters showing something
/// are listed; changes come as `user.cosmetics_updated` on their `user:` topics.
#[utoipa::path(
    get,
    path = "/v1/cosmetics/users",
    tag = "cosmetics",
    params(UserCosmeticsQuery),
    responses(
        (status = 200, body = UserCosmeticsResult),
        (status = 304, description = "Not modified."),
        (status = 400, description = "`too_many`: more than 100 IDs.", body = ErrorBody)
    )
)]
async fn get_user_cosmetics(
    State(state): State<AppState>,
    Query(query): Query<UserCosmeticsQuery>,
    headers: HeaderMap,
) -> ApiResult<Response> {
    let ids: Vec<String> = query
        .ids
        .split(',')
        .map(str::to_owned)
        .filter(|s| !s.trim().is_empty())
        .collect();
    let users = cosmetics::user_cosmetics(&state, query.platform, &ids).await?;
    cached(&headers, &UserCosmeticsResult { users }, READ_MAX_AGE)
}
