//! The events outbox (ADR 0005). A write that clients should hear about inserts its event here in
//! the same transaction. An insert trigger sends `NOTIFY cyex_tv_events, '<id>'`, which wakes the
//! events service and the api's subscription feed.
//!
//! Event IDs come from a sequence. A transaction that started earlier can commit a lower ID after
//! a higher one is visible, so the readers must allow for that gap; this module only
//! stores and reads.

use std::collections::{HashMap, HashSet};

use chrono::{DateTime, Utc};
use cyex_tv_shared::model::{EventType, Platform};
use serde_json::Value;
use sqlx::{FromRow, PgConnection};
use uuid::Uuid;

use crate::error::DbResult;
use crate::ids::to_text;
use crate::public_ids;

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

/// Adds an event and returns its ID. It is stored as clients get it (ADR 0027): `set:` topics
/// and the payload's account, emote and set IDs by their Snowflakes, and the set's Snowflake as
/// `payload.set_id`. The `set_id` column keeps the row's uuid for the read caches.
pub async fn publish(conn: &mut PgConnection, event: &NewEvent) -> DbResult<i64> {
    let (topics, payload) = public_form(&mut *conn, event).await?;
    Ok(sqlx::query_scalar(
        "INSERT INTO events_outbox (type, topics, set_id, payload)
         VALUES ($1, $2, $3, $4)
         RETURNING id",
    )
    .bind(type_name(event.event_type))
    .bind(&topics)
    .bind(event.set_id)
    .bind(&payload)
    .fetch_one(conn)
    .await?)
}

fn topic_set(topic: &str) -> Option<Uuid> {
    topic
        .strip_prefix("set:")
        .filter(|id| crate::ids::is_ulid(id))
        .and_then(crate::ids::from_text)
}

/// The topics and payload of `event` with Snowflakes in place of ULIDs.
async fn public_form(conn: &mut PgConnection, event: &NewEvent) -> DbResult<(Vec<String>, Value)> {
    let mut found = HashSet::new();
    public_ids::collect_json(&event.payload, &mut found);
    found.extend(event.topics.iter().filter_map(|t| topic_set(t)));
    found.extend(event.set_id);
    let found: Vec<Uuid> = found.into_iter().collect();
    let map = public_ids::publics(conn, &found).await?;
    let topics = event
        .topics
        .iter()
        .map(|topic| match topic_set(topic).and_then(|id| map.get(&id)) {
            Some(public) => format!("set:{public}"),
            None => topic.clone(),
        })
        .collect();
    let mut payload = event.payload.clone();
    public_ids::replace_json(&mut payload, &map);
    if let (Some(set), Value::Object(fields)) = (event.set_id, &mut payload) {
        fields.entry("set_id").or_insert_with(|| {
            Value::String(map.get(&set).map_or_else(|| to_text(set), i64::to_string))
        });
    }
    Ok((topics, payload))
}

/// A `set:` topic as stored and sent (by Snowflake), from either form a client may use. `None`
/// when the set is unknown; other topics come back as they are.
pub async fn public_topic(conn: &mut PgConnection, topic: &str) -> DbResult<Option<String>> {
    let Some(id) = topic.strip_prefix("set:") else {
        return Ok(Some(topic.to_owned()));
    };
    if public_ids::parse(id).is_some() {
        return Ok(Some(topic.to_owned()));
    }
    let Some(set) = topic_set(topic) else {
        return Ok(None);
    };
    Ok(public_ids::publics(conn, &[set])
        .await?
        .get(&set)
        .map(|public| format!("set:{public}")))
}

/// [`public_topic`] for many topics in one lookup: each topic to its stored form. A `set:` topic
/// naming an unknown set is left out; every other topic maps to itself.
pub async fn public_topics(
    conn: &mut PgConnection,
    topics: &[String],
) -> DbResult<HashMap<String, String>> {
    let legacy: Vec<Uuid> = topics
        .iter()
        .filter(|t| {
            t.strip_prefix("set:")
                .is_some_and(|id| public_ids::parse(id).is_none())
        })
        .filter_map(|t| topic_set(t))
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();
    let map = public_ids::publics(conn, &legacy).await?;
    Ok(topics
        .iter()
        .filter_map(|topic| {
            let Some(id) = topic.strip_prefix("set:") else {
                return Some((topic.clone(), topic.clone()));
            };
            if public_ids::parse(id).is_some() {
                return Some((topic.clone(), topic.clone()));
            }
            let public = map.get(&topic_set(topic)?)?;
            Some((topic.clone(), format!("set:{public}")))
        })
        .collect())
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
