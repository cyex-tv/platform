//! The events outbox (ADR 0005). A write that clients should hear about inserts its event here in
//! the same transaction. An insert trigger sends `NOTIFY cyex_tv_events, '<id>'`, which wakes the
//! events service and the api's subscription feed (Phase 5).
//!
//! Event IDs come from a sequence. A transaction that started earlier can commit a lower ID after
//! a higher one is visible, so the Phase 5 readers must allow for that gap; this module only
//! stores and reads.

use chrono::{DateTime, Utc};
use cyex_tv_shared::model::{EventType, Platform};
use serde_json::Value;
use sqlx::{FromRow, PgConnection};
use uuid::Uuid;

use crate::error::DbResult;
use crate::ids::to_text;

/// The `LISTEN` channel the insert trigger notifies.
pub const NOTIFY_CHANNEL: &str = "cyex_tv_events";

#[derive(Clone, Debug, PartialEq, FromRow)]
pub struct Event {
    pub id: i64,
    /// The dotted name, for example `set.emote_added`.
    #[sqlx(rename = "type")]
    pub event_type: String,
    pub topics: Vec<String>,
    pub set_id: Option<Uuid>,
    pub payload: Value,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Debug)]
pub struct NewEvent {
    pub event_type: EventType,
    /// At least one. Build them with the `*_topic` helpers.
    pub topics: Vec<String>,
    pub set_id: Option<Uuid>,
    pub payload: Value,
}

pub fn channel_topic(platform: Platform, platform_channel_id: &str) -> String {
    format!("channel:{platform}:{platform_channel_id}")
}

pub fn set_topic(set_id: Uuid) -> String {
    format!("set:{}", to_text(set_id))
}

/// The cosmetics catalog's topic (ADR 0015): badge and paint changes.
pub const COSMETICS_TOPIC: &str = "cosmetics";

pub fn user_topic(platform: Platform, platform_user_id: &str) -> String {
    format!("user:{platform}:{platform_user_id}")
}

/// The topic of internal events: no client can watch it (it isn't a valid topic name), so they
/// only reach the feeders' taps on every api instance.
pub const INTERNAL_TOPIC: &str = "internal:cache";

/// The internal event that tells every api instance to drop cached reads: `payload.keys` names
/// them (read-cache keys, such as a set's topic, or `announcement`).
pub const CACHE_FORGET: &str = "cache.forget";

/// Publishes [`CACHE_FORGET`] for `keys`, in the caller's transaction, so every instance drops
/// them once the change commits. Never sent to clients.
pub async fn publish_cache_forget(conn: &mut PgConnection, keys: &[String]) -> DbResult<i64> {
    Ok(sqlx::query_scalar(
        "INSERT INTO events_outbox (type, topics, payload)
         VALUES ($1, $2, $3)
         RETURNING id",
    )
    .bind(CACHE_FORGET)
    .bind(vec![INTERNAL_TOPIC.to_owned()])
    .bind(serde_json::json!({ "keys": keys }))
    .fetch_one(conn)
    .await?)
}

/// The keys a [`CACHE_FORGET`] event names.
pub fn forget_keys(event: &Event) -> Vec<String> {
    event
        .payload
        .get("keys")
        .and_then(Value::as_array)
        .map(|keys| {
            keys.iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

/// The dotted name an event type is stored and sent as.
pub fn type_name(event_type: EventType) -> String {
    match serde_json::to_value(event_type) {
        Ok(Value::String(name)) => name,
        // EventType is a plain enum with renamed variants; it always serializes to a string.
        _ => unreachable!("event types serialize to strings"),
    }
}

/// Adds an event and returns its ID.
pub async fn publish(conn: &mut PgConnection, event: &NewEvent) -> DbResult<i64> {
    Ok(sqlx::query_scalar(
        "INSERT INTO events_outbox (type, topics, set_id, payload)
         VALUES ($1, $2, $3, $4)
         RETURNING id",
    )
    .bind(type_name(event.event_type))
    .bind(&event.topics)
    .bind(event.set_id)
    .bind(&event.payload)
    .fetch_one(conn)
    .await?)
}

/// Events after `after_id` for any of `topics`, oldest first. Used for resume.
pub async fn since(
    conn: &mut PgConnection,
    after_id: i64,
    topics: &[String],
    limit: i64,
) -> DbResult<Vec<Event>> {
    Ok(sqlx::query_as(
        "SELECT id, type, topics, set_id, payload, created_at
         FROM events_outbox
         WHERE id > $1 AND topics && $2
         ORDER BY id
         LIMIT $3",
    )
    .bind(after_id)
    .bind(topics)
    .bind(limit)
    .fetch_all(conn)
    .await?)
}

pub async fn get(conn: &mut PgConnection, id: i64) -> DbResult<Option<Event>> {
    Ok(sqlx::query_as(
        "SELECT id, type, topics, set_id, payload, created_at FROM events_outbox WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(conn)
    .await?)
}

/// The lowest event ID still kept, so a resume can tell whether it missed too much.
pub async fn oldest_id(conn: &mut PgConnection) -> DbResult<Option<i64>> {
    Ok(sqlx::query_scalar("SELECT min(id) FROM events_outbox")
        .fetch_one(conn)
        .await?)
}

/// Rows `delete_older_than` removes per statement.
pub const DELETE_BATCH: i64 = 10_000;

/// Removes events older than `cutoff` (48 h, run on a schedule). In batches of
/// [`DELETE_BATCH`], each its own statement, so a busy day's events never make one long delete
/// that holds its locks and its WAL until the end.
pub async fn delete_older_than(conn: &mut PgConnection, cutoff: DateTime<Utc>) -> DbResult<u64> {
    let mut removed = 0;
    loop {
        let done = sqlx::query(
            "DELETE FROM events_outbox WHERE id IN (
                 SELECT id FROM events_outbox WHERE created_at < $1 ORDER BY created_at LIMIT $2)",
        )
        .bind(cutoff)
        .bind(DELETE_BATCH)
        .execute(&mut *conn)
        .await?;
        removed += done.rows_affected();
        if done.rows_affected() < DELETE_BATCH as u64 {
            return Ok(removed);
        }
    }
}

/// Every event after `after_id`, oldest first, whatever its topics. The live feeder reads this.
pub async fn after(conn: &mut PgConnection, after_id: i64, limit: i64) -> DbResult<Vec<Event>> {
    Ok(sqlx::query_as(
        "SELECT id, type, topics, set_id, payload, created_at
         FROM events_outbox
         WHERE id > $1
         ORDER BY id
         LIMIT $2",
    )
    .bind(after_id)
    .bind(limit)
    .fetch_all(conn)
    .await?)
}

/// The newest event ID, or 0 when there is none. Where a fresh feeder starts.
pub async fn latest_id(conn: &mut PgConnection) -> DbResult<i64> {
    Ok(
        sqlx::query_scalar("SELECT coalesce(max(id), 0) FROM events_outbox")
            .fetch_one(conn)
            .await?,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn topics_and_type_names() {
        assert_eq!(channel_topic(Platform::Twitch, "123"), "channel:twitch:123");
        assert_eq!(user_topic(Platform::Kick, "9"), "user:kick:9");
        let id = crate::ids::new_id();
        assert_eq!(set_topic(id), format!("set:{}", to_text(id)));
        assert_eq!(type_name(EventType::SetEmoteAdded), "set.emote_added");
        assert_eq!(type_name(EventType::Resync), "resync");
    }
}
