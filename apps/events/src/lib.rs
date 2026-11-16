//! The event WebSocket at `/v1/events` (ADR 0005). Every frame is `{ "op": …, "d": { … } }`.
//!
//! - On connect the server sends `hello` (`session_id`, `heartbeat_interval`, `max_subscriptions`).
//! - `subscribe` / `unsubscribe` take `topics`; the server answers `ack` with the topics taken and
//!   an `errors` map for the ones refused.
//! - Events arrive as `dispatch` (`id`, `type`, `data`). A `resync` dispatch means: refetch.
//! - Both sides send `heartbeat` every 20 s. A client silent for 60 s is closed.
//! - `resume` (`session_id`, `last_event_id`) on a new connection takes over the old session's
//!   topics and replays what was missed from the outbox, within 24 h; anything older gets
//!   `resync`. Sessions live in Postgres, so this works across restarts and instances.
//! - Limits: 500 topics, 10 frames a second, 16 KB a frame. Breaking one ends the connection
//!   with `error`.
//! - On shutdown the server sends `reconnect` and closes.

use std::collections::{BTreeMap, HashSet};
use std::time::{Duration, Instant};

use axum::Router;
use axum::extract::State;
use axum::extract::ws::{CloseFrame, Message as WsMessage, WebSocket, WebSocketUpgrade};
use axum::response::Response;
use axum::routing::get;
use cyex_tv_db::{event_sessions, ids, outbox};
use cyex_tv_hub::topics::{self, MAX_TOPICS, TopicError};
use cyex_tv_hub::{Hub, Message, Subscription, wire};
use serde::Deserialize;
use serde_json::{Value, json};
use sqlx::PgPool;
use tokio::sync::watch;
use uuid::Uuid;

pub const HEARTBEAT: Duration = Duration::from_secs(20);
/// Three missed heartbeats.
pub const IDLE: Duration = Duration::from_secs(60);
pub const MAX_FRAME: usize = 16 * 1024;
pub const FRAMES_PER_SECOND: f64 = 10.0;
/// Resume replays at most this much; a longer gap gets `resync`.
pub const REPLAY_LIMIT: i64 = 5000;
pub const REPLAY_WINDOW: chrono::Duration = chrono::Duration::hours(24);
/// How often an open connection marks its session as in use.
pub const TOUCH_EVERY: Duration = Duration::from_secs(600);

#[derive(Clone)]
pub struct EventsState {
    pub db: PgPool,
    pub hub: Hub,
    /// Flips to `true` on shutdown: every connection sends `reconnect`.
    pub stopping: watch::Receiver<bool>,
}

pub fn router(state: EventsState) -> Router {
    Router::new()
        .route("/v1/events", get(upgrade))
        .with_state(state)
}

async fn upgrade(State(state): State<EventsState>, ws: WebSocketUpgrade) -> Response {
    ws.max_message_size(MAX_FRAME)
        .max_frame_size(MAX_FRAME)
        .on_upgrade(move |socket| async move {
            Connection::new(state, socket).run().await;
        })
}

#[derive(Deserialize)]
struct Frame {
    op: String,
    #[serde(default)]
    d: Value,
}

#[derive(Deserialize)]
struct TopicsBody {
    topics: Vec<String>,
}

#[derive(Deserialize)]
struct ResumeBody {
    session_id: String,
    last_event_id: Value,
}

fn frame(op: &str, d: Value) -> WsMessage {
    WsMessage::Text(json!({ "op": op, "d": d }).to_string().into())
}

fn dispatch(id: &str, event_type: &str, data: Value) -> WsMessage {
    frame(
        "dispatch",
        json!({ "id": id, "type": event_type, "data": data }),
    )
}

fn resync() -> WsMessage {
    dispatch(
        "0",
        "resync",
        json!({ "created_at": chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true) }),
    )
}

/// A token bucket: `FRAMES_PER_SECOND` on average, bursts up to the same.
struct Bucket {
    tokens: f64,
    at: Instant,
}

impl Bucket {
    fn new() -> Self {
        Self {
            tokens: FRAMES_PER_SECOND,
            at: Instant::now(),
        }
    }

    fn take(&mut self) -> bool {
        let now = Instant::now();
        let refill = now.duration_since(self.at).as_secs_f64() * FRAMES_PER_SECOND;
        self.tokens = (self.tokens + refill).min(FRAMES_PER_SECOND);
        self.at = now;
        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            true
        } else {
            false
        }
    }
}

/// Why a connection ends.
enum End {
    /// The client closed or the socket broke.
    Gone,
    Error(&'static str, &'static str),
    Reconnect,
}

struct Connection {
    state: EventsState,
    socket: WebSocket,
    session: Uuid,
    sub: Subscription,
    /// Event IDs sent by a resume replay, so the live queue doesn't repeat them.
    replayed: HashSet<i64>,
    last_heard: Instant,
    last_touch: Instant,
    bucket: Bucket,
}

impl Connection {
    fn new(state: EventsState, socket: WebSocket) -> Self {
        let sub = state.hub.subscriber();
        Self {
            session: ids::new_id(),
            sub,
            state,
            socket,
            replayed: HashSet::new(),
            last_heard: Instant::now(),
            last_touch: Instant::now(),
            bucket: Bucket::new(),
        }
    }

    async fn send(&mut self, msg: WsMessage) -> Result<(), End> {
        self.socket.send(msg).await.map_err(|_| End::Gone)
    }

    async fn run(mut self) {
        let hello = frame(
            "hello",
            json!({
                "session_id": ids::to_text(self.session),
                "heartbeat_interval": HEARTBEAT.as_millis() as u64,
                "max_subscriptions": MAX_TOPICS,
            }),
        );
        if self.send(hello).await.is_err() {
            return;
        }
        // `serve` only ever returns through `Err(End)`: an endless loop until something ends it.
        let end = self.serve().await.err().unwrap_or(End::Gone);
        let close = match end {
            End::Gone => return,
            End::Error(code, message) => {
                let _ = self
                    .send(frame("error", json!({ "code": code, "message": message })))
                    .await;
                Some(CloseFrame {
                    code: 4000,
                    reason: code.into(),
                })
            }
            End::Reconnect => {
                let _ = self.send(frame("reconnect", json!({}))).await;
                Some(CloseFrame {
                    code: 1012,
                    reason: "restarting".into(),
                })
            }
        };
        let _ = self.socket.send(WsMessage::Close(close)).await;
    }

    async fn serve(&mut self) -> Result<(), End> {
        let mut beat = tokio::time::interval(HEARTBEAT);
        beat.tick().await;
        let mut stopping = self.state.stopping.clone();
        loop {
            tokio::select! {
                incoming = self.socket.recv() => {
                    match incoming {
                        Some(Ok(WsMessage::Text(text))) => self.on_frame(text.as_str()).await?,
                        Some(Ok(WsMessage::Binary(_))) => {
                            return Err(End::Error("bad_frame", "Frames are JSON text."));
                        }
                        Some(Ok(WsMessage::Ping(_) | WsMessage::Pong(_))) => {
                            self.last_heard = Instant::now();
                        }
                        Some(Ok(WsMessage::Close(_))) | None => return Err(End::Gone),
                        Some(Err(_)) => {
                            return Err(End::Error("frame_too_large", "Frames are at most 16 KB."));
                        }
                    }
                }
                message = self.sub.recv() => {
                    match message {
                        Some(Message::Event(d)) => {
                            if self.replayed.remove(&d.event.id) {
                                continue;
                            }
                            let msg = dispatch(&d.event.id.to_string(), &d.event.event_type, wire(&d.event, &d.topic));
                            self.send(msg).await?;
                        }
                        Some(Message::Lagged) => self.send(resync()).await?,
                        None => return Err(End::Reconnect),
                    }
                }
                _ = beat.tick() => {
                    if self.last_heard.elapsed() > IDLE {
                        return Err(End::Error("timeout", "No heartbeat for 60 s."));
                    }
                    self.send(frame("heartbeat", json!({}))).await?;
                    if self.last_touch.elapsed() > TOUCH_EVERY {
                        self.last_touch = Instant::now();
                        if let Ok(mut conn) = self.state.db.acquire().await {
                            let _ = event_sessions::touch(&mut conn, self.session).await;
                        }
                    }
                }
                changed = stopping.changed() => {
                    if changed.is_err() || *stopping.borrow() {
                        return Err(End::Reconnect);
                    }
                }
            }
        }
    }

    async fn on_frame(&mut self, text: &str) -> Result<(), End> {
        self.last_heard = Instant::now();
        if text.len() > MAX_FRAME {
            return Err(End::Error("frame_too_large", "Frames are at most 16 KB."));
        }
        if !self.bucket.take() {
            return Err(End::Error("rate_limited", "At most 10 frames a second."));
        }
        let Ok(frame_in) = serde_json::from_str::<Frame>(text) else {
            return Err(End::Error(
                "bad_frame",
                "Frames are {\"op\": …, \"d\": {…}}.",
            ));
        };
        match frame_in.op.as_str() {
            "heartbeat" => Ok(()),
            "subscribe" | "unsubscribe" => {
                let Ok(body) = serde_json::from_value::<TopicsBody>(frame_in.d) else {
                    return Err(End::Error(
                        "bad_frame",
                        "`d.topics` must be a list of strings.",
                    ));
                };
                self.change(&frame_in.op, body.topics).await
            }
            "resume" => {
                let Ok(body) = serde_json::from_value::<ResumeBody>(frame_in.d) else {
                    return Err(End::Error(
                        "bad_frame",
                        "`resume` takes `session_id` and `last_event_id`.",
                    ));
                };
                self.resume(body).await
            }
            _ => Err(End::Error("unknown_op", "Unknown op.")),
        }
    }

    async fn save(&mut self) {
        let topics = self.sub.topics();
        if let Ok(mut conn) = self.state.db.acquire().await
            && let Err(err) = event_sessions::save(&mut conn, self.session, &topics).await
        {
            tracing::warn!(error = %err, "saving an event session failed");
        }
        self.last_touch = Instant::now();
    }

    async fn change(&mut self, op: &str, requested: Vec<String>) -> Result<(), End> {
        let mut errors: BTreeMap<String, &'static str> = BTreeMap::new();
        let mut taken: Vec<String> = Vec::new();
        if op == "subscribe" {
            let current: HashSet<String> = self.sub.topics().into_iter().collect();
            let mut room = MAX_TOPICS.saturating_sub(current.len());
            for topic in requested {
                if !topics::is_valid(&topic) {
                    errors.insert(topic, TopicError::Invalid.code());
                } else if current.contains(&topic) || taken.contains(&topic) {
                    taken.push(topic);
                } else if room == 0 {
                    errors.insert(topic, TopicError::TooMany.code());
                } else {
                    room -= 1;
                    taken.push(topic);
                }
            }
            taken.dedup();
            self.sub.subscribe(taken.iter().map(String::as_str));
        } else {
            self.sub.unsubscribe(requested.iter().map(String::as_str));
            taken = requested;
        }
        self.save().await;
        self.send(frame(
            "ack",
            json!({ "op": op, "topics": taken, "errors": errors }),
        ))
        .await
    }

    async fn resume(&mut self, body: ResumeBody) -> Result<(), End> {
        let last = match &body.last_event_id {
            Value::Number(n) => n.as_i64(),
            Value::String(s) => s.parse().ok(),
            _ => None,
        };
        let (Some(old), Some(last)) = (ids::from_text(&body.session_id), last) else {
            return Err(End::Error(
                "bad_frame",
                "Bad `session_id` or `last_event_id`.",
            ));
        };
        let Ok(mut conn) = self.state.db.acquire().await else {
            return self.send(resync()).await;
        };
        let topics = match event_sessions::topics(&mut conn, old, REPLAY_WINDOW).await {
            Ok(Some(topics)) => topics,
            _ => {
                drop(conn);
                return self
                    .send(frame(
                        "ack",
                        json!({ "op": "resume", "error": "unknown_session" }),
                    ))
                    .await;
            }
        };
        // Take over the old session; live events start queueing now, before the replay reads.
        self.session = old;
        self.sub.subscribe(topics.iter().map(String::as_str));
        let oldest = outbox::oldest_id(&mut conn).await.ok().flatten();
        let missed = outbox::since(&mut conn, last, &topics, REPLAY_LIMIT).await;
        drop(conn);
        self.save().await;
        self.send(frame(
            "ack",
            json!({ "op": "resume", "session_id": ids::to_text(old), "topics": topics }),
        ))
        .await?;

        let window_start = chrono::Utc::now() - REPLAY_WINDOW;
        let too_old = oldest.is_some_and(|o| o > last + 1);
        let events = match missed {
            Ok(events) if !too_old => events,
            _ => return self.send(resync()).await,
        };
        if i64::try_from(events.len()).unwrap_or(i64::MAX) >= REPLAY_LIMIT
            || events.first().is_some_and(|e| e.created_at < window_start)
        {
            return self.send(resync()).await;
        }
        let wanted: HashSet<&String> = topics.iter().collect();
        for event in events {
            let topic = event
                .topics
                .iter()
                .find(|t| wanted.contains(t))
                .cloned()
                .unwrap_or_default();
            self.replayed.insert(event.id);
            let msg = dispatch(
                &event.id.to_string(),
                &event.event_type,
                wire(&event, &topic),
            );
            self.send(msg).await?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_bucket_allows_ten_a_second() {
        let mut b = Bucket::new();
        for _ in 0..10 {
            assert!(b.take());
        }
        assert!(!b.take());
    }
}
