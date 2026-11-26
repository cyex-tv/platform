//! Developer API keys (ADR 0033). See `20261228000501_developer_keys_webhooks.sql`.

use chrono::{DateTime, NaiveDate, Utc};
use sqlx::{FromRow, PgConnection};
use uuid::Uuid;

use crate::error::DbResult;
use crate::sessions::TokenHash;

#[derive(Clone, Debug, PartialEq, Eq, FromRow)]
pub struct ApiKey {
    pub id: Uuid,
    pub account_id: Uuid,
    pub name: String,
    pub prefix: String,
    pub created_at: DateTime<Utc>,
    pub last_used_at: Option<DateTime<Utc>>,
}

const KEY: &str = "id, account_id, name, prefix, created_at, last_used_at";

/// Makes a key, unless the account already has `max` (then `None`). The count and the insert run
/// under a lock on the account's row, so two at once can't pass the limit together.
pub async fn create(
    conn: &mut PgConnection,
    account_id: Uuid,
    name: &str,
    prefix: &str,
    hash: &TokenHash,
    max: i64,
) -> DbResult<Option<ApiKey>> {
    sqlx::query("SELECT 1 FROM accounts WHERE id = $1 FOR UPDATE")
        .bind(account_id)
        .execute(&mut *conn)
        .await?;
    let have: i64 = sqlx::query_scalar("SELECT count(*) FROM api_keys WHERE account_id = $1")
        .bind(account_id)
        .fetch_one(&mut *conn)
        .await?;
    if have >= max {
        return Ok(None);
    }
    Ok(Some(
        sqlx::query_as(&format!(
            "INSERT INTO api_keys (id, account_id, name, prefix, key_hash)
             VALUES ($1, $2, $3, $4, $5)
             RETURNING {KEY}"
        ))
        .bind(crate::ids::new_id())
        .bind(account_id)
        .bind(name)
        .bind(prefix)
        .bind(&hash[..])
        .fetch_one(conn)
        .await?,
    ))
}

/// The account's keys, oldest first.
pub async fn list(conn: &mut PgConnection, account_id: Uuid) -> DbResult<Vec<ApiKey>> {
    Ok(sqlx::query_as(&format!(
        "SELECT {KEY} FROM api_keys WHERE account_id = $1 ORDER BY created_at, id"
    ))
    .bind(account_id)
    .fetch_all(conn)
    .await?)
}

/// The key for a hash, and whether its account is live (neither deleted nor blocked), so a
/// cache can remember a blocked account's key by its account and forget it when that changes.
pub async fn find(conn: &mut PgConnection, hash: &TokenHash) -> DbResult<Option<(ApiKey, bool)>> {
    #[derive(FromRow)]
    struct Row {
        #[sqlx(flatten)]
        key: ApiKey,
        live: bool,
    }
    let row: Option<Row> = sqlx::query_as(
        "SELECT k.id, k.account_id, k.name, k.prefix, k.created_at, k.last_used_at,
                a.deleted_at IS NULL AND (a.blocked_at IS NULL OR a.blocked_until <= now()) AS live
         FROM api_keys k
         JOIN accounts a ON a.id = k.account_id
         WHERE k.key_hash = $1",
    )
    .bind(&hash[..])
    .fetch_optional(conn)
    .await?;
    Ok(row.map(|r| (r.key, r.live)))
}

/// The key for a hash, if its account is neither deleted nor blocked.
pub async fn find_active(conn: &mut PgConnection, hash: &TokenHash) -> DbResult<Option<ApiKey>> {
    Ok(sqlx::query_as(
        "SELECT k.id, k.account_id, k.name, k.prefix, k.created_at, k.last_used_at
         FROM api_keys k
         JOIN accounts a ON a.id = k.account_id
         WHERE k.key_hash = $1
           AND a.deleted_at IS NULL AND (a.blocked_at IS NULL OR a.blocked_until <= now())",
    )
    .bind(&hash[..])
    .fetch_optional(conn)
    .await?)
}

/// Revokes one of the account's keys. True if it existed.
pub async fn delete(conn: &mut PgConnection, account_id: Uuid, id: Uuid) -> DbResult<bool> {
    let done = sqlx::query("DELETE FROM api_keys WHERE account_id = $1 AND id = $2")
        .bind(account_id)
        .bind(id)
        .execute(conn)
        .await?;
    Ok(done.rows_affected() > 0)
}

/// Revokes every key of the account (deletion, a staff block). Returns how many.
pub async fn delete_for_account(conn: &mut PgConnection, account_id: Uuid) -> DbResult<u64> {
    let done = sqlx::query("DELETE FROM api_keys WHERE account_id = $1")
        .bind(account_id)
        .execute(conn)
        .await?;
    Ok(done.rows_affected())
}

/// Adds counted requests: per key, on `day`, and marks the keys used. Keys revoked in the meantime
/// are skipped.
pub async fn add_usage(
    conn: &mut PgConnection,
    day: NaiveDate,
    counts: &[(Uuid, i64)],
) -> DbResult<()> {
    if counts.is_empty() {
        return Ok(());
    }
    let keys: Vec<Uuid> = counts.iter().map(|(k, _)| *k).collect();
    let requests: Vec<i64> = counts.iter().map(|(_, n)| *n).collect();
    sqlx::query(
        "INSERT INTO api_key_usage (key_id, day, requests)
         SELECT u.key_id, $1, u.requests
           FROM unnest($2::uuid[], $3::bigint[]) AS u (key_id, requests)
           JOIN api_keys k ON k.id = u.key_id
         ON CONFLICT (key_id, day) DO UPDATE
         SET requests = api_key_usage.requests + EXCLUDED.requests",
    )
    .bind(day)
    .bind(&keys)
    .bind(&requests)
    .execute(&mut *conn)
    .await?;
    sqlx::query("UPDATE api_keys SET last_used_at = now() WHERE id = ANY($1)")
        .bind(&keys)
        .execute(conn)
        .await?;
    Ok(())
}

/// Each key's requests per day since `from`, oldest first.
pub async fn usage_since(
    conn: &mut PgConnection,
    keys: &[Uuid],
    from: NaiveDate,
) -> DbResult<Vec<(Uuid, NaiveDate, i64)>> {
    Ok(sqlx::query_as(
        "SELECT key_id, day, requests FROM api_key_usage
          WHERE key_id = ANY($1) AND day >= $2
          ORDER BY key_id, day",
    )
    .bind(keys)
    .bind(from)
    .fetch_all(conn)
    .await?)
}

/// Removes usage older than `before`. Run with the hourly clean-up.
pub async fn delete_usage_before(conn: &mut PgConnection, before: NaiveDate) -> DbResult<u64> {
    let done = sqlx::query("DELETE FROM api_key_usage WHERE day < $1")
        .bind(before)
        .execute(conn)
        .await?;
    Ok(done.rows_affected())
}
