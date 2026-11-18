//! Signing in end to end against fake Twitch and Kick servers that answer with the fixtures in
//! `tests/fixtures`. Each test gets its own database; they run when `CYEX_TV_TEST_DATABASE_URL`
//! is set (CI sets it) and skip otherwise.
//!
//! The fake platforms take the user ID as the authorization `code`, so a test chooses who signs
//! in by the code it sends back. The code `bad` makes the token exchange fail.

mod common;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::body::Body;
use axum::extract::{Form, Query, State};
use axum::http::{HeaderMap, Request, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use common::TestDb;
use cyex_tv_api::state::AppState;
use cyex_tv_auth::config::{ClientCreds, ProviderConfig};
use cyex_tv_auth::providers::Endpoints;
use cyex_tv_auth::{Auth, AuthConfig, tokens};
use cyex_tv_db::sessions::{self, SessionKind};
use cyex_tv_shared::model::Platform;
use http_body_util::BodyExt;
use serde_json::{Value, json};
use tower::ServiceExt;
use url::Url;

// The fake platforms

#[derive(Default)]
struct Recorded {
    token_forms: Vec<(Platform, HashMap<String, String>)>,
    revoked: Vec<(Platform, String)>,
}

type Shared = Arc<Mutex<Recorded>>;

fn fixture(name: &str) -> Value {
    let path = format!("{}/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"));
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

fn token_answer(platform: Platform, code: &str) -> Value {
    let mut token = fixture(&format!("{platform}_token.json"));
    token["access_token"] = json!(format!("{platform}-token-{code}"));
    token
}

/// The user ID a fake access token was issued for.
fn user_of(headers: &HeaderMap, platform: Platform) -> Option<String> {
    let auth = headers.get("authorization")?.to_str().ok()?;
    auth.strip_prefix(&format!("Bearer {platform}-token-"))
        .map(str::to_owned)
}

fn exchange(shared: &Shared, platform: Platform, form: HashMap<String, String>) -> Response {
    shared
        .lock()
        .unwrap()
        .token_forms
        .push((platform, form.clone()));
    let secret_ok = form.get("client_secret").map(String::as_str)
        == Some(match platform {
            Platform::Twitch => "twitch-secret",
            Platform::Kick => "kick-secret",
        });
    let code = form.get("code").cloned().unwrap_or_default();
    if !secret_ok
        || code == "bad"
        || form.get("grant_type").map(String::as_str) != Some("authorization_code")
    {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "status": 400, "message": "Invalid authorization code" })),
        )
            .into_response();
    }
    if platform == Platform::Kick && !form.contains_key("code_verifier") {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "invalid_request", "error_description": "code_verifier required" })),
        )
            .into_response();
    }
    Json(token_answer(platform, &code)).into_response()
}

fn fake_platforms(shared: Shared) -> Router {
    Router::new()
        .route(
            "/twitch/oauth2/token",
            post(
                |State(s): State<Shared>, Form(f): Form<HashMap<String, String>>| async move {
                    exchange(&s, Platform::Twitch, f)
                },
            ),
        )
        .route(
            "/twitch/helix/users",
            get(|headers: HeaderMap| async move {
                let client_ok = headers.get("client-id").is_some_and(|v| v == "twitch-id");
                let Some(id) = user_of(&headers, Platform::Twitch).filter(|_| client_ok) else {
                    return StatusCode::UNAUTHORIZED.into_response();
                };
                let mut body = fixture("twitch_users.json");
                body["data"][0]["id"] = json!(id);
                body["data"][0]["login"] = json!(format!("twitch{id}"));
                body["data"][0]["display_name"] = json!(format!("Twitch{id}"));
                Json(body).into_response()
            }),
        )
        .route(
            "/twitch/oauth2/revoke",
            post(
                |State(s): State<Shared>, Form(f): Form<HashMap<String, String>>| async move {
                    let token = f.get("token").cloned().unwrap_or_default();
                    s.lock().unwrap().revoked.push((Platform::Twitch, token));
                    StatusCode::OK
                },
            ),
        )
        .route(
            "/kick/oauth/token",
            post(
                |State(s): State<Shared>, Form(f): Form<HashMap<String, String>>| async move {
                    exchange(&s, Platform::Kick, f)
                },
            ),
        )
        .route(
            "/kick/public/v1/users",
            get(|headers: HeaderMap| async move {
                let Some(id) = user_of(&headers, Platform::Kick) else {
                    return StatusCode::UNAUTHORIZED.into_response();
                };
                let mut body = fixture("kick_users.json");
                body["data"][0]["user_id"] = json!(id.parse::<i64>().unwrap());
                body["data"][0]["name"] = json!(format!("Kick{id}"));
                Json(body).into_response()
            }),
        )
        .route(
            "/kick/oauth/revoke",
            post(
                |State(s): State<Shared>, Query(q): Query<HashMap<String, String>>| async move {
                    let token = q.get("token").cloned().unwrap_or_default();
                    s.lock().unwrap().revoked.push((Platform::Kick, token));
                    StatusCode::OK
                },
            ),
        )
        .with_state(shared)
}

// The harness

struct Reply {
    status: StatusCode,
    headers: HeaderMap,
    body: Value,
}

impl Reply {
    fn code(&self) -> &str {
        self.body["error"].as_str().unwrap_or_default()
    }

    fn set_cookies(&self) -> Vec<String> {
        self.headers
            .get_all("set-cookie")
            .iter()
            .map(|v| v.to_str().unwrap().to_owned())
            .collect()
    }

    /// The value a `Set-Cookie` gives cookie `name`, if one does.
    fn cookie(&self, name: &str) -> Option<String> {
        self.set_cookies().iter().find_map(|line| {
            let first = line.split(';').next()?;
            let (key, value) = first.split_once('=')?;
            (key == name).then(|| value.to_owned())
        })
    }

    fn location(&self) -> Url {
        Url::parse(self.headers["location"].to_str().unwrap()).unwrap()
    }
}

fn query_param(url: &Url, name: &str) -> Option<String> {
    url.query_pairs()
        .find(|(k, _)| k == name)
        .map(|(_, v)| v.into_owned())
}

struct Harness {
    db: TestDb,
    app: Router,
    recorded: Shared,
}

/// The cookies a browser would send: the pre-login state and the session.
fn cookie_header(state: Option<&str>, session: Option<&str>) -> Option<String> {
    let parts: Vec<String> = [("cyex_tv_oauth", state), ("cyex_tv_session", session)]
        .into_iter()
        .filter_map(|(name, value)| value.map(|v| format!("{name}={v}")))
        .collect();
    (!parts.is_empty()).then(|| parts.join("; "))
}

impl Harness {
    async fn new() -> Option<Self> {
        let db = TestDb::new().await?;
        let recorded = Shared::default();
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = Url::parse(&format!("http://{}", listener.local_addr().unwrap())).unwrap();
        let platforms = fake_platforms(recorded.clone());
        tokio::spawn(async move { axum::serve(listener, platforms).await });

        let mut cfg = AuthConfig::from_source(|_| None).unwrap();
        let client = |id: &str, secret: &str, platform| ProviderConfig {
            creds: ClientCreds {
                client_id: id.to_owned(),
                client_secret: secret.to_owned(),
            },
            endpoints: Endpoints::at(&base, platform),
        };
        cfg.twitch = Some(client("twitch-id", "twitch-secret", Platform::Twitch));
        cfg.kick = Some(client("kick-id", "kick-secret", Platform::Kick));
        let state = AppState::new(db.pool.clone(), Arc::new(Auth::new(cfg).unwrap()));
        Some(Self {
            app: cyex_tv_api::app(state),
            db,
            recorded,
        })
    }

    async fn send(&self, request: Request<Body>) -> Reply {
        let response = self.app.clone().oneshot(request).await.unwrap();
        let status = response.status();
        let headers = response.headers().clone();
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        let body = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
        Reply {
            status,
            headers,
            body,
        }
    }

    async fn get(&self, uri: &str, cookies: Option<String>) -> Reply {
        let mut request = Request::get(uri);
        if let Some(cookies) = cookies {
            request = request.header("cookie", cookies);
        }
        self.send(request.body(Body::empty()).unwrap()).await
    }

    /// `GET /login`: returns the platform URL the browser is sent to and the state cookie.
    async fn start(&self, platform: Platform, query: &str, session: Option<&str>) -> (Url, String) {
        let reply = self
            .get(
                &format!("/v1/auth/{platform}/login?{query}"),
                cookie_header(None, session),
            )
            .await;
        assert_eq!(reply.status, StatusCode::FOUND, "{:?}", reply.body);
        let state_cookie = reply.cookie("cyex_tv_oauth").expect("state cookie");
        (reply.location(), state_cookie)
    }

    async fn callback(
        &self,
        platform: Platform,
        query: &str,
        state_cookie: Option<&str>,
        session: Option<&str>,
    ) -> Reply {
        self.get(
            &format!("/v1/auth/{platform}/callback?{query}"),
            cookie_header(state_cookie, session),
        )
        .await
    }

    /// A whole sign-in (or link, with `link`) as `user_id`. Returns the reply to the callback.
    async fn sign_in_reply(
        &self,
        platform: Platform,
        user_id: &str,
        session: Option<&str>,
        link: bool,
    ) -> Reply {
        let query = if link { "link=true" } else { "" };
        let (authorize, state_cookie) = self.start(platform, query, session).await;
        let state = query_param(&authorize, "state").unwrap();
        self.callback(
            platform,
            &format!("code={user_id}&state={state}"),
            Some(&state_cookie),
            session,
        )
        .await
    }

    /// A whole sign-in that must succeed. Returns the new session token.
    async fn sign_in(
        &self,
        platform: Platform,
        user_id: &str,
        session: Option<&str>,
        link: bool,
    ) -> String {
        let reply = self.sign_in_reply(platform, user_id, session, link).await;
        assert_eq!(reply.status, StatusCode::FOUND, "{:?}", reply.body);
        reply.cookie("cyex_tv_session").expect("session cookie")
    }

    /// A GraphQL request from the site, with the session cookie if given.
    async fn gql(&self, query: &str, session: Option<&str>) -> Value {
        let mut request = Request::post("/v1/gql")
            .header("content-type", "application/json")
            .header("origin", "https://cyex.tv");
        if let Some(session) = session {
            request = request.header("cookie", format!("cyex_tv_session={session}"));
        }
        let request = request
            .body(Body::from(json!({ "query": query }).to_string()))
            .unwrap();
        let reply = self.send(request).await;
        assert_eq!(reply.status, StatusCode::OK, "{:?}", reply.body);
        reply.body
    }

    async fn me(&self, session: &str) -> Value {
        self.gql(
            "{ me { id identities { platform platformUserId login displayName avatar } permissions createdAt } }",
            Some(session),
        )
        .await
    }

    async fn sql(&self, statement: &str) {
        let mut conn = self.db.conn().await;
        sqlx::query(statement).execute(&mut *conn).await.unwrap();
    }

    /// Waits for the spawned token revoke to reach the fake platform.
    async fn wait_for_revoke(&self, platform: Platform, token: &str) {
        for _ in 0..100 {
            if self
                .recorded
                .lock()
                .unwrap()
                .revoked
                .iter()
                .any(|(p, t)| *p == platform && t == token)
            {
                return;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        panic!("{platform} token {token} was never revoked");
    }
}

fn gql_code(response: &Value) -> &str {
    response["errors"][0]["extensions"]["code"]
        .as_str()
        .unwrap_or_default()
}

// The tests

#[tokio::test]
async fn twitch_sign_in_end_to_end() {
    let Some(h) = Harness::new().await else {
        return;
    };

    let (authorize, state_cookie) = h
        .start(Platform::Twitch, "return_to=%2Fsettings", None)
        .await;
    assert!(authorize.path().ends_with("/twitch/oauth2/authorize"));
    assert_eq!(
        query_param(&authorize, "client_id").as_deref(),
        Some("twitch-id")
    );
    assert_eq!(
        query_param(&authorize, "redirect_uri").as_deref(),
        Some("https://api.cyex.tv/v1/auth/twitch/callback")
    );
    assert_eq!(query_param(&authorize, "scope").as_deref(), Some(""));
    assert_eq!(query_param(&authorize, "code_challenge"), None);
    let state = query_param(&authorize, "state").unwrap();
    assert_eq!(state, state_cookie);

    let reply = h
        .callback(
            Platform::Twitch,
            &format!("code=141981764&state={state}"),
            Some(&state_cookie),
            None,
        )
        .await;
    assert_eq!(reply.status, StatusCode::FOUND, "{:?}", reply.body);
    assert_eq!(reply.location().as_str(), "https://cyex.tv/settings");
    assert_eq!(reply.headers["cache-control"], "no-store");
    let cookies = reply.set_cookies();
    let session_line = cookies
        .iter()
        .find(|c| c.starts_with("cyex_tv_session="))
        .unwrap();
    for part in [
        "Path=/",
        "Domain=cyex.tv",
        "Secure",
        "HttpOnly",
        "SameSite=Lax",
        "Max-Age=2592000",
    ] {
        assert!(session_line.contains(part), "{session_line} lacks {part}");
    }
    assert!(
        cookies
            .iter()
            .any(|c| c.starts_with("cyex_tv_oauth=;") && c.contains("Max-Age=0"))
    );
    let session = reply.cookie("cyex_tv_session").unwrap();
    assert!(tokens::is_well_formed(&session));

    // The token exchange sent the secret and the registered redirect URI, and no PKCE.
    {
        let recorded = h.recorded.lock().unwrap();
        let (platform, form) = recorded.token_forms.last().unwrap();
        assert_eq!(*platform, Platform::Twitch);
        assert_eq!(form["code"], "141981764");
        assert_eq!(
            form["redirect_uri"],
            "https://api.cyex.tv/v1/auth/twitch/callback"
        );
        assert!(!form.contains_key("code_verifier"));
    }
    h.wait_for_revoke(Platform::Twitch, "twitch-token-141981764")
        .await;

    let me = h.me(&session).await;
    let identities = me["data"]["me"]["identities"].as_array().unwrap();
    assert_eq!(identities.len(), 1);
    assert_eq!(identities[0]["platform"], "TWITCH");
    assert_eq!(identities[0]["platformUserId"], "141981764");
    assert_eq!(identities[0]["login"], "twitch141981764");
    assert_eq!(identities[0]["displayName"], "Twitch141981764");
    assert!(
        identities[0]["avatar"]
            .as_str()
            .unwrap()
            .starts_with("https://")
    );
    assert_eq!(me["data"]["me"]["permissions"], json!([]));

    // Only the hash of the session token is stored.
    let mut conn = h.db.conn().await;
    let stored = sessions::find_active(&mut conn, &tokens::hash(&session))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(stored.kind, SessionKind::Site);
    let raw: Option<Vec<u8>> =
        sqlx::query_scalar("SELECT id_hash FROM sessions WHERE id_hash = $1")
            .bind(session.as_bytes())
            .fetch_optional(&mut *conn)
            .await
            .unwrap();
    assert_eq!(raw, None);
    drop(conn);

    // Signing in made the user's channel.
    let channels = h
        .gql(
            "{ myChannels { platform platformChannelId login } }",
            Some(&session),
        )
        .await;
    assert_eq!(
        channels["data"]["myChannels"],
        json!([{ "platform": "TWITCH", "platformChannelId": "141981764", "login": "twitch141981764" }])
    );

    // Signing in again reuses the account.
    let again = h.sign_in(Platform::Twitch, "141981764", None, false).await;
    assert_eq!(
        h.me(&again).await["data"]["me"]["id"],
        me["data"]["me"]["id"]
    );
    h.db.close().await;
}

#[tokio::test]
async fn kick_sign_in_uses_pkce_and_never_stores_email() {
    let Some(h) = Harness::new().await else {
        return;
    };

    let (authorize, state_cookie) = h.start(Platform::Kick, "", None).await;
    assert!(authorize.path().ends_with("/kick/oauth/authorize"));
    assert_eq!(
        query_param(&authorize, "scope").as_deref(),
        Some("user:read")
    );
    assert_eq!(
        query_param(&authorize, "code_challenge_method").as_deref(),
        Some("S256")
    );
    let challenge = query_param(&authorize, "code_challenge").unwrap();
    let state = query_param(&authorize, "state").unwrap();

    let reply = h
        .callback(
            Platform::Kick,
            &format!("code=4377088&state={state}"),
            Some(&state_cookie),
            None,
        )
        .await;
    assert_eq!(reply.status, StatusCode::FOUND, "{:?}", reply.body);
    assert_eq!(reply.location().as_str(), "https://cyex.tv/");

    let verifier = {
        let recorded = h.recorded.lock().unwrap();
        let (platform, form) = recorded.token_forms.last().unwrap();
        assert_eq!(*platform, Platform::Kick);
        assert_eq!(form["client_secret"], "kick-secret");
        form["code_verifier"].clone()
    };
    assert_eq!(tokens::pkce_challenge(&verifier), challenge);
    // The verifier was never in the browser's hands.
    assert!(!authorize.as_str().contains(&verifier));
    assert_ne!(verifier, state_cookie);
    h.wait_for_revoke(Platform::Kick, "kick-token-4377088")
        .await;

    let session = reply.cookie("cyex_tv_session").unwrap();
    let me = h.me(&session).await;
    let identity = &me["data"]["me"]["identities"][0];
    assert_eq!(identity["platform"], "KICK");
    assert_eq!(identity["platformUserId"], "4377088");
    assert_eq!(identity["login"], "Kick4377088");

    let mut conn = h.db.conn().await;
    let leaked: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM identities WHERE row_to_json(identities)::text LIKE '%example.com%'",
    )
    .fetch_one(&mut *conn)
    .await
    .unwrap();
    assert_eq!(leaked, 0);
    drop(conn);
    h.db.close().await;
}

#[tokio::test]
async fn linking_both_platforms_to_one_account() {
    let Some(h) = Harness::new().await else {
        return;
    };

    let twitch_session = h.sign_in(Platform::Twitch, "1001", None, false).await;
    let account = h.me(&twitch_session).await["data"]["me"]["id"].clone();

    let linked_session = h
        .sign_in(Platform::Kick, "2002", Some(&twitch_session), true)
        .await;
    let me = h.me(&linked_session).await;
    assert_eq!(me["data"]["me"]["id"], account);
    let platforms: Vec<&str> = me["data"]["me"]["identities"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| i["platform"].as_str().unwrap())
        .collect();
    assert_eq!(platforms, ["TWITCH", "KICK"]);

    // Linking rotated the session: the old token is dead.
    assert_eq!(gql_code(&h.me(&twitch_session).await), "signed_out");

    // Signing in with Kick now lands on the same account.
    let via_kick = h.sign_in(Platform::Kick, "2002", None, false).await;
    assert_eq!(h.me(&via_kick).await["data"]["me"]["id"], account);

    // Linking Kick again with a different Kick user: one Kick per account.
    let reply = h
        .sign_in_reply(Platform::Kick, "2003", Some(&via_kick), true)
        .await;
    assert_eq!(reply.status, StatusCode::CONFLICT);
    assert_eq!(reply.code(), "platform_already_linked");
    h.db.close().await;
}

#[tokio::test]
async fn linking_someone_elses_identity_is_refused() {
    let Some(h) = Harness::new().await else {
        return;
    };

    let _a = h.sign_in(Platform::Twitch, "3001", None, false).await;
    let b = h.sign_in(Platform::Kick, "3002", None, false).await;

    let reply = h
        .sign_in_reply(Platform::Twitch, "3001", Some(&b), true)
        .await;
    assert_eq!(reply.status, StatusCode::CONFLICT);
    assert_eq!(reply.code(), "identity_taken");
    // B keeps one identity and stays signed in.
    let me = h.me(&b).await;
    assert_eq!(me["data"]["me"]["identities"].as_array().unwrap().len(), 1);
    h.db.close().await;
}

#[tokio::test]
async fn link_needs_a_session() {
    let Some(h) = Harness::new().await else {
        return;
    };
    let reply = h.get("/v1/auth/kick/login?link=true", None).await;
    assert_eq!(reply.status, StatusCode::UNAUTHORIZED);
    assert_eq!(reply.code(), "signed_out");

    // A link started by one session can't be finished by another.
    let a = h.sign_in(Platform::Twitch, "4001", None, false).await;
    let b = h.sign_in(Platform::Twitch, "4002", None, false).await;
    let (authorize, state_cookie) = h.start(Platform::Kick, "link=true", Some(&a)).await;
    let state = query_param(&authorize, "state").unwrap();
    let reply = h
        .callback(
            Platform::Kick,
            &format!("code=4003&state={state}"),
            Some(&state_cookie),
            Some(&b),
        )
        .await;
    assert_eq!(reply.status, StatusCode::BAD_REQUEST);
    assert_eq!(reply.code(), "bad_state");
    h.db.close().await;
}

#[tokio::test]
async fn state_must_match_the_cookie_and_is_single_use() {
    let Some(h) = Harness::new().await else {
        return;
    };

    let (authorize, state_cookie) = h.start(Platform::Twitch, "", None).await;
    let state = query_param(&authorize, "state").unwrap();
    let query = format!("code=5001&state={state}");

    // No cookie (a callback link opened in another browser), or someone else's cookie.
    let other = tokens::random_token();
    for cookie in [None, Some(other.as_str())] {
        let reply = h.callback(Platform::Twitch, &query, cookie, None).await;
        assert_eq!(reply.status, StatusCode::BAD_REQUEST, "{cookie:?}");
        assert_eq!(reply.code(), "bad_state");
    }
    // Those failed attempts used the state up; the real browser can't finish either.
    let reply = h
        .callback(Platform::Twitch, &query, Some(&state_cookie), None)
        .await;
    assert_eq!(reply.code(), "bad_state");

    // A state from Twitch can't finish a Kick sign-in.
    let (authorize, state_cookie) = h.start(Platform::Twitch, "", None).await;
    let state = query_param(&authorize, "state").unwrap();
    let reply = h
        .callback(
            Platform::Kick,
            &format!("code=5002&state={state}"),
            Some(&state_cookie),
            None,
        )
        .await;
    assert_eq!(reply.code(), "bad_state");

    // Replaying a finished callback finds nothing.
    let (authorize, state_cookie) = h.start(Platform::Twitch, "", None).await;
    let state = query_param(&authorize, "state").unwrap();
    let query = format!("code=5003&state={state}");
    let first = h
        .callback(Platform::Twitch, &query, Some(&state_cookie), None)
        .await;
    assert_eq!(first.status, StatusCode::FOUND);
    let replay = h
        .callback(Platform::Twitch, &query, Some(&state_cookie), None)
        .await;
    assert_eq!(replay.code(), "bad_state");

    // Missing pieces.
    for query in [
        "",
        "code=1",
        "state=x",
        &format!("code=1&state={}", "x".repeat(43)),
    ] {
        let reply = h
            .callback(Platform::Twitch, query, Some(&state_cookie), None)
            .await;
        assert_eq!(reply.code(), "bad_state", "{query:?}");
    }

    // An expired state.
    let (authorize, state_cookie) = h.start(Platform::Twitch, "", None).await;
    let state = query_param(&authorize, "state").unwrap();
    let mut conn = h.db.conn().await;
    sqlx::query("UPDATE oauth_states SET created_at = now() - interval '1 hour', expires_at = now() - interval '1 minute'")
        .execute(&mut *conn)
        .await
        .unwrap();
    drop(conn);
    let reply = h
        .callback(
            Platform::Twitch,
            &format!("code=5004&state={state}"),
            Some(&state_cookie),
            None,
        )
        .await;
    assert_eq!(reply.code(), "bad_state");
    h.db.close().await;
}

#[tokio::test]
async fn declined_and_platform_errors() {
    let Some(h) = Harness::new().await else {
        return;
    };

    let (authorize, state_cookie) = h.start(Platform::Kick, "", None).await;
    let state = query_param(&authorize, "state").unwrap();
    let reply = h
        .callback(
            Platform::Kick,
            &format!("error=access_denied&error_description=The+user+denied&state={state}"),
            Some(&state_cookie),
            None,
        )
        .await;
    assert_eq!(reply.status, StatusCode::BAD_REQUEST);
    assert_eq!(reply.code(), "declined");
    // Declining used the state up.
    let reply = h
        .callback(
            Platform::Kick,
            &format!("code=6001&state={state}"),
            Some(&state_cookie),
            None,
        )
        .await;
    assert_eq!(reply.code(), "bad_state");

    let reply = h.sign_in_reply(Platform::Twitch, "bad", None, false).await;
    assert_eq!(reply.status, StatusCode::BAD_GATEWAY);
    assert_eq!(reply.code(), "platform_error");
    // What the platform said stays in the logs, not in the answer.
    assert!(!reply.body["message"].as_str().unwrap().contains("Invalid"));
    h.db.close().await;
}

#[tokio::test]
async fn return_to_must_be_the_site() {
    let Some(h) = Harness::new().await else {
        return;
    };
    for bad in [
        "https%3A%2F%2Fevil.example%2F",
        "%2F%2Fevil.example",
        "https%3A%2F%2Fapi.cyex.tv%2F",
    ] {
        let reply = h
            .get(&format!("/v1/auth/twitch/login?return_to={bad}"), None)
            .await;
        assert_eq!(reply.status, StatusCode::BAD_REQUEST, "{bad}");
        assert_eq!(reply.code(), "bad_return_to");
    }
    h.db.close().await;
}

#[tokio::test]
async fn sign_out_ends_only_that_session() {
    let Some(h) = Harness::new().await else {
        return;
    };
    let first = h.sign_in(Platform::Twitch, "7001", None, false).await;
    let second = h.sign_in(Platform::Twitch, "7001", None, false).await;

    let logout = |session: String, origin: Option<&'static str>| {
        let mut request =
            Request::post("/v1/auth/logout").header("cookie", format!("cyex_tv_session={session}"));
        if let Some(origin) = origin {
            request = request.header("origin", origin);
        }
        request.body(Body::empty()).unwrap()
    };

    // From another site: refused, and the session lives on.
    let reply = h
        .send(logout(first.clone(), Some("https://evil.example")))
        .await;
    assert_eq!(reply.status, StatusCode::FORBIDDEN);
    assert_eq!(reply.code(), "bad_origin");
    assert!(h.me(&first).await["data"]["me"]["id"].is_string());

    let reply = h.send(logout(first.clone(), Some("https://cyex.tv"))).await;
    assert_eq!(reply.status, StatusCode::NO_CONTENT);
    assert!(reply.set_cookies()[0].starts_with("cyex_tv_session=;"));
    assert_eq!(gql_code(&h.me(&first).await), "signed_out");
    assert!(h.me(&second).await["data"]["me"]["id"].is_string());
    h.db.close().await;
}

#[tokio::test]
async fn app_sessions_use_bearer_only() {
    let Some(h) = Harness::new().await else {
        return;
    };
    let site = h.sign_in(Platform::Twitch, "8001", None, false).await;
    let account = h.me(&site).await["data"]["me"]["id"].clone();
    let account_id = cyex_tv_db::ids::from_text(account.as_str().unwrap()).unwrap();

    // App sessions have no sign-in flow yet (D19); make one directly.
    let app_token = tokens::random_token();
    let mut conn = h.db.conn().await;
    sessions::create(
        &mut conn,
        &tokens::hash(&app_token),
        account_id,
        SessionKind::App,
        chrono::Duration::days(90),
    )
    .await
    .unwrap();
    drop(conn);

    let bearer = |token: &str| {
        Request::post("/v1/gql")
            .header("content-type", "application/json")
            .header("authorization", format!("Bearer {token}"))
            // No Origin: Bearer callers don't need one.
            .body(Body::from(json!({ "query": "{ me { id } }" }).to_string()))
            .unwrap()
    };
    let reply = h.send(bearer(&app_token)).await;
    assert_eq!(reply.body["data"]["me"]["id"], account);

    // A site token isn't a Bearer token, and an app token isn't a cookie.
    let reply = h.send(bearer(&site)).await;
    assert_eq!(gql_code(&reply.body), "signed_out");
    assert_eq!(gql_code(&h.me(&app_token).await), "signed_out");
    h.db.close().await;
}

#[tokio::test]
async fn unlinking_keeps_the_last_identity() {
    let Some(h) = Harness::new().await else {
        return;
    };
    let s = h.sign_in(Platform::Twitch, "9001", None, false).await;
    let s = h.sign_in(Platform::Kick, "9002", Some(&s), true).await;

    let reply = h
        .gql(
            "mutation { unlinkIdentity(platform: KICK) { identities { platform } } }",
            Some(&s),
        )
        .await;
    assert_eq!(
        reply["data"]["unlinkIdentity"]["identities"],
        json!([{ "platform": "TWITCH" }])
    );
    let reply = h
        .gql(
            "mutation { unlinkIdentity(platform: KICK) { id } }",
            Some(&s),
        )
        .await;
    assert_eq!(gql_code(&reply), "not_found");
    // The Kick channel went with the Kick identity.
    let channels = h.gql("{ myChannels { platform } }", Some(&s)).await;
    assert_eq!(
        channels["data"]["myChannels"],
        json!([{ "platform": "TWITCH" }])
    );
    let reply = h
        .gql(
            "mutation { unlinkIdentity(platform: TWITCH) { id } }",
            Some(&s),
        )
        .await;
    assert_eq!(gql_code(&reply), "last_identity");

    // The unlinked Kick user can sign in again, as a new account.
    let kick = h.sign_in(Platform::Kick, "9002", None, false).await;
    assert_ne!(
        h.me(&kick).await["data"]["me"]["id"],
        h.me(&s).await["data"]["me"]["id"]
    );
    h.db.close().await;
}

#[tokio::test]
async fn restricted_accounts_cant_unlink() {
    let Some(h) = Harness::new().await else {
        return;
    };
    let s = h.sign_in(Platform::Twitch, "9201", None, false).await;
    let s = h.sign_in(Platform::Kick, "9202", Some(&s), true).await;
    for column in ["restricted_at", "blocked_at"] {
        h.sql(&format!("UPDATE accounts SET {column} = now()"))
            .await;
        let reply = h
            .gql(
                "mutation { unlinkIdentity(platform: KICK) { id } }",
                Some(&s),
            )
            .await;
        if column == "blocked_at" {
            // A block ends the session too; either way nothing is unlinked.
            assert!(
                matches!(gql_code(&reply), "restricted" | "signed_out"),
                "{reply}"
            );
        } else {
            assert_eq!(gql_code(&reply), "restricted", "{reply}");
            assert_eq!(
                reply["errors"][0]["message"],
                "A restricted account can't unlink a platform. Write to us if you need it changed."
            );
        }
        h.sql(&format!("UPDATE accounts SET {column} = NULL")).await;
    }
    let mut conn = h.db.conn().await;
    let kick: i64 = sqlx::query_scalar("SELECT count(*) FROM identities WHERE platform = 'kick'")
        .fetch_one(&mut *conn)
        .await
        .unwrap();
    assert_eq!(kick, 1);
    h.db.close().await;
}

#[tokio::test]
async fn a_dropped_identity_brings_its_restriction_to_a_new_account() {
    let Some(h) = Harness::new().await else {
        return;
    };
    let s = h.sign_in(Platform::Twitch, "9301", None, false).await;
    let _ = h.sign_in(Platform::Kick, "9302", Some(&s), true).await;
    // The identity leaves the restricted account by any path (here straight in SQL): a hold is
    // kept.
    h.sql("UPDATE accounts SET restricted_at = now()").await;
    h.sql("DELETE FROM identities WHERE platform = 'kick'")
        .await;
    let mut conn = h.db.conn().await;
    let holds: i64 = sqlx::query_scalar("SELECT count(*) FROM identity_holds")
        .fetch_one(&mut *conn)
        .await
        .unwrap();
    assert_eq!(holds, 1);
    drop(conn);

    // Signing in with it makes a new account that is restricted too.
    let fresh = h.sign_in(Platform::Kick, "9302", None, false).await;
    let fresh_id = h.me(&fresh).await["data"]["me"]["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let mut conn = h.db.conn().await;
    let restricted: bool = sqlx::query_scalar(
        "SELECT a.restricted_at IS NOT NULL FROM accounts a
           JOIN identities i ON i.account_id = a.id
          WHERE i.platform = 'kick' AND i.platform_user_id = '9302'",
    )
    .fetch_one(&mut *conn)
    .await
    .unwrap();
    assert!(restricted, "{fresh_id}");
    let holds: i64 = sqlx::query_scalar("SELECT count(*) FROM identity_holds")
        .fetch_one(&mut *conn)
        .await
        .unwrap();
    assert_eq!(holds, 0, "the hold is used up");
    drop(conn);

    // A blocked account's identity: the next sign-in with it is refused.
    let b = h.sign_in(Platform::Twitch, "9401", None, false).await;
    let _ = h.sign_in(Platform::Kick, "9402", Some(&b), true).await;
    h.sql(
        "UPDATE accounts SET blocked_at = now() WHERE id = (SELECT account_id FROM identities
          WHERE platform = 'twitch' AND platform_user_id = '9401')",
    )
    .await;
    h.sql("DELETE FROM identities WHERE platform = 'kick' AND platform_user_id = '9402'")
        .await;
    let reply = h.sign_in_reply(Platform::Kick, "9402", None, false).await;
    assert_eq!(reply.status, StatusCode::FORBIDDEN);
    assert_eq!(reply.code(), "account_blocked");

    // Linking a held identity onto a clean account copies the restriction there.
    let c = h.sign_in(Platform::Twitch, "9501", None, false).await;
    h.sql(
        "INSERT INTO identity_holds (platform, platform_user_id, account_id, restricted_at)
         VALUES ('kick', '9502', gen_random_uuid(), now())",
    )
    .await;
    let c = h.sign_in(Platform::Kick, "9502", Some(&c), true).await;
    let id = h.me(&c).await["data"]["me"]["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let mut conn = h.db.conn().await;
    let restricted: bool = sqlx::query_scalar(
        "SELECT restricted_at IS NOT NULL FROM accounts a
           JOIN identities i ON i.account_id = a.id
          WHERE i.platform = 'twitch' AND i.platform_user_id = '9501'",
    )
    .fetch_one(&mut *conn)
    .await
    .unwrap();
    assert!(restricted, "{id}");
    drop(conn);
    h.db.close().await;
}

#[tokio::test]
async fn deleted_accounts_cant_sign_in() {
    let Some(h) = Harness::new().await else {
        return;
    };
    let s = h.sign_in(Platform::Twitch, "9101", None, false).await;
    let mut conn = h.db.conn().await;
    sqlx::query("UPDATE accounts SET deleted_at = now()")
        .execute(&mut *conn)
        .await
        .unwrap();
    drop(conn);
    assert_eq!(gql_code(&h.me(&s).await), "signed_out");
    let reply = h.sign_in_reply(Platform::Twitch, "9101", None, false).await;
    assert_eq!(reply.status, StatusCode::FORBIDDEN);
    assert_eq!(reply.code(), "account_deleted");
    h.db.close().await;
}

#[tokio::test]
async fn permissions_come_from_roles() {
    let Some(h) = Harness::new().await else {
        return;
    };
    let s = h.sign_in(Platform::Twitch, "9201", None, false).await;
    let id = h.me(&s).await["data"]["me"]["id"]
        .as_str()
        .unwrap()
        .to_owned();

    // Without a role, a moderation field is forbidden rather than signed out.
    let reply = h.gql("{ reviewQueue { total } }", Some(&s)).await;
    assert_eq!(gql_code(&reply), "forbidden");

    let mut conn = h.db.conn().await;
    let role = cyex_tv_db::roles::create(
        &mut conn,
        &cyex_tv_db::roles::NewRole {
            name: "reviewers",
            description: None,
            position: 10,
            permissions: &["emotes.review"],
            created_by: None,
        },
    )
    .await
    .unwrap();
    cyex_tv_db::roles::assign(
        &mut conn,
        cyex_tv_db::ids::from_text(&id).unwrap(),
        role.id,
        None,
    )
    .await
    .unwrap();
    drop(conn);

    assert_eq!(
        h.me(&s).await["data"]["me"]["permissions"],
        json!(["emotes.review"])
    );
    // The permission passes, and the (empty) queue comes back.
    let reply = h.gql("{ reviewQueue { total } }", Some(&s)).await;
    assert_eq!(reply["data"]["reviewQueue"]["total"], 0, "{reply}");
    h.db.close().await;
}
