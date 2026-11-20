//! Both live-update sockets end to end, from one website mutation: a `graphql-transport-ws`
//! subscriber on the api's `/v1/gql/ws` and a client of the events service's `/v1/events`, each
//! fed from the outbox of the same Postgres, receive the set change.

use std::net::SocketAddr;
use std::time::Duration;

use cyex_tv_db::emotes::{self, NewEmote};
use cyex_tv_db::ids;
use cyex_tv_events::{EventsState, router};
use cyex_tv_hub::Hub;
use cyex_tv_shared::model::EmoteState;
use futures_util::{SinkExt, StreamExt};
use serde_json::{Value, json};
use tokio::net::TcpStream;
use tokio::sync::watch;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};

use crate::world::{As, World};

type Socket = WebSocketStream<MaybeTlsStream<TcpStream>>;

async fn serve(app: axum::Router, stop: &watch::Sender<bool>) -> SocketAddr {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let mut stopping = stop.subscribe();
    tokio::spawn(async move {
        axum::serve(
            listener,
            app.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .with_graceful_shutdown(async move {
            let _ = stopping.wait_for(|s| *s).await;
        })
        .await
        .unwrap();
    });
    addr
}

fn feed(w: &World, hub: Hub, stop: &watch::Sender<bool>) {
    let mut stopping = stop.subscribe();
    tokio::spawn(cyex_tv_hub::feeder::run(
        w.db.pool.clone(),
        hub,
        async move {
            let _ = stopping.wait_for(|s| *s).await;
        },
    ));
}

/// The next text frame that isn't a keep-alive, as JSON.
async fn next(ws: &mut Socket) -> Value {
    loop {
        let msg = tokio::time::timeout(Duration::from_secs(5), ws.next())
            .await
            .expect("a frame within 5 s")
            .expect("the socket is open")
            .unwrap();
        if let Message::Text(text) = msg {
            let v: Value = serde_json::from_str(&text).unwrap();
            if !matches!(v["type"].as_str(), Some("ping" | "pong")) && v["op"] != "heartbeat" {
                return v;
            }
        }
    }
}

async fn send(ws: &mut Socket, v: Value) {
    ws.send(Message::Text(v.to_string().into())).await.unwrap();
}

#[tokio::test]
async fn a_set_change_reaches_both_sockets() {
    let Some(w) = World::new().await else {
        return;
    };
    let (stop, _) = watch::channel(false);
    feed(&w, w.state.hub.clone(), &stop);
    let api = serve(cyex_tv_api::app(w.state.clone()), &stop).await;
    let events_hub = Hub::new();
    feed(&w, events_hub.clone(), &stop);
    let events = serve(
        router(EventsState::new(
            w.db.pool.clone(),
            events_hub.clone(),
            stop.subscribe(),
        )),
        &stop,
    )
    .await;
    // Let both feeders take their starting point before anything is written.
    crate::common::wait_for_feeders(&w.db, 2).await;

    let owner = w.user("7301").await;
    let set = w
        .gql(
            As::App(&owner),
            r#"mutation { createSet(input: { name: "Live", kind: CHANNEL }) { id } }"#,
            json!({}),
        )
        .await["data"]["createSet"]["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let emote = {
        let mut conn = w.db.conn().await;
        let e = emotes::create(
            &mut conn,
            &NewEmote {
                owner_account_id: owner.id,
                name: "liveBoth",
                ..NewEmote::default()
            },
        )
        .await
        .unwrap();
        emotes::set_state(&mut conn, e.id, EmoteState::Approved)
            .await
            .unwrap();
        ids::to_text(e.id)
    };
    let topic = format!("set:{set}");

    // GraphQL subscriber.
    let mut request = format!("ws://{api}/v1/gql/ws")
        .into_client_request()
        .unwrap();
    request.headers_mut().insert(
        "sec-websocket-protocol",
        "graphql-transport-ws".parse().unwrap(),
    );
    let (mut gql, _) = tokio_tungstenite::connect_async(request).await.unwrap();
    send(
        &mut gql,
        json!({ "type": "connection_init", "payload": {} }),
    )
    .await;
    assert_eq!(next(&mut gql).await["type"], "connection_ack");
    send(
        &mut gql,
        json!({
            "id": "1",
            "type": "subscribe",
            "payload": {
                "query": "subscription($t: [String!]!) { events(topics: $t) { eventType topic setId emoteId name } }",
                "variables": { "t": [topic] }
            }
        }),
    )
    .await;

    // Events client.
    let (mut ev, _) = tokio_tungstenite::connect_async(format!("ws://{events}/v1/events"))
        .await
        .unwrap();
    assert_eq!(next(&mut ev).await["op"], "hello");
    send(
        &mut ev,
        json!({ "op": "subscribe", "d": { "topics": [topic] } }),
    )
    .await;
    let ack = next(&mut ev).await;
    assert_eq!(ack["op"], "ack", "{ack}");
    assert_eq!(ack["d"]["topics"], json!([topic]), "{ack}");
    crate::common::wait_until("both subscriptions to register", || async {
        w.state.hub.stats().0 > 0 && events_hub.stats().0 > 0
    })
    .await;

    let r = w
        .gql(
            As::App(&owner),
            "mutation($s: String!, $e: String!) { addSetEmote(setId: $s, emoteId: $e) { name } }",
            json!({ "s": set, "e": emote }),
        )
        .await;
    assert!(r["errors"].is_null(), "{r}");

    let got = next(&mut gql).await;
    assert_eq!(got["type"], "next", "{got}");
    let event = &got["payload"]["data"]["events"];
    assert_eq!(event["eventType"], "SET_EMOTE_ADDED", "{got}");
    assert_eq!(event["topic"], topic.as_str());
    assert_eq!(event["emoteId"], emote.as_str());
    assert_eq!(event["name"], "liveBoth");

    let got = next(&mut ev).await;
    assert_eq!(got["op"], "dispatch", "{got}");
    assert_eq!(got["d"]["type"], "set.emote_added", "{got}");
    assert_eq!(got["d"]["data"]["topic"], topic.as_str(), "{got}");
    assert_eq!(got["d"]["data"]["name"], "liveBoth", "{got}");

    let _ = stop.send(true);
    w.close().await;
}
