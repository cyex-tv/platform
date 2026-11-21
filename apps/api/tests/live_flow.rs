//! The GraphQL `events` subscription end to end: a real server, a `graphql-transport-ws` client,
//! a real Postgres and the outbox feeder. A mutation on one connection reaches a subscriber on
//! another. Runs when `CYEX_TV_TEST_DATABASE_URL` is set (CI sets it), skips otherwise.

mod common;

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};

use axum::body::Body;
use axum::http::Request;
use chrono::Duration as ChronoDuration;
use common::TestDb;
use cyex_tv_api::state::AppState;
use cyex_tv_auth::{Auth, AuthConfig, tokens};
use cyex_tv_db::accounts::{self, Profile};
use cyex_tv_db::emotes::{self, NewEmote};
use cyex_tv_db::sessions::{self, SessionKind};
use cyex_tv_db::{ids, public_ids, sets};
use cyex_tv_shared::model::{EmoteState, Platform, SetKind};
use futures_util::{SinkExt, StreamExt};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use tokio::net::TcpStream;
use tokio::sync::watch;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};
use tower::ServiceExt;

type Socket = WebSocketStream<MaybeTlsStream<TcpStream>>;

struct Live {
    db: TestDb,
    app: axum::Router,
    addr: SocketAddr,
    hub: cyex_tv_hub::Hub,
    stop: watch::Sender<bool>,
}

impl Live {
    async fn start() -> Option<Self> {
        let db = TestDb::new().await?;
        let auth = Arc::new(Auth::new(AuthConfig::from_source(|_| None).unwrap()).unwrap());
        let state = AppState::new(db.pool.clone(), auth);
        let hub = state.hub.clone();
        let (stop, rx) = watch::channel(false);
        let mut feeder_rx = rx.clone();
        tokio::spawn(cyex_tv_hub::feeder::run(
            db.pool.clone(),
            state.hub.clone(),
            async move {
                let _ = feeder_rx.wait_for(|s| *s).await;
            },
        ));
        let app = cyex_tv_api::app(state);
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let served = app.clone();
        let mut serve_rx = rx;
        tokio::spawn(async move {
            axum::serve(listener, served)
                .with_graceful_shutdown(async move {
                    let _ = serve_rx.wait_for(|s| *s).await;
                })
                .await
                .unwrap();
        });
        common::wait_for_feeders(&db, 1).await;
        Some(Self {
            db,
            app,
            addr,
            hub,
            stop,
        })
    }

    async fn subscribe(&self, topics: &[&str]) -> Socket {
        self.open(topics, true).await
    }

    /// Subscribes; with `registered`, returns once the hub holds the subscription (a refused one
    /// never gets there, so a test of refusals passes false and reads the error instead).
    async fn open(&self, topics: &[&str], registered: bool) -> Socket {
        let before = self.hub.stats().0;
        let mut request = format!("ws://{}/v1/gql/ws", self.addr)
            .into_client_request()
            .unwrap();
        request.headers_mut().insert(
            "sec-websocket-protocol",
            "graphql-transport-ws".parse().unwrap(),
        );
        let (mut ws, _) = tokio_tungstenite::connect_async(request).await.unwrap();
        send(&mut ws, json!({ "type": "connection_init", "payload": {} })).await;
        assert_eq!(next(&mut ws).await["type"], "connection_ack");
        send(
            &mut ws,
            json!({
                "id": "1",
                "type": "subscribe",
                "payload": {
                    "query": "subscription($t: [String!]!) { events(topics: $t) { id eventType topic setId emoteId name } }",
                    "variables": { "t": topics }
                }
            }),
        )
        .await;
        // The subscription is in the hub once the server has read the message.
        if registered {
            common::wait_until("the subscription to register", || async {
                self.hub.stats().0 > before
            })
            .await;
        }
        ws
    }

    async fn user(&self, twitch_id: &str) -> (uuid::Uuid, String) {
        let mut conn = self.db.conn().await;
        let login = format!("user{twitch_id}");
        let signed = accounts::sign_in(
            &mut conn,
            Profile {
                platform: Platform::Twitch,
                platform_user_id: twitch_id,
                login: &login,
                display_name: &login,
                avatar: None,
            },
        )
        .await
        .unwrap();
        let token = tokens::random_token();
        sessions::create(
            &mut conn,
            &tokens::hash(&token),
            signed.account.id,
            SessionKind::Site,
            ChronoDuration::days(1),
        )
        .await
        .unwrap();
        (signed.account.id, token)
    }

    async fn gql(&self, token: &str, query: &str, variables: Value) -> Value {
        let request = Request::post("/v1/gql")
            .header("content-type", "application/json")
            .header(
                "cookie",
                format!("{}={token}", cyex_tv_auth::cookies::SESSION),
            )
            .header("origin", "https://cyex.tv")
            .body(Body::from(
                json!({ "query": query, "variables": variables }).to_string(),
            ))
            .unwrap();
        let response = self.app.clone().oneshot(request).await.unwrap();
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        serde_json::from_slice(&bytes).unwrap()
    }

    async fn stop(self) {
        let _ = self.stop.send(true);
        // Closing the pool waits for the feeder's connection to come back.
        self.db.close().await;
    }
}

async fn send(ws: &mut Socket, v: Value) {
    ws.send(Message::Text(v.to_string().into())).await.unwrap();
}

async fn next(ws: &mut Socket) -> Value {
    loop {
        let msg = tokio::time::timeout(Duration::from_secs(5), ws.next())
            .await
            .expect("a message within 5 s")
            .expect("the socket is open")
            .unwrap();
        if let Message::Text(text) = msg {
            let v: Value = serde_json::from_str(&text).unwrap();
            if v["type"] != "ping" && v["type"] != "pong" {
                return v;
            }
        }
    }
}

#[tokio::test]
async fn a_mutation_reaches_a_graphql_subscriber() {
    let Some(live) = Live::start().await else {
        return;
    };
    let (alice, token) = live.user("55").await;
    let mut conn = live.db.conn().await;
    let set = sets::create(&mut conn, alice, "Main", SetKind::Channel)
        .await
        .unwrap();
    let emote = emotes::create(
        &mut conn,
        &NewEmote {
            owner_account_id: alice,
            name: "livePog",
            ..NewEmote::default()
        },
    )
    .await
    .unwrap();
    emotes::set_state(&mut conn, emote.id, EmoteState::Approved)
        .await
        .unwrap();
    let set_public = public_ids::public_text(&mut conn, set.id)
        .await
        .unwrap()
        .unwrap();
    let emote_public = public_ids::public_text(&mut conn, emote.id)
        .await
        .unwrap()
        .unwrap();
    drop(conn);
    // By the set's old ULID (ADR 0026) and by its Snowflake: both hear the event, each under
    // the name it asked for.
    let set_topic = format!("set:{}", ids::to_text(set.id));
    let mut ws = live.subscribe(&[&set_topic]).await;
    let new_topic = format!("set:{set_public}");
    let mut ws_new = live.subscribe(&[&new_topic]).await;

    let start = Instant::now();
    let r = live
        .gql(
            &token,
            "mutation($s: String!, $e: String!) { addSetEmote(setId: $s, emoteId: $e) { name } }",
            json!({ "s": ids::to_text(set.id), "e": ids::to_text(emote.id) }),
        )
        .await;
    assert!(r["errors"].is_null(), "{r}");
    let msg = next(&mut ws).await;
    let took = start.elapsed();
    assert_eq!(msg["type"], "next", "{msg}");
    let event = &msg["payload"]["data"]["events"];
    assert_eq!(event["eventType"], "SET_EMOTE_ADDED");
    assert_eq!(event["topic"], set_topic.as_str());
    assert_eq!(event["setId"], set_public.as_str());
    assert_eq!(event["emoteId"], emote_public.as_str());
    assert_eq!(event["name"], "livePog");
    assert!(took < Duration::from_secs(1), "took {took:?}");
    let msg = next(&mut ws_new).await;
    let event = &msg["payload"]["data"]["events"];
    assert_eq!(event["topic"], new_topic.as_str(), "{msg}");
    assert_eq!(event["setId"], set_public.as_str());
    assert_eq!(event["emoteId"], emote_public.as_str());
    eprintln!("mutation to GraphQL subscriber: {took:?}");
    live.stop().await;
}

#[tokio::test]
async fn personal_sets_send_personal_updated() {
    let Some(live) = Live::start().await else {
        return;
    };
    let (alice, token) = live.user("66").await;
    let mut conn = live.db.conn().await;
    let set = sets::create(&mut conn, alice, "Mine", SetKind::Personal)
        .await
        .unwrap();
    let emote = emotes::create(
        &mut conn,
        &NewEmote {
            owner_account_id: alice,
            name: "mine",
            ..NewEmote::default()
        },
    )
    .await
    .unwrap();
    emotes::set_state(&mut conn, emote.id, EmoteState::Approved)
        .await
        .unwrap();
    drop(conn);
    let mut ws = live.subscribe(&["user:twitch:66"]).await;
    live.gql(
        &token,
        "mutation($s: String!, $e: String!) { addSetEmote(setId: $s, emoteId: $e) { name } }",
        json!({ "s": ids::to_text(set.id), "e": ids::to_text(emote.id) }),
    )
    .await;
    let msg = next(&mut ws).await;
    assert_eq!(
        msg["payload"]["data"]["events"]["eventType"], "PERSONAL_UPDATED",
        "{msg}"
    );
    assert_eq!(msg["payload"]["data"]["events"]["topic"], "user:twitch:66");
    live.stop().await;
}

#[tokio::test]
async fn bad_topics_are_refused() {
    let Some(live) = Live::start().await else {
        return;
    };
    let mut ws = live.open(&["channel:youtube:1"], false).await;
    let msg = next(&mut ws).await;
    let text = msg.to_string();
    assert!(text.contains("invalid_topic"), "{text}");
    live.stop().await;
}
