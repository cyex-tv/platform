//! Live updates (ADR 0005): one fan-out used by both sockets, the events service's
//! `/v1/events` and the api's GraphQL `events` subscription.
//!
//! - [`feeder`] reads `events_outbox` when `NOTIFY cyex_tv_events` says something is new (and
//!   polls, in case a notification is lost) and publishes each event to the [`Hub`].
//! - The [`Hub`] knows which subscriber watches which topic and hands each event to every
//!   subscriber of any of its topics, once.
//! - A subscriber that can't keep up isn't waited for: its queue fills, it is marked lagged, and
//!   it gets a `resync` (refetch) instead of the events it missed.

pub mod feeder;
pub mod topics;

use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use chrono::SecondsFormat;
use cyex_tv_db::outbox::Event;
use cyex_tv_shared::model::{self, EventType};
use serde_json::{Value, json};
use tokio::sync::mpsc;

/// Events queued per subscriber before it counts as lagging.
pub const QUEUE: usize = 1024;

/// One event for one subscriber, with the topic it came by.
#[derive(Clone, Debug)]
pub struct Delivery {
    pub event: Arc<Event>,
    pub topic: String,
}

/// What a subscriber receives.
#[derive(Clone, Debug)]
pub enum Message {
    Event(Delivery),
    /// Events were dropped because the subscriber was slow: refetch what you show.
    Lagged,
}

struct Sub {
    topics: HashSet<String>,
    tx: mpsc::Sender<Delivery>,
    lagged: Arc<AtomicBool>,
}

#[derive(Default)]
struct Inner {
    next: u64,
    subs: HashMap<u64, Sub>,
    by_topic: HashMap<String, HashSet<u64>>,
}

/// The fan-out. Cheap to clone.
#[derive(Clone, Default)]
pub struct Hub {
    inner: Arc<Mutex<Inner>>,
}

fn lock(inner: &Mutex<Inner>) -> MutexGuard<'_, Inner> {
    // Nothing here panics while holding the lock; if something did, the map is still usable.
    inner.lock().unwrap_or_else(PoisonError::into_inner)
}

impl Hub {
    pub fn new() -> Self {
        Self::default()
    }

    /// A new subscriber with no topics yet.
    pub fn subscriber(&self) -> Subscription {
        let (tx, rx) = mpsc::channel(QUEUE);
        let lagged = Arc::new(AtomicBool::new(false));
        let mut inner = lock(&self.inner);
        inner.next += 1;
        let id = inner.next;
        inner.subs.insert(
            id,
            Sub {
                topics: HashSet::new(),
                tx,
                lagged: lagged.clone(),
            },
        );
        Subscription {
            id,
            hub: self.clone(),
            rx,
            lagged,
        }
    }

    /// Hands `event` to every subscriber of any of its topics, once each.
    pub fn publish(&self, event: Arc<Event>) {
        let inner = lock(&self.inner);
        let mut seen: HashSet<u64> = HashSet::new();
        for topic in &event.topics {
            let Some(ids) = inner.by_topic.get(topic) else {
                continue;
            };
            for id in ids {
                if !seen.insert(*id) {
                    continue;
                }
                let Some(sub) = inner.subs.get(id) else {
                    continue;
                };
                if sub
                    .tx
                    .try_send(Delivery {
                        event: event.clone(),
                        topic: topic.clone(),
                    })
                    .is_err()
                {
                    sub.lagged.store(true, Ordering::Relaxed);
                }
            }
        }
    }

    /// Subscribers and topics watched, for metrics and tests.
    pub fn stats(&self) -> (usize, usize) {
        let inner = lock(&self.inner);
        (inner.subs.len(), inner.by_topic.len())
    }
}

/// One subscriber: a connection, or one GraphQL subscription. Leaves the hub when dropped.
pub struct Subscription {
    id: u64,
    hub: Hub,
    rx: mpsc::Receiver<Delivery>,
    lagged: Arc<AtomicBool>,
}

impl Subscription {
    /// Adds topics. Returns how many this subscriber watches now.
    pub fn subscribe<'a>(&self, topics: impl IntoIterator<Item = &'a str>) -> usize {
        let mut inner = lock(&self.hub.inner);
        let new: Vec<String> = topics.into_iter().map(str::to_owned).collect();
        for topic in &new {
            inner
                .by_topic
                .entry(topic.clone())
                .or_default()
                .insert(self.id);
        }
        let sub = inner.subs.get_mut(&self.id).map(|s| {
            s.topics.extend(new);
            s.topics.len()
        });
        sub.unwrap_or(0)
    }

    pub fn unsubscribe<'a>(&self, topics: impl IntoIterator<Item = &'a str>) -> usize {
        let mut inner = lock(&self.hub.inner);
        for topic in topics {
            if let Some(ids) = inner.by_topic.get_mut(topic) {
                ids.remove(&self.id);
                if ids.is_empty() {
                    inner.by_topic.remove(topic);
                }
            }
            if let Some(sub) = inner.subs.get_mut(&self.id) {
                sub.topics.remove(topic);
            }
        }
        inner.subs.get(&self.id).map_or(0, |s| s.topics.len())
    }

    /// The topics watched, sorted.
    pub fn topics(&self) -> Vec<String> {
        let inner = lock(&self.hub.inner);
        let mut out: Vec<String> = inner
            .subs
            .get(&self.id)
            .map(|s| s.topics.iter().cloned().collect())
            .unwrap_or_default();
        out.sort();
        out
    }

    /// The next message. `None` only if the hub is gone.
    pub async fn recv(&mut self) -> Option<Message> {
        if self.lagged.swap(false, Ordering::Relaxed) {
            // Throw away what's queued: the client refetches anyway.
            while self.rx.try_recv().is_ok() {}
            return Some(Message::Lagged);
        }
        self.rx.recv().await.map(Message::Event)
    }
}

impl Drop for Subscription {
    fn drop(&mut self) {
        let mut inner = lock(&self.hub.inner);
        if let Some(sub) = inner.subs.remove(&self.id) {
            for topic in sub.topics {
                if let Some(ids) = inner.by_topic.get_mut(&topic) {
                    ids.remove(&self.id);
                    if ids.is_empty() {
                        inner.by_topic.remove(&topic);
                    }
                }
            }
        }
    }
}

/// The event type as stored (`set.emote_added`), back as the enum.
pub fn event_type(name: &str) -> Option<EventType> {
    serde_json::from_value(Value::String(name.to_owned())).ok()
}

fn text(payload: &Value, key: &str) -> Option<String> {
    payload.get(key).and_then(Value::as_str).map(str::to_owned)
}

/// The JSON the event WebSocket sends as a dispatch's `data`.
pub fn wire(event: &Event, topic: &str) -> Value {
    json!({
        "topic": topic,
        "set_id": event.set_id.map(cyex_tv_db::ids::to_text),
        "emote_id": text(&event.payload, "emote_id"),
        "name": text(&event.payload, "name"),
        "created_at": event.created_at.to_rfc3339_opts(SecondsFormat::Millis, true),
    })
}

/// The same event as the GraphQL `Event` type.
pub fn model_event(event: &Event, topic: &str) -> model::Event {
    model::Event {
        id: event.id.to_string(),
        event_type: event_type(&event.event_type).unwrap_or(EventType::Resync),
        topic: topic.to_owned(),
        set_id: event.set_id.map(cyex_tv_db::ids::to_text),
        emote_id: text(&event.payload, "emote_id"),
        name: text(&event.payload, "name"),
        created_at: event
            .created_at
            .to_rfc3339_opts(SecondsFormat::Millis, true),
    }
}

/// The `resync` event, for a subscriber that lagged or resumed from too far back.
pub fn resync(topic: &str) -> model::Event {
    model::Event {
        id: "0".to_owned(),
        event_type: EventType::Resync,
        topic: topic.to_owned(),
        set_id: None,
        emote_id: None,
        name: None,
        created_at: chrono::Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true),
    }
}

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use super::*;

    fn event(id: i64, topics: &[&str]) -> Arc<Event> {
        Arc::new(Event {
            id,
            event_type: "set.emote_added".to_owned(),
            topics: topics.iter().map(|t| (*t).to_owned()).collect(),
            set_id: None,
            payload: json!({ "emote_id": "E1", "name": "Pog" }),
            created_at: Utc::now(),
        })
    }

    async fn next(sub: &mut Subscription) -> Option<Message> {
        tokio::time::timeout(std::time::Duration::from_millis(50), sub.recv())
            .await
            .ok()
            .flatten()
    }

    #[tokio::test]
    async fn events_reach_their_subscribers_once() {
        let hub = Hub::new();
        let mut a = hub.subscriber();
        let mut b = hub.subscriber();
        assert_eq!(a.subscribe(["set:1", "channel:twitch:1"]), 2);
        b.subscribe(["set:2"]);

        // On two of `a`'s topics at once: delivered once.
        hub.publish(event(1, &["set:1", "channel:twitch:1"]));
        let Some(Message::Event(d)) = next(&mut a).await else {
            panic!("a got nothing")
        };
        assert_eq!((d.event.id, d.topic.as_str()), (1, "set:1"));
        assert!(next(&mut a).await.is_none());
        assert!(next(&mut b).await.is_none());

        hub.publish(event(2, &["set:2"]));
        assert!(matches!(next(&mut b).await, Some(Message::Event(_))));
        assert!(next(&mut a).await.is_none());
    }

    #[tokio::test]
    async fn unsubscribing_and_dropping_clean_up() {
        let hub = Hub::new();
        let mut a = hub.subscriber();
        a.subscribe(["set:1", "set:2"]);
        assert_eq!(a.unsubscribe(["set:1"]), 1);
        assert_eq!(a.topics(), ["set:2"]);
        hub.publish(event(1, &["set:1"]));
        assert!(next(&mut a).await.is_none());
        assert_eq!(hub.stats(), (1, 1));
        drop(a);
        assert_eq!(hub.stats(), (0, 0));
    }

    #[tokio::test]
    async fn a_slow_subscriber_gets_a_resync() {
        let hub = Hub::new();
        let mut slow = hub.subscriber();
        slow.subscribe(["set:1"]);
        for i in 0..(QUEUE as i64 + 10) {
            hub.publish(event(i + 1, &["set:1"]));
        }
        assert!(matches!(next(&mut slow).await, Some(Message::Lagged)));
        // The backlog was thrown away; new events flow again.
        assert!(next(&mut slow).await.is_none());
        hub.publish(event(99_999, &["set:1"]));
        assert!(matches!(next(&mut slow).await, Some(Message::Event(_))));
    }

    #[test]
    fn events_have_both_shapes() {
        let e = event(7, &["set:1"]);
        let w = wire(&e, "set:1");
        assert_eq!(w["emote_id"], "E1");
        assert_eq!(w["topic"], "set:1");
        let m = model_event(&e, "set:1");
        assert_eq!(
            (m.id.as_str(), m.event_type),
            ("7", EventType::SetEmoteAdded)
        );
        assert_eq!(event_type("nope"), None);
        assert_eq!(resync("set:1").event_type, EventType::Resync);
    }
}
