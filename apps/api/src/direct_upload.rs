//! Direct uploads of badge and paint pictures (ADR 0036): the client asks for a presigned `PUT`
//! URL, sends the bytes straight to object storage, then finalizes. The bytes never pass through
//! the API process, so a 16 MB original (the largest role ceiling) costs it nothing.
//!
//! The URL is given only after the checks the API makes before it takes bytes on
//! `POST /v1/cosmetics/images`: signed in, `cosmetics.manage`, the cosmetics rate limit, a cap on
//! open uploads, and a declared size and type within the uploader's limits. The size and type are
//! signed into the URL, so storage refuses anything else. Finalize then checks the stored object
//! again (its size against the declared one, then the same format, decode, limit and shape checks
//! as a multipart upload) before anything is made; a refused object is deleted. An upload not
//! finalized within [`FINALIZE_WITHIN`] is deleted by the worker.

use std::time::Duration;

use axum::http::StatusCode;
use chrono::{DateTime, Utc};
use cyex_tv_auth::Caller;
use cyex_tv_db::direct_uploads::{self as db, NewUpload};
use cyex_tv_db::ids;
use cyex_tv_shared::error::{ApiError, ApiResult};
use cyex_tv_shared::model::{CosmeticImage, CosmeticKind};
use cyex_tv_shared::permissions;
use cyex_tv_storage::upload_key;

use crate::catalog::{db_error, parse_id, require_any};
use crate::cosmetics::{image_from_bytes, picture_limits_for, upload_rate_limited};
use crate::errors;
use crate::state::AppState;
use crate::upload::too_large_over;

/// How long the presigned URL takes a `PUT`.
pub const URL_VALID: Duration = Duration::from_secs(15 * 60);
/// How long after the URL is made the upload may be finalized; past it the worker deletes it.
pub const FINALIZE_WITHIN: Duration = Duration::from_secs(60 * 60);
/// Uploads one account may have open (neither finalized nor expired) at once.
pub const MAX_OPEN: u32 = 10;
/// The types a picture may be declared as: the formats `POST /v1/cosmetics/images` takes.
pub const TYPES: [&str; 4] = ["image/png", "image/gif", "image/webp", "image/jpeg"];

/// A presigned upload, as the presign answers it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Presigned {
    pub id: String,
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub expires_at: DateTime<Utc>,
    pub kind: CosmeticKind,
}

fn off() -> ApiError {
    ApiError::new(
        StatusCode::NOT_IMPLEMENTED,
        "direct_upload_off",
        "Direct uploads aren't available here. Send the picture to POST /v1/cosmetics/images.",
    )
}

fn unavailable() -> ApiError {
    ApiError::unavailable("Uploads aren't available right now.")
}

/// Checks a presign request before anything is stored: the declared type and size.
pub fn check_declared(content_type: &str, size: u64, max_bytes: usize) -> ApiResult<()> {
    if !TYPES.contains(&content_type) {
        return Err(ApiError::new(
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            "unsupported_format",
            "Upload PNG, APNG, GIF, WebP or JPEG (content_type image/png, image/gif, image/webp or image/jpeg).",
        ));
    }
    if size == 0 {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "bad_form",
            "size is the file's length in bytes, at least 1.",
        ));
    }
    if size > max_bytes as u64 {
        let limits = cyex_tv_media::Limits {
            max_bytes,
            ..cyex_tv_media::Limits::default()
        };
        return Err(too_large_over(
            &limits,
            usize::try_from(size).unwrap_or(usize::MAX),
        ));
    }
    Ok(())
}

/// A presigned `PUT` for a `kind` picture of `size` bytes of `content_type`.
pub async fn presign(
    state: &AppState,
    caller: &Caller,
    kind: CosmeticKind,
    size: u64,
    content_type: &str,
) -> ApiResult<Presigned> {
    let mut conn = errors::conn(state).await?;
    require_any(&mut conn, caller, &[permissions::COSMETICS_MANAGE]).await?;
    let store = state.store.clone().ok_or_else(unavailable)?;
    let own = state.limits.load(&state.db, caller.account_id).await;
    let limits = picture_limits_for(kind, &own);
    check_declared(content_type, size, limits.max_bytes)?;
    let open = db::open_count(&mut conn, caller.account_id)
        .await
        .map_err(db_error)?;
    if open >= i64::from(MAX_OPEN) {
        return Err(ApiError::limit(
            "too_many_uploads",
            format!(
                "You have {MAX_OPEN} direct uploads open. Finalize them, or wait an hour for them to expire."
            ),
            MAX_OPEN,
            u64::try_from(open).unwrap_or(u64::MAX),
        ));
    }
    let id = ids::new_id();
    let key = upload_key(&ids::to_text(id));
    let put = store
        .presign_put(&key, content_type, size, URL_VALID)
        .ok_or_else(off)?;
    // Counted here, the moment the API would have taken the bytes; finalize isn't counted again.
    upload_rate_limited(state, caller)?;
    let expires_at = Utc::now()
        + chrono::Duration::from_std(FINALIZE_WITHIN).map_err(|_| ApiError::internal())?;
    db::insert(
        &mut conn,
        &NewUpload {
            id,
            account_id: caller.account_id,
            kind,
            storage_key: &key,
            content_type,
            declared_bytes: i64::try_from(size).map_err(|_| ApiError::internal())?,
            expires_at,
        },
    )
    .await
    .map_err(db_error)?;
    Ok(Presigned {
        id: ids::to_text(id),
        url: put.url,
        headers: put.headers,
        expires_at,
        kind,
    })
}

/// Checks the uploaded object and makes the picture, as `POST /v1/cosmetics/images` would.
pub async fn finalize(
    state: &AppState,
    caller: &Caller,
    raw_id: &str,
) -> ApiResult<(CosmeticKind, CosmeticImage)> {
    let mut conn = errors::conn(state).await?;
    require_any(&mut conn, caller, &[permissions::COSMETICS_MANAGE]).await?;
    let store = state.store.clone().ok_or_else(unavailable)?;
    let id = parse_id(raw_id)?;
    // Someone else's upload reads as missing, like an unknown one.
    let upload = db::get(&mut conn, id)
        .await
        .map_err(db_error)?
        .filter(|u| u.account_id == caller.account_id)
        .ok_or_else(crate::catalog::not_found)?;
    if upload.finalized_at.is_some() {
        return Err(ApiError::new(
            StatusCode::CONFLICT,
            "already_finalized",
            "This upload was already finalized.",
        ));
    }
    if upload.expires_at <= Utc::now() {
        return Err(ApiError::new(
            StatusCode::GONE,
            "upload_expired",
            "This upload has expired. Ask for a new upload URL.",
        ));
    }
    drop(conn);
    let gone = |err: cyex_tv_storage::StorageError| {
        tracing::error!(error = %err, "reading a direct upload failed");
        unavailable()
    };
    let Some(object) = store.get(&upload.storage_key).await.map_err(gone)? else {
        return Err(ApiError::new(
            StatusCode::CONFLICT,
            "upload_missing",
            "Nothing has been uploaded yet. PUT the file to the upload URL, then finalize.",
        ));
    };
    // Refused from here on: the object and the row go, so nothing half-made stays behind.
    let discard = || async {
        if let Err(err) = store.delete(&upload.storage_key).await {
            tracing::warn!(error = %err, "deleting a refused direct upload failed; the clean-up retries");
        }
        if let Ok(mut conn) = errors::conn(state).await {
            let _ = db::delete(&mut conn, upload.id).await;
        }
    };
    let declared = u64::try_from(upload.declared_bytes).unwrap_or(0);
    if object.bytes.len() as u64 != declared {
        discard().await;
        return Err(ApiError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "size_mismatch",
            format!(
                "The upload is {} bytes, not the {declared} declared. Ask for a new upload URL.",
                object.bytes.len()
            ),
        ));
    }
    // The same checks as a multipart upload (bytes, format, decode, side, frames, shape), under
    // the uploader's limits as they are now.
    let image = match image_from_bytes(state, caller, upload.kind, object.bytes).await {
        Ok(image) => image,
        Err(err) => {
            // A storage or database hiccup leaves the upload to try again; a refusal doesn't.
            if err.status.is_client_error() {
                discard().await;
            }
            return Err(err);
        }
    };
    let image_id = ids::from_text(&image.id).ok_or_else(ApiError::internal)?;
    let mut conn = errors::conn(state).await?;
    if !db::finalize(&mut conn, upload.id, image_id)
        .await
        .map_err(db_error)?
    {
        return Err(ApiError::new(
            StatusCode::CONFLICT,
            "already_finalized",
            "This upload was already finalized.",
        ));
    }
    // The picture keeps its own copy under `originals/`; the staging object isn't needed.
    if let Err(err) = store.delete(&upload.storage_key).await {
        tracing::warn!(error = %err, "deleting a finalized direct upload failed; the clean-up retries");
    }
    Ok((upload.kind, image))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_declared_type_and_size_are_checked_before_a_url_is_made() {
        let cap = 8 * 1024 * 1024;
        assert!(check_declared("image/png", 1, cap).is_ok());
        assert!(check_declared("image/webp", cap as u64, cap).is_ok());
        let over = check_declared("image/png", cap as u64 + 1, cap).unwrap_err();
        assert_eq!(
            (over.status, over.code),
            (StatusCode::PAYLOAD_TOO_LARGE, "too_large")
        );
        let details = over.details.unwrap();
        assert_eq!(
            (details.limit, details.current),
            (cap as u64, cap as u64 + 1)
        );
        for bad in [
            "image/svg+xml",
            "text/html",
            "image/avif",
            "",
            "image/png; x=1",
        ] {
            let err = check_declared(bad, 10, cap).unwrap_err();
            assert_eq!(err.code, "unsupported_format", "{bad}");
        }
        assert_eq!(
            check_declared("image/png", 0, cap).unwrap_err().code,
            "bad_form"
        );
    }

    #[test]
    fn the_url_expires_before_the_upload_does() {
        assert!(URL_VALID < FINALIZE_WITHIN);
    }
}
