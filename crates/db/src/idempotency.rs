//! Idempotency keys: one stored answer per (account, operation, key) for 24 hours, so a client
//! that retries a request it isn't sure went through gets the first answer instead of acting
//! twice. The api decides what an operation is and what goes in the stored answer.

use chrono::{DateTime, Utc};
use sqlx::PgConnection;
use uuid::Uuid;

use crate::error::DbResult;

/// How long a key is remembered.
pub const KEEP_HOURS: i64 = 24;

/// What claiming a key found.
#[derive(Clone, Debug, PartialEq)]
pub enum Claim {
    /// The key is new: the caller runs the request, then [`finish`] or [`release`].
    New,
    /// The key was used for a request that finished: its hash and answer.
    Done {
        request_hash: Vec<u8>,
        response: serde_json::Value,
    },
    /// The key's first request is still running (or stopped without storing its answer).
    Running { request_hash: Vec<u8> },
}

/// Claims `key` for `operation` of `account`, or says what it holds. A row older than
/// [`KEEP_HOURS`] counts as gone. Each statement commits on its own, so a second request with the
/// same key sees the claim at once.
pub async fn claim(
    conn: &mut PgConnection,
    account: Uuid,
    operation: &str,
    key: Uuid,
    request_hash: &[u8; 32],
) -> DbResult<Claim> {
    sqlx::query(
        "DELETE FROM idempotency_keys
         WHERE account_id = $1 AND operation = $2 AND key = $3
           AND created_at < now() - make_interval(hours => $4)",
    )
    .bind(account)
    .bind(operation)
    .bind(key)
    .bind(KEEP_HOURS as i32)
    .execute(&mut *conn)
    .await?;
    let inserted = sqlx::query(
        "INSERT INTO idempotency_keys (account_id, operation, key, request_hash)
         VALUES ($1, $2, $3, $4)
         ON CONFLICT DO NOTHING",
    )
    .bind(account)
    .bind(operation)
    .bind(key)
    .bind(&request_hash[..])
    .execute(&mut *conn)
    .await?
    .rows_affected();
    if inserted == 1 {
        return Ok(Claim::New);
    }
    let row: Option<(Vec<u8>, Option<serde_json::Value>)> = sqlx::query_as(
        "SELECT request_hash, response FROM idempotency_keys
         WHERE account_id = $1 AND operation = $2 AND key = $3",
    )
    .bind(account)
    .bind(operation)
    .bind(key)
    .fetch_optional(&mut *conn)
    .await?;
    Ok(match row {
        Some((request_hash, Some(response))) => Claim::Done {
            request_hash,
            response,
        },
        Some((request_hash, None)) => Claim::Running { request_hash },
        // Removed between the insert and the read (an expiry sweep): busy, try again.
        None => Claim::Running {
            request_hash: request_hash.to_vec(),
        },
    })
}

/// Stores the answer of a claimed key.
pub async fn finish(
    conn: &mut PgConnection,
    account: Uuid,
    operation: &str,
    key: Uuid,
    response: &serde_json::Value,
) -> DbResult<()> {
    sqlx::query(
        "UPDATE idempotency_keys SET response = $4
         WHERE account_id = $1 AND operation = $2 AND key = $3",
    )
    .bind(account)
    .bind(operation)
    .bind(key)
    .bind(response)
    .execute(conn)
    .await?;
    Ok(())
}

/// Gives a claimed key back when its request failed, so a retry with it acts again.
pub async fn release(
    conn: &mut PgConnection,
    account: Uuid,
    operation: &str,
    key: Uuid,
) -> DbResult<()> {
    sqlx::query(
        "DELETE FROM idempotency_keys
         WHERE account_id = $1 AND operation = $2 AND key = $3 AND response IS NULL",
    )
    .bind(account)
    .bind(operation)
    .bind(key)
    .execute(conn)
    .await?;
    Ok(())
}

/// Removes keys created before `before`, 10 000 at a time. Returns how many.
pub async fn delete_older_than(conn: &mut PgConnection, before: DateTime<Utc>) -> DbResult<u64> {
    let mut n = 0;
    loop {
        let done = sqlx::query(
            "DELETE FROM idempotency_keys WHERE ctid IN (
                 SELECT ctid FROM idempotency_keys WHERE created_at < $1 LIMIT 10000)",
        )
        .bind(before)
        .execute(&mut *conn)
        .await?
        .rows_affected();
        n += done;
        if done < 10_000 {
            return Ok(n);
        }
    }
}
