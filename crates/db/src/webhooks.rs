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
/// The other way to be turned off: at least this many failed in the 24-hour window...
pub const WINDOW_MIN_FAILED: i32 = 20;
/// ...and more than this percentage of the window's deliveries failed.
pub const WINDOW_FAILED_PERCENT: i32 = 80;
/// Live webhooks, across all channels, that may ask for the cosmetics catalog's events (each of
/// those events is sent to all of them).
pub const MAX_COSMETIC_SUBSCRIBERS: i64 = 1000;
/// Live webhooks one account's channel may have asking for those events, so a few accounts
/// can't fill [`MAX_COSMETIC_SUBSCRIBERS`] between them.
pub const MAX_COSMETIC_PER_ACCOUNT: i64 = 2;
/// The cosmetics catalog's event types.
pub const COSMETIC_EVENTS: [&str; 3] = ["cosmetic.created", "cosmetic.updated", "cosmetic.deleted"];

#[derive(Clone, Debug, PartialEq, Eq, FromRow)]
pub struct Webhook {
    pub id: Uuid,
    pub channel_id: Uuid,
    pub topic: String,
    pub url: String,
    pub event_types: Vec<String>,
    pub secret: String,
    /// The secret before the last rotation, still signing until `previous_secret_until`.
    pub previous_secret: Option<String>,
    pub previous_secret_until: Option<DateTime<Utc>>,
    pub enabled: bool,
    pub disabled_reason: Option<String>,
    pub failed_deliveries: i32,
    pub window_started_at: DateTime<Utc>,
    pub window_finished: i32,
    pub window_failed: i32,
    pub disabled_notice_at: Option<DateTime<Utc>>,
    pub created_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

const WEBHOOK: &str = "id, channel_id, topic, url, event_types, secret, previous_secret, \
                       previous_secret_until, enabled, disabled_reason, failed_deliveries, \
                       window_started_at, window_finished, window_failed, disabled_notice_at, \
                       created_by, created_at, updated_at";

impl Webhook {
    /// The secrets that sign a delivery now: the current one, and the one before a rotation for
    /// its 24 hours.
    pub fn signing_secrets(&self, now: DateTime<Utc>) -> Vec<&str> {
        let mut out = vec![self.secret.as_str()];
        if let (Some(prev), Some(until)) = (&self.previous_secret, self.previous_secret_until)
            && until > now
        {
            out.push(prev.as_str());
        }
        out
    }

    /// Whether `event_types` asks for any of the cosmetics catalog's events.
    pub fn wants_cosmetics(event_types: &[String]) -> bool {
        event_types
            .iter()
            .any(|t| COSMETIC_EVENTS.contains(&t.as_str()))
    }
}

/// Live webhooks that ask for the cosmetics catalog's events, `except` one (being changed).
pub async fn cosmetic_subscribers(conn: &mut PgConnection, except: Option<Uuid>) -> DbResult<i64> {
    Ok(sqlx::query_scalar(
        "SELECT count(*) FROM webhooks
          WHERE enabled AND event_types && $1 AND ($2::uuid IS NULL OR id <> $2)",
    )
    .bind(&COSMETIC_EVENTS[..])
    .bind(except)
    .fetch_one(conn)
    .await?)
}

/// Live webhooks on `account_id`'s channel that ask for the cosmetics catalog's events, `except`
/// one (being changed).
pub async fn cosmetic_subscribers_of(
    conn: &mut PgConnection,
    account_id: Uuid,
    except: Option<Uuid>,
) -> DbResult<i64> {
    Ok(sqlx::query_scalar(
        "SELECT count(*) FROM webhooks w JOIN channels c ON c.id = w.channel_id
          WHERE c.account_id = $1 AND w.enabled AND w.event_types && $2
            AND ($3::uuid IS NULL OR w.id <> $3)",
    )
    .bind(account_id)
    .bind(&COSMETIC_EVENTS[..])
    .bind(except)
    .fetch_one(conn)
    .await?)
}

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
             window_started_at = CASE WHEN $4 IS TRUE THEN now() ELSE window_started_at END,
             window_finished = CASE WHEN $4 IS TRUE THEN 0 ELSE window_finished END,
             window_failed = CASE WHEN $4 IS TRUE THEN 0 ELSE window_failed END,
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
        "UPDATE webhooks
         SET previous_secret = secret, previous_secret_until = now() + interval '24 hours',
             secret = $2, updated_at = now()
         WHERE id = $1 RETURNING {WEBHOOK}"
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

/// Turns off the live webhooks of every channel of the account (a staff block). Returns how many.
pub async fn disable_for_account(conn: &mut PgConnection, account_id: Uuid) -> DbResult<u64> {
    let done = sqlx::query(
        "UPDATE webhooks SET enabled = false, disabled_reason = 'blocked', updated_at = now()
          WHERE enabled AND channel_id IN (SELECT id FROM channels WHERE account_id = $1)",
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

/// What a finished delivery did to its webhook.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Outcome {
    /// This delivery turned the webhook off.
    pub turned_off: bool,
    /// Tell the channel's owner: turned off, and nobody was told in the last 24 hours.
    pub notify: bool,
}

/// Counts a delivery that finished (`ok`, or failed after every attempt) against its webhook:
/// the failures in a row, and the 24-hour window (started over once it is a day old). Turns the
/// webhook off at `DISABLE_AFTER` failures in a row, or at `WINDOW_MIN_FAILED` failures making
/// more than `WINDOW_FAILED_PERCENT`% of the window, so an occasional success can't keep a
/// receiver that mostly fails alive. Tells at most once a day (`disabled_notice_at`).
pub async fn record_outcome(
    conn: &mut PgConnection,
    webhook_id: Uuid,
    ok: bool,
) -> DbResult<Outcome> {
    let row: Option<(bool, bool)> = sqlx::query_as(
        "WITH w AS (
             SELECT id, enabled, disabled_notice_at,
                    CASE WHEN $2 THEN 0 ELSE failed_deliveries + 1 END AS in_row,
                    window_started_at < now() - interval '24 hours' AS stale,
                    window_finished, window_failed
               FROM webhooks WHERE id = $1 FOR UPDATE
         ), n AS (
             SELECT id, enabled, disabled_notice_at, in_row, stale,
                    CASE WHEN stale THEN 1 ELSE window_finished + 1 END AS finished,
                    CASE WHEN stale THEN 0 ELSE window_failed END
                        + CASE WHEN $2 THEN 0 ELSE 1 END AS failed
               FROM w
         ), d AS (
             SELECT n.*,
                    n.enabled AND NOT $2
                    AND (n.in_row >= $3 OR (n.failed >= $4 AND n.failed * 100 > n.finished * $5)) AS off
               FROM n
         ), e AS (
             SELECT d.*,
                    d.off AND (d.disabled_notice_at IS NULL
                               OR d.disabled_notice_at < now() - interval '24 hours') AS tell
               FROM d
         )
         UPDATE webhooks h
            SET failed_deliveries = e.in_row,
                window_started_at = CASE WHEN e.stale THEN now() ELSE h.window_started_at END,
                window_finished = e.finished,
                window_failed = e.failed,
                enabled = h.enabled AND NOT e.off,
                disabled_reason = CASE WHEN e.off THEN 'failures' ELSE h.disabled_reason END,
                disabled_notice_at = CASE WHEN e.tell THEN now() ELSE h.disabled_notice_at END,
                updated_at = CASE WHEN e.off THEN now() ELSE h.updated_at END
           FROM e
          WHERE h.id = e.id
         RETURNING e.off, e.tell",
    )
    .bind(webhook_id)
    .bind(ok)
    .bind(DISABLE_AFTER)
    .bind(WINDOW_MIN_FAILED)
    .bind(WINDOW_FAILED_PERCENT)
    .fetch_optional(conn)
    .await?;
    Ok(
        row.map_or_else(Outcome::default, |(turned_off, notify)| Outcome {
            turned_off,
            notify,
        }),
    )
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
