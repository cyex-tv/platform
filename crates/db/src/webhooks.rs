//! Channel webhooks and their deliveries (ADR 0034). See
//! `20261228000501_developer_keys_webhooks.sql`: a trigger on the outbox makes the deliveries and
//! their `webhook.deliver` jobs.

use chrono::{DateTime, Utc};
use serde_json::Value;
use sqlx::{FromRow, PgConnection};
use uuid::Uuid;

use crate::error::DbResult;

/// The job kind that sends one delivery.
pub const DELIVER_KIND: &str = "webhook.deliver";
/// Attempts one delivery gets (10 s doubling between them: about 21 minutes in all).
pub const DELIVER_ATTEMPTS: i32 = 8;
/// Deliveries kept per webhook.
pub const KEEP_DELIVERIES: i64 = 50;
/// Deliveries in a row that fail for good before a webhook is turned off.
pub const DISABLE_AFTER: i32 = 5;

#[derive(Clone, Debug, PartialEq, Eq, FromRow)]
pub struct Webhook {
    pub id: Uuid,
    pub channel_id: Uuid,
    pub topic: String,
    pub url: String,
    pub event_types: Vec<String>,
    pub secret: String,
    pub enabled: bool,
    pub disabled_reason: Option<String>,
    pub failed_deliveries: i32,
    pub created_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

const WEBHOOK: &str = "id, channel_id, topic, url, event_types, secret, enabled, disabled_reason, \
                       failed_deliveries, created_by, created_at, updated_at";

#[derive(Clone, Debug, PartialEq, FromRow)]
pub struct Delivery {
    pub id: i64,
    pub webhook_id: Uuid,
    pub event_id: Option<i64>,
    pub event_type: String,
    pub body: Value,
    pub state: String,
    pub attempts: i32,
    pub response_status: Option<i32>,
    pub error: Option<String>,
    pub duration_ms: Option<i32>,
    pub created_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
}

const DELIVERY: &str = "id, webhook_id, event_id, event_type, body, state, attempts, \
                        response_status, error, duration_ms, created_at, finished_at";

/// Makes a webhook, unless the channel already has `max` (then `None`). The channel's row is
/// locked for the count, so two at once can't pass the limit together.
#[allow(clippy::too_many_arguments)]
pub async fn create(
    conn: &mut PgConnection,
    channel_id: Uuid,
    topic: &str,
    url: &str,
    event_types: &[String],
    secret: &str,
    created_by: Uuid,
    max: i64,
) -> DbResult<Option<Webhook>> {
    sqlx::query("SELECT 1 FROM channels WHERE id = $1 FOR UPDATE")
        .bind(channel_id)
        .execute(&mut *conn)
        .await?;
    let have: i64 = sqlx::query_scalar("SELECT count(*) FROM webhooks WHERE channel_id = $1")
        .bind(channel_id)
        .fetch_one(&mut *conn)
        .await?;
    if have >= max {
        return Ok(None);
    }
    Ok(Some(
        sqlx::query_as(&format!(
            "INSERT INTO webhooks (id, channel_id, topic, url, event_types, secret, created_by)
             VALUES ($1, $2, $3, $4, $5, $6, $7)
             RETURNING {WEBHOOK}"
        ))
        .bind(crate::ids::new_id())
        .bind(channel_id)
        .bind(topic)
        .bind(url)
        .bind(event_types)
        .bind(secret)
        .bind(created_by)
        .fetch_one(conn)
        .await?,
    ))
}

pub async fn get(conn: &mut PgConnection, id: Uuid) -> DbResult<Option<Webhook>> {
    Ok(
        sqlx::query_as(&format!("SELECT {WEBHOOK} FROM webhooks WHERE id = $1"))
            .bind(id)
            .fetch_optional(conn)
            .await?,
    )
}

/// The channel's webhooks, oldest first.
pub async fn list_for_channel(conn: &mut PgConnection, channel_id: Uuid) -> DbResult<Vec<Webhook>> {
    Ok(sqlx::query_as(&format!(
        "SELECT {WEBHOOK} FROM webhooks WHERE channel_id = $1 ORDER BY created_at, id"
    ))
    .bind(channel_id)
    .fetch_all(conn)
    .await?)
}

/// Changes what is given. Turning it on clears the reason and the failure count; turning it off
/// by hand records `manual`.
pub async fn update(
    conn: &mut PgConnection,
    id: Uuid,
    url: Option<&str>,
    event_types: Option<&[String]>,
    enabled: Option<bool>,
) -> DbResult<Option<Webhook>> {
    Ok(sqlx::query_as(&format!(
        "UPDATE webhooks
         SET url = coalesce($2, url),
             event_types = coalesce($3, event_types),
             enabled = coalesce($4, enabled),
             disabled_reason = CASE WHEN $4 IS TRUE THEN NULL
                                    WHEN $4 IS FALSE THEN 'manual'
                                    ELSE disabled_reason END,
             failed_deliveries = CASE WHEN $4 IS TRUE THEN 0 ELSE failed_deliveries END,
             updated_at = now()
         WHERE id = $1
         RETURNING {WEBHOOK}"
    ))
    .bind(id)
    .bind(url)
    .bind(event_types)
    .bind(enabled)
    .fetch_optional(conn)
    .await?)
}

pub async fn set_secret(
    conn: &mut PgConnection,
    id: Uuid,
    secret: &str,
) -> DbResult<Option<Webhook>> {
    Ok(sqlx::query_as(&format!(
        "UPDATE webhooks SET secret = $2, updated_at = now() WHERE id = $1 RETURNING {WEBHOOK}"
    ))
    .bind(id)
    .bind(secret)
    .fetch_optional(conn)
    .await?)
}

pub async fn delete(conn: &mut PgConnection, id: Uuid) -> DbResult<bool> {
    let done = sqlx::query("DELETE FROM webhooks WHERE id = $1")
        .bind(id)
        .execute(conn)
        .await?;
    Ok(done.rows_affected() > 0)
}

/// Removes the webhooks of every channel of the account (account deletion).
pub async fn delete_for_account(conn: &mut PgConnection, account_id: Uuid) -> DbResult<u64> {
    let done = sqlx::query(
        "DELETE FROM webhooks WHERE channel_id IN (SELECT id FROM channels WHERE account_id = $1)",
    )
    .bind(account_id)
    .execute(conn)
    .await?;
    Ok(done.rows_affected())
}

/// Queues a "Send test" delivery (one attempt, never counted towards turning it off).
pub async fn queue_test(
    conn: &mut PgConnection,
    webhook_id: Uuid,
    body: &Value,
) -> DbResult<Delivery> {
    let delivery: Delivery = sqlx::query_as(&format!(
        "INSERT INTO webhook_deliveries (webhook_id, event_type, body)
         VALUES ($1, $2, $3)
         RETURNING {DELIVERY}"
    ))
    .bind(webhook_id)
    .bind(cyex_tv_shared::webhooks::TEST_EVENT)
    .bind(body)
    .fetch_one(&mut *conn)
    .await?;
    crate::jobs::enqueue(
        conn,
        DELIVER_KIND,
        &serde_json::json!({ "delivery_id": delivery.id }),
        1,
    )
    .await?;
    Ok(delivery)
}

/// The latest deliveries of a webhook, newest first.
pub async fn deliveries(
    conn: &mut PgConnection,
    webhook_id: Uuid,
    limit: i64,
) -> DbResult<Vec<Delivery>> {
    Ok(sqlx::query_as(&format!(
        "SELECT {DELIVERY} FROM webhook_deliveries WHERE webhook_id = $1 ORDER BY id DESC LIMIT $2"
    ))
    .bind(webhook_id)
    .bind(limit)
    .fetch_all(conn)
    .await?)
}

pub async fn delivery(conn: &mut PgConnection, id: i64) -> DbResult<Option<Delivery>> {
    Ok(sqlx::query_as(&format!(
        "SELECT {DELIVERY} FROM webhook_deliveries WHERE id = $1"
    ))
    .bind(id)
    .fetch_optional(conn)
    .await?)
}

/// What one attempt ended with.
pub struct Attempt<'a> {
    pub attempts: i32,
    pub response_status: Option<i32>,
    pub error: Option<&'a str>,
    pub duration_ms: i32,
    /// `pending` while it will be tried again, else `delivered` or `failed`.
    pub state: &'a str,
}

pub async fn record_attempt(conn: &mut PgConnection, id: i64, a: &Attempt<'_>) -> DbResult<()> {
    let error: Option<String> = a.error.map(|e| e.chars().take(500).collect());
    sqlx::query(
        "UPDATE webhook_deliveries
         SET attempts = $2, response_status = $3, error = $4, duration_ms = $5, state = $6,
             finished_at = CASE WHEN $6 = 'pending' THEN NULL ELSE now() END
         WHERE id = $1",
    )
    .bind(id)
    .bind(a.attempts)
    .bind(a.response_status)
    .bind(error)
    .bind(a.duration_ms)
    .bind(a.state)
    .execute(conn)
    .await?;
    Ok(())
}

/// A delivery went through: the webhook's failure count starts over.
pub async fn delivered(conn: &mut PgConnection, webhook_id: Uuid) -> DbResult<()> {
    sqlx::query(
        "UPDATE webhooks SET failed_deliveries = 0 WHERE id = $1 AND failed_deliveries > 0",
    )
    .bind(webhook_id)
    .execute(conn)
    .await?;
    Ok(())
}

/// A delivery failed for good: counts it, and turns the webhook off once `DISABLE_AFTER` in a row
/// failed. True when this call turned it off.
pub async fn failed_for_good(conn: &mut PgConnection, webhook_id: Uuid) -> DbResult<bool> {
    let turned_off: Option<bool> = sqlx::query_scalar(
        "UPDATE webhooks
         SET failed_deliveries = failed_deliveries + 1,
             enabled = enabled AND failed_deliveries + 1 < $2,
             disabled_reason = CASE WHEN enabled AND failed_deliveries + 1 >= $2 THEN 'failures'
                                    ELSE disabled_reason END,
             updated_at = now()
         WHERE id = $1
         RETURNING (disabled_reason = 'failures' AND NOT enabled AND failed_deliveries = $2)",
    )
    .bind(webhook_id)
    .bind(DISABLE_AFTER)
    .fetch_optional(conn)
    .await?;
    Ok(turned_off.unwrap_or(false))
}

/// Keeps the latest `KEEP_DELIVERIES` finished deliveries of a webhook.
pub async fn prune(conn: &mut PgConnection, webhook_id: Uuid) -> DbResult<u64> {
    let done = sqlx::query(
        "DELETE FROM webhook_deliveries
          WHERE webhook_id = $1 AND state <> 'pending'
            AND id < (SELECT id FROM webhook_deliveries WHERE webhook_id = $1
                       ORDER BY id DESC OFFSET $2 - 1 LIMIT 1)",
    )
    .bind(webhook_id)
    .bind(KEEP_DELIVERIES)
    .execute(conn)
    .await?;
    Ok(done.rows_affected())
}

/// Marks deliveries still pending after a day as failed (their job was lost). Run with the hourly
/// clean-up.
pub async fn fail_stale(conn: &mut PgConnection) -> DbResult<u64> {
    let done = sqlx::query(
        "UPDATE webhook_deliveries
            SET state = 'failed', error = coalesce(error, 'not delivered in time'), finished_at = now()
          WHERE state = 'pending' AND created_at < now() - interval '1 day'",
    )
    .execute(conn)
    .await?;
    Ok(done.rows_affected())
}
