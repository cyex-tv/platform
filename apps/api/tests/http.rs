//! HTTP tests against the real router: REST, GraphQL and the generated descriptions. They need no
//! database, except `ready_with_database`, which runs when `CYEX_TV_TEST_DATABASE_URL` is set (CI
//! sets it).

use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use cyex_tv_api::state::AppState;
use cyex_tv_auth::{Auth, AuthConfig};
use cyex_tv_shared::config::Config;
use cyex_tv_shared::error::ErrorBody;
use http_body_util::BodyExt;
use serde_json::{Value, json};
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

/// Sign-in config with no platform clients: both platforms answer 503.
fn auth() -> Arc<Auth> {
    Arc::new(Auth::new(AuthConfig::from_source(|_| None).unwrap()).unwrap())
}

/// A pool pointing at a port nothing listens on.
fn offline_state() -> AppState {
    let cfg = config("postgres://nobody:nothing@127.0.0.1:1/none");
    AppState::new(cyex_tv_db::connect_lazy(&cfg).unwrap(), auth())
}

async fn send(state: AppState, request: Request<Body>) -> (StatusCode, Vec<u8>) {
    let response = cyex_tv_api::app(state).oneshot(request).await.unwrap();
    let status = response.status();
    let body = response
        .into_body()
        .collect()
        .await
        .unwrap()
        .to_bytes()
        .to_vec();
    (status, body)
}

async fn get(state: AppState, uri: &str) -> (StatusCode, Vec<u8>) {
    send(state, Request::get(uri).body(Body::empty()).unwrap()).await
}

/// Runs a GraphQL query and returns the JSON response.
async fn gql(query: &str) -> Value {
    let request = Request::post("/v1/gql")
        .header("content-type", "application/json")
        .body(Body::from(json!({ "query": query }).to_string()))
        .unwrap();
    let (status, body) = send(offline_state(), request).await;
    assert_eq!(status, StatusCode::OK, "{query}");
    serde_json::from_slice(&body).unwrap()
}

fn error_code(response: &Value) -> &str {
    response["errors"][0]["extensions"]["code"]
        .as_str()
        .unwrap_or_default()
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
async fn rest_stubs_answer_not_implemented() {
    // Only the upload is left (Phase 3).
    let request = Request::post("/v1/emotes").body(Body::empty()).unwrap();
    let (status, body) = send(offline_state(), request).await;
    assert_eq!(status, StatusCode::NOT_IMPLEMENTED);
    let error: ErrorBody = serde_json::from_slice(&body).unwrap();
    assert_eq!(error.error, "not_implemented");
}

#[tokio::test]
async fn sign_in_checks_come_before_the_database() {
    for (uri, status, code) in [
        (
            "/v1/auth/youtube/login",
            StatusCode::BAD_REQUEST,
            "unknown_platform",
        ),
        (
            "/v1/auth/twitch/login",
            StatusCode::SERVICE_UNAVAILABLE,
            "unavailable",
        ),
        (
            "/v1/auth/kick/callback?code=c&state=s",
            StatusCode::SERVICE_UNAVAILABLE,
            "unavailable",
        ),
    ] {
        let (got, body) = get(offline_state(), uri).await;
        assert_eq!(got, status, "{uri}");
        let error: ErrorBody = serde_json::from_slice(&body).unwrap();
        assert_eq!(error.error, code, "{uri}");
    }
}

#[tokio::test]
async fn me_is_signed_out_without_a_session() {
    let response = gql("{ me { id } }").await;
    assert_eq!(error_code(&response), "signed_out");
    let response = gql(r#"mutation { unlinkIdentity(platform: KICK) { id } }"#).await;
    assert_eq!(error_code(&response), "signed_out");
}

#[tokio::test]
async fn logout_without_a_session_clears_the_cookie() {
    let request = Request::post("/v1/auth/logout")
        .body(Body::empty())
        .unwrap();
    let response = cyex_tv_api::app(offline_state())
        .oneshot(request)
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NO_CONTENT);
    let cookie = response.headers()["set-cookie"].to_str().unwrap();
    assert!(cookie.starts_with("cyex_tv_session=;"), "{cookie}");
    assert!(cookie.contains("Max-Age=0"), "{cookie}");
}

#[tokio::test]
async fn cookie_writes_need_the_site_origin() {
    // The Origin check runs before the session lookup, so it needs no database.
    let cookie = format!("cyex_tv_session={}", "a".repeat(43));
    for origin in [
        None,
        Some("https://evil.example"),
        Some("https://api.cyex.tv"),
    ] {
        let mut request = Request::post("/v1/gql")
            .header("content-type", "application/json")
            .header("cookie", &cookie);
        if let Some(origin) = origin {
            request = request.header("origin", origin);
        }
        let request = request
            .body(Body::from(json!({ "query": "{ me { id } }" }).to_string()))
            .unwrap();
        let (status, body) = send(offline_state(), request).await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{origin:?}");
        let error: ErrorBody = serde_json::from_slice(&body).unwrap();
        assert_eq!(error.error, "bad_origin");
    }
}

#[tokio::test]
async fn unknown_paths_are_404() {
    let (status, _) = get(offline_state(), "/v1/nope").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn graphql_stubs_answer_not_implemented() {
    let response = gql(r#"{ searchEmotes(query: "pog") { total } }"#).await;
    assert_eq!(error_code(&response), "not_implemented");
}

#[tokio::test]
async fn graphql_admin_fields_need_a_permission() {
    for query in [
        "{ reviewQueue { total } }",
        "{ roles { id } }",
        r#"mutation { approveEmote(id: "e1") { id } }"#,
        r#"mutation { createRole(input: { name: "mods", position: 1, permissions: [] }) { id } }"#,
    ] {
        let response = gql(query).await;
        assert_eq!(error_code(&response), "signed_out", "{query}");
    }
}

#[tokio::test]
async fn graphql_permission_names_are_public() {
    let response = gql("{ permissions }").await;
    let names = response["data"]["permissions"].as_array().unwrap();
    assert!(names.iter().any(|n| n == "emotes.review"));
    assert!(names.iter().any(|n| n == "roles.manage"));
}

#[tokio::test]
async fn graphql_introspection_is_on() {
    let response = gql("{ __schema { queryType { name } } }").await;
    assert_eq!(response["data"]["__schema"]["queryType"]["name"], "Query");
}

#[tokio::test]
async fn graphiql_opens_in_a_browser() {
    let (status, body) = get(offline_state(), "/v1/gql").await;
    assert_eq!(status, StatusCode::OK);
    assert!(String::from_utf8_lossy(&body).contains("graphiql"));
}

#[test]
fn openapi_lists_only_the_rest_part() {
    let spec = cyex_tv_api::openapi();
    for path in [
        "/health",
        "/v1/auth/{platform}/login",
        "/v1/emotes",
        "/v1/channels/{platform}/{id}",
        "/v1/sets/{id}",
        "/v1/sets/global",
        "/v1/personal-sets/lookup",
    ] {
        assert!(spec.paths.paths.contains_key(path), "missing {path}");
    }
    for graphql_only in ["/v1/me", "/v1/roles", "/v1/search/emotes"] {
        assert!(
            !spec.paths.paths.contains_key(graphql_only),
            "{graphql_only} belongs in GraphQL"
        );
    }
}

#[test]
fn sdl_lists_the_contract() {
    let sdl = cyex_tv_api::gql::sdl();
    for needle in [
        "type Query",
        "type Mutation",
        "type Subscription",
        "reviewQueue",
        "searchEmotes",
        "createRole",
        "addSetEmote",
        "events(topics: [String!]!)",
        "enum Platform",
    ] {
        assert!(sdl.contains(needle), "missing {needle}");
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
    let (status, _) = get(AppState::new(db, auth()), "/health/ready").await;
    assert_eq!(status, StatusCode::OK);
}
