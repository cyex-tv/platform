//! The event WebSocket end to end: real servers on 127.0.0.1, a real WebSocket client, a real
//! Postgres. Runs when `CYEX_TV_TEST_DATABASE_URL` is set (CI sets it), skips otherwise.

use std::net::SocketAddr;
use std::time::{Duration, Instant};

use cyex_tv_db::outbox::{self, NewEvent};
use cyex_tv_db::testing::TestDb;
use cyex_tv_events::{EventsState, router};
use cyex_tv_hub::Hub;
use cyex_tv_shared::model::EventType;
use futures_util::{SinkExt, StreamExt};
use serde_json::{Value, json};
use tokio::net::TcpStream;
use tokio::sync::watch;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};

type Socket = WebSocketStream<MaybeTlsStream<TcpStream>>;

struct Server {
    addr: SocketAddr,
    stop: watch::Sender<bool>,
    tasks: Vec<tokio::task::JoinHandle<()>>,
}

impl Server {
    async fn start(db: &TestDb) -> Self {
        let hub = Hub::new();
        let (stop, stopping) = watch::channel(false);
        let mut feeder_stop = stopping.clone();
        let feeder = tokio::spawn({
            let db = db.pool.clone();
            let hub = hub.clone();
            async move {
                cyex_tv_hub::feeder::run(db, hub, async move {
                    let _ = feeder_stop.wait_for(|s| *s).await;
                })
                .await
                .unwrap();
            }
        });
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let app = router(EventsState::new(db.pool.clone(), hub, stopping));
        let mut serve_stop = stop.subscribe();
        let server = tokio::spawn(async move {
            axum::serve(listener, app)
                .with_graceful_shutdown(async move {
                    let _ = serve_stop.wait_for(|s| *s).await;
                })
                .await
                .unwrap();
        });
        // Let the feeder take its starting point before anything is written.
        tokio::time::sleep(Duration::from_millis(200)).await;
        Self {
            addr,
            stop,
            tasks: vec![feeder, server],
        }
    }

    async fn connect(&self) -> (Socket, Value) {
        let (mut ws, _) = tokio_tungstenite::connect_async(format!("ws://{}/v1/events", self.addr))
            .await
            .unwrap();
        let hello = next(&mut ws).await;
        assert_eq!(hello["op"], "hello", "{hello}");
        (ws, hello["d"].clone())
    }

    async fn stop(self) {
        let _ = self.stop.send(true);
        for task in self.tasks {
            let _ = tokio::time::timeout(Duration::from_secs(5), task).await;
        }
    }
}

async fn send(ws: &mut Socket, op: &str, d: Value) {
    ws.send(Message::Text(
        json!({ "op": op, "d": d }).to_string().into(),
    ))
    .await
    .unwrap();
}

/// The next frame that isn't a heartbeat.
async fn next(ws: &mut Socket) -> Value {
    loop {
        let msg = tokio::time::timeout(Duration::from_secs(5), ws.next())
            .await
            .expect("a frame within 5 s")
            .expect("the socket is open")
            .unwrap();
        if let Message::Text(text) = msg {
            let v: Value = serde_json::from_str(&text).unwrap();
            if v["op"] != "heartbeat" {
                return v;
            }
        }
    }
}

/// Nothing more arrives for a moment.
async fn quiet(ws: &mut Socket) {
    if let Ok(Some(Ok(Message::Text(text)))) =
        tokio::time::timeout(Duration::from_millis(300), ws.next()).await
    {
        panic!("unexpected frame: {text}");
    }
}

async fn publish(db: &TestDb, event_type: EventType, topic: &str, name: &str) -> i64 {
    let mut conn = db.conn().await;
    outbox::publish(
        &mut conn,
        &NewEvent {
            event_type,
            topics: vec![topic.to_owned()],
            set_id: None,
            payload: json!({ "emote_id": "E1", "name": name }),
        },
    )
    .await
    .unwrap()
}

const CHANNEL: &str = "channel:twitch:123";
const OTHER: &str = "channel:kick:9";

#[tokio::test]
async fn subscribe_and_receive_within_a_second() {
    let Some(db) = TestDb::new().await else {
        return;
    };
    let server = Server::start(&db).await;
    let (mut ws, hello) = server.connect().await;
    assert_eq!(hello["heartbeat_interval"], 20_000);
    assert_eq!(hello["max_subscriptions"], 500);
    assert_eq!(hello["session_id"].as_str().unwrap().len(), 26);

    send(
        &mut ws,
        "subscribe",
        json!({ "topics": [CHANNEL, CHANNEL, "channel:youtube:1", "set:nope"] }),
    )
    .await;
    let ack = next(&mut ws).await;
    assert_eq!(ack["op"], "ack");
    assert_eq!(ack["d"]["topics"], json!([CHANNEL]));
    assert_eq!(ack["d"]["errors"]["channel:youtube:1"], "invalid_topic");
    assert_eq!(ack["d"]["errors"]["set:nope"], "invalid_topic");

    let start = Instant::now();
    let id = publish(&db, EventType::SetEmoteAdded, CHANNEL, "Pog").await;
    let got = next(&mut ws).await;
    let took = start.elapsed();
    assert_eq!(got["op"], "dispatch", "{got}");
    assert_eq!(got["d"]["id"], id.to_string());
    assert_eq!(got["d"]["type"], "set.emote_added");
    assert_eq!(got["d"]["data"]["topic"], CHANNEL);
    assert_eq!(got["d"]["data"]["name"], "Pog");
    assert!(took < Duration::from_secs(1), "took {took:?}");
    eprintln!("write to delivery: {took:?}");

    // Other topics don't arrive; after unsubscribing, neither does this one.
    publish(&db, EventType::SetEmoteAdded, OTHER, "x").await;
    quiet(&mut ws).await;
    send(&mut ws, "unsubscribe", json!({ "topics": [CHANNEL] })).await;
    assert_eq!(next(&mut ws).await["d"]["op"], "unsubscribe");
    publish(&db, EventType::SetEmoteAdded, CHANNEL, "y").await;
    quiet(&mut ws).await;
    server.stop().await;
    db.close().await;
}

#[tokio::test]
async fn resume_replays_what_was_missed_once() {
    let Some(db) = TestDb::new().await else {
        return;
    };
    let server = Server::start(&db).await;
    let (mut ws, hello) = server.connect().await;
    let session = hello["session_id"].as_str().unwrap().to_owned();
    send(&mut ws, "subscribe", json!({ "topics": [CHANNEL] })).await;
    next(&mut ws).await;
    let first = publish(&db, EventType::SetEmoteAdded, CHANNEL, "one").await;
    assert_eq!(next(&mut ws).await["d"]["id"], first.to_string());
    ws.close(None).await.unwrap();

    let two = publish(&db, EventType::SetEmoteRenamed, CHANNEL, "two").await;
    publish(&db, EventType::SetEmoteAdded, OTHER, "not mine").await;
    let three = publish(&db, EventType::SetEmoteRemoved, CHANNEL, "three").await;

    let (mut ws, _) = server.connect().await;
    send(
        &mut ws,
        "resume",
        json!({ "session_id": session, "last_event_id": first }),
    )
    .await;
    let ack = next(&mut ws).await;
    assert_eq!(ack["d"]["op"], "resume", "{ack}");
    assert_eq!(ack["d"]["session_id"], session.as_str());
    assert_eq!(ack["d"]["topics"], json!([CHANNEL]));
    assert_eq!(next(&mut ws).await["d"]["id"], two.to_string());
    assert_eq!(next(&mut ws).await["d"]["id"], three.to_string());
    // Live again, and the replayed ones don't come twice.
    let four = publish(&db, EventType::SetEmoteAdded, CHANNEL, "four").await;
    assert_eq!(next(&mut ws).await["d"]["id"], four.to_string());
    quiet(&mut ws).await;

    // An unknown session, and a gap older than what's kept.
    let (mut ws2, _) = server.connect().await;
    send(&mut ws2, "resume", json!({ "session_id": cyex_tv_db::ids::to_text(cyex_tv_db::ids::new_id()), "last_event_id": 1 })).await;
    assert_eq!(next(&mut ws2).await["d"]["error"], "unknown_session");
    let mut conn = db.conn().await;
    sqlx::query("DELETE FROM events_outbox WHERE id <= $1")
        .bind(three)
        .execute(&mut *conn)
        .await
        .unwrap();
    drop(conn);
    send(
        &mut ws2,
        "resume",
        json!({ "session_id": session, "last_event_id": first }),
    )
    .await;
    assert_eq!(next(&mut ws2).await["d"]["op"], "resume");
    assert_eq!(next(&mut ws2).await["d"]["type"], "resync");
    server.stop().await;
    db.close().await;
}

#[tokio::test]
async fn resume_works_across_a_restart() {
    let Some(db) = TestDb::new().await else {
        return;
    };
    let first_server = Server::start(&db).await;
    let (mut ws, hello) = first_server.connect().await;
    let session = hello["session_id"].as_str().unwrap().to_owned();
    send(&mut ws, "subscribe", json!({ "topics": [CHANNEL] })).await;
    next(&mut ws).await;
    let seen = publish(&db, EventType::SetEmoteAdded, CHANNEL, "before").await;
    assert_eq!(next(&mut ws).await["d"]["id"], seen.to_string());

    // The server restarts: the client hears `reconnect`.
    let _ = first_server.stop.send(true);
    assert_eq!(next(&mut ws).await["op"], "reconnect");
    first_server.stop().await;
    let missed = publish(&db, EventType::SetEmoteRenamed, CHANNEL, "during").await;

    // A fresh instance, with an empty hub, picks the session up from Postgres.
    let second = Server::start(&db).await;
    let (mut ws, _) = second.connect().await;
    send(
        &mut ws,
        "resume",
        json!({ "session_id": session, "last_event_id": seen.to_string() }),
    )
    .await;
    assert_eq!(next(&mut ws).await["d"]["op"], "resume");
    let replay = next(&mut ws).await;
    assert_eq!(replay["d"]["id"], missed.to_string());
    assert_eq!(replay["d"]["data"]["name"], "during");
    let after = publish(&db, EventType::SetEmoteAdded, CHANNEL, "after").await;
    assert_eq!(next(&mut ws).await["d"]["id"], after.to_string());
    second.stop().await;
    db.close().await;
}

#[tokio::test]
async fn new_connections_are_limited_per_ip() {
    let Some(db) = TestDb::new().await else {
        return;
    };
    let (_stop, stopping) = watch::channel(false);
    let app = router(
        EventsState::new(db.pool.clone(), Hub::new(), stopping)
            .with_connect_rate(cyex_tv_shared::ratelimit::Rate::per_minute(2)),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(
            listener,
            app.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .await
        .unwrap();
    });
    let url = format!("ws://{addr}/v1/events");
    let _a = tokio_tungstenite::connect_async(&url).await.unwrap();
    let _b = tokio_tungstenite::connect_async(&url).await.unwrap();
    let refused = tokio_tungstenite::connect_async(&url).await;
    match refused {
        Err(tokio_tungstenite::tungstenite::Error::Http(response)) => {
            assert_eq!(response.status(), 429);
            assert!(response.headers().contains_key("retry-after"));
        }
        other => panic!("expected 429, got {other:?}"),
    }
    db.close().await;
}

#[tokio::test]
async fn limits_end_the_connection() {
    let Some(db) = TestDb::new().await else {
        return;
    };
    let server = Server::start(&db).await;

    // 500 topics at most.
    let (mut ws, _) = server.connect().await;
    let many: Vec<String> = (0..501).map(|i| format!("channel:twitch:{i}")).collect();
    // Two frames, as one frame of 501 topics would be over 16 KB.
    send(&mut ws, "subscribe", json!({ "topics": many[..300] })).await;
    assert_eq!(
        next(&mut ws).await["d"]["topics"].as_array().unwrap().len(),
        300
    );
    send(&mut ws, "subscribe", json!({ "topics": many[300..] })).await;
    let ack = next(&mut ws).await;
    assert_eq!(ack["d"]["topics"].as_array().unwrap().len(), 200);
    assert_eq!(ack["d"]["errors"]["channel:twitch:500"], "too_many_topics");

    // Too many frames a second.
    let (mut ws, _) = server.connect().await;
    for _ in 0..15 {
        send(&mut ws, "heartbeat", json!({})).await;
    }
    let err = next(&mut ws).await;
    assert_eq!(
        (err["op"].as_str(), err["d"]["code"].as_str()),
        (Some("error"), Some("rate_limited"))
    );

    // Not JSON, an unknown op, binary.
    for (bad, code) in [
        (Message::Text("nope".into()), "bad_frame"),
        (
            Message::Text(json!({ "op": "dance", "d": {} }).to_string().into()),
            "unknown_op",
        ),
        (Message::Binary(vec![1, 2, 3].into()), "bad_frame"),
    ] {
        let (mut ws, _) = server.connect().await;
        ws.send(bad).await.unwrap();
        assert_eq!(next(&mut ws).await["d"]["code"], code);
    }

    // A frame over 16 KB.
    let (mut ws, _) = server.connect().await;
    let huge = "x".repeat(17 * 1024);
    let _ = ws.send(Message::Text(huge.into())).await;
    let closed = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            match ws.next().await {
                None | Some(Err(_)) | Some(Ok(Message::Close(_))) => return true,
                Some(Ok(_)) => continue,
            }
        }
    })
    .await
    .unwrap_or(false);
    assert!(closed);
    server.stop().await;
    db.close().await;
}
