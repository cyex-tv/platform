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
async fn upload_needs_a_session() {
    // Every REST route is built now; an upload without a session is refused before anything
    // is read or stored.
    let boundary = "b0undary";
    let body = format!(
        "--{boundary}\r\nContent-Disposition: form-data; name=\"name\"\r\n\r\nx\r\n--{boundary}--\r\n"
    );
    let request = Request::post("/v1/emotes")
        .header(
            "content-type",
            format!("multipart/form-data; boundary={boundary}"),
        )
        .body(Body::from(body))
        .unwrap();
    let (status, body) = send(offline_state(), request).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let error: ErrorBody = serde_json::from_slice(&body).unwrap();
    assert_eq!(error.error, "signed_out");
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
        "/v1/cosmetics",
        "/v1/cosmetics/users",
        "/v1/cosmetics/images",
        "/v1/limits",
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
        "cosmetics: CosmeticCatalog!",
        "userCosmetics(platform: Platform!, userIds: [String!]!): [UserCosmetics!]!",
        "selectCosmetics(badgeId: String, paintId: String, ownerId: String): OwnedCosmetics!",
        "grantCosmetic(input: GrantCosmeticInput!): Entitlement!",
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

fn limited_state(trust_cloudflare: bool) -> AppState {
    let tiny = cyex_tv_shared::ratelimit::Rate::per_minute(2);
    let rates = cyex_tv_api::limits::Rates {
        auth: tiny,
        reads: tiny,
        ..cyex_tv_api::limits::Rates::default()
    };
    offline_state().with_limits(cyex_tv_api::limits::Limits::new(rates, trust_cloudflare))
}

async fn status_of(
    app: &axum::Router,
    uri: &str,
    ip: Option<&str>,
) -> (StatusCode, Option<String>) {
    let mut request = Request::get(uri);
    if let Some(ip) = ip {
        request = request.header("cf-connecting-ip", ip);
    }
    let response = app
        .clone()
        .oneshot(request.body(Body::empty()).unwrap())
        .await
        .unwrap();
    let retry = response
        .headers()
        .get("retry-after")
        .map(|v| v.to_str().unwrap().to_owned());
    (response.status(), retry)
}

#[tokio::test]
async fn requests_over_the_limit_get_429() {
    let app = cyex_tv_api::app(limited_state(false));
    // Sign-in has its own bucket (unconfigured platform: 503, but counted).
    for _ in 0..2 {
        assert_eq!(
            status_of(&app, "/v1/auth/twitch/login", None).await.0,
            StatusCode::SERVICE_UNAVAILABLE
        );
    }
    let (status, retry) = status_of(&app, "/v1/auth/twitch/login", None).await;
    assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);
    assert!(retry.unwrap().parse::<u64>().unwrap() >= 1);
    // Health checks are never limited.
    for _ in 0..5 {
        assert_eq!(status_of(&app, "/health", None).await.0, StatusCode::OK);
    }
    // Without trusting Cloudflare, its header changes nothing: still the same client.
    assert_eq!(
        status_of(&app, "/v1/auth/kick/login", Some("203.0.113.9"))
            .await
            .0,
        StatusCode::TOO_MANY_REQUESTS
    );
}

#[tokio::test]
async fn behind_cloudflare_each_client_has_its_own_bucket() {
    let app = cyex_tv_api::app(limited_state(true));
    for _ in 0..2 {
        status_of(&app, "/v1/auth/twitch/login", Some("203.0.113.1")).await;
    }
    assert_eq!(
        status_of(&app, "/v1/auth/twitch/login", Some("203.0.113.1"))
            .await
            .0,
        StatusCode::TOO_MANY_REQUESTS
    );
    assert_eq!(
        status_of(&app, "/v1/auth/twitch/login", Some("203.0.113.2"))
            .await
            .0,
        StatusCode::SERVICE_UNAVAILABLE
    );
}

async fn cors(method: &str, uri: &str, origin: &str, preflight: bool) -> axum::http::HeaderMap {
    let mut request = Request::builder()
        .method(method)
        .uri(uri)
        .header("origin", origin);
    if preflight {
        request = request.header("access-control-request-method", "POST");
    }
    let response = cyex_tv_api::app(offline_state())
        .oneshot(request.body(Body::empty()).unwrap())
        .await
        .unwrap();
    response.headers().clone()
}

#[tokio::test]
async fn the_site_gets_credentialed_cors() {
    let h = cors("OPTIONS", "/v1/gql", "https://cyex.tv", true).await;
    assert_eq!(h["access-control-allow-origin"], "https://cyex.tv");
    assert_eq!(h["access-control-allow-credentials"], "true");
    assert!(
        h["access-control-allow-methods"]
            .to_str()
            .unwrap()
            .contains("POST")
    );
    assert!(
        h["access-control-allow-headers"]
            .to_str()
            .unwrap()
            .contains("content-type")
    );
    let h = cors("GET", "/health", "https://cyex.tv", false).await;
    assert_eq!(h["access-control-allow-origin"], "https://cyex.tv");
}

#[tokio::test]
async fn other_origins_get_the_public_reads_only() {
    // Public reads: anyone, without credentials.
    let h = cors("GET", "/v1/sets/global", "https://www.twitch.tv", false).await;
    assert_eq!(h["access-control-allow-origin"], "*");
    assert!(!h.contains_key("access-control-allow-credentials"));
    assert!(
        h["access-control-expose-headers"]
            .to_str()
            .unwrap()
            .contains("etag")
    );
    let h = cors(
        "OPTIONS",
        "/v1/personal-sets/lookup",
        "https://www.twitch.tv",
        true,
    )
    .await;
    assert_eq!(h["access-control-allow-origin"], "*");
    // GraphQL, uploads and sign-in: nothing for other origins.
    for (method, uri) in [
        ("OPTIONS", "/v1/gql"),
        ("OPTIONS", "/v1/emotes"),
        ("GET", "/v1/auth/twitch/login"),
    ] {
        let h = cors(method, uri, "https://evil.example", method == "OPTIONS").await;
        assert!(
            !h.contains_key("access-control-allow-origin"),
            "{method} {uri}"
        );
    }
}

#[tokio::test]
async fn requests_are_counted_per_route() {
    let state = offline_state();
    for uri in ["/health", "/health", "/health/ready", "/nowhere"] {
        get(state.clone(), uri).await;
    }
    let scrape = cyex_tv_shared::metrics::router(state.metrics.clone(), None)
        .oneshot(Request::get("/metrics").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(scrape.status(), StatusCode::OK);
    assert!(
        scrape.headers()["content-type"]
            .to_str()
            .unwrap()
            .starts_with("text/plain; version=0.0.4")
    );
    let body = scrape.into_body().collect().await.unwrap().to_bytes();
    let text = String::from_utf8(body.to_vec()).unwrap();
    assert!(
        text.contains(
            r#"cyex_tv_http_requests_total{method="GET",route="/health",status="200"} 2"#
        ),
        "{text}"
    );
    assert!(
        text.contains(
            r#"cyex_tv_http_requests_total{method="GET",route="/health/ready",status="503"} 1"#
        ),
        "{text}"
    );
    assert!(
        !text.contains("nowhere"),
        "unrouted paths make no series: {text}"
    );
    assert!(
        text.contains(r#"cyex_tv_http_request_duration_seconds_count{method="GET",route="/health",status="200"} 2"#),
        "{text}"
    );

    let gauges = cyex_tv_api::scrape_gauges(&state).await;
    assert!(gauges.contains("cyex_tv_hub_subscribers 0\n"), "{gauges}");
    assert!(
        gauges.contains("# TYPE cyex_tv_db_pool_connections gauge"),
        "{gauges}"
    );
}

fn quota_of(headers: &axum::http::HeaderMap) -> Option<(u32, u32, u64)> {
    let n = |name: &str| headers.get(name)?.to_str().ok()?.parse::<u64>().ok();
    Some((
        n("ratelimit-limit")? as u32,
        n("ratelimit-remaining")? as u32,
        n("ratelimit-reset")?,
    ))
}

#[tokio::test]
async fn reads_and_graphql_publish_their_limit_and_a_429_says_when_to_retry() {
    let tiny = cyex_tv_shared::ratelimit::Rate::per_minute(2);
    let rates = cyex_tv_api::limits::Limits::new(
        cyex_tv_api::limits::Rates {
            reads: tiny,
            gql: tiny,
            auth: tiny,
            ..cyex_tv_api::limits::Rates::default()
        },
        false,
    );
    let app = cyex_tv_api::app(offline_state().with_limits(rates));
    let read = || Request::get("/v1/sets/global").body(Body::empty()).unwrap();
    let gql = || {
        Request::post("/v1/gql")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"query":"{ __typename }"}"#))
            .unwrap()
    };
    for (name, make) in [("read", &read as &dyn Fn() -> Request<Body>), ("gql", &gql)] {
        let first = app.clone().oneshot(make()).await.unwrap();
        let (limit, left, reset) = quota_of(first.headers()).unwrap_or_else(|| panic!("{name}"));
        assert_eq!((limit, left), (2, 1), "{name}");
        assert!((1..=60).contains(&reset), "{name} {reset}");
        let second = app.clone().oneshot(make()).await.unwrap();
        assert_eq!(quota_of(second.headers()).unwrap().1, 0, "{name}");
        let third = app.clone().oneshot(make()).await.unwrap();
        assert_eq!(third.status(), StatusCode::TOO_MANY_REQUESTS, "{name}");
        assert!(
            third.headers()["retry-after"]
                .to_str()
                .unwrap()
                .parse::<u64>()
                .unwrap()
                >= 1
        );
        assert_eq!(quota_of(third.headers()).unwrap().0, 2, "{name}");
        assert_eq!(quota_of(third.headers()).unwrap().1, 0, "{name}");
    }
    // Sign-in is limited too but publishes no quota; its 429 still says when to retry.
    for _ in 0..2 {
        app.clone()
            .oneshot(
                Request::get("/v1/auth/twitch/login")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
    }
    let r = app
        .oneshot(
            Request::get("/v1/auth/twitch/login")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::TOO_MANY_REQUESTS);
    assert!(r.headers().contains_key("retry-after"));
    assert!(quota_of(r.headers()).is_none());
}

#[tokio::test]
async fn meta_has_version_build_limits_link_and_deprecations() {
    let response = cyex_tv_api::app(offline_state())
        .oneshot(Request::get("/v1/meta").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers()["x-cyex-api-version"],
        env!("CARGO_PKG_VERSION")
    );
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let meta: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(meta["api_version"], env!("CARGO_PKG_VERSION"));
    assert!(meta["build"].as_str().is_some_and(|b| !b.is_empty()));
    assert_eq!(meta["limits"]["rest"], "/v1/limits");
    assert_eq!(meta["limits"]["graphql_field"], "limits");
    let ulid = &meta["deprecations"][0];
    assert_eq!(ulid["id"], "ulid_id_input");
    assert_eq!(ulid["since"], "2026-11-26");
    assert_eq!(ulid["sunset"], "2027-06-01");
    for key in ["what", "replacement"] {
        assert!(ulid[key].as_str().is_some_and(|t| !t.is_empty()), "{key}");
    }
}

#[tokio::test]
async fn graphql_meta_matches_rest_meta() {
    let r = gql(
        "{ meta { apiVersion build limits { rest graphqlField } deprecations { id what since sunset replacement } } }",
    )
    .await;
    assert!(r["errors"].is_null(), "{r}");
    let meta = &r["data"]["meta"];
    assert_eq!(meta["apiVersion"], env!("CARGO_PKG_VERSION"));
    assert_eq!(meta["limits"]["graphqlField"], "limits");
    assert_eq!(meta["deprecations"][0]["id"], "ulid_id_input");
    assert_eq!(meta["deprecations"][0]["sunset"], "2027-06-01");
}

#[tokio::test]
async fn every_answer_carries_the_api_version_and_no_deprecation_by_default() {
    for request in [
        Request::get("/health").body(Body::empty()).unwrap(),
        Request::get("/v1/nothing-here")
            .body(Body::empty())
            .unwrap(),
        Request::post("/v1/gql")
            .header("content-type", "application/json")
            .body(Body::from(
                json!({ "query": "{ meta { build } }" }).to_string(),
            ))
            .unwrap(),
    ] {
        let response = cyex_tv_api::app(offline_state())
            .oneshot(request)
            .await
            .unwrap();
        let headers = response.headers();
        assert_eq!(headers["x-cyex-api-version"], env!("CARGO_PKG_VERSION"));
        assert!(!headers.contains_key("deprecation") && !headers.contains_key("sunset"));
    }
}

#[tokio::test]
async fn cors_exposes_the_version_and_deprecation_headers() {
    let response = cyex_tv_api::app(offline_state())
        .oneshot(
            Request::get("/v1/meta")
                .header("origin", "https://example.org")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let exposed = response.headers()["access-control-expose-headers"]
        .to_str()
        .unwrap()
        .to_owned();
    for name in ["x-cyex-api-version", "deprecation", "sunset", "link"] {
        assert!(exposed.contains(name), "{name} in {exposed}");
    }
}

#[tokio::test]
async fn limits_read_is_cached_long_and_varies_only_by_encoding() {
    let response = cyex_tv_api::app(offline_state())
        .oneshot(Request::get("/v1/limits").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers()["cache-control"],
        "public, max-age=300, stale-while-revalidate=600, stale-if-error=86400"
    );
    assert_eq!(response.headers()["vary"], "Accept-Encoding");
    assert!(response.headers().contains_key("etag"));
}

#[tokio::test]
async fn meta_lists_the_scale_deprecation_for_badge_and_paint_files() {
    let response = cyex_tv_api::app(offline_state())
        .oneshot(Request::get("/v1/meta").body(Body::empty()).unwrap())
        .await
        .unwrap();
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let meta: Value = serde_json::from_slice(&body).unwrap();
    let entry = meta["deprecations"]
        .as_array()
        .unwrap()
        .iter()
        .find(|d| d["id"] == "file_scale_non_emote")
        .expect("the scale deprecation is listed");
    assert_eq!(entry["since"], "2026-11-26");
    assert!(entry["replacement"].as_str().unwrap().contains("width"));
    // The schema says the same, and emote files are not named as going away.
    let sdl = cyex_tv_api::gql::sdl();
    let line = sdl
        .lines()
        .find(|l| l.trim_start().starts_with("scale: Int!"))
        .expect("EmoteFile.scale");
    assert!(line.contains("@deprecated"), "{line}");
    assert!(line.contains("Emote files keep"), "{line}");
}

/// The properties a schema has, following one `allOf`.
fn schema_properties(spec: &Value, name: &str) -> std::collections::BTreeSet<String> {
    let schema = &spec["components"]["schemas"][name];
    assert!(schema.is_object(), "no schema {name}");
    let mut found = std::collections::BTreeSet::new();
    let mut parts = vec![schema.clone()];
    while let Some(part) = parts.pop() {
        if let Some(reference) = part["$ref"].as_str() {
            let target = reference.rsplit('/').next().unwrap();
            parts.push(spec["components"]["schemas"][target].clone());
        }
        for key in part["allOf"].as_array().into_iter().flatten() {
            parts.push(key.clone());
        }
        for key in part["properties"]
            .as_object()
            .into_iter()
            .flat_map(|o| o.keys())
        {
            found.insert(key.clone());
        }
    }
    found
}

#[test]
fn both_upload_responses_carry_the_one_upload_result_shape() {
    let spec = serde_json::to_value(cyex_tv_api::openapi()).unwrap();
    let common = schema_properties(&spec, "UploadResult");
    let required: Vec<&str> = spec["components"]["schemas"]["UploadResult"]["required"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    assert_eq!(required, ["id", "state", "files", "limits_used"]);
    for response in ["EmoteUploaded", "CosmeticImageUploaded"] {
        let own = schema_properties(&spec, response);
        for field in &required {
            assert!(own.contains(*field), "{response} lacks {field}");
        }
        assert!(common.is_subset(&own), "{response}");
    }
    // Both routes answer with them, and share the documented error statuses.
    let status_list = |path: &str| -> Vec<String> {
        spec["paths"][path]["post"]["responses"]
            .as_object()
            .unwrap()
            .keys()
            .cloned()
            .collect()
    };
    let (mut emote, mut picture) = (
        status_list("/v1/emotes"),
        status_list("/v1/cosmetics/images"),
    );
    emote.sort();
    picture.sort();
    assert_eq!(emote, picture);
    for (path, schema) in [
        ("/v1/emotes", "EmoteUploaded"),
        ("/v1/cosmetics/images", "CosmeticImageUploaded"),
    ] {
        let reference = spec["paths"][path]["post"]["responses"]["202"]["content"]
            ["application/json"]["schema"]["$ref"]
            .as_str()
            .unwrap();
        assert!(reference.ends_with(schema), "{path}: {reference}");
    }
}
