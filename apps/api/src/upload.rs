//! `POST /v1/emotes`: an upload becomes a pending emote (the transcode job finishes it).
//!
//! 1. The file is read with a hard cap, so an oversized one stops at 8 MB (`too_large`).
//! 2. It is decoded in full under the limits (`crates/media`), never trusted by name or type.
//! 3. Its sha256 and dHash are checked against the blocklist. A file seen before is reused:
//!    same object, same URLs, no second transcode.
//! 4. The original goes to storage (`originals/{sha256}`, never public), the emote is made
//!    `pending`, and a transcode job is queued, all before the transaction commits.
//!
//! An account editor with `emote_upload` on every set may upload for the account they edit
//! (`owner_id`, ADR 0014): it owns the emote, `uploaded_by` names the editor, and the upload is
//! audited (`emote.uploaded`, `"via": "account_editor"`). A hold on uploads of either account
//! refuses it, and the hourly limit counts it against both.
//!
//! An account has at most `pending_emotes` (20 unless its roles or `CYEX_TV_LIMIT_PENDING_EMOTES`
//! say otherwise) emotes waiting for review at once; an upload past that is refused
//! (`too_many_pending`), counted against the emote's owner, under the same lock as the hourly
//! limit so two uploads at once can't both get the last place.

use axum::http::StatusCode;
use std::sync::Arc;

use bytes::Bytes;
use chrono::Utc;
use cyex_tv_auth::Caller;
use cyex_tv_db::emotes::{self, NewEmote};
use cyex_tv_db::files::{self, NewFile};
use cyex_tv_db::{accounts, audit, blocklist, ids, jobs, reads};
use cyex_tv_media::config::MediaKind;
use cyex_tv_media::{Limits, MediaError, Probe};
use cyex_tv_shared::error::{ApiError, ApiResult};
use cyex_tv_shared::model::{EditorPermission, Emote, UploadQuota};
use cyex_tv_shared::permissions;
use cyex_tv_storage::{Put, original_key, sigv4};
use serde_json::json;
use sha2::{Digest, Sha256};
use sqlx::{Connection, PgConnection};
use uuid::Uuid;

use crate::catalog::{db_error, forbidden, member_name, parse_id};
use crate::errors::sqlx_error;
use crate::limits::{AccountLimit, AccountLimits};
use crate::moderation::clean_tags;
use crate::state::AppState;
use crate::views;

/// The job the worker turns an original into emote files with.
pub const TRANSCODE_JOB: &str = "emote.transcode";

pub fn media_error(err: MediaError) -> ApiError {
    match err {
        MediaError::Unsupported(message) => ApiError::new(
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            "unsupported_format",
            message,
        ),
        MediaError::TooLarge(message) => {
            ApiError::new(StatusCode::PAYLOAD_TOO_LARGE, "too_large", message)
        }
        MediaError::Encode(message) => {
            tracing::error!(%message, "encoder failed during an upload check");
            ApiError::internal()
        }
        // Only the worker cancels; an upload check never does.
        MediaError::Cancelled => ApiError::internal(),
        // The decoder's own words stay in the logs: they can name internals.
        MediaError::Invalid(message) => {
            tracing::info!(%message, "upload refused: not a valid image");
            ApiError::new(
                StatusCode::UNPROCESSABLE_ENTITY,
                "invalid_image",
                INVALID_IMAGE,
            )
        }
    }
}

/// What a client hears about a file that doesn't decode.
pub const INVALID_IMAGE: &str =
    "That file isn't a valid image. Try a PNG, GIF, WebP or JPEG that opens elsewhere.";

/// An emote file over 8 MB: refused while it is read, before any of it is decoded.
pub fn too_large() -> ApiError {
    too_large_for(&Limits::emote())
}

/// A file of `size` bytes over `limits.max_bytes`: `too_large` with the cap and the size sent.
pub fn too_large_over(limits: &Limits, size: usize) -> ApiError {
    too_large_for(limits).with_details(
        cyex_tv_shared::limit_errors::n(limits.max_bytes),
        cyex_tv_shared::limit_errors::n(size),
    )
}

/// `media_error` for a file of `size` bytes checked under `limits`: a `too_large` for the byte
/// cap says the cap and the size (`details`); one for the side or the frames names them in its
/// message.
pub fn media_error_sized(err: MediaError, limits: &Limits, size: usize) -> ApiError {
    let over = size > limits.max_bytes && matches!(err, MediaError::TooLarge(_));
    let error = media_error(err);
    if over {
        error.with_details(
            cyex_tv_shared::limit_errors::n(limits.max_bytes),
            cyex_tv_shared::limit_errors::n(size),
        )
    } else {
        error
    }
}

/// A file over `limits.max_bytes`.
pub fn too_large_for(limits: &Limits) -> ApiError {
    media_error(MediaError::TooLarge(cyex_tv_media::too_large_message(
        limits,
    )))
}

/// What the form gave us.
pub struct Upload {
    pub name: String,
    pub tags: Vec<String>,
    pub flags: Flags,
    /// The account to upload for (an account the caller edits with `emote_upload`); the caller's
    /// own when `None` or empty.
    pub owner_id: Option<String>,
    pub file: Bytes,
}

/// The flags an upload may set at once; all off unless the form says otherwise. Uploads are public
/// unless made private; NSFW emotes aren't allowed at all (ADR 0020).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Flags {
    pub zero_width: bool,
    pub private: bool,
    /// Never with `private` (ADR 0020). Staff only (`emotes.flags` or `emotes.edit_any`):
    /// taking an emote off the listings is a moderator's call, not the uploader's.
    pub unlisted: bool,
}

/// `400 bad_visibility`: only staff take an emote off the listings (ADR 0020).
pub fn unlisted_staff_only() -> ApiError {
    ApiError::new(
        StatusCode::BAD_REQUEST,
        "bad_visibility",
        "Emotes are public unless you make them private. Only moderators can take an emote off the listings.",
    )
}

/// `400 nsfw_not_allowed`: NSFW emotes aren't allowed on cyex.tv (ADR 0020).
pub fn nsfw_not_allowed() -> ApiError {
    ApiError::new(
        StatusCode::BAD_REQUEST,
        "nsfw_not_allowed",
        "NSFW emotes aren't allowed on cyex.tv, so there is no NSFW flag. Emotes that break the guidelines are removed.",
    )
}

/// A form flag: `true`/`false`, `1`/`0` or `on`/`off` (any case, spaces trimmed). An empty field
/// is `false`, as an unticked box would be. Anything else is refused.
pub fn parse_flag(raw: &str) -> Option<bool> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "true" | "1" | "on" => Some(true),
        "false" | "0" | "off" | "" => Some(false),
        _ => None,
    }
}

/// Comma-separated tags from the form field.
pub fn split_tags(raw: Option<&str>) -> Vec<String> {
    raw.unwrap_or_default()
        .split(',')
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .map(str::to_owned)
        .collect()
}

/// What the uploader's and the owner's roles allow (the same when you upload for yourself).
struct Allowed {
    mine: Arc<AccountLimits>,
    theirs: Arc<AccountLimits>,
}

fn count_of(limits: &AccountLimits, limit: AccountLimit) -> i64 {
    i64::from(limits.get(limit))
}

/// At most `uploads_per_hour` uploads an hour: what you uploaded, for yourself and for others,
/// and what the account got from anyone, each against its own limit. The owners aren't limited.
async fn hourly_limit(
    conn: &mut PgConnection,
    caller: &Caller,
    owner: Uuid,
    allowed: &Allowed,
) -> ApiResult<()> {
    let hour_ago = Utc::now() - chrono::Duration::hours(1);
    let mine = emotes::count_uploaded_since(conn, caller.account_id, hour_ago)
        .await
        .map_err(db_error)?;
    let theirs = emotes::count_since(conn, owner, hour_ago)
        .await
        .map_err(db_error)?;
    let mine_over = mine >= count_of(&allowed.mine, AccountLimit::UploadsPerHour);
    let theirs_over = theirs >= count_of(&allowed.theirs, AccountLimit::UploadsPerHour);
    if (mine_over || theirs_over) && !caller.owner {
        return Err(ApiError::new(
            StatusCode::TOO_MANY_REQUESTS,
            "rate_limited",
            if mine_over || owner == caller.account_id {
                "That's a lot of uploads. Try again in an hour."
            } else {
                "That account has had a lot of uploads this hour. Try again later."
            },
        )
        .retry_after(3600));
    }
    Ok(())
}

/// What a refusal at the pending cap says.
pub fn too_many_pending(max: u32, for_someone: bool) -> ApiError {
    let waiting = if max == 1 {
        "1 emote".to_owned()
    } else {
        format!("{max} emotes")
    };
    ApiError::limit(
        "too_many_pending",
        if for_someone {
            format!(
                "That account has {waiting} waiting for review. Upload more once some are reviewed."
            )
        } else {
            format!("You have {waiting} waiting for review. Upload more once some are reviewed.")
        },
        max,
        max,
    )
}

/// At most `pending_emotes` of the owner's emotes wait for review at once.
async fn pending_limit(
    conn: &mut PgConnection,
    caller: &Caller,
    owner: Uuid,
    for_someone: bool,
    allowed: &Allowed,
) -> ApiResult<()> {
    // The owners aren't capped, like the hourly limit.
    if caller.owner {
        return Ok(());
    }
    let pending = emotes::count_pending(conn, owner).await.map_err(db_error)?;
    let max = allowed.theirs.get(AccountLimit::PendingEmotes);
    if pending >= i64::from(max) {
        return Err(too_many_pending(max, for_someone));
    }
    Ok(())
}

/// Where `owner` stands with uploads: its pending emotes and their cap, its uploads this hour
/// and their limit. Yours, or an account you may upload for (`emote_upload` on every set).
pub async fn quota(
    state: &AppState,
    caller: &Caller,
    owner_id: Option<&str>,
) -> ApiResult<UploadQuota> {
    let mut conn = state.db.acquire().await.map_err(sqlx_error)?;
    let owner = match owner_id.map(str::trim).filter(|s| !s.is_empty()) {
        Some(raw) => parse_id(raw)?,
        None => caller.account_id,
    };
    if owner != caller.account_id
        && !crate::editors::has_emote_flag(
            &mut conn,
            owner,
            caller.account_id,
            EditorPermission::EmoteUpload,
        )
        .await?
    {
        return Err(forbidden(
            "You can see another account's uploads only as its editor with the upload permission on every set.",
        ));
    }
    let limits = state.limits.load(&state.db, owner).await;
    let pending = emotes::count_pending(&mut conn, owner)
        .await
        .map_err(db_error)?;
    let hour = emotes::count_since(&mut conn, owner, Utc::now() - chrono::Duration::hours(1))
        .await
        .map_err(db_error)?;
    // The caller uploads, so their roles' media limits apply (to the owner's account too).
    let emote = state
        .limits
        .load(&state.db, caller.account_id)
        .await
        .media(MediaKind::Emote);
    let int = |n: i64| i32::try_from(n).unwrap_or(i32::MAX);
    let lim = |l: AccountLimit| i32::try_from(limits.get(l)).unwrap_or(i32::MAX);
    Ok(UploadQuota {
        account_id: ids::to_text(owner),
        pending: int(pending),
        pending_max: (!caller.owner).then(|| lim(AccountLimit::PendingEmotes)),
        uploads_this_hour: int(hour),
        uploads_per_hour: lim(AccountLimit::UploadsPerHour),
        // The caller's effective media limits: `CYEX_TV_LIMIT_EMOTE_*` with their roles'
        // overrides (`emote_max_bytes`, `emote_max_frames`; the side only goes down from 1000).
        max_bytes: int(i64::try_from(emote.max_bytes).unwrap_or(i64::MAX)),
        max_side: int(i64::from(emote.max_side)),
        max_frames: int(i64::from(emote.max_frames)),
    })
}

pub async fn upload(state: &AppState, caller: &Caller, input: Upload) -> ApiResult<Emote> {
    let name = member_name(input.name.trim())?.to_owned();
    let tags = clean_tags(&input.tags)?;
    let Some(store) = state.store.clone() else {
        return Err(ApiError::unavailable("Uploads aren't available right now."));
    };

    let mut conn = state.db.acquire().await.map_err(sqlx_error)?;
    let owner = match input
        .owner_id
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        Some(raw) => parse_id(raw)?,
        None => caller.account_id,
    };
    let for_someone = owner != caller.account_id;
    if for_someone
        && !crate::editors::has_emote_flag(
            &mut conn,
            owner,
            caller.account_id,
            EditorPermission::EmoteUpload,
        )
        .await?
    {
        return Err(forbidden(
            "You can upload for another account only as its editor with the upload permission on every set.",
        ));
    }
    let account = accounts::get(&mut conn, caller.account_id)
        .await
        .map_err(db_error)?
        .ok_or_else(crate::catalog::not_found)?;
    if account.restricted_at.is_some() {
        return Err(ApiError::new(
            StatusCode::FORBIDDEN,
            "restricted",
            "Your account can't upload right now.",
        ));
    }
    // Everyone may upload through the User role, unless a more senior role denies it (a "Muted"
    // role, ADR 0010 amendment 2026/11/22).
    if !crate::catalog::holds(&mut conn, caller, permissions::EMOTES_UPLOAD).await? {
        return Err(ApiError::new(
            StatusCode::FORBIDDEN,
            "forbidden",
            "Your roles don't allow uploading emotes.",
        ));
    }
    if input.flags.unlisted
        && !input.flags.private
        && !crate::catalog::holds_any(
            &mut conn,
            caller,
            &[permissions::EMOTES_FLAGS, permissions::EMOTES_EDIT_ANY],
        )
        .await?
    {
        return Err(unlisted_staff_only());
    }
    if for_someone
        && accounts::get(&mut conn, owner)
            .await
            .map_err(db_error)?
            .is_none_or(|a| a.restricted_at.is_some())
    {
        return Err(ApiError::new(
            StatusCode::FORBIDDEN,
            "restricted",
            "That account can't upload right now, so nobody can upload for it.",
        ));
    }
    let allowed = Allowed {
        mine: state.limits.load(&state.db, caller.account_id).await,
        theirs: state.limits.load(&state.db, owner).await,
    };
    // Checked early so a refused upload isn't decoded, and again under the lock below. One
    // snapshot for both checks: read separately, an upload committing between them could be
    // counted by the pending check but not the hourly one, refusing with `too_many_pending`
    // where `rate_limited` is the answer.
    let mut early = conn.begin().await.map_err(sqlx_error)?;
    sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ, READ ONLY")
        .execute(&mut *early)
        .await
        .map_err(sqlx_error)?;
    hourly_limit(&mut early, caller, owner, &allowed).await?;
    pending_limit(&mut early, caller, owner, for_someone, &allowed).await?;
    early.rollback().await.map_err(sqlx_error)?;
    drop(conn);

    // Decoding is CPU work; keep it off the request threads. The uploader's effective limits
    // apply, and ride on the transcode job so the worker checks the same numbers.
    let file = input.file.clone();
    let limits = allowed.mine.media(MediaKind::Emote);
    let (limits_for_error, size) = (limits, input.file.len());
    let probe: Probe = tokio::task::spawn_blocking(move || cyex_tv_media::probe(&file, &limits))
        .await
        .map_err(|_| ApiError::internal())?
        .map_err(|err| media_error_sized(err, &limits_for_error, size))?;
    let sha256: [u8; 32] = Sha256::digest(&input.file).into();
    let sha_hex = sigv4::hex(&sha256);
    // The first frame's hash is what `files.dhash` keeps (always the first entry).
    let dhash = probe.frame_hashes.first().copied().unwrap_or(0);
    // Stored as the bigint with the same bits.
    let dhash = i64::from_ne_bytes(dhash.to_ne_bytes());

    let mut conn = state.db.acquire().await.map_err(sqlx_error)?;
    // Every frame counts: a blocked image can be the second frame of an animation.
    let mut hashes: Vec<i64> = probe
        .frame_hashes
        .iter()
        .map(|h| i64::from_ne_bytes(h.to_ne_bytes()))
        .collect();
    if !hashes.contains(&dhash) {
        hashes.push(dhash);
    }
    if blocklist::check_frames(&mut conn, &sha256, &hashes)
        .await
        .map_err(db_error)?
        .is_some()
    {
        return Err(ApiError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "blocked",
            "This image can't be uploaded to cyex.tv.",
        ));
    }

    let mut tx = conn.begin().await.map_err(sqlx_error)?;
    // Two uploads at once could both pass the early check; under this lock the second one sees
    // the first.
    emotes::lock_upload_quota(&mut tx, &[caller.account_id, owner])
        .await
        .map_err(db_error)?;
    hourly_limit(&mut tx, caller, owner, &allowed).await?;
    pending_limit(&mut tx, caller, owner, for_someone, &allowed).await?;
    let side = |v: u32| i32::try_from(v).unwrap_or(i32::MAX);
    let key = original_key(&sha_hex);
    let (file, created) = files::insert_or_get(
        &mut tx,
        &NewFile {
            sha256: &sha256,
            dhash: Some(dhash),
            mime: probe.format.mime(),
            width: side(probe.width),
            height: side(probe.height),
            frame_count: side(probe.frames),
            storage_key: &key,
        },
    )
    .await
    .map_err(db_error)?;
    // The same bytes as a file a deleted account took along: store and transcode it again.
    let revived = !created
        && file.purged_at.is_some()
        && files::revive(&mut tx, file.id).await.map_err(db_error)?;
    // Bytes we know whose original is gone from storage (it moved, or lost it): store it and make
    // the files again, and forget the old files until they are.
    let lost = !created && !revived && {
        let gone = |err| {
            tracing::error!(error = %err, "checking an emote file in storage failed");
            ApiError::unavailable("Uploads aren't available right now.")
        };
        !store.exists(&key).await.map_err(gone)?
    };
    if lost {
        files::set_variants(&mut tx, file.id, &json!([]))
            .await
            .map_err(db_error)?;
    }
    // An earlier upload of these bytes failed to transcode: try again.
    let retry = !created
        && !revived
        && !lost
        && files::clear_failed(&mut tx, file.id)
            .await
            .map_err(db_error)?;
    if created || revived || lost || retry {
        store
            .put(
                &key,
                input.file.clone(),
                Put {
                    content_type: probe.format.mime(),
                    cache_control: None,
                },
            )
            .await
            .map_err(|err| {
                tracing::error!(error = %err, "storing an original failed");
                ApiError::unavailable("Uploads aren't available right now.")
            })?;
        jobs::enqueue(
            &mut tx,
            TRANSCODE_JOB,
            &json!({
                "file_id": ids::to_text(file.id),
                "limits": {
                    "max_bytes": limits.max_bytes,
                    "max_side": limits.max_side,
                    "max_frames": limits.max_frames,
                },
            }),
            5,
        )
        .await
        .map_err(db_error)?;
    }
    let emote = emotes::create(
        &mut tx,
        &NewEmote {
            owner_account_id: owner,
            file_id: Some(file.id),
            name: &name,
            animated: probe.animated,
            zero_width: input.flags.zero_width,
            private: input.flags.private,
            unlisted: input.flags.unlisted && !input.flags.private,
            nsfw: false,
        },
    )
    .await
    .map_err(db_error)?;
    if !tags.is_empty() {
        emotes::set_tags(&mut tx, emote.id, &tags)
            .await
            .map_err(db_error)?;
    }
    if for_someone {
        emotes::set_uploader(&mut tx, emote.id, caller.account_id)
            .await
            .map_err(db_error)?;
        audit::write_for(
            &mut tx,
            &audit::NewEntry {
                actor: Some(caller.account_id),
                action: "emote.uploaded",
                target_kind: "emote",
                target_id: &ids::to_text(emote.id),
                before: None,
                after: Some(json!({
                    "name": name,
                    "owner_id": ids::to_text(owner),
                    "uploaded_by": ids::to_text(caller.account_id),
                    "via": "account_editor",
                })),
            },
            Some(owner),
        )
        .await
        .map_err(db_error)?;
    }
    tx.commit().await.map_err(sqlx_error)?;
    tracing::info!(
        emote = %ids::to_text(emote.id),
        sha256 = %sha_hex,
        reused = !created,
        format = probe.format.mime(),
        frames = probe.frames,
        "emote uploaded"
    );

    let rows = reads::emotes(&mut conn, &[emote.id])
        .await
        .map_err(db_error)?;
    let mut built = views::emotes(&mut conn, &rows, &state.cdn_url)
        .await
        .map_err(db_error)?;
    built.pop().ok_or_else(ApiError::internal)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decoder_errors_stay_in_the_logs() {
        let err = media_error(MediaError::Invalid(
            "png: chunk IDAT at offset 0x1f crc mismatch".into(),
        ));
        assert_eq!(err.code, "invalid_image");
        assert_eq!(err.message, INVALID_IMAGE);
        assert!(!err.message.contains("IDAT"));
    }

    #[test]
    fn the_pending_refusal_says_how_many_wait() {
        let err = too_many_pending(20, false);
        assert_eq!(err.code, "too_many_pending");
        assert_eq!(err.status, StatusCode::CONFLICT);
        assert_eq!(
            err.message,
            "You have 20 emotes waiting for review. Upload more once some are reviewed."
        );
        assert!(
            too_many_pending(1, true)
                .message
                .starts_with("That account has 1 emote waiting")
        );
    }

    #[test]
    fn tags_split_on_commas() {
        assert_eq!(
            split_tags(Some(" pog, funny ,,cat ")),
            ["pog", "funny", "cat"]
        );
        assert!(split_tags(None).is_empty());
    }

    #[test]
    fn flags_parse_like_a_form() {
        for on in ["true", "TRUE", " 1 ", "on"] {
            assert_eq!(parse_flag(on), Some(true), "{on}");
        }
        for off in ["false", "0", "off", "", "  "] {
            assert_eq!(parse_flag(off), Some(false), "{off:?}");
        }
        assert_eq!(parse_flag("yes"), None);
        assert_eq!(parse_flag("2"), None);
        assert_eq!(
            Flags::default(),
            Flags {
                zero_width: false,
                private: false,
                unlisted: false,
            }
        );
    }

    #[test]
    fn media_errors_map_to_http() {
        assert_eq!(
            media_error(MediaError::Unsupported("x")).status,
            StatusCode::UNSUPPORTED_MEDIA_TYPE
        );
        assert_eq!(too_large().status, StatusCode::PAYLOAD_TOO_LARGE);
        assert_eq!(
            too_large().message,
            "The file is over 8 MB, the most an upload can be."
        );
        assert_eq!(
            too_large_for(&Limits::default()).message,
            "The file is over 5 MB, the most an upload can be."
        );
        assert_eq!(
            media_error(MediaError::Invalid("x".into())).code,
            "invalid_image"
        );
    }

    #[test]
    fn dhash_round_trips_through_bigint() {
        for v in [0u64, 1, u64::MAX, 0x8000_0000_0000_0000] {
            let stored = i64::from_ne_bytes(v.to_ne_bytes());
            assert_eq!(u64::from_ne_bytes(stored.to_ne_bytes()), v);
        }
    }
}
