//! HTTP tests against the real router. They need no database, except
//! `ready_with_database`, which runs when `CYEX_TV_TEST_DATABASE_URL` is set (CI sets it).

use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use axum::body::Body;
use axum::http::{Request, StatusCode};
use cyex_tv_api::state::AppState;
use cyex_tv_shared::config::Config;
use cyex_tv_shared::error::ErrorBody;
use http_body_util::BodyExt;
use tower::ServiceExt;

const BIND: SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0);

fn config(database_url: &str) -> Config {
    let url = database_url.to_owned();
    Config::from_source(
        move |name| match name {
            "CYEX_TV_DATABASE_URL" => Some(url.clone()),
            "CYEX_TV_DB_ACQUIRE_TIMEOUT_MS" => Some("300".to_owned()),
            _ => None,
        },
        BIND,
    )
    .unwrap()
}

/// A pool pointing at a port nothing listens on.
fn offline_state() -> AppState {
    let cfg = config("postgres://nobody:nothing@127.0.0.1:1/none");
    AppState { db: cyex_tv_db::connect_lazy(&cfg).unwrap() }
}

async fn get(state: AppState, uri: &str) -> (StatusCode, Vec<u8>) {
    let response = cyex_tv_api::app(state)
        .oneshot(Request::get(uri).body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let body = response.into_body().collect().await.unwrap().to_bytes().to_vec();
    (status, body)
}

#[tokio::test]
async fn health_is_ok() {
    let (status, body) = get(offline_state(), "/health").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, br#"{"status":"ok"}"#);
}

#[tokio::test]
async fn ready_is_503_without_a_database() {
    let (status, body) = get(offline_state(), "/health/ready").await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    let error: ErrorBody = serde_json::from_slice(&body).unwrap();
    assert_eq!(error.error, "unavailable");
}

#[tokio::test]
async fn stubs_answer_not_implemented() {
    for uri in ["/v1/me", "/v1/sets/global", "/v1/channels/twitch/123", "/v1/search/emotes?q=x"] {
        let (status, body) = get(offline_state(), uri).await;
        assert_eq!(status, StatusCode::NOT_IMPLEMENTED, "{uri}");
        let error: ErrorBody = serde_json::from_slice(&body).unwrap();
        assert_eq!(error.error, "not_implemented", "{uri}");
    }
}

#[tokio::test]
async fn unknown_paths_are_404() {
    let (status, _) = get(offline_state(), "/v1/nope").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[test]
fn spec_lists_the_contract() {
    let spec = cyex_tv_api::openapi();
    for path in [
        "/health",
        "/v1/me",
        "/v1/sets/{id}",
        "/v1/sets/{id}/emotes/{emote_id}",
        "/v1/personal-sets/lookup",
        "/v1/roles",
        "/v1/events",
    ] {
        assert!(spec.paths.paths.contains_key(path), "missing {path}");
    }
    let json = spec.to_pretty_json().unwrap();
    for schema in ["\"Emote\"", "\"EmoteSet\"", "\"ErrorBody\"", "\"Platform\""] {
        assert!(json.contains(schema), "missing schema {schema}");
    }
}

#[tokio::test]
async fn ready_with_database() {
    let Ok(url) = std::env::var("CYEX_TV_TEST_DATABASE_URL") else {
        eprintln!("CYEX_TV_TEST_DATABASE_URL not set; skipping");
        return;
    };
    let cfg = config(&url);
    let db = cyex_tv_db::connect(&cfg).await.unwrap();
    cyex_tv_db::MIGRATOR.run(&db).await.unwrap();
    let (status, _) = get(AppState { db }, "/health/ready").await;
    assert_eq!(status, StatusCode::OK);
}
