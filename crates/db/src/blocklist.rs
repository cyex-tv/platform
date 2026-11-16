//! The image blocklist. See `20261118000003_blocklist.sql`.

use uuid::Uuid;

use crate::error::DbResult;
use crate::ids::new_id;
use sqlx::PgConnection;

/// How many dHash bits may differ for an upload to count as a blocked image.
pub const DHASH_DISTANCE: i32 = 4;

/// The reason, if the image is blocked: an exact sha256 match, or a dHash within
/// [`DHASH_DISTANCE`] bits.
pub async fn check(
    conn: &mut PgConnection,
    sha256: &[u8; 32],
    dhash: i64,
) -> DbResult<Option<String>> {
    Ok(sqlx::query_scalar(
        "SELECT reason FROM blocked_images
         WHERE sha256 = $1
            OR (dhash IS NOT NULL AND bit_count((dhash # $2)::bit(64)) <= $3)
         ORDER BY (sha256 = $1) DESC NULLS LAST
         LIMIT 1",
    )
    .bind(&sha256[..])
    .bind(dhash)
    .bind(DHASH_DISTANCE)
    .fetch_optional(conn)
    .await?)
}

/// Adds an entry. Phase 7 puts the moderator API and its audit row around this.
pub async fn add(
    conn: &mut PgConnection,
    sha256: Option<&[u8; 32]>,
    dhash: Option<i64>,
    reason: &str,
    created_by: Option<Uuid>,
) -> DbResult<Uuid> {
    Ok(sqlx::query_scalar(
        "INSERT INTO blocked_images (id, sha256, dhash, reason, created_by)
         VALUES ($1, $2, $3, $4, $5)
         RETURNING id",
    )
    .bind(new_id())
    .bind(sha256.map(|s| &s[..]))
    .bind(dhash)
    .bind(reason)
    .bind(created_by)
    .fetch_one(conn)
    .await?)
}
