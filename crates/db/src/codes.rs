//! Redeem codes (ADR 0016): batches staff make, each code stored only as the sha256 of its
//! normalized text, and who redeemed which. See the migration `20261122000001_shop.sql`.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use cyex_tv_shared::model::CosmeticKind;
use sqlx::{FromRow, PgConnection};
use uuid::Uuid;

use crate::error::{DbError, DbResult};
use crate::ids::new_id;
use crate::shop::Bundle;

#[derive(Clone, Debug, PartialEq, Eq, FromRow)]
pub struct BatchRow {
    pub id: Uuid,
    pub name: String,
    pub prefix: String,
    pub item_id: Option<Uuid>,
    pub code_count: i32,
    pub uses_per_code: i32,
    pub expires_at: Option<DateTime<Utc>>,
    pub disabled_at: Option<DateTime<Utc>>,
    pub disabled_by: Option<Uuid>,
    pub created_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    /// Uses over every code.
    pub redemptions: i64,
}

const BATCH: &str = "b.id, b.name, b.prefix, b.item_id, b.code_count, b.uses_per_code, \
                     b.expires_at, b.disabled_at, b.disabled_by, b.created_by, b.created_at, \
                     coalesce((SELECT sum(c.uses) FROM redeem_codes c WHERE c.batch_id = b.id), 0)::bigint AS redemptions";

#[derive(Clone, Debug)]
pub struct NewBatch<'a> {
    pub name: &'a str,
    pub prefix: &'a str,
    pub item_id: Option<Uuid>,
    pub uses_per_code: i32,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_by: Option<Uuid>,
}

/// Makes a batch with its cosmetics and one code per hash. `NotFound` when a cosmetic doesn't
/// exist; `CodeTaken` in the (astronomically unlikely) case a hash exists already.
pub async fn create_batch(
    conn: &mut PgConnection,
    new: &NewBatch<'_>,
    bundle: &Bundle,
    hashes: &[[u8; 32]],
) -> DbResult<BatchRow> {
    let id = new_id();
    sqlx::query(
        "INSERT INTO code_batches (id, name, prefix, item_id, code_count, uses_per_code, expires_at, created_by)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
    )
    .bind(id)
    .bind(new.name)
    .bind(new.prefix)
    .bind(new.item_id)
    .bind(i32::try_from(hashes.len()).unwrap_or(i32::MAX))
    .bind(new.uses_per_code)
    .bind(new.expires_at)
    .bind(new.created_by)
    .execute(&mut *conn)
    .await?;
    for (kind, cosmetic) in bundle {
        let (badge, paint) = match kind {
            CosmeticKind::Badge => (Some(*cosmetic), None),
            CosmeticKind::Paint => (None, Some(*cosmetic)),
        };
        sqlx::query(
            "INSERT INTO code_batch_cosmetics (batch_id, badge_id, paint_id) VALUES ($1, $2, $3)
             ON CONFLICT DO NOTHING",
        )
        .bind(id)
        .bind(badge)
        .bind(paint)
        .execute(&mut *conn)
        .await
        .map_err(|err| match err.as_database_error() {
            Some(db) if db.is_foreign_key_violation() => DbError::NotFound,
            _ => err.into(),
        })?;
    }
    let ids: Vec<Uuid> = hashes.iter().map(|_| new_id()).collect();
    let hash_vecs: Vec<Vec<u8>> = hashes.iter().map(|h| h.to_vec()).collect();
    sqlx::query(
        "INSERT INTO redeem_codes (id, batch_id, code_hash)
         SELECT unnest($1::uuid[]), $2, unnest($3::bytea[])",
    )
    .bind(&ids)
    .bind(id)
    .bind(&hash_vecs)
    .execute(&mut *conn)
    .await
    .map_err(|err| match err.as_database_error() {
        Some(db) if db.is_unique_violation() => DbError::CodeTaken,
        _ => err.into(),
    })?;
    get_batch(conn, id).await?.ok_or(DbError::NotFound)
}

pub async fn get_batch(conn: &mut PgConnection, id: Uuid) -> DbResult<Option<BatchRow>> {
    Ok(sqlx::query_as(&format!(
        "SELECT {BATCH} FROM code_batches b WHERE b.id = $1"
    ))
    .bind(id)
    .fetch_optional(conn)
    .await?)
}

/// Batches newest first, with the total.
pub async fn batches(
    conn: &mut PgConnection,
    limit: i64,
    offset: i64,
) -> DbResult<(Vec<BatchRow>, i64)> {
    let rows = sqlx::query_as(&format!(
        "SELECT {BATCH} FROM code_batches b ORDER BY b.id DESC LIMIT $1 OFFSET $2"
    ))
    .bind(limit)
    .bind(offset)
    .fetch_all(&mut *conn)
    .await?;
    let total = sqlx::query_scalar("SELECT count(*) FROM code_batches")
        .fetch_one(conn)
        .await?;
    Ok((rows, total))
}

/// The direct cosmetics of each batch (not its item's).
pub async fn batch_bundles(
    conn: &mut PgConnection,
    batches: &[Uuid],
) -> DbResult<HashMap<Uuid, Vec<(CosmeticKind, Uuid)>>> {
    crate::shop::bundles(conn, "code_batch_cosmetics", "batch_id", batches).await
}

/// Disables a batch: none of its codes work any more. False when it was disabled already or
/// doesn't exist.
pub async fn disable_batch(conn: &mut PgConnection, id: Uuid, by: Option<Uuid>) -> DbResult<bool> {
    let done = sqlx::query(
        "UPDATE code_batches SET disabled_at = now(), disabled_by = $2
         WHERE id = $1 AND disabled_at IS NULL",
    )
    .bind(id)
    .bind(by)
    .execute(conn)
    .await?;
    Ok(done.rows_affected() > 0)
}

#[derive(Clone, Debug, PartialEq, Eq, FromRow)]
pub struct CodeRow {
    pub id: Uuid,
    pub batch_id: Uuid,
    pub code_hash: Vec<u8>,
    pub uses: i32,
}

/// The code with this hash, locked for redeeming.
pub async fn lock_code(conn: &mut PgConnection, hash: &[u8; 32]) -> DbResult<Option<CodeRow>> {
    Ok(sqlx::query_as(
        "SELECT id, batch_id, code_hash, uses FROM redeem_codes WHERE code_hash = $1 FOR UPDATE",
    )
    .bind(&hash[..])
    .fetch_optional(conn)
    .await?)
}

pub async fn has_redeemed(conn: &mut PgConnection, code: Uuid, account: Uuid) -> DbResult<bool> {
    Ok(sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM code_redemptions WHERE code_id = $1 AND account_id = $2)",
    )
    .bind(code)
    .bind(account)
    .fetch_one(conn)
    .await?)
}

/// Records a use of a code by an account.
pub async fn record_redemption(conn: &mut PgConnection, code: Uuid, account: Uuid) -> DbResult<()> {
    sqlx::query("INSERT INTO code_redemptions (code_id, account_id) VALUES ($1, $2)")
        .bind(code)
        .bind(account)
        .execute(&mut *conn)
        .await?;
    sqlx::query("UPDATE redeem_codes SET uses = uses + 1 WHERE id = $1")
        .bind(code)
        .execute(conn)
        .await?;
    Ok(())
}
