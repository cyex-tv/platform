//! The `emote.transcode` job (queued by the upload): an uploaded original becomes its
//! emote files (ADR 0012), stored under `emotes/{sha256}/{name}` with an immutable cache header.
//!
//! WebP goes first and is recorded on the file, so the emote is usable; then AVIF. Running the job
//! again is safe: the keys are content-addressed and the variants are simply written again.

use cyex_tv_db::jobs::Job;
use cyex_tv_db::{blocklist, files};
use cyex_tv_media::encode::{self, Kind, Profile, Variant};
use cyex_tv_shared::model::{EmoteFile, ImageFormat};
use cyex_tv_storage::{Put, Store, cache_control_for, emote_key, sigv4};
use serde_json::Value;

use crate::{CancelOnDrop, Ctx, JobError, media_failure};

/// The job kind the upload queues.
pub const KIND: &str = "emote.transcode";

fn retry(err: impl std::fmt::Display) -> JobError {
    JobError::Retry(err.to_string())
}

fn file_entry(v: &Variant) -> EmoteFile {
    EmoteFile {
        name: v.name.clone(),
        format: match v.kind {
            Kind::Webp => ImageFormat::Webp,
            Kind::Avif => ImageFormat::Avif,
        },
        scale: u8::try_from(v.scale).unwrap_or(u8::MAX),
        width: u16::try_from(v.width).unwrap_or(u16::MAX),
        height: u16::try_from(v.height).unwrap_or(u16::MAX),
        size: u32::try_from(v.bytes.len()).unwrap_or(u32::MAX),
        is_static: v.is_static,
        url: String::new(),
        mime: String::new(),
    }
}

fn entries(variants: &[&Variant]) -> Result<Value, JobError> {
    serde_json::to_value(variants.iter().map(|v| file_entry(v)).collect::<Vec<_>>())
        .map_err(|err| JobError::Permanent(err.to_string()))
}

async fn store_all(store: &Store, sha_hex: &str, variants: &[Variant]) -> Result<(), JobError> {
    for v in variants {
        let key = emote_key(sha_hex, &v.name);
        store
            .put(
                &key,
                v.bytes.clone().into(),
                Put {
                    content_type: v.kind.mime(),
                    cache_control: cache_control_for(&key),
                },
            )
            .await
            .map_err(retry)?;
    }
    Ok(())
}

/// Whether the file has been blocked (a takedown adds its sha256 to the blocklist) since it was
/// uploaded.
async fn is_blocked(ctx: &Ctx, file: &files::File) -> Result<bool, JobError> {
    let sha: [u8; 32] = file
        .sha256
        .as_slice()
        .try_into()
        .map_err(|_| JobError::Permanent("the file's sha256 isn't 32 bytes".into()))?;
    let mut conn = ctx.db.acquire().await.map_err(retry)?;
    Ok(blocklist::check(&mut conn, &sha, file.dhash.unwrap_or(0))
        .await
        .map_err(retry)?
        .is_some())
}

/// Removes everything a transcode may have written for a file that turned out to be blocked.
async fn remove_written(store: &Store, sha_hex: &str) -> Result<(), JobError> {
    for name in encode::all_names() {
        store
            .delete(&emote_key(sha_hex, &name))
            .await
            .map_err(retry)?;
    }
    Ok(())
}

pub async fn run(ctx: Ctx, job: Job) -> Result<(), JobError> {
    let file_id = job
        .payload
        .get("file_id")
        .and_then(Value::as_str)
        .and_then(cyex_tv_db::ids::from_text)
        .ok_or_else(|| JobError::Permanent("the job has no valid file_id".into()))?;
    let store = ctx
        .store
        .clone()
        // Retried, so the job finishes once storage is configured.
        .ok_or_else(|| JobError::Retry("storage isn't configured".into()))?;

    let mut conn = ctx.db.acquire().await.map_err(retry)?;
    let file = files::get(&mut conn, file_id)
        .await
        .map_err(retry)?
        .ok_or_else(|| JobError::Permanent("the file is gone".into()))?;
    drop(conn);
    let original = store
        .get(&file.storage_key)
        .await
        .map_err(retry)?
        .ok_or_else(|| JobError::Permanent(format!("{} is missing", file.storage_key)))?;

    let bytes = original.bytes.clone();
    // The fixed emote rules, the same the upload was checked under (a `limits` an older job
    // carries is ignored: emote rules are the same for everyone).
    let limits = cyex_tv_media::Limits::emote();
    // The guard sets the flag if the time limit drops this future, so the thread stops too.
    let cancel = CancelOnDrop::default();
    let flag = cancel.flag();
    let encoded = tokio::task::spawn_blocking(move || {
        encode::transcode_cancellable(&bytes, &limits, &Profile::EMOTE, &flag)
    })
    .await
    .map_err(retry)?
    // The upload passed these checks; failing now means the file can't be made.
    .map_err(media_failure)?;
    drop(cancel);

    let sha_hex = sigv4::hex(&file.sha256);
    if is_blocked(&ctx, &file).await? {
        tracing::info!(file = %cyex_tv_db::ids::to_text(file.id), "blocked; not transcoded");
        return Ok(());
    }
    store_all(&store, &sha_hex, &encoded.webp).await?;
    // A takedown may have landed while this ran: delete what was written, record nothing.
    if is_blocked(&ctx, &file).await? {
        remove_written(&store, &sha_hex).await?;
        tracing::info!(file = %cyex_tv_db::ids::to_text(file.id), "blocked while transcoding; output removed");
        return Ok(());
    }
    let webp: Vec<&Variant> = encoded.webp.iter().collect();
    let mut conn = ctx.db.acquire().await.map_err(retry)?;
    files::set_variants(&mut conn, file.id, &entries(&webp)?)
        .await
        .map_err(retry)?;
    drop(conn);

    store_all(&store, &sha_hex, &encoded.avif).await?;
    if is_blocked(&ctx, &file).await? {
        remove_written(&store, &sha_hex).await?;
        let mut conn = ctx.db.acquire().await.map_err(retry)?;
        files::set_variants(&mut conn, file.id, &serde_json::json!([]))
            .await
            .map_err(retry)?;
        tracing::info!(file = %cyex_tv_db::ids::to_text(file.id), "blocked while transcoding; output removed");
        return Ok(());
    }
    let all: Vec<&Variant> = encoded.webp.iter().chain(&encoded.avif).collect();
    let mut conn = ctx.db.acquire().await.map_err(retry)?;
    files::set_variants(&mut conn, file.id, &entries(&all)?)
        .await
        .map_err(retry)?;
    tracing::info!(
        file = %cyex_tv_db::ids::to_text(file.id),
        frames = encoded.frames,
        files = all.len(),
        "transcoded"
    );
    Ok(())
}
