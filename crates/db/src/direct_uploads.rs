//! Direct uploads (ADR 0036): presigned uploads of badge and paint pictures, from the presign to
//! finalize or expiry.

use chrono::{DateTime, Utc};
use cyex_tv_shared::model::CosmeticKind;
use sqlx::{PgConnection, Row};
use uuid::Uuid;

use crate::error::DbResult;

/// One presigned upload.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DirectUpload {
    pub id: Uuid,
    pub account_id: Uuid,
    pub kind: CosmeticKind,
    pub storage_key: String,
    pub content_type: String,
    pub declared_bytes: i64,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub finalized_at: Option<DateTime<Utc>>,
    pub image_id: Option<Uuid>,
}

/// What a new upload records.
#[derive(Clone, Debug)]
pub struct NewUpload<'a> {
    pub id: Uuid,
    pub account_id: Uuid,
    pub kind: CosmeticKind,
    pub storage_key: &'a str,
    pub content_type: &'a str,
    pub declared_bytes: i64,
    pub expires_at: DateTime<Utc>,
}

const COLUMNS: &str = "id, account_id, kind, storage_key, content_type, declared_bytes, \
     created_at, expires_at, finalized_at, image_id";

fn from_row(row: &sqlx::postgres::PgRow) -> DbResult<DirectUpload> {
    let kind: String = row.try_get("kind")?;
    Ok(DirectUpload {
        id: row.try_get("id")?,
        account_id: row.try_get("account_id")?,
        // The column only holds these two (a CHECK).
        kind: CosmeticKind::parse(&kind).unwrap_or(CosmeticKind::Badge),
        storage_key: row.try_get("storage_key")?,
        content_type: row.try_get("content_type")?,
        declared_bytes: row.try_get("declared_bytes")?,
        created_at: row.try_get("created_at")?,
        expires_at: row.try_get("expires_at")?,
        finalized_at: row.try_get("finalized_at")?,
        image_id: row.try_get("image_id")?,
    })
}

pub async fn insert(conn: &mut PgConnection, new: &NewUpload<'_>) -> DbResult<DirectUpload> {
    let row = sqlx::query(&format!(
        "INSERT INTO direct_uploads
             (id, account_id, kind, storage_key, content_type, declared_bytes, expires_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7)
         RETURNING {COLUMNS}"
    ))
    .bind(new.id)
    .bind(new.account_id)
    .bind(new.kind.as_str())
    .bind(new.storage_key)
    .bind(new.content_type)
    .bind(new.declared_bytes)
    .bind(new.expires_at)
    .fetch_one(&mut *conn)
    .await?;
    from_row(&row)
}

pub async fn get(conn: &mut PgConnection, id: Uuid) -> DbResult<Option<DirectUpload>> {
    let row = sqlx::query(&format!(
        "SELECT {COLUMNS} FROM direct_uploads WHERE id = $1"
    ))
    .bind(id)
    .fetch_optional(&mut *conn)
    .await?;
    row.as_ref().map(from_row).transpose()
}

/// Unfinalized uploads of `account` that haven't expired.
pub async fn open_count(conn: &mut PgConnection, account: Uuid) -> DbResult<i64> {
    Ok(sqlx::query_scalar(
        "SELECT count(*) FROM direct_uploads
         WHERE account_id = $1 AND finalized_at IS NULL AND expires_at > now()",
    )
    .bind(account)
    .fetch_one(&mut *conn)
    .await?)
}

/// Marks an upload finalized (once): `false` when another finalize got there first.
pub async fn finalize(conn: &mut PgConnection, id: Uuid, image: Uuid) -> DbResult<bool> {
    Ok(sqlx::query(
        "UPDATE direct_uploads SET finalized_at = now(), image_id = $2
         WHERE id = $1 AND finalized_at IS NULL",
    )
    .bind(id)
    .bind(image)
    .execute(&mut *conn)
    .await?
    .rows_affected()
        == 1)
}

/// Forgets an upload (its bytes were refused at finalize, or cleaned up).
pub async fn delete(conn: &mut PgConnection, id: Uuid) -> DbResult<()> {
    sqlx::query("DELETE FROM direct_uploads WHERE id = $1")
        .bind(id)
        .execute(&mut *conn)
        .await?;
    Ok(())
}

/// Up to `limit` uploads to clean up: unfinalized ones past `expires_at`, and finalized ones
/// older than `finalized_before` (their bytes were already moved; only the row is left).
pub async fn due(
    conn: &mut PgConnection,
    finalized_before: DateTime<Utc>,
    limit: i64,
) -> DbResult<Vec<DirectUpload>> {
    let rows = sqlx::query(&format!(
        "SELECT {COLUMNS} FROM direct_uploads
         WHERE (finalized_at IS NULL AND expires_at < now())
            OR (finalized_at IS NOT NULL AND finalized_at < $1)
         ORDER BY expires_at
         LIMIT $2"
    ))
    .bind(finalized_before)
    .bind(limit)
    .fetch_all(&mut *conn)
    .await?;
    rows.iter().map(from_row).collect()
}
