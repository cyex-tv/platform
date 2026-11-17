//! The Phase 5 load test for the GraphQL socket (`/v1/gql/ws`, `graphql-transport-ws`): many
//! subscriptions to `events` with ten topics each, events written to the outbox at a steady rate,
//! and the time from each write to each delivery. The event WebSocket has its own
//! (`apps/events/tests/load.rs`); this one is the same run against the API's socket. It runs in
//! CI's `load` job (`cargo test --release -p cyex-tv-api --test gql_load -- --ignored --nocapture`)
//! and prints its numbers; PROGRESS.md records them.
//!
//! - `CYEX_TV_LOAD_CONNECTIONS` (default 2000) and `CYEX_TV_LOAD_EVENTS` (default 300).
//! - Client and server share one machine and one process, so the numbers include both.

mod common;

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use common::TestDb;
use cyex_tv_api::limits::{Limits, Rates};
use cyex_tv_api::state::AppState;
use cyex_tv_auth::{Auth, AuthConfig};
use cyex_tv_db::outbox::{self, NewEvent};
use cyex_tv_shared::model::EventType;
use cyex_tv_shared::ratelimit::Rate;
use futures_util::{SinkExt, StreamExt};
use serde_json::{Value, json};
use tokio::sync::{Mutex, mpsc, watch};
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;

const TOPICS_PER_CONNECTION: usize = 10;
const TOPIC_POOL: usize = 1000;
const EVENTS_PER_SECOND: u64 = 30;
const BATCH: usize = 250;

fn env(name: &str, default: usize) -> usize {
    std::env::var(name)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

/// Connection `i`'s topics: ten distinct ones, spread over the pool.
fn topics_of(i: usize) -> Vec<usize> {
    (0..TOPICS_PER_CONNECTION)
        .map(|k| (i * 7919 + k * 104_729) % TOPIC_POOL)
        .collect()
}

fn topic(n: usize) -> String {
    format!("channel:twitch:{n}")
}

fn percentile(sorted: &[Duration], p: f64) -> Duration {
    if sorted.is_empty() {
        return Duration::ZERO;
    }
    let rank = ((sorted.len() as f64 - 1.0) * p).round() as usize;
    sorted[rank.min(sorted.len() - 1)]
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "load test: run explicitly (CI's load job)"]
async fn many_subscriptions_get_their_events_fast() {
    let Some(db) = TestDb::new().await else {
        return;
    };
    let connections = env("CYEX_TV_LOAD_CONNECTIONS", 2000);
    let events = env("CYEX_TV_LOAD_EVENTS", 300);

    // The server: the API with its hub, the feeder, the socket. Every client comes from
    // 127.0.0.1, so the per-IP socket limit is lifted here.
    let auth = Arc::new(Auth::new(AuthConfig::from_source(|_| None).unwrap()).unwrap());
    let state = AppState::new(db.pool.clone(), auth).with_limits(Limits::new(
        Rates {
            socket: Rate::per_minute(1_000_000),
            ..Rates::default()
        },
        false,
    ));
    let hub = state.hub.clone();
    let (stop, stopping) = watch::channel(false);
    let mut feeder_stop = stopping.clone();
    let feeder = tokio::spawn(cyex_tv_hub::feeder::run(
        db.pool.clone(),
        hub.clone(),
        async move {
            let _ = feeder_stop.wait_for(|s| *s).await;
        },
    ));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let app = cyex_tv_api::app(state);
    let mut serve_stop = stopping.clone();
    tokio::spawn(async move {
        axum::serve(listener, app)
            .with_graceful_shutdown(async move {
                let _ = serve_stop.wait_for(|s| *s).await;
            })
            .await
            .unwrap();
    });
    tokio::time::sleep(Duration::from_millis(300)).await;

    // The clients.
    let (got_tx, mut got_rx) = mpsc::unbounded_channel::<(i64, Instant)>();
    let ready = Arc::new(AtomicUsize::new(0));
    let failed = Arc::new(AtomicUsize::new(0));
    let connect_start = Instant::now();
    for batch in 0..connections.div_ceil(BATCH) {
        let end = ((batch + 1) * BATCH).min(connections);
        for i in (batch * BATCH)..end {
            let got = got_tx.clone();
            let ready = ready.clone();
            let failed = failed.clone();
            tokio::spawn(async move {
                let mut request = format!("ws://{addr}/v1/gql/ws")
                    .into_client_request()
                    .unwrap();
                request.headers_mut().insert(
                    "sec-websocket-protocol",
                    "graphql-transport-ws".parse().unwrap(),
                );
                let Ok((ws, _)) = tokio_tungstenite::connect_async(request).await else {
                    failed.fetch_add(1, Ordering::Relaxed);
                    return;
                };
                let (mut tx, mut rx) = ws.split();
                let init = json!({ "type": "connection_init", "payload": {} }).to_string();
                if tx.send(Message::Text(init.into())).await.is_err() {
                    failed.fetch_add(1, Ordering::Relaxed);
                    return;
                }
                let _ack = rx.next().await;
                let wanted: Vec<String> = topics_of(i).into_iter().map(topic).collect();
                let sub = json!({
                    "id": "1",
                    "type": "subscribe",
                    "payload": {
                        "query": "subscription($t: [String!]!) { events(topics: $t) { id } }",
                        "variables": { "t": wanted }
                    }
                })
                .to_string();
                if tx.send(Message::Text(sub.into())).await.is_err() {
                    failed.fetch_add(1, Ordering::Relaxed);
                    return;
                }
                // The subscription has no acknowledgement in this protocol; give the server a
                // moment to register it before counting the client as ready.
                tokio::time::sleep(Duration::from_millis(50)).await;
                ready.fetch_add(1, Ordering::Relaxed);
                while let Some(msg) = rx.next().await {
                    let Ok(Message::Text(text)) = msg else {
                        continue;
                    };
                    let now = Instant::now();
                    let Ok(v) = serde_json::from_str::<Value>(&text) else {
                        continue;
                    };
                    match v["type"].as_str() {
                        Some("next") => {
                            if let Some(id) = v["payload"]["data"]["events"]["id"]
                                .as_str()
                                .and_then(|s| s.parse().ok())
                            {
                                let _ = got.send((id, now));
                            }
                        }
                        Some("ping") => {
                            let pong = json!({ "type": "pong" }).to_string();
                            if tx.send(Message::Text(pong.into())).await.is_err() {
                                return;
                            }
                        }
                        _ => {}
                    }
                }
            });
        }
        // Each batch connects before the next starts, like clients arriving over time.
        let _ = tokio::time::timeout(Duration::from_secs(30), async {
            while ready.load(Ordering::Relaxed) + failed.load(Ordering::Relaxed) < end {
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        })
        .await;
    }
    drop(got_tx);
    let connected_in = connect_start.elapsed();
    let ready_count = ready.load(Ordering::Relaxed);
    eprintln!(
        "gql load: {ready_count} of {connections} subscriptions ({} failed) in {connected_in:.1?}; hub {:?}",
        failed.load(Ordering::Relaxed),
        hub.stats()
    );
    assert_eq!(ready_count, connections, "every connection subscribes");
    // Let the last subscriptions reach the hub.
    tokio::time::sleep(Duration::from_millis(500)).await;

    // How many deliveries each topic should produce.
    let mut fanout: HashMap<usize, usize> = HashMap::new();
    for i in 0..connections {
        for t in topics_of(i) {
            *fanout.entry(t).or_default() += 1;
        }
    }

    // Write the events at a steady rate.
    let written: Arc<Mutex<HashMap<i64, Instant>>> = Arc::default();
    let mut expected = 0usize;
    let gap = Duration::from_micros(1_000_000 / EVENTS_PER_SECOND);
    let write_start = Instant::now();
    for n in 0..events {
        let t = (n * 37) % TOPIC_POOL;
        expected += fanout.get(&t).copied().unwrap_or(0);
        let mut conn = db.conn().await;
        let before = Instant::now();
        let id = outbox::publish(
            &mut conn,
            &NewEvent {
                event_type: EventType::SetEmoteAdded,
                topics: vec![topic(t)],
                set_id: None,
                payload: json!({ "name": format!("e{n}") }),
            },
        )
        .await
        .unwrap();
        written.lock().await.insert(id, before);
        drop(conn);
        tokio::time::sleep(gap).await;
    }
    eprintln!(
        "gql load: wrote {events} events in {:.1?}",
        write_start.elapsed()
    );

    // Collect until everything arrived or 30 s passed.
    let mut latencies: Vec<Duration> = Vec::with_capacity(expected);
    let deadline = Instant::now() + Duration::from_secs(30);
    while latencies.len() < expected {
        let left = deadline.saturating_duration_since(Instant::now());
        let Ok(Some((id, at))) = tokio::time::timeout(left, got_rx.recv()).await else {
            break;
        };
        if let Some(before) = written.lock().await.get(&id) {
            latencies.push(at.saturating_duration_since(*before));
        }
    }
    latencies.sort();
    let p50 = percentile(&latencies, 0.50);
    let p95 = percentile(&latencies, 0.95);
    let p99 = percentile(&latencies, 0.99);
    let max = latencies.last().copied().unwrap_or_default();
    eprintln!(
        "gql load: {} of {expected} deliveries; write to delivery p50 {p50:.1?}, p95 {p95:.1?}, p99 {p99:.1?}, max {max:.1?}",
        latencies.len()
    );
    let _ = stop.send(true);
    let _ = feeder.await;
    assert_eq!(latencies.len(), expected, "every delivery arrives");
    assert!(
        p95 < Duration::from_secs(1),
        "p95 {p95:?} is under a second"
    );
    db.close().await;
}
