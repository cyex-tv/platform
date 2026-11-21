//! The REST part of v1 (ADR 0008). Everything else is GraphQL at `/v1/gql`. REST covers only:
//! - sign-in redirects (Phase 2, built);
//! - the upload (Phase 3, built; transcoding is the worker's);
//! - the small, cacheable reads chat clients poll (built);
//! - Paddle's webhook for the shop (ADR 0016), which needs the raw body for its signature.
//!
//! The others answer `501 not_implemented` until their phase builds them.

use axum::Json;
use axum::extract::multipart::Field;
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
    EmoteVisibility, LoginQuery, PersonalLookup, PersonalLookupResult, Platform,
    UploadCosmeticImage, UploadEmote, UserCosmeticsQuery, UserCosmeticsResult,
};
use serde::Serialize;
use sha2::{Digest, Sha256};
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use std::sync::Arc;

use crate::read_cache::{self, Body};
use crate::state::{AppState, MaybeCaller};
use crate::{catalog, cosmetics, moderation, shop, upload};

/// The upload body may be a little over the 8 MB emote file limit (multipart framing, the name
/// and tags); anything beyond this is cut off before it is read. Cosmetic pictures stop at 5 MB
/// while they are read (`file_field`).
const UPLOAD_BODY_LIMIT: usize = cyex_tv_media::EMOTE_MAX_BYTES + 64 * 1024;

/// Paddle's notifications are a few KB; anything over 64 KB is refused before it is read.
const WEBHOOK_BODY_LIMIT: usize = 64 * 1024;

pub fn router() -> OpenApiRouter<AppState> {
    let upload = OpenApiRouter::new()
        .routes(routes!(upload_emote))
        .routes(routes!(upload_cosmetic_image))
        .layer(DefaultBodyLimit::max(UPLOAD_BODY_LIMIT));
    let webhook = OpenApiRouter::new()
        .routes(routes!(paddle_webhook))
        .layer(DefaultBodyLimit::max(WEBHOOK_BODY_LIMIT));
    OpenApiRouter::new()
        .merge(upload)
        .merge(webhook)
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
                user_agent: headers
                    .get(axum::http::header::USER_AGENT)
                    .and_then(|v| v.to_str().ok()),
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
    method: axum::http::Method,
    headers: axum::http::HeaderMap,
    MaybeCaller(caller): MaybeCaller,
) -> ApiResult<Response> {
    // A session signed in only to restore a deleted account (ADR 0019) ends too.
    let caller = match caller {
        Some(c) => Some(c),
        None => {
            state
                .auth
                .restorable_caller(&state.db, &method, &headers)
                .await?
        }
    };
    let clear = state.auth.sign_out(&state.db, caller.as_ref()).await?;
    let mut response = StatusCode::NO_CONTENT.into_response();
    response.headers_mut().insert(
        SET_COOKIE,
        HeaderValue::from_str(&clear).map_err(|_| ApiError::internal())?,
    );
    Ok(response)
}

// Upload

/// App sign-in sessions (ADR 0021) can't upload: no scope covers it.
fn no_app_scope(caller: &cyex_tv_auth::Caller) -> ApiResult<()> {
    match caller.app_client {
        Some(_) => Err(ApiError::new(
            StatusCode::FORBIDDEN,
            "insufficient_scope",
            "This app may not do that. Use cyex.tv for it.",
        )),
        None => Ok(()),
    }
}

/// A text form field, read chunk by chunk and refused (`bad_form`) as soon as it passes `max`
/// bytes, so a long field is never held whole.
async fn capped_text(field: &mut Field<'_>, max: usize) -> ApiResult<String> {
    let bad_form = |message: &str| ApiError::new(StatusCode::BAD_REQUEST, "bad_form", message);
    let mut bytes = Vec::new();
    while let Some(chunk) = field
        .chunk()
        .await
        .map_err(|_| bad_form("A form field couldn't be read."))?
    {
        if bytes.len() + chunk.len() > max {
            return Err(bad_form("A form field is too long."));
        }
        bytes.extend_from_slice(&chunk);
    }
    String::from_utf8(bytes).map_err(|_| bad_form("A form field couldn't be read."))
}

/// Upload an emote. It starts `pending`: usable in your own sets, hidden from everyone else until a
/// moderator approves it. Manage it afterwards through GraphQL.
///
/// PNG, APNG, GIF, WebP (still or animated) or JPEG; at most 8 MB, 1000×1000 px and 1000
/// frames. Sign in first (with the cookie, from `https://cyex.tv`). At most 20 uploads an hour.
/// The flags `zero_width`, `private` and `nsfw` may be set at once (each `true` or `false`,
/// default `false`), and `visibility` (`public`, `unlisted` or `private`; with `private` too,
/// they must agree); `updateEmote` changes them later. `owner_id` uploads for an account you edit
/// with `EMOTE_UPLOAD` (ADR 0014): it owns the emote, and the upload counts against both of you.
#[utoipa::path(
    post,
    path = "/v1/emotes",
    tag = "emotes",
    request_body(content = UploadEmote, content_type = "multipart/form-data"),
    responses(
        (status = 202, description = "Accepted; the files are being made. The emote is `pending`.", body = Emote),
        (status = 400, description = "`bad_name`, `bad_tag`, `too_many_tags` or `bad_form`.", body = ErrorBody),
        (status = 401, description = "`signed_out`.", body = ErrorBody),
        (status = 403, description = "`restricted` (you or the owner you upload for), `forbidden` (no `EMOTE_UPLOAD` there), or `bad_origin` for a cookie from another site.", body = ErrorBody),
        (status = 409, description = "`too_many_pending`: the owner already has as many emotes waiting for review as it may (20 unless its roles say otherwise; `uploadQuota`).", body = ErrorBody),
        (status = 413, description = "`too_large`: over 8 MB, 1000×1000 px or 1000 frames.", body = ErrorBody),
        (status = 415, description = "`unsupported_format`: not PNG, APNG, GIF, WebP or JPEG (AVIF isn't taken yet).", body = ErrorBody),
        (status = 422, description = "`invalid_image` (doesn't decode) or `blocked`.", body = ErrorBody),
        (status = 429, description = "`rate_limited`: the uploads an hour (20 unless the account's roles say otherwise) or the per-IP limit.", body = ErrorBody),
        (status = 503, description = "`unavailable`: storage isn't reachable.", body = ErrorBody)
    )
)]
async fn upload_emote(
    State(state): State<AppState>,
    MaybeCaller(caller): MaybeCaller,
    mut form: Multipart,
) -> ApiResult<(StatusCode, Json<Emote>)> {
    let caller = caller.ok_or_else(|| ApiError::from(AuthError::SignedOut))?;
    no_app_scope(&caller)?;
    let bad_form = |message: &str| ApiError::new(StatusCode::BAD_REQUEST, "bad_form", message);
    let mut name = None;
    let mut tags = None;
    let mut flags = upload::Flags::default();
    let mut private: Option<bool> = None;
    let mut visibility: Option<EmoteVisibility> = None;
    let mut owner_id: Option<String> = None;
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
                    if bytes.len() + chunk.len() > cyex_tv_media::EMOTE_MAX_BYTES {
                        return Err(upload::too_large());
                    }
                    bytes.extend_from_slice(&chunk);
                }
                file = Some(bytes);
            }
            Some(key @ ("name" | "tags")) => {
                let key = key.to_owned();
                let text = capped_text(&mut field, 1000).await?;
                if key == "name" {
                    name = Some(text);
                } else {
                    tags = Some(text);
                }
            }
            Some(key @ ("zero_width" | "private" | "nsfw")) => {
                let key = key.to_owned();
                let text = capped_text(&mut field, 16).await?;
                let on = upload::parse_flag(&text)
                    .ok_or_else(|| bad_form("A flag is true or false."))?;
                match key.as_str() {
                    "zero_width" => flags.zero_width = on,
                    "private" => private = Some(on),
                    _ => flags.nsfw = on,
                }
            }
            Some("owner_id") => {
                owner_id = Some(capped_text(&mut field, 64).await?);
            }
            Some("visibility") => {
                let text = capped_text(&mut field, 16).await?;
                visibility = Some(
                    text.parse()
                        .map_err(|_| bad_form("Visibility is public, unlisted or private."))?,
                );
            }
            _ => {
                return Err(bad_form(
                    "Unknown form field. Send name, tags, zero_width, private, visibility, nsfw, owner_id and file.",
                ));
            }
        }
    }
    let chosen = EmoteVisibility::after(EmoteVisibility::Public, visibility, private)
        .ok_or_else(|| bad_form("`visibility` and `private` say different things."))?;
    (flags.private, flags.unlisted) = chosen.flags();
    let name = name.ok_or_else(|| bad_form("Give the emote a name."))?;
    let file = file.ok_or_else(|| bad_form("Attach the image as `file`."))?;
    let emote = upload::upload(
        &state,
        &caller,
        upload::Upload {
            name,
            tags: upload::split_tags(tags.as_deref()),
            flags,
            owner_id,
            file: file.into(),
        },
    )
    .await?;
    Ok((StatusCode::ACCEPTED, Json(emote)))
}

/// Reads a multipart file field under the 8 MB limit of badge and paint pictures.
async fn file_field(
    field: &mut axum::extract::multipart::Field<'_>,
    bad_form: impl Fn(&str) -> ApiError,
) -> ApiResult<Vec<u8>> {
    // Badges and paints share the 8 MB cap; the kind may come after the file, and the per-kind
    // cap is checked when the picture is decoded.
    let limits = cyex_tv_media::Limits::badge();
    let mut bytes = Vec::new();
    while let Some(chunk) = field.chunk().await.map_err(|err| {
        if err.status() == StatusCode::PAYLOAD_TOO_LARGE {
            upload::too_large_for(&limits)
        } else {
            bad_form("The file couldn't be read.")
        }
    })? {
        if bytes.len() + chunk.len() > limits.max_bytes {
            return Err(upload::too_large_for(&limits));
        }
        bytes.extend_from_slice(&chunk);
    }
    Ok(bytes)
}

/// Upload a badge or paint picture (staff, `cosmetics.manage`).
///
/// The worker makes its files (WebP and AVIF) under `cosmetics/`; poll `cosmeticImage(id)` until
/// `ready`, then use the ID in `createBadge` or `createPaint`.
///
/// Badges follow the cyex badge size ladder: square 18, 36 and 72 px (`1x`, `2x`, `4x`), then
/// 128, 256 and 512 px (`128.webp`, `256.webp`, `512.webp`), all scaled from one 512 px master.
/// PNG, APNG, JPEG, WebP or GIF, square, at least 72×72 px (at most 2048), up to 8 MB. Animated:
/// at most 120 frames, 10 seconds and 64 MP (width × height × frames), and at most 512 KB per
/// chat size once converted; 128, 256 and 512 px animate only within 1, 2 and 4 MB (their
/// `_static` files always exist). Paints: the same file types, size (8 MB) and animation limits
/// (120 frames, 10 seconds, 64 MP); at least 64 px tall, at most 8:1 wide, up to 4096 px a side.
/// Made at 64, 128, 256 and 512 px tall (`1x`, `2x`, `4x`, `512`), never above the original, the
/// 512 px animated only within 4 MB.
///
/// The same bytes give the same picture again; for a badge made before the larger sizes, they
/// are made then (as `remakeCosmeticImage` does).
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
    no_app_scope(&caller)?;
    let bad_form = |message: &str| ApiError::new(StatusCode::BAD_REQUEST, "bad_form", message);
    let mut kind = None;
    let mut file: Option<Vec<u8>> = None;
    while let Some(mut field) = form.next_field().await.map_err(|err| {
        // A body over the upload limit can fail here, before the file field is read.
        if err.status() == StatusCode::PAYLOAD_TOO_LARGE {
            upload::too_large_for(&cyex_tv_media::Limits::badge())
        } else {
            bad_form("The form couldn't be read.")
        }
    })? {
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

/// The body and strong `ETag` (a hash of the body) of a read's answer.
fn built<T: Serialize>(value: &T) -> ApiResult<Arc<Body>> {
    let json = serde_json::to_vec(value).map_err(|_| ApiError::internal())?;
    let etag = format!("\"{}\"", hex(&Sha256::digest(&json)[..16]));
    Ok(Arc::new(Body { json, etag }))
}

/// Answers JSON with a strong `ETag` (a hash of the body), or `304` when `If-None-Match` has it.
fn cached<T: Serialize>(headers: &HeaderMap, value: &T, max_age: u32) -> ApiResult<Response> {
    let body = built(value)?;
    respond(headers, &body, max_age)
}

/// `cached` for an answer already built.
fn respond(headers: &HeaderMap, body: &Body, max_age: u32) -> ApiResult<Response> {
    let etag = body.etag.as_str();
    let cache = format!("public, max-age={max_age}, stale-while-revalidate=60");
    let fresh = headers
        .get(IF_NONE_MATCH)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| {
            v.split(',')
                .map(str::trim)
                .any(|t| t == "*" || t == etag || t.strip_prefix("W/") == Some(etag))
        });
    let mut response = if fresh {
        StatusCode::NOT_MODIFIED.into_response()
    } else {
        ([(CONTENT_TYPE, "application/json")], body.json.clone()).into_response()
    };
    let out = response.headers_mut();
    out.insert(
        ETAG,
        HeaderValue::from_str(etag).map_err(|_| ApiError::internal())?,
    );
    out.insert(
        CACHE_CONTROL,
        HeaderValue::from_str(&cache).map_err(|_| ApiError::internal())?,
    );
    Ok(response)
}

/// A public read through the instance's read cache (`read_cache`): built once for everyone
/// asking at the same time, answered with its `ETag`, `404` when there is nothing.
async fn shared_read<T, F, Fut>(
    state: &AppState,
    headers: &HeaderMap,
    key: Option<String>,
    load: F,
) -> ApiResult<Response>
where
    T: Serialize,
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = ApiResult<Option<T>>>,
{
    let build = || async move {
        match load().await? {
            Some(value) => built(&value).map(Some),
            None => Ok(None),
        }
    };
    let answer = match key {
        Some(key) => state.reads.get_or_load(&key, build).await?,
        None => build().await?,
    };
    let body = answer.ok_or_else(catalog::not_found)?;
    respond(headers, &body, READ_MAX_AGE)
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
    let key = cyex_tv_db::outbox::channel_topic(platform, id.trim());
    shared_read(&state, &headers, Some(key), || {
        catalog::channel(&state, platform, &id)
    })
    .await
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
    let key = cyex_tv_db::ids::from_text(id.trim()).map(read_cache::set_key);
    shared_read(&state, &headers, key, || catalog::set(&state, None, &id)).await
}

/// The cyex.tv global set, shown in every channel.
#[utoipa::path(
    get,
    path = "/v1/sets/global",
    tag = "reads",
    responses((status = 200, body = EmoteSet), (status = 304, description = "Not modified."))
)]
async fn get_global_set(State(state): State<AppState>, headers: HeaderMap) -> ApiResult<Response> {
    shared_read(
        &state,
        &headers,
        Some(read_cache::GLOBAL_SET_KEY.to_owned()),
        || async { catalog::global_set(&state, None).await.map(Some) },
    )
    .await
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
    let key = cyex_tv_db::ids::from_text(id.trim()).map(read_cache::emote_key);
    shared_read(&state, &headers, key, || {
        moderation::emote(&state, None, &id)
    })
    .await
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

/// The cyex.tv chat badges and name paints that are for sale in the shop or held by at least one
/// account (ADR 0015); cosmetics that are neither are left out (staff with a `cosmetics.*`
/// permission, signed in, get them too). Cache it with `If-None-Match`; live changes come as
/// `cosmetic.*` events on the `cosmetics` topic.
#[utoipa::path(
    get,
    path = "/v1/cosmetics",
    tag = "cosmetics",
    responses((status = 200, body = CosmeticCatalog), (status = 304, description = "Not modified."))
)]
async fn get_cosmetics(
    State(state): State<AppState>,
    MaybeCaller(caller): MaybeCaller,
    headers: HeaderMap,
) -> ApiResult<Response> {
    // Staff see everything, so their answer is built for them and never shared or cached.
    if let Some(caller) = &caller {
        let mut conn = state
            .db
            .acquire()
            .await
            .map_err(crate::errors::sqlx_error)?;
        if cosmetics::sees_unreleased(&mut conn, caller).await? {
            drop(conn);
            let all = cosmetics::catalog_all(&state).await?;
            // Only for this caller: no shared cache or CDN may keep it.
            let mut response = cached(&headers, &all, 0)?;
            response
                .headers_mut()
                .insert(CACHE_CONTROL, HeaderValue::from_static("private, no-cache"));
            return Ok(response);
        }
    }
    shared_read(
        &state,
        &headers,
        Some(read_cache::COSMETICS_KEY.to_owned()),
        || async { cosmetics::catalog(&state).await.map(Some) },
    )
    .await
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

// Shop

/// Paddle's webhook for cyex.tv's own Paddle account (ADR 0016). Not for clients.
///
/// The raw body is checked against `Paddle-Signature` (HMAC-SHA256 of `"<ts>:<body>"` with the
/// endpoint's secret; `ts` within 5 minutes). Each event is handled once, by its `event_id`.
/// `transaction.completed` fulfils an order, `transaction.canceled` fails a pending one, and an
/// approved `refund` or `chargeback` adjustment takes the cosmetics back. Other events are
/// acknowledged and ignored.
#[utoipa::path(
    post,
    path = "/v1/shop/paddle/webhook",
    tag = "shop",
    request_body(content = String, content_type = "application/json", description = "A Paddle notification."),
    params(("Paddle-Signature" = String, Header, description = "`ts=<unix>;h1=<hex>`.")),
    responses(
        (status = 200, description = "Handled, seen before, or not used."),
        (status = 400, description = "`bad_payload`.", body = ErrorBody),
        (status = 401, description = "`bad_signature` or `stale_signature`.", body = ErrorBody),
        (status = 503, description = "`unavailable`: the webhook secret isn't set.", body = ErrorBody)
    )
)]
async fn paddle_webhook(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: bytes::Bytes,
) -> ApiResult<Response> {
    let signature = headers
        .get("paddle-signature")
        .and_then(|v| v.to_str().ok());
    shop::webhook(&state, signature, &body).await?;
    Ok(([(CONTENT_TYPE, "application/json")], r#"{"ok":true}"#).into_response())
}
