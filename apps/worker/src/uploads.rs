//! Direct uploads (ADR 0036): removes the bytes of uploads never finalized once they expire, and
//! forgets finalized ones after a day (their bytes were moved to the picture's original).

use cyex_tv_db::direct_uploads;
use cyex_tv_storage::Store;
use sqlx::PgPool;

/// Uploads handled per clean-up round; the rest wait for the next.
pub const BATCH: i64 = 200;

/// Deletes what is due: each upload's stored object, then its row. An object storage refuses to
/// delete keeps its row, so the next round tries again. Returns how many were removed.
pub async fn clean_up(db: &PgPool, store: &Store) -> Result<u64, cyex_tv_db::DbError> {
    let mut conn = db.acquire().await?;
    let due = direct_uploads::due(
        &mut conn,
        chrono::Utc::now() - chrono::Duration::days(1),
        BATCH,
    )
    .await?;
    let mut removed = 0;
    for upload in due {
        if let Err(err) = store.delete(&upload.storage_key).await {
            tracing::warn!(error = %err, key = %upload.storage_key, "deleting an expired direct upload failed");
            continue;
        }
        direct_uploads::delete(&mut conn, upload.id).await?;
        removed += 1;
    }
    Ok(removed)
}
