//! `POST /v1/emotes`: an upload becomes a pending emote (PLAN.md Phase 3, steps 1 to 3 and 6).
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

use axum::http::StatusCode;
use bytes::Bytes;
use chrono::Utc;
use cyex_tv_auth::Caller;
use cyex_tv_db::emotes::{self, NewEmote};
use cyex_tv_db::files::{self, NewFile};
use cyex_tv_db::{DbError, accounts, audit, blocklist, ids, jobs, reads};
use cyex_tv_media::{Limits, MediaError, Probe};
use cyex_tv_shared::error::{ApiError, ApiResult};
use cyex_tv_shared::model::{EditorPermission, Emote};
use cyex_tv_storage::{Put, original_key, sigv4};
use serde_json::json;
use sha2::{Digest, Sha256};
use sqlx::{Connection, PgConnection};
use uuid::Uuid;

use crate::catalog::{db_error, forbidden, member_name, parse_id};
use crate::moderation::clean_tags;
use crate::state::AppState;
use crate::views;

/// Uploads one account may make per hour.
pub const PER_HOUR: i64 = 20;
/// The job the worker turns an original into emote files with.
pub const TRANSCODE_JOB: &str = "emote.transcode";

fn sqlx_error(err: sqlx::Error) -> ApiError {
    db_error(DbError::Sqlx(err))
}

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

/// The flags an upload may set at once; all off unless the form says otherwise.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Flags {
    pub zero_width: bool,
    pub private: bool,
    /// Never with `private` (ADR 0020).
    pub unlisted: bool,
    pub nsfw: bool,
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

/// At most `PER_HOUR` uploads an hour: what you uploaded, for yourself and for others, and what
/// the account got from anyone. The owners aren't limited.
async fn hourly_limit(conn: &mut PgConnection, caller: &Caller, owner: Uuid) -> ApiResult<()> {
    let hour_ago = Utc::now() - chrono::Duration::hours(1);
    let mine = emotes::count_uploaded_since(conn, caller.account_id, hour_ago)
        .await
        .map_err(db_error)?;
    let theirs = emotes::count_since(conn, owner, hour_ago)
        .await
        .map_err(db_error)?;
    if (mine >= PER_HOUR || theirs >= PER_HOUR) && !caller.owner {
        return Err(ApiError::new(
            StatusCode::TOO_MANY_REQUESTS,
            "rate_limited",
            if mine >= PER_HOUR || owner == caller.account_id {
                "That's a lot of uploads. Try again in an hour."
            } else {
                "That account has had a lot of uploads this hour. Try again later."
            },
        ));
    }
    Ok(())
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
    // Checked early so a refused upload isn't decoded, and again under the lock below.
    hourly_limit(&mut conn, caller, owner).await?;
    drop(conn);

    // Decoding is CPU work; keep it off the request threads.
    let file = input.file.clone();
    let probe: Probe =
        tokio::task::spawn_blocking(move || cyex_tv_media::probe(&file, &Limits::emote()))
            .await
            .map_err(|_| ApiError::internal())?
            .map_err(media_error)?;
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
    hourly_limit(&mut tx, caller, owner).await?;
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
    if created || revived {
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
            &json!({ "file_id": ids::to_text(file.id) }),
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
            nsfw: input.flags.nsfw,
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
                nsfw: false
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
