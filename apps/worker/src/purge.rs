//! The `files.delete` job (PLAN.md Phase 7, takedown): a taken-down file's objects are deleted
//! from storage, and their CDN URLs purged from Cloudflare's cache.
//!
//! An account deletion queues it too (`"reason": "account_deleted"`) for files no live emote
//! uses. Those run under the file's row lock and only while the file is still marked purged and
//! unused: an upload of the same bytes in the meantime revives the file instead.
//!
//! Purging needs a Cloudflare API token with Cache Purge on the `cyex.tv` zone
//! (`CYEX_TV_CLOUDFLARE_ZONE_ID`, `CYEX_TV_CLOUDFLARE_PURGE_TOKEN`). Without one the objects are
//! still deleted, and the cached copies expire on their own; a warning says so.

use cyex_tv_db::files;
use cyex_tv_db::jobs::Job;
use cyex_tv_storage::{emote_key, sigv4};
use serde_json::{Value, json};
use url::Url;

use crate::{Ctx, JobError};

pub const KIND: &str = "files.delete";
/// URLs per purge request (Cloudflare's limit on most plans).
pub const PURGE_BATCH: usize = 30;

/// Where and how to purge.
#[derive(Clone)]
pub struct Purge {
    pub http: reqwest::Client,
    /// `https://api.cloudflare.com/client/v4/` (tests point it elsewhere).
    pub api: Url,
    pub zone_id: String,
    pub token: String,
    /// The CDN origin, without a trailing slash: `https://cdn.cyex.tv`.
    pub cdn_url: String,
}

impl std::fmt::Debug for Purge {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Purge")
            .field("api", &self.api.as_str())
            .field("zone_id", &self.zone_id)
            .field("token", &"<redacted>")
            .field("cdn_url", &self.cdn_url)
            .finish()
    }
}

impl Purge {
    /// From `CYEX_TV_CLOUDFLARE_*` and `CYEX_TV_CDN_URL`; `None` without a token.
    pub fn from_env(get: impl Fn(&str) -> Option<String>) -> Result<Option<Self>, reqwest::Error> {
        let (Some(zone_id), Some(token)) = (
            get("CYEX_TV_CLOUDFLARE_ZONE_ID").filter(|v| !v.trim().is_empty()),
            get("CYEX_TV_CLOUDFLARE_PURGE_TOKEN").filter(|v| !v.trim().is_empty()),
        ) else {
            return Ok(None);
        };
        let _ = rustls::crypto::ring::default_provider().install_default();
        Ok(Some(Self {
            http: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(20))
                .build()?,
            // A constant; it parses.
            api: Url::parse("https://api.cloudflare.com/client/v4/").expect("a valid URL"),
            zone_id: zone_id.trim().to_owned(),
            token: token.trim().to_owned(),
            cdn_url: get("CYEX_TV_CDN_URL")
                .unwrap_or_else(|| "https://cdn.cyex.tv".to_owned())
                .trim_end_matches('/')
                .to_owned(),
        }))
    }

    async fn purge(&self, urls: &[String]) -> Result<(), JobError> {
        let endpoint = self
            .api
            .join(&format!("zones/{}/purge_cache", self.zone_id))
            .map_err(|err| JobError::Permanent(err.to_string()))?;
        for batch in urls.chunks(PURGE_BATCH) {
            let response = self
                .http
                .post(endpoint.clone())
                .bearer_auth(&self.token)
                .json(&json!({ "files": batch }))
                .send()
                .await
                .map_err(|err| JobError::Retry(format!("purge request: {err}")))?;
            let status = response.status();
            let body: Value = response.json().await.unwrap_or(Value::Null);
            if !status.is_success() || body["success"] != true {
                return Err(JobError::Retry(format!("purge answered {status}: {body}")));
            }
        }
        Ok(())
    }
}

fn retry(err: impl std::fmt::Display) -> JobError {
    JobError::Retry(err.to_string())
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
        .ok_or_else(|| JobError::Retry("storage isn't configured".into()))?;
    if job.payload.get("reason").and_then(Value::as_str) == Some("account_deleted") {
        return orphan(ctx, store, file_id).await;
    }
    let mut conn = ctx.db.acquire().await.map_err(retry)?;
    let file = files::get(&mut conn, file_id)
        .await
        .map_err(retry)?
        .ok_or_else(|| JobError::Permanent("the file is gone".into()))?;
    drop(conn);
    delete_objects(&ctx, &store, &file).await?;
    let mut conn = ctx.db.acquire().await.map_err(retry)?;
    files::set_variants(&mut conn, file.id, &json!([]))
        .await
        .map_err(retry)?;
    Ok(())
}

/// A file a deleted account left: deleted under its lock, unless it was revived or is in use.
async fn orphan(
    ctx: Ctx,
    store: cyex_tv_storage::Store,
    file_id: sqlx::types::Uuid,
) -> Result<(), JobError> {
    let mut tx = ctx.db.begin().await.map_err(retry)?;
    let file = files::lock(&mut tx, file_id)
        .await
        .map_err(retry)?
        .ok_or_else(|| JobError::Permanent("the file is gone".into()))?;
    if file.purged_at.is_none() {
        tracing::info!(file = %cyex_tv_db::ids::to_text(file.id), "file uploaded again; not deleted");
        return Ok(());
    }
    if files::in_use(&mut tx, file.id).await.map_err(retry)? {
        files::revive(&mut tx, file.id).await.map_err(retry)?;
        tx.commit().await.map_err(retry)?;
        tracing::info!(file = %cyex_tv_db::ids::to_text(file.id), "file in use again; not deleted");
        return Ok(());
    }
    delete_objects(&ctx, &store, &file).await?;
    files::set_variants(&mut tx, file.id, &json!([]))
        .await
        .map_err(retry)?;
    tx.commit().await.map_err(retry)?;
    Ok(())
}

/// Deletes every object of a file and purges its CDN URLs.
async fn delete_objects(
    ctx: &Ctx,
    store: &cyex_tv_storage::Store,
    file: &files::File,
) -> Result<(), JobError> {
    let sha_hex = sigv4::hex(&file.sha256);
    // The recorded names, plus every name the encoder can make: a transcode that was queued or
    // running at takedown time writes files the record doesn't know about yet.
    let mut names: Vec<String> = file
        .variants
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter_map(|v| v["name"].as_str().map(str::to_owned))
                .collect()
        })
        .unwrap_or_default();
    for name in cyex_tv_media::encode::all_names() {
        if !names.contains(&name) {
            names.push(name);
        }
    }
    for name in &names {
        store
            .delete(&emote_key(&sha_hex, name))
            .await
            .map_err(retry)?;
    }
    store.delete(&file.storage_key).await.map_err(retry)?;

    // Purge before forgetting the names, so a failed purge is retried with them.
    match &ctx.purge {
        Some(purge) => {
            let urls: Vec<String> = names
                .iter()
                .map(|n| format!("{}/emotes/{sha_hex}/{n}", purge.cdn_url))
                .collect();
            purge.purge(&urls).await?;
        }
        None => tracing::warn!(
            file = %cyex_tv_db::ids::to_text(file.id),
            "no Cloudflare purge token; cached copies expire on their own"
        ),
    }
    tracing::info!(file = %cyex_tv_db::ids::to_text(file.id), objects = names.len() + 1, "files deleted");
    Ok(())
}
