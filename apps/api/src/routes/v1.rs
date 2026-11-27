//! The REST part of v1 (ADR 0008). Everything else is GraphQL at `/v1/gql`. REST covers only:
//! - sign-in redirects;
//! - the upload (transcoding is the worker's);
//! - the small, cacheable reads chat clients poll;
//! - Paddle's webhook for the shop (ADR 0016), which needs the raw body for its signature.
//!
//! The others answer `501 not_implemented` until their phase builds them.

use axum::Json;
use axum::extract::multipart::Field;
use axum::extract::{DefaultBodyLimit, Multipart, Path, Query, State};
use axum::http::header::{
    CACHE_CONTROL, CONTENT_TYPE, ETAG, IF_NONE_MATCH, LINK, LOCATION, SET_COOKIE, VARY,
};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use cyex_tv_auth::AuthError;
use cyex_tv_auth::cookies;
use cyex_tv_auth::flow::Callback;
use cyex_tv_media::config::MediaKind;
use cyex_tv_shared::error::{ApiError, ApiResult, ErrorBody};
use cyex_tv_shared::model::{
    CallbackQuery, Channel, ChannelsLookup, ChannelsLookupResult, ConsoleToken, CosmeticCatalog,
    CosmeticKind, Emote, EmoteSet, EmoteVisibility, LoginQuery, PersonalLookup,
    PersonalLookupResult, Platform, PublicLimits, SetEmotePage, SetEmotesQuery, SetQuery,
    SetsLookup, SetsLookupResult, UploadCosmeticImage, UploadEmote, UserCosmeticsQuery,
    UserCosmeticsResult,
};
use cyex_tv_shared::upload::{CosmeticImageUploaded, EmoteUploaded, LimitsUsed};
use serde::Serialize;
use sha2::{Digest, Sha256};
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use std::collections::HashMap;
use std::sync::Arc;

use crate::read_cache::{self, Body};
use crate::state::{AppState, MaybeCaller};
use crate::{catalog, cosmetics, moderation, shop, upload};

/// The upload body may be a little over the 8 MB emote file limit (multipart framing, the name
/// and tags); anything beyond this is cut off before it is read. The biggest of the configured
/// emote, badge and paint file caps (`CYEX_TV_LIMIT_*_MAX_BYTES`); each kind's own cap is checked
/// while the file is read.
fn upload_body_limit() -> usize {
    cyex_tv_media::config::MediaConfig::current().max_upload_bytes() + 64 * 1024
}

/// Paddle's notifications are a few KB; anything over 64 KB is refused before it is read.
const WEBHOOK_BODY_LIMIT: usize = 64 * 1024;

pub fn router() -> OpenApiRouter<AppState> {
    let upload = OpenApiRouter::new()
        .routes(routes!(upload_emote))
        .routes(routes!(upload_cosmetic_image))
        .layer(DefaultBodyLimit::max(upload_body_limit()));
    let webhook = OpenApiRouter::new()
        .routes(routes!(paddle_webhook))
        .layer(DefaultBodyLimit::max(WEBHOOK_BODY_LIMIT));
    OpenApiRouter::new()
        .merge(upload)
        .merge(webhook)
        .routes(routes!(login))
        .routes(routes!(callback))
        .routes(routes!(logout))
        .routes(routes!(export_token))
        .routes(routes!(get_channel))
        .routes(routes!(get_set))
        .routes(routes!(lookup_sets))
        .routes(routes!(lookup_channels))
        .routes(routes!(get_set_emotes))
        .routes(routes!(get_global_set))
        .routes(routes!(get_limits))
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

/// Make a personal API token (a console token).
///
/// A Bearer token that acts as you, for a GraphQL playground, a script or moving things between
/// your accounts (ADR 0028). Send it as `Authorization: Bearer <token>` to `/v1/gql` and the REST
/// calls. It has your account's own rights (your roles count; owner-only and website-only actions
/// such as deleting the account don't work with it) and lasts 3 days. Only one at a time: making
/// a new one signs the last one out. It shows as `Console token` under your sessions, where you
/// can sign it out; signing out any session, or everywhere, ends it too (ADR 0029). The token is
/// in this answer only.
///
/// The website's cookie only, from `https://cyex.tv` (`Origin`); a Bearer token can't make one.
/// 5 an hour. On cyex.tv, run `copy(await cyextv.token())` in the browser console.
#[utoipa::path(
    post,
    path = "/v1/auth/token/export",
    tag = "auth",
    responses(
        (status = 200, description = "The new token. Never cached.", body = ConsoleToken),
        (status = 401, description = "`signed_out`: no website session.", body = ErrorBody),
        (status = 403, description = "`bad_origin`: the cookie was sent from another site, or `website_only`: sent with a Bearer token.", body = ErrorBody),
        (status = 429, description = "`rate_limited`.", body = ErrorBody)
    )
)]
async fn export_token(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    MaybeCaller(caller): MaybeCaller,
) -> ApiResult<Response> {
    let caller = caller.ok_or_else(|| ApiError::from(AuthError::SignedOut))?;
    let user_agent = headers
        .get(axum::http::header::USER_AGENT)
        .and_then(|v| v.to_str().ok());
    let token = crate::console_token::mint(&state, &caller, user_agent).await?;
    let mut response = Json(token).into_response();
    let h = response.headers_mut();
    h.insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));
    h.insert(
        axum::http::header::PRAGMA,
        HeaderValue::from_static("no-cache"),
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
/// An emote is public unless the form makes it private. The flags `zero_width` and `private` may
/// be set at once (each `true` or `false`, default `false`), and `visibility` (`public` or
/// `private`; with `private` too, they must agree; `unlisted` is for staff with `emotes.flags`,
/// `bad_visibility` for anyone else); `updateEmote` changes them later. NSFW emotes aren't
/// allowed: `nsfw=true` is `nsfw_not_allowed` (`nsfw=false` is still taken). `owner_id` uploads for an account you edit
/// with `EMOTE_UPLOAD` (ADR 0014): it owns the emote, and the upload counts against both of you.
#[utoipa::path(
    post,
    path = "/v1/emotes",
    tag = "emotes",
    request_body(content = UploadEmote, content_type = "multipart/form-data"),
    responses(
        (status = 202, description = "Accepted; the files are being made. The emote is `pending`. `id`, `state`, `files` and `limits_used` are the same fields `POST /v1/cosmetics/images` answers with (`UploadResult`).", body = EmoteUploaded),
        (status = 400, description = "`bad_form`, or for emotes `bad_name`, `bad_tag`, `too_many_tags` (with `details`), `bad_visibility` (`unlisted` without `emotes.flags`) or `nsfw_not_allowed`; for pictures `bad_kind` (kind is badge or paint).", body = ErrorBody),
        (status = 401, description = "`signed_out`.", body = ErrorBody),
        (status = 403, description = "`restricted` (you or the owner you upload for), `forbidden` (no `EMOTE_UPLOAD` there, or no `cosmetics.manage` for a picture), or `bad_origin` for a cookie from another site.", body = ErrorBody),
        (status = 409, description = "`too_many_pending` (emotes only): the owner already has as many emotes waiting for review as it may (20 unless its roles say otherwise; `uploadQuota`). `details` has `limit` and `current`.", body = ErrorBody),
        (status = 413, description = "`too_large`: over the byte cap of the account's role, or the side or frame cap. For bytes, `details` has the cap and the size sent.", body = ErrorBody),
        (status = 415, description = "`unsupported_format`: not PNG, APNG, GIF, WebP or JPEG (AVIF isn't taken yet).", body = ErrorBody),
        (status = 422, description = "`invalid_image` (doesn't decode), `blocked`, or for pictures `bad_dimensions`.", body = ErrorBody),
        (status = 429, description = "`rate_limited`: the uploads an hour (20 unless the account's roles say otherwise) or the per-IP limit.", body = ErrorBody),
        (status = 503, description = "`unavailable`: storage isn't reachable.", body = ErrorBody)
    )
)]
async fn upload_emote(
    State(state): State<AppState>,
    MaybeCaller(caller): MaybeCaller,
    mut form: Multipart,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    let caller = caller.ok_or_else(|| ApiError::from(AuthError::SignedOut))?;
    no_app_scope(&caller)?;
    // The emote rules are fixed for everyone (`EmoteRules`); the size is checked as the file
    // streams in.
    let emote_limits = cyex_tv_media::Limits::emote();
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
                        upload::too_large_for(&emote_limits)
                    } else {
                        bad_form("The file couldn't be read.")
                    }
                })? {
                    if bytes.len() + chunk.len() > emote_limits.max_bytes {
                        return Err(upload::too_large_over(
                            &emote_limits,
                            bytes.len() + chunk.len(),
                        ));
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
                    // Kept so old clients sending `nsfw=false` still work; NSFW isn't allowed.
                    _ if on => return Err(upload::nsfw_not_allowed()),
                    _ => {}
                }
            }
            Some("owner_id") => {
                let text = capped_text(&mut field, 64).await?;
                owner_id = Some(crate::public_ids::canonical(&state, &text).await?);
            }
            Some("visibility") => {
                let text = capped_text(&mut field, 16).await?;
                visibility = Some(
                    text.parse()
                        .map_err(|_| bad_form("Visibility is public or private."))?,
                );
            }
            _ => {
                return Err(bad_form(
                    "Unknown form field. Send name, tags, zero_width, private, visibility, owner_id and file.",
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
    let mut body = crate::public_ids::publish(&state, &emote).await?;
    if let serde_json::Value::Object(map) = &mut body {
        map.insert(
            "limits_used".to_owned(),
            serde_json::to_value(limits_used("emote", &emote_limits))
                .map_err(|_| ApiError::internal())?,
        );
    }
    Ok((StatusCode::ACCEPTED, Json(body)))
}

/// The limits an upload was checked under, as the answer reports them.
fn limits_used(kind: &str, limits: &cyex_tv_media::Limits) -> LimitsUsed {
    LimitsUsed {
        kind: kind.to_owned(),
        max_bytes: cyex_tv_shared::limit_errors::n(limits.max_bytes),
        max_side: limits.max_side,
        max_frames: limits.max_frames,
    }
}

/// Reads a multipart file field under the bigger of the uploader's badge and paint caps.
async fn file_field(
    field: &mut axum::extract::multipart::Field<'_>,
    bad_form: impl Fn(&str) -> ApiError,
    mut limits: cyex_tv_media::Limits,
    paint: &cyex_tv_media::Limits,
) -> ApiResult<Vec<u8>> {
    // The kind may come after the file, so it is read under the bigger cap and the per-kind cap
    // is checked when the picture is decoded.
    limits.max_bytes = limits.max_bytes.max(paint.max_bytes);
    let mut bytes = Vec::new();
    while let Some(chunk) = field.chunk().await.map_err(|err| {
        if err.status() == StatusCode::PAYLOAD_TOO_LARGE {
            upload::too_large_for(&limits)
        } else {
            bad_form("The file couldn't be read.")
        }
    })? {
        if bytes.len() + chunk.len() > limits.max_bytes {
            return Err(upload::too_large_over(&limits, bytes.len() + chunk.len()));
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
        (status = 202, description = "Accepted; the files are being made (`state` is `processing`, then `ready`). `id`, `state`, `files` and `limits_used` are the same fields `POST /v1/emotes` answers with (`UploadResult`).", body = CosmeticImageUploaded),
        (status = 400, description = "`bad_form`, or `bad_kind` (kind is badge or paint).", body = ErrorBody),
        (status = 401, description = "`signed_out`.", body = ErrorBody),
        (status = 403, description = "`restricted`, `forbidden` (needs `cosmetics.manage`), or `bad_origin` for a cookie from another site.", body = ErrorBody),
        (status = 409, description = "Not used for pictures; `too_many_pending` is for emotes.", body = ErrorBody),
        (status = 413, description = "`too_large`: over the byte cap of the account's role, or the side or frame cap. For bytes, `details` has the cap and the size sent.", body = ErrorBody),
        (status = 415, description = "`unsupported_format`: not PNG, APNG, GIF, WebP or JPEG.", body = ErrorBody),
        (status = 422, description = "`invalid_image` (doesn't decode), or `bad_dimensions`; emotes also `blocked`.", body = ErrorBody),
        (status = 429, description = "`rate_limited`: the per-account or per-IP limit.", body = ErrorBody),
        (status = 503, description = "`unavailable`: storage isn't reachable.", body = ErrorBody)
    )
)]
async fn upload_cosmetic_image(
    State(state): State<AppState>,
    MaybeCaller(caller): MaybeCaller,
    mut form: Multipart,
) -> ApiResult<(StatusCode, Json<CosmeticImageUploaded>)> {
    let caller = caller.ok_or_else(|| ApiError::from(AuthError::SignedOut))?;
    no_app_scope(&caller)?;
    let own = state.limits.load(&state.db, caller.account_id).await;
    let (badge_limits, paint_limits) = (own.media(MediaKind::Badge), own.media(MediaKind::Paint));
    let bad_form = |message: &str| ApiError::new(StatusCode::BAD_REQUEST, "bad_form", message);
    let mut kind = None;
    let mut file: Option<Vec<u8>> = None;
    while let Some(mut field) = form.next_field().await.map_err(|err| {
        // A body over the upload limit can fail here, before the file field is read.
        if err.status() == StatusCode::PAYLOAD_TOO_LARGE {
            upload::too_large_for(&badge_limits)
        } else {
            bad_form("The form couldn't be read.")
        }
    })? {
        match field.name() {
            Some("file") if file.is_none() => {
                file = Some(file_field(&mut field, bad_form, badge_limits, &paint_limits).await?);
            }
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
    let used = match kind {
        CosmeticKind::Badge => limits_used("badge", &badge_limits),
        CosmeticKind::Paint => limits_used("paint", &paint_limits),
    };
    let image_state = if image.ready { "ready" } else { "processing" };
    Ok((
        StatusCode::ACCEPTED,
        Json(CosmeticImageUploaded {
            image,
            state: image_state.to_owned(),
            limits_used: used,
        }),
    ))
}

// Chat-client reads: public, no auth, `ETag` and `Cache-Control`.

/// How one read may be cached (API review 2026-11-26, item 10). One table, [`ReadCache`]'s
/// constants: how long it is fresh (`max-age`; then `If-None-Match` gets a cheap 304), how long
/// a stale copy may be served while a fresh one is fetched (`stale-while-revalidate`), and how
/// long when the API is failing (`stale-if-error`). Live updates (the events socket) push
/// changes sooner than any of these.
///
/// Every read here is the same for everyone, so they are `public`; they vary only by
/// `Accept-Encoding`. `GET /v1/cosmetics` is the exception: staff see more, so it also varies by
/// `Cookie` and `Authorization`, and a signed-in answer is `private` (a staff one is
/// `private, no-cache`), never kept by a shared cache.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ReadCache {
    max_age: u32,
    stale_while_revalidate: u32,
    stale_if_error: u32,
    /// What the answer depends on besides the URL, for `Vary`.
    vary: &'static str,
}

impl ReadCache {
    const fn new(max_age: u32, stale_while_revalidate: u32, stale_if_error: u32) -> Self {
        Self {
            max_age,
            stale_while_revalidate,
            stale_if_error,
            vary: "Accept-Encoding",
        }
    }

    /// A channel's active set pointer: the one thing a chat client polls, and what a set switch
    /// changes. Short.
    const CHANNEL: Self = Self::new(10, 30, 300);
    /// A set and its emotes, and a page of them.
    const SET: Self = Self::new(30, 60, 600);
    /// What changed in a set since a version (`?since=`, ADR 0035): as short as the set, so a
    /// poll never sees an older delta than the set itself.
    const SET_CHANGES: Self = Self::new(30, 60, 600);
    /// The global set changes rarely (staff edit it): longer, and long past an outage.
    const GLOBAL_SET: Self = Self::new(300, 600, 86_400);
    /// One emote object (its files never change; its name and state rarely).
    const EMOTE: Self = Self::new(30, 60, 86_400);
    /// The limits are configuration: they change on a deploy or a role edit.
    const LIMITS: Self = Self::new(300, 600, 86_400);
    /// Badges and paints, for everyone (a signed-in answer is `private`, see `cookie_aware`).
    const COSMETICS: Self = Self {
        max_age: 60,
        stale_while_revalidate: 120,
        stale_if_error: 3600,
        vary: "Accept-Encoding, Cookie, Authorization",
    };
    /// What chatters show; changes come as `user.cosmetics_updated`.
    const USER_COSMETICS: Self = Self::new(30, 60, 600);

    /// The `Cache-Control` of a public answer.
    fn public_header(self) -> String {
        format!(
            "public, max-age={}, stale-while-revalidate={}, stale-if-error={}",
            self.max_age, self.stale_while_revalidate, self.stale_if_error
        )
    }

    /// The `Cache-Control` of an answer for a signed-in client: its own browser may keep it, no
    /// shared cache.
    fn private_header(self) -> String {
        format!("private, max-age={}", self.max_age)
    }
}

/// Whether the request names an account (a session cookie or an `Authorization` header), so a
/// shared cache must not keep the answer for others.
fn names_an_account(headers: &HeaderMap) -> bool {
    headers.contains_key(axum::http::header::COOKIE)
        || headers.contains_key(axum::http::header::AUTHORIZATION)
}

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
fn cached<T: Serialize>(headers: &HeaderMap, value: &T, cache: ReadCache) -> ApiResult<Response> {
    let body = built(value)?;
    respond(headers, &body, cache)
}

/// `cached` for an answer already built.
fn respond(headers: &HeaderMap, body: &Body, cache: ReadCache) -> ApiResult<Response> {
    let etag = body.etag.as_str();
    // Only the cosmetics catalog depends on who asks; its signed-in answers stay out of shared
    // caches.
    let control = if cache.vary.contains("Cookie") && names_an_account(headers) {
        cache.private_header()
    } else {
        cache.public_header()
    };
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
        HeaderValue::from_str(&control).map_err(|_| ApiError::internal())?,
    );
    out.append(VARY, HeaderValue::from_static(cache.vary));
    Ok(response)
}

/// A public read through the instance's read cache (`read_cache`): built once for everyone
/// asking at the same time, answered with its `ETag`, `404` when there is nothing.
async fn shared_read<T, F, Fut>(
    state: &AppState,
    headers: &HeaderMap,
    key: Option<String>,
    cache: ReadCache,
    load: F,
) -> ApiResult<Response>
where
    T: Serialize,
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = ApiResult<Option<T>>>,
{
    let build = || async move {
        match load().await? {
            // Account, emote and set IDs by Snowflake (ADR 0027), built into the cached body.
            Some(value) => built(&crate::public_ids::publish(state, &value).await?).map(Some),
            None => Ok(None),
        }
    };
    let answer = match key {
        Some(key) => state.reads.get_or_load(&key, build).await?,
        None => build().await?,
    };
    let body = answer.ok_or_else(catalog::not_found)?;
    respond(headers, &body, cache)
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
    shared_read(&state, &headers, Some(key), ReadCache::CHANNEL, || {
        catalog::channel(&state, platform, &id)
    })
    .await
}

/// The answer of `GET /v1/sets/{id}`: the set, or with `since` what changed since that version.
#[derive(Serialize, utoipa::ToSchema)]
#[allow(dead_code)]
#[serde(untagged)]
enum SetOrChanges {
    Set(EmoteSet),
    Changes(cyex_tv_shared::model::SetChanges),
}

/// A set with its emotes, and its `version` (ADR 0035).
///
/// With `since` (a `version` you hold), only what changed after it: a `SetChanges` with the
/// members added, renamed and updated (apply as upserts by emote ID) and `removed_ids` (deletes).
/// `full: true` with the whole set in `set` when the change log no longer reaches back that far
/// (7 days or 1.000 versions) or `since` isn't a version of this set. Both answers have an `ETag`.
#[utoipa::path(
    get,
    path = "/v1/sets/{id}",
    tag = "reads",
    params(
        ("id" = String, Path, description = "The set's ID: its Snowflake (digits), or its old ULID while clients move over."),
        SetQuery
    ),
    responses(
        (status = 200, description = "The set, or with `since` a `SetChanges`.", body = SetOrChanges),
        (status = 304, description = "Not modified."),
        (status = 404, body = ErrorBody)
    )
)]
async fn get_set(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(query): Query<SetQuery>,
    headers: HeaderMap,
) -> ApiResult<Response> {
    // By Snowflake, or by the set's old ULID (ADR 0027).
    let id = crate::public_ids::canonical(&state, &id).await?;
    if let Some(since) = query.since {
        return shared_read(&state, &headers, None, ReadCache::SET_CHANGES, || {
            catalog::set_changes(&state, None, &id, since)
        })
        .await;
    }
    let key = cyex_tv_db::ids::from_text(id.trim()).map(read_cache::set_key);
    shared_read(&state, &headers, key, ReadCache::SET, || {
        catalog::set(&state, None, &id)
    })
    .await
}

/// Several sets at once, in the order asked (ADR 0035): what `GET /v1/sets/{id}` answers for
/// each, approved emotes only. Unknown and hidden sets are left out. Counts as one read per ID
/// sent (the same as asking for each set on its own), and each set comes from the same
/// per-instance cache as `GET /v1/sets/{id}`.
#[utoipa::path(
    post,
    path = "/v1/sets/lookup",
    tag = "reads",
    request_body = SetsLookup,
    responses(
        (status = 200, body = SetsLookupResult),
        (status = 400, description = "`too_many`: more than 25 set IDs.", body = ErrorBody)
    )
)]
async fn lookup_sets(
    State(state): State<AppState>,
    ip: Option<axum::Extension<crate::limits::ClientIp>>,
    key: Option<axum::Extension<crate::developers::KeyUse>>,
    Json(lookup): Json<SetsLookup>,
) -> ApiResult<Response> {
    if lookup.ids.len() > catalog::SETS_LOOKUP_LIMIT {
        return Err(catalog::too_many_ids(
            "sets",
            catalog::SETS_LOOKUP_LIMIT,
            lookup.ids.len(),
        ));
    }
    let ip = ip.map(|axum::Extension(ip)| ip.0).unwrap_or_default();
    let key = key.map(|axum::Extension(k)| k);
    // The request itself took one read; the other IDs take one each.
    let quota = match crate::limits::take_reads(&state, &ip, key.as_ref(), lookup.ids.len(), 1) {
        Ok(quota) => quota,
        Err((wait, quota)) => return Ok(crate::limits::too_many_quota(wait, quota)),
    };
    let ids = crate::public_ids::canonical_many(&state, &lookup.ids).await?;
    let bodies = cached_sets(&state, &ids).await?;
    let mut json = b"{\"sets\":[".to_vec();
    for (i, body) in bodies.iter().enumerate() {
        if i > 0 {
            json.push(b',');
        }
        json.extend_from_slice(&body.json);
    }
    json.extend_from_slice(b"]}");
    let mut response = ([(CONTENT_TYPE, "application/json")], json).into_response();
    if let Some(quota) = quota {
        crate::limits::set_quota(response.headers_mut(), quota);
    }
    Ok(response)
}

/// The sets `GET /v1/sets/{id}` would answer for `ids` (in order, each once, unknown and hidden
/// ones left out), through the read cache: the ones cached already are served from it, the rest
/// are loaded together (`catalog::sets_by_ids`, a fixed handful of queries) and kept there under
/// the same keys, so a following `GET /v1/sets/{id}` is served from them too.
async fn cached_sets(state: &AppState, ids: &[String]) -> ApiResult<Vec<Arc<Body>>> {
    let wanted = catalog::distinct_ids(ids);
    let mut found: Vec<Option<read_cache::Cached>> = wanted
        .iter()
        .map(|id| state.reads.peek(&read_cache::set_key(*id)))
        .collect();
    let missing: Vec<String> = wanted
        .iter()
        .zip(&found)
        .filter(|(_, cached)| cached.is_none())
        .map(|(id, _)| cyex_tv_db::ids::to_text(*id))
        .collect();
    if !missing.is_empty() {
        let loaded = catalog::sets_by_ids(state, None, &missing).await?;
        // Snowflake IDs (ADR 0027) for the whole batch at once, then each set's body as
        // `shared_read` builds it.
        let published = crate::public_ids::publish(state, &loaded).await?;
        let published = published.as_array().cloned().unwrap_or_default();
        let mut by_id: HashMap<String, serde_json::Value> = loaded
            .iter()
            .map(|set| set.id.clone())
            .zip(published)
            .collect();
        for (id, slot) in wanted.iter().zip(found.iter_mut()) {
            if slot.is_some() {
                continue;
            }
            let body = match by_id.remove(&cyex_tv_db::ids::to_text(*id)) {
                Some(value) => Some(built(&value)?),
                None => None,
            };
            state.reads.put(&read_cache::set_key(*id), body.clone());
            *slot = Some(body);
        }
    }
    Ok(found.into_iter().flatten().flatten().collect())
}

/// Several channels of one platform at once, in the order asked (ADR 0035): what
/// `GET /v1/channels/{platform}/{id}` answers for each. Channels not on cyex.tv are left out.
/// Counts as one read.
#[utoipa::path(
    post,
    path = "/v1/channels/lookup",
    tag = "reads",
    request_body = ChannelsLookup,
    responses(
        (status = 200, body = ChannelsLookupResult),
        (status = 400, description = "`too_many`: more than 100 channel IDs.", body = ErrorBody)
    )
)]
async fn lookup_channels(
    State(state): State<AppState>,
    Json(lookup): Json<ChannelsLookup>,
) -> ApiResult<Json<serde_json::Value>> {
    let channels = catalog::channels_by_ids(&state, lookup.platform, &lookup.ids).await?;
    Ok(Json(
        crate::public_ids::publish(&state, &ChannelsLookupResult { channels }).await?,
    ))
}

/// Every upload and field limit: per kind (emote, badge, paint) the file size, side, frames,
/// pixels and duration, the name and tag lengths and the per-IP rates. The same for everyone, so
/// `account` is empty here; the GraphQL `limits` query adds the signed-in caller's own limits.
#[utoipa::path(
    get,
    path = "/v1/limits",
    tag = "reads",
    responses((status = 200, body = PublicLimits), (status = 304, description = "Not modified."))
)]
async fn get_limits(State(state): State<AppState>, headers: HeaderMap) -> ApiResult<Response> {
    let limits = crate::public_limits::load(&state, None).await;
    cached(&headers, &limits, ReadCache::LIMITS)
}

/// A page of a set's emotes, oldest first, for clients that read big sets in parts (cursor
/// paging, ADR 0030). `next` (and the `Link: <...>; rel="next"` header) is where the following
/// page starts; it is null, and the header missing, on the last page. Approved emotes only, as
/// in `GET /v1/sets/{id}`.
#[utoipa::path(
    get,
    path = "/v1/sets/{id}/emotes",
    tag = "reads",
    params(
        ("id" = String, Path, description = "The set's ID: its Snowflake (digits), or its old ULID while clients move over."),
        SetEmotesQuery
    ),
    responses(
        (status = 200, body = SetEmotePage, headers(("Link" = String, description = "`<...>; rel=\"next\"`: the next page, when there is one."))),
        (status = 304, description = "Not modified."),
        (status = 400, description = "`bad_cursor`: `after` isn't a cursor this list gave out.", body = ErrorBody),
        (status = 404, body = ErrorBody)
    )
)]
async fn get_set_emotes(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(query): Query<SetEmotesQuery>,
    headers: HeaderMap,
) -> ApiResult<Response> {
    let asked = id.trim().to_owned();
    let id = crate::public_ids::canonical(&state, &id).await?;
    let page = catalog::set_emotes(
        &state,
        None,
        &id,
        None,
        None,
        query.per_page,
        query.after.as_deref(),
    )
    .await?;
    // An unknown or hidden set reads as an empty page; here it is a 404 like `GET /v1/sets/{id}`.
    if page.total == 0 && catalog::set(&state, None, &id).await?.is_none() {
        return Err(catalog::not_found());
    }
    // IDs are digits or ULID letters; anything else never names a set (and is no header text).
    let plain = asked.bytes().all(|b| b.is_ascii_alphanumeric());
    let link = page.next.as_deref().filter(|_| plain).map(|next| {
        let per_page = query
            .per_page
            .map(|n| format!("per_page={n}&"))
            .unwrap_or_default();
        format!(
            "</v1/sets/{}/emotes?{per_page}after={next}>; rel=\"next\"",
            asked
        )
    });
    let mut response = cached(
        &headers,
        &crate::public_ids::publish(&state, &page).await?,
        ReadCache::SET,
    )?;
    if let Some(link) = link {
        response.headers_mut().insert(
            LINK,
            HeaderValue::from_str(&link).map_err(|_| ApiError::internal())?,
        );
    }
    Ok(response)
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
        ReadCache::GLOBAL_SET,
        || async { catalog::global_set(&state, None).await.map(Some) },
    )
    .await
}

/// An approved emote.
#[utoipa::path(
    get,
    path = "/v1/emotes/{id}",
    tag = "reads",
    params(("id" = String, Path, description = "The emote's ID: its Snowflake (digits), or its old ULID while clients move over.")),
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
    let id = crate::public_ids::canonical(&state, &id).await?;
    let key = cyex_tv_db::ids::from_text(id.trim()).map(read_cache::emote_key);
    shared_read(&state, &headers, key, ReadCache::EMOTE, || {
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
) -> ApiResult<Json<serde_json::Value>> {
    let sets = catalog::personal_sets(&state, lookup.platform, &lookup.user_ids).await?;
    Ok(Json(
        crate::public_ids::publish(&state, &PersonalLookupResult { sets }).await?,
    ))
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
            let mut response = cached(&headers, &all, ReadCache::COSMETICS)?;
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
        ReadCache::COSMETICS,
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
    cached(
        &headers,
        &UserCosmeticsResult { users },
        ReadCache::USER_COSMETICS,
    )
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
