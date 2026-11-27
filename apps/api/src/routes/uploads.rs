//! Direct uploads of badge and paint pictures (ADR 0036): a presigned `PUT` to object storage,
//! then finalize. The logic is in `crate::direct_upload`; `POST /v1/cosmetics/images` (multipart)
//! stays as it is.

use std::collections::BTreeMap;

use axum::Json;
use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use cyex_tv_auth::AuthError;
use cyex_tv_media::config::MediaKind;
use cyex_tv_shared::error::{ApiError, ApiResult, ErrorBody};
use cyex_tv_shared::model::CosmeticKind;
use cyex_tv_shared::upload::CosmeticImageUploaded;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::direct_upload;
use crate::routes::v1::{limits_used, no_app_scope};
use crate::state::{AppState, MaybeCaller};

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(create_upload))
        .routes(routes!(finalize_upload))
}

/// What a direct upload is for.
#[derive(Clone, Debug, Deserialize, ToSchema)]
pub struct NewDirectUpload {
    /// `badge` or `paint`.
    pub kind: String,
    /// The file's exact length in bytes. Signed into the URL: storage takes no other length.
    pub size: u64,
    /// `image/png`, `image/gif`, `image/webp` or `image/jpeg`. Signed into the URL: send it as
    /// the `Content-Type` of the `PUT`.
    pub content_type: String,
}

/// Where to send the bytes.
#[derive(Clone, Debug, Serialize, ToSchema)]
pub struct DirectUpload {
    /// The upload's ID, for `POST /v1/cosmetics/images/uploads/{id}/finalize`.
    pub id: String,
    /// `badge` or `paint`.
    pub kind: String,
    /// Always `PUT`.
    pub method: String,
    /// The presigned address. Valid for 15 minutes; send the file as the whole body.
    pub url: String,
    /// Headers to send exactly as given (`content-type`, `content-length`).
    pub headers: BTreeMap<String, String>,
    /// Finalize before this (one hour); then the upload is deleted.
    pub expires_at: String,
}

fn bad_form(message: &str) -> ApiError {
    ApiError::new(StatusCode::BAD_REQUEST, "bad_form", message)
}

/// Ask for a direct upload URL for a badge or paint picture.
///
/// For large originals: the bytes go straight to storage instead of through the API. Send the
/// file with `PUT` to `url` with exactly `headers`, then call
/// `POST /v1/cosmetics/images/uploads/{id}/finalize`. Needs `cosmetics.manage`, like
/// `POST /v1/cosmetics/images`, and is checked the same way before the URL is given: the
/// declared `size` against your byte cap for the kind (your roles' limits), the type, the
/// cosmetics rate limit, and at most 10 uploads open at once. Where the server has no public
/// storage address the answer is `501 direct_upload_off`: use `POST /v1/cosmetics/images`.
#[utoipa::path(
    post,
    path = "/v1/cosmetics/images/uploads",
    tag = "cosmetics",
    request_body = NewDirectUpload,
    responses(
        (status = 201, description = "The presigned upload.", body = DirectUpload),
        (status = 400, description = "`bad_form` or `bad_kind`.", body = ErrorBody),
        (status = 401, description = "`signed_out`.", body = ErrorBody),
        (status = 403, description = "`forbidden` (needs `cosmetics.manage`), `restricted`, `insufficient_scope` or `bad_origin`.", body = ErrorBody),
        (status = 409, description = "`too_many_uploads`: 10 open uploads (`details`).", body = ErrorBody),
        (status = 413, description = "`too_large`: `size` is over your byte cap for the kind (`details`).", body = ErrorBody),
        (status = 415, description = "`unsupported_format`: not PNG, GIF, WebP or JPEG.", body = ErrorBody),
        (status = 429, description = "`rate_limited`.", body = ErrorBody),
        (status = 501, description = "`direct_upload_off`: use `POST /v1/cosmetics/images`.", body = ErrorBody),
        (status = 503, description = "`unavailable`: storage isn't configured.", body = ErrorBody)
    )
)]
async fn create_upload(
    State(state): State<AppState>,
    MaybeCaller(caller): MaybeCaller,
    body: Result<Json<NewDirectUpload>, JsonRejection>,
) -> ApiResult<(StatusCode, Json<DirectUpload>)> {
    let caller = caller.ok_or_else(|| ApiError::from(AuthError::SignedOut))?;
    no_app_scope(&caller)?;
    let Json(body) = body.map_err(|_| bad_form("Send JSON: { kind, size, content_type }."))?;
    let kind = CosmeticKind::parse(body.kind.trim()).ok_or_else(|| {
        ApiError::new(
            StatusCode::BAD_REQUEST,
            "bad_kind",
            "kind is badge or paint.",
        )
    })?;
    let up =
        direct_upload::presign(&state, &caller, kind, body.size, body.content_type.trim()).await?;
    Ok((
        StatusCode::CREATED,
        Json(DirectUpload {
            id: up.id,
            kind: up.kind.as_str().to_owned(),
            method: "PUT".to_owned(),
            url: up.url,
            headers: up.headers.into_iter().collect(),
            expires_at: crate::views::time(up.expires_at),
        }),
    ))
}

/// Finalize a direct upload: check the stored file and make the picture.
///
/// The stored object is checked before anything is made: its size against the declared one
/// (`size_mismatch`), then exactly what `POST /v1/cosmetics/images` checks (format, decoding,
/// your byte, side and frame caps, the shape, animated sizes). A refused file is deleted. The
/// answer is the one `POST /v1/cosmetics/images` gives (`UploadResult` fields): poll
/// `cosmeticImage(id)` until `ready`.
#[utoipa::path(
    post,
    path = "/v1/cosmetics/images/uploads/{id}/finalize",
    tag = "cosmetics",
    params(("id" = String, Path, description = "The upload's ID.")),
    responses(
        (status = 202, description = "Accepted; the files are being made, as for `POST /v1/cosmetics/images`.", body = CosmeticImageUploaded),
        (status = 401, description = "`signed_out`.", body = ErrorBody),
        (status = 403, description = "`forbidden`, `restricted`, `insufficient_scope` or `bad_origin`.", body = ErrorBody),
        (status = 404, description = "`not_found`: no such upload of yours.", body = ErrorBody),
        (status = 409, description = "`upload_missing` (nothing was PUT yet) or `already_finalized`.", body = ErrorBody),
        (status = 410, description = "`upload_expired`: ask for a new URL.", body = ErrorBody),
        (status = 413, description = "`too_large`: over your byte, side or frame cap.", body = ErrorBody),
        (status = 415, description = "`unsupported_format`.", body = ErrorBody),
        (status = 422, description = "`size_mismatch`, `invalid_image` or `bad_dimensions`.", body = ErrorBody),
        (status = 503, description = "`unavailable`.", body = ErrorBody)
    )
)]
async fn finalize_upload(
    State(state): State<AppState>,
    MaybeCaller(caller): MaybeCaller,
    Path(id): Path<String>,
) -> ApiResult<(StatusCode, Json<CosmeticImageUploaded>)> {
    let caller = caller.ok_or_else(|| ApiError::from(AuthError::SignedOut))?;
    no_app_scope(&caller)?;
    let (kind, image) = direct_upload::finalize(&state, &caller, &id).await?;
    let own = state.limits.load(&state.db, caller.account_id).await;
    let used = match kind {
        CosmeticKind::Badge => limits_used("badge", &own.media(MediaKind::Badge)),
        CosmeticKind::Paint => limits_used("paint", &own.media(MediaKind::Paint)),
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
