//! Stored images. Uploads with the same bytes share one file (the upload writes them).

use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgConnection};
use uuid::Uuid;

use crate::error::DbResult;
use crate::ids::new_id;

#[derive(Clone, Debug, PartialEq, Eq, FromRow)]
pub struct File {
    pub id: Uuid,
    pub sha256: Vec<u8>,
    pub dhash: Option<i64>,
    pub mime: String,
    pub width: i32,
    pub height: i32,
    pub frame_count: i32,
    pub variants: serde_json::Value,
    pub storage_key: String,
    pub created_at: DateTime<Utc>,
    /// A `files.delete` for a file no live emote uses is queued or done (an account deletion).
    pub purged_at: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug)]
pub struct NewFile<'a> {
    pub sha256: &'a [u8; 32],
    pub dhash: Option<i64>,
    pub mime: &'a str,
    pub width: i32,
    pub height: i32,
    pub frame_count: i32,
    pub storage_key: &'a str,
}

const FILE: &str = "id, sha256, dhash, mime, width, height, frame_count, variants, storage_key, \
                    created_at, purged_at";

/// Stores a file, or returns the one with the same sha256. The flag is true when it's new.
pub async fn insert_or_get(conn: &mut PgConnection, new: &NewFile<'_>) -> DbResult<(File, bool)> {
    let inserted: Option<File> = sqlx::query_as(&format!(
        "INSERT INTO files (id, sha256, dhash, mime, width, height, frame_count, storage_key)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
         ON CONFLICT (sha256) DO NOTHING
         RETURNING {FILE}"
    ))
    .bind(new_id())
    .bind(&new.sha256[..])
    .bind(new.dhash)
    .bind(new.mime)
    .bind(new.width)
    .bind(new.height)
    .bind(new.frame_count)
    .bind(new.storage_key)
    .fetch_optional(&mut *conn)
    .await?;
    if let Some(file) = inserted {
        return Ok((file, true));
    }
    let file = sqlx::query_as(&format!("SELECT {FILE} FROM files WHERE sha256 = $1"))
        .bind(&new.sha256[..])
        .fetch_one(conn)
        .await?;
    Ok((file, false))
}

pub async fn get(conn: &mut PgConnection, id: Uuid) -> DbResult<Option<File>> {
    Ok(
        sqlx::query_as(&format!("SELECT {FILE} FROM files WHERE id = $1"))
            .bind(id)
            .fetch_optional(conn)
            .await?,
    )
}

pub async fn find_by_sha256(conn: &mut PgConnection, sha256: &[u8; 32]) -> DbResult<Option<File>> {
    Ok(
        sqlx::query_as(&format!("SELECT {FILE} FROM files WHERE sha256 = $1"))
            .bind(&sha256[..])
            .fetch_optional(conn)
            .await?,
    )
}

/// Files used only by deleted emotes of `account_id` (deduped by sha256, so a file is shared by
/// every upload of the same bytes), not already marked purged. Marks them purged and returns
/// them, for one `files.delete` each.
pub async fn mark_orphans_purged(conn: &mut PgConnection, account_id: Uuid) -> DbResult<Vec<Uuid>> {
    Ok(sqlx::query_scalar(
        "UPDATE files f SET purged_at = now()
          WHERE f.purged_at IS NULL
            AND f.id IN (SELECT e.file_id FROM emotes e
                          WHERE e.owner_account_id = $1 AND e.deleted_at IS NOT NULL
                            AND e.file_id IS NOT NULL)
            AND NOT EXISTS (SELECT 1 FROM emotes live
                             WHERE live.file_id = f.id AND live.deleted_at IS NULL)
          RETURNING f.id",
    )
    .bind(account_id)
    .fetch_all(conn)
    .await?)
}

/// An upload of the bytes of a purged file: the file is live again (its objects have to be
/// stored and made again). Waits for a running purge of it to finish. False when it wasn't
/// purged.
pub async fn revive(conn: &mut PgConnection, id: Uuid) -> DbResult<bool> {
    let done = sqlx::query(
        "UPDATE files SET purged_at = NULL, variants = '[]' WHERE id = $1 AND purged_at IS NOT NULL",
    )
    .bind(id)
    .execute(conn)
    .await?;
    Ok(done.rows_affected() > 0)
}

/// Locks a file for its purge. `None` when it is gone.
pub async fn lock(conn: &mut PgConnection, id: Uuid) -> DbResult<Option<File>> {
    Ok(sqlx::query_as(&format!(
        "SELECT {FILE} FROM files WHERE id = $1 FOR UPDATE"
    ))
    .bind(id)
    .fetch_optional(conn)
    .await?)
}

/// Whether an emote that isn't deleted uses the file.
pub async fn in_use(conn: &mut PgConnection, id: Uuid) -> DbResult<bool> {
    Ok(sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM emotes WHERE file_id = $1 AND deleted_at IS NULL)",
    )
    .bind(id)
    .fetch_one(conn)
    .await?)
}

/// Records the variants the worker made (WebP and AVIF at 1x-4x, static frames).
pub async fn set_variants(
    conn: &mut PgConnection,
    id: Uuid,
    variants: &serde_json::Value,
) -> DbResult<bool> {
    let done = sqlx::query("UPDATE files SET variants = $2 WHERE id = $1")
        .bind(id)
        .bind(variants)
        .execute(conn)
        .await?;
    Ok(done.rows_affected() > 0)
}
