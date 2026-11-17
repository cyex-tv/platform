//! `resume` is bounded: a connection resumes once, and only when the old and current topics
//! together stay within the 500 limit. Real servers on 127.0.0.1, a real WebSocket client, a real
//! Postgres. Runs when `CYEX_TV_TEST_DATABASE_URL` is set (CI sets it), skips otherwise.

use std::net::SocketAddr;
use std::time::Duration;

use cyex_tv_db::testing::TestDb;
use cyex_tv_events::{EventsState, router};
use cyex_tv_hub::Hub;
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

/// After a resume: at most a `resync` (when the outbox holds older events), then nothing.
async fn settle(ws: &mut Socket) {
    if let Ok(Some(Ok(Message::Text(text)))) =
        tokio::time::timeout(Duration::from_millis(300), ws.next()).await
    {
        let v: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(v["d"]["type"], "resync", "{v}");
    }
    quiet(ws).await;
}

fn topics(platform: &str, n: usize) -> Vec<String> {
    (0..n).map(|i| format!("channel:{platform}:{i}")).collect()
}

/// A session subscribed to `topics`, then left.
async fn old_session(server: &Server, topics: &[String]) -> String {
    let (mut ws, hello) = server.connect().await;
    send(&mut ws, "subscribe", json!({ "topics": topics })).await;
    let ack = next(&mut ws).await;
    assert_eq!(
        ack["d"]["topics"].as_array().unwrap().len(),
        topics.len(),
        "{ack}"
    );
    ws.close(None).await.unwrap();
    hello["session_id"].as_str().unwrap().to_owned()
}

#[tokio::test]
async fn a_connection_resumes_once() {
    let Some(db) = TestDb::new().await else {
        return;
    };
    let server = Server::start(&db).await;
    let first = old_session(&server, &topics("twitch", 3)).await;
    let second = old_session(&server, &topics("kick", 3)).await;

    let (mut ws, _) = server.connect().await;
    send(
        &mut ws,
        "resume",
        json!({ "session_id": first, "last_event_id": 0 }),
    )
    .await;
    let ack = next(&mut ws).await;
    assert_eq!(ack["d"]["session_id"], first.as_str(), "{ack}");
    settle(&mut ws).await;

    // A second resume on the same connection is refused, and takes nothing over.
    for session in [&second, &first] {
        send(
            &mut ws,
            "resume",
            json!({ "session_id": session, "last_event_id": 0 }),
        )
        .await;
        let refused = next(&mut ws).await;
        assert_eq!(refused["op"], "ack", "{refused}");
        assert_eq!(refused["d"]["op"], "resume");
        assert_eq!(refused["d"]["error"], "already_resumed", "{refused}");
        assert!(refused["d"]["topics"].is_null(), "{refused}");
    }
    quiet(&mut ws).await;

    // The connection kept the first session's three topics and nothing more.
    let fill = topics("twitch", 500);
    send(&mut ws, "subscribe", json!({ "topics": fill[3..300] })).await;
    assert_eq!(
        next(&mut ws).await["d"]["topics"].as_array().unwrap().len(),
        297
    );
    send(&mut ws, "subscribe", json!({ "topics": fill[300..] })).await;
    let ack = next(&mut ws).await;
    assert_eq!(ack["d"]["topics"].as_array().unwrap().len(), 200, "{ack}");
    assert!(ack["d"]["errors"].as_object().unwrap().is_empty(), "{ack}");
    server.stop().await;
    db.close().await;
}

#[tokio::test]
async fn resume_past_the_topic_limit_is_refused() {
    let Some(db) = TestDb::new().await else {
        return;
    };
    let server = Server::start(&db).await;
    let old = topics("twitch", 300);
    let session = old_session(&server, &old).await;

    let (mut ws, _) = server.connect().await;
    let current = topics("kick", 300);
    send(&mut ws, "subscribe", json!({ "topics": current })).await;
    assert_eq!(
        next(&mut ws).await["d"]["topics"].as_array().unwrap().len(),
        300
    );
    send(
        &mut ws,
        "resume",
        json!({ "session_id": session, "last_event_id": 0 }),
    )
    .await;
    let refused = next(&mut ws).await;
    assert_eq!(refused["d"]["op"], "resume", "{refused}");
    assert_eq!(refused["d"]["error"], "too_many_topics", "{refused}");
    quiet(&mut ws).await;

    // Still 300 topics: room for exactly 200 more.
    send(
        &mut ws,
        "subscribe",
        json!({ "topics": topics("kick", 501)[300..] }),
    )
    .await;
    let ack = next(&mut ws).await;
    assert_eq!(ack["d"]["topics"].as_array().unwrap().len(), 200, "{ack}");
    assert_eq!(ack["d"]["errors"]["channel:kick:500"], "too_many_topics");

    // Topics the connection already has don't count twice: 300 old + 200 overlapping fits.
    let (mut ws, _) = server.connect().await;
    send(&mut ws, "subscribe", json!({ "topics": old[..200] })).await;
    next(&mut ws).await;
    send(
        &mut ws,
        "subscribe",
        json!({ "topics": topics("kick", 200) }),
    )
    .await;
    next(&mut ws).await;
    send(
        &mut ws,
        "resume",
        json!({ "session_id": session, "last_event_id": 0 }),
    )
    .await;
    let ack = next(&mut ws).await;
    assert_eq!(ack["d"]["session_id"], session.as_str(), "{ack}");
    server.stop().await;
    db.close().await;
}
