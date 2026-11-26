//! The Phase 5 load test (PLAN.md): many open connections, ten topics each, events written to
//! the outbox at a steady rate, and the time from each write to each delivery. It runs in CI's
//! `load` job (`cargo test --release -p cyex-tv-events --test load -- --ignored --nocapture`)
//! and prints its numbers; PROGRESS.md records them.
//!
//! - `CYEX_TV_LOAD_CONNECTIONS` (default 5000) and `CYEX_TV_LOAD_EVENTS` (default 300).
//! - Client and server share one machine and one process, so the numbers include both.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use cyex_tv_db::outbox::{self, NewEvent};
use cyex_tv_db::testing::TestDb;
use cyex_tv_events::{EventsState, router};
use cyex_tv_hub::Hub;
use cyex_tv_shared::model::EventType;
use futures_util::{SinkExt, StreamExt};
use serde_json::{Value, json};
use tokio::sync::{Mutex, mpsc, watch};
use tokio_tungstenite::tungstenite::Message;

const TOPICS_PER_CONNECTION: usize = 10;
const TOPIC_POOL: usize = 1000;
const EVENTS_PER_SECOND: u64 = 30;

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
async fn many_connections_get_their_events_fast() {
    let Some(db) = TestDb::new().await else {
        return;
    };
    let connections = env("CYEX_TV_LOAD_CONNECTIONS", 5000);
    let events = env("CYEX_TV_LOAD_EVENTS", 300);

    // The server: hub, feeder, socket.
    let hub = Hub::new();
    let (stop, stopping) = watch::channel(false);
    let mut feeder_stop = stopping.clone();
    tokio::spawn(cyex_tv_hub::feeder::run(
        db.pool.clone(),
        hub.clone(),
        async move {
            let _ = feeder_stop.wait_for(|s| *s).await;
        },
    ));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    // Every client comes from 127.0.0.1, so the per-IP connect limit and socket cap are lifted here.
    let app = router(
        EventsState::new(db.pool.clone(), hub.clone(), stopping)
            .with_connect_rate(cyex_tv_shared::ratelimit::Rate::per_minute(1_000_000))
            .with_sockets_per_ip(u32::MAX),
    );
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    tokio::time::sleep(Duration::from_millis(300)).await;

    // The clients.
    let (got_tx, mut got_rx) = mpsc::unbounded_channel::<(i64, Instant)>();
    let ready = Arc::new(AtomicUsize::new(0));
    let failed = Arc::new(AtomicUsize::new(0));
    let connect_start = Instant::now();
    for batch in 0..connections.div_ceil(250) {
        let mut handles = Vec::new();
        for i in (batch * 250)..((batch + 1) * 250).min(connections) {
            let got = got_tx.clone();
            let ready = ready.clone();
            let failed = failed.clone();
            handles.push(tokio::spawn(async move {
                let url = format!("ws://{addr}/v1/events");
                let Ok((ws, _)) = tokio_tungstenite::connect_async(url).await else {
                    failed.fetch_add(1, Ordering::Relaxed);
                    return;
                };
                let (mut tx, mut rx) = ws.split();
                let _hello = rx.next().await;
                let wanted: Vec<String> = topics_of(i).into_iter().map(topic).collect();
                let sub = json!({ "op": "subscribe", "d": { "topics": wanted } }).to_string();
                if tx.send(Message::Text(sub.into())).await.is_err() {
                    failed.fetch_add(1, Ordering::Relaxed);
                    return;
                }
                let _ack = rx.next().await;
                ready.fetch_add(1, Ordering::Relaxed);
                let mut beat = tokio::time::interval(Duration::from_secs(15));
                beat.tick().await;
                loop {
                    tokio::select! {
                        msg = rx.next() => {
                            let Some(Ok(Message::Text(text))) = msg else {
                                if msg.is_none() { return; }
                                continue;
                            };
                            let now = Instant::now();
                            let Ok(v) = serde_json::from_str::<Value>(&text) else { continue };
                            if v["op"] == "dispatch"
                                && let Some(id) = v["d"]["id"].as_str().and_then(|s| s.parse().ok())
                            {
                                let _ = got.send((id, now));
                            }
                        }
                        _ = beat.tick() => {
                            let hb = json!({ "op": "heartbeat", "d": {} }).to_string();
                            if tx.send(Message::Text(hb.into())).await.is_err() { return; }
                        }
                    }
                }
            }));
        }
        // Each batch connects before the next starts, like clients arriving over time.
        for h in handles {
            let _ = tokio::time::timeout(Duration::from_secs(30), async {
                while !h.is_finished()
                    && ready.load(Ordering::Relaxed) + failed.load(Ordering::Relaxed)
                        < ((batch + 1) * 250).min(connections)
                {
                    tokio::time::sleep(Duration::from_millis(5)).await;
                }
            })
            .await;
        }
    }
    drop(got_tx);
    let connected_in = connect_start.elapsed();
    let ready_count = ready.load(Ordering::Relaxed);
    eprintln!(
        "load: {ready_count} of {connections} connections subscribed ({} failed) in {connected_in:.1?}; hub {:?}",
        failed.load(Ordering::Relaxed),
        hub.stats()
    );
    assert_eq!(ready_count, connections, "every connection subscribes");

    // How many deliveries each topic should produce.
    let mut fanout: HashMap<usize, usize> = HashMap::new();
    for i in 0..connections {
        for t in topics_of(i) {
            *fanout.entry(t).or_default() += 1;
        }
    }

    // Write the events at a steady rate.
    let written: Arc<Mutex<HashMap<i64, (Instant, usize)>>> = Arc::default();
    let mut expected = 0usize;
    let gap = Duration::from_micros(1_000_000 / EVENTS_PER_SECOND);
    let write_start = Instant::now();
    for n in 0..events {
        let t = (n * 37) % TOPIC_POOL;
        let reach = fanout.get(&t).copied().unwrap_or(0);
        expected += reach;
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
        written.lock().await.insert(id, (before, reach));
        drop(conn);
        tokio::time::sleep(gap).await;
    }
    eprintln!(
        "load: wrote {events} events in {:.1?}",
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
        if let Some((before, _)) = written.lock().await.get(&id) {
            latencies.push(at.saturating_duration_since(*before));
        }
    }
    latencies.sort();
    let p50 = percentile(&latencies, 0.50);
    let p95 = percentile(&latencies, 0.95);
    let p99 = percentile(&latencies, 0.99);
    let max = latencies.last().copied().unwrap_or_default();
    eprintln!(
        "load: {} of {expected} deliveries; write to delivery p50 {p50:.1?}, p95 {p95:.1?}, p99 {p99:.1?}, max {max:.1?}",
        latencies.len()
    );
    let _ = stop.send(true);
    assert_eq!(latencies.len(), expected, "every delivery arrives");
    assert!(
        p95 < Duration::from_secs(1),
        "p95 {p95:?} is under a second"
    );
    db.close().await;
}
