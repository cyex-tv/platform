//! The GraphQL socket runs subscriptions only, and at most `MAX_SOCKET_SUBSCRIPTIONS` at once per
//! connection. A real server and a `graphql-transport-ws` client; nothing here touches the
//! database (the pool is lazy and never connects), so these always run.

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use cyex_tv_api::gql::MAX_SOCKET_SUBSCRIPTIONS;
use cyex_tv_api::state::AppState;
use cyex_tv_auth::{Auth, AuthConfig};
use cyex_tv_hub::Hub;
use futures_util::{SinkExt, StreamExt};
use serde_json::{Value, json};
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};

type Socket = WebSocketStream<MaybeTlsStream<TcpStream>>;

const EVENTS: &str = "subscription($t: [String!]!) { events(topics: $t) { id } }";

async fn start() -> (SocketAddr, Hub) {
    let pool = sqlx::postgres::PgPoolOptions::new()
        .connect_lazy("postgres://nobody@127.0.0.1:1/none")
        .unwrap();
    let auth = Arc::new(Auth::new(AuthConfig::from_source(|_| None).unwrap()).unwrap());
    let state = AppState::new(pool, auth);
    let hub = state.hub.clone();
    let app = cyex_tv_api::app(state);
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
    (addr, hub)
}

async fn connect(addr: SocketAddr) -> Socket {
    let mut request = format!("ws://{addr}/v1/gql/ws")
        .into_client_request()
        .unwrap();
    request.headers_mut().insert(
        "sec-websocket-protocol",
        "graphql-transport-ws".parse().unwrap(),
    );
    let (mut ws, _) = tokio_tungstenite::connect_async(request).await.unwrap();
    send(&mut ws, json!({ "type": "connection_init", "payload": {} })).await;
    assert_eq!(next(&mut ws).await["type"], "connection_ack");
    ws
}

async fn send(ws: &mut Socket, v: Value) {
    ws.send(Message::Text(v.to_string().into())).await.unwrap();
}

async fn subscribe(ws: &mut Socket, id: &str, query: &str, variables: Value) {
    send(
        ws,
        json!({
            "id": id,
            "type": "subscribe",
            "payload": { "query": query, "variables": variables }
        }),
    )
    .await;
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

/// Nothing more arrives for a moment.
async fn quiet(ws: &mut Socket) {
    if let Ok(Some(Ok(Message::Text(text)))) =
        tokio::time::timeout(Duration::from_millis(300), ws.next()).await
    {
        panic!("unexpected message: {text}");
    }
}

/// The error a refused operation gets: one `next` carrying errors and no data, then `complete`.
async fn refused(ws: &mut Socket, id: &str) -> String {
    let msg = next(ws).await;
    assert_eq!(msg["id"], id, "{msg}");
    let payload = match msg["type"].as_str() {
        Some("next") => msg["payload"].clone(),
        Some("error") => json!({ "errors": msg["payload"] }),
        _ => panic!("expected an error, got {msg}"),
    };
    assert!(
        payload["data"].is_null(),
        "a refused operation returns no data: {msg}"
    );
    let code = payload["errors"][0]["extensions"]["code"]
        .as_str()
        .unwrap_or_default()
        .to_owned();
    if msg["type"] == "next" {
        let done = next(ws).await;
        assert_eq!(done["type"], "complete", "{done}");
        assert_eq!(done["id"], id);
    }
    code
}

#[tokio::test]
async fn queries_and_mutations_on_the_socket_are_refused() {
    let (addr, _) = start().await;
    let mut ws = connect(addr).await;
    subscribe(&mut ws, "q", "query { __typename }", json!({})).await;
    assert_eq!(refused(&mut ws, "q").await, "subscriptions_only");
    subscribe(
        &mut ws,
        "m",
        "mutation { markNotificationsRead }",
        json!({}),
    )
    .await;
    assert_eq!(refused(&mut ws, "m").await, "subscriptions_only");
    // The socket still runs subscriptions afterwards.
    subscribe(&mut ws, "s", EVENTS, json!({ "t": ["channel:twitch:1"] })).await;
    quiet(&mut ws).await;
}

#[tokio::test]
async fn one_socket_holds_at_most_eight_subscriptions() {
    let (addr, hub) = start().await;
    let mut ws = connect(addr).await;
    for i in 0..MAX_SOCKET_SUBSCRIPTIONS {
        let topic = format!("channel:twitch:{i}");
        subscribe(&mut ws, &i.to_string(), EVENTS, json!({ "t": [topic] })).await;
    }
    quiet(&mut ws).await;
    assert_eq!(hub.stats().0, MAX_SOCKET_SUBSCRIPTIONS);

    subscribe(&mut ws, "extra", EVENTS, json!({ "t": ["channel:kick:1"] })).await;
    assert_eq!(refused(&mut ws, "extra").await, "too_many_subscriptions");
    assert_eq!(hub.stats().0, MAX_SOCKET_SUBSCRIPTIONS);

    // Ending one frees its place.
    send(&mut ws, json!({ "id": "0", "type": "complete" })).await;
    let done = next(&mut ws).await;
    assert_eq!(
        (done["type"].as_str(), done["id"].as_str()),
        (Some("complete"), Some("0"))
    );
    tokio::time::sleep(Duration::from_millis(100)).await;
    subscribe(&mut ws, "again", EVENTS, json!({ "t": ["channel:kick:1"] })).await;
    quiet(&mut ws).await;
    assert_eq!(hub.stats().0, MAX_SOCKET_SUBSCRIPTIONS);

    // The cap is per connection: another socket gets its own eight.
    let mut other = connect(addr).await;
    subscribe(&mut other, "1", EVENTS, json!({ "t": ["channel:kick:2"] })).await;
    quiet(&mut other).await;
    assert_eq!(hub.stats().0, MAX_SOCKET_SUBSCRIPTIONS + 1);

    // Closing the socket ends its streams.
    ws.close(None).await.unwrap();
    tokio::time::sleep(Duration::from_millis(200)).await;
    assert_eq!(hub.stats().0, 1);
}
