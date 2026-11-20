//! App sign-in (ADR 0021): the authorization code flow with PKCE and a loopback redirect, refresh
//! rotation with reuse detection, revoking, the app's session in `mySessions`, its scopes, and the
//! calls cyex deck makes with the token. Through the real router against a real Postgres; runs
//! when `CYEX_TV_TEST_DATABASE_URL` is set (CI sets it) and skips otherwise.

mod common;

use std::sync::Arc;

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use chrono::{Duration, Utc};
use common::TestDb;
use cyex_tv_api::state::AppState;
use cyex_tv_auth::{Auth, AuthConfig, cookies, tokens};
use cyex_tv_db::accounts::{self, Profile};
use cyex_tv_db::emotes::{self, NewEmote};
use cyex_tv_db::sessions::{self, SessionKind};
use cyex_tv_db::{channels, ids, sets};
use cyex_tv_shared::model::{EmoteState, Platform};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use tower::ServiceExt;
use url::Url;
use uuid::Uuid;

/// Holds every permission in these tests only (`CYEX_TV_OWNER_IDS`).
const STAFF: &str = "90001";
const SITE: &str = "https://cyex.tv";
const REDIRECT: &str = "http://127.0.0.1:53682/callback";

struct User {
    id: Uuid,
    /// A website cookie session.
    site: String,
    /// An unscoped app session, for setting things up.
    full: String,
    twitch_id: String,
}

struct World {
    db: TestDb,
    app: Router,
}

struct Answer {
    status: StatusCode,
    location: Option<String>,
    body: Value,
}

/// What the app holds after signing in.
struct AppTokens {
    access: String,
    refresh: String,
}

fn code(r: &Value) -> &str {
    r["errors"][0]["extensions"]["code"]
        .as_str()
        .unwrap_or_default()
}

fn ok(r: &Value) {
    assert!(r["errors"].is_null(), "{r}");
}

fn query_of(location: &str) -> Vec<(String, String)> {
    Url::parse(location)
        .unwrap()
        .query_pairs()
        .map(|(k, v)| (k.into_owned(), v.into_owned()))
        .collect()
}

fn param(location: &str, name: &str) -> Option<String> {
    query_of(location)
        .into_iter()
        .find(|(k, _)| k == name)
        .map(|(_, v)| v)
}

fn form(pairs: &[(&str, &str)]) -> String {
    url::form_urlencoded::Serializer::new(String::new())
        .extend_pairs(pairs)
        .finish()
}

const ME: &str = "{ me { id } }";
const SESSIONS: &str = "{ mySessions { id kind device expiresAt current appClientId } }";
const REVOKE_SESSION: &str = "mutation($id: String!) { revokeSession(id: $id) }";
const CREATE_SET: &str =
    "mutation($n: String!) { createSet(input: { name: $n, kind: CHANNEL }) { id } }";
const ACTIVATE: &str = "mutation($c: String!, $s: String) {
    updateChannelSettings(platform: TWITCH, id: $c, input: { activeSetId: $s }) { activeSetId }
}";
const ADD: &str =
    "mutation($s: String!, $e: String!) { addSetEmote(setId: $s, emoteId: $e) { name } }";
const RENAME: &str = "mutation($s: String!, $e: String!, $n: String!) {
    updateSetEmote(setId: $s, emoteId: $e, input: { name: $n }) { name }
}";
const REMOVE: &str =
    "mutation($s: String!, $e: String!) { removeSetEmote(setId: $s, emoteId: $e) }";
const EDITABLE_BY_ID: &str = "query($c: String!) {
    channelEditableSets(platform: TWITCH, id: $c) {
        channel { login activeSetId } role canEditActiveSet
        sets { activeHere access set { id name capacity slotsUsed } }
    }
}";
const EDITABLE_BY_LOGIN: &str = "query($l: String!) {
    channelEditableSets(platform: TWITCH, login: $l) { role sets { access set { id } } }
}";
const CREATE_PAINT: &str = "mutation($p: PaintInput!) { createPaint(input: $p) { id } }";
const GRANT: &str = "mutation($a: String!, $c: String!) {
    grantCosmetic(input: { accountId: $a, kind: PAINT, cosmeticId: $c, reason: \"test\" }) { cosmeticId }
}";
const MINE: &str = "{ myCosmetics { account { id } paintId paints { id } } }";
const SELECT: &str = "mutation($p: String) { selectCosmetics(paintId: $p) { paintId } }";

impl World {
    async fn new() -> Option<Self> {
        let db = TestDb::new().await?;
        let cfg = AuthConfig::from_source(|name| {
            (name == "CYEX_TV_OWNER_IDS").then(|| format!("twitch:{STAFF}"))
        })
        .unwrap();
        let state = AppState::new(db.pool.clone(), Arc::new(Auth::new(cfg).unwrap()));
        Some(Self {
            app: cyex_tv_api::app(state),
            db,
        })
    }

    /// A Twitch user `user<id>` with a channel, a website session and an unscoped app session.
    async fn user(&self, twitch_id: &str) -> User {
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
        channels::upsert(
            &mut conn,
            signed.account.id,
            Platform::Twitch,
            twitch_id,
            &login,
        )
        .await
        .unwrap();
        let site = tokens::random_token();
        sessions::create_with(
            &mut conn,
            &tokens::hash(&site),
            signed.account.id,
            SessionKind::Site,
            Duration::days(30),
            Some("Mozilla/5.0 (Windows NT 10.0) Firefox/140.0"),
        )
        .await
        .unwrap();
        let full = tokens::random_token();
        sessions::create(
            &mut conn,
            &tokens::hash(&full),
            signed.account.id,
            SessionKind::App,
            Duration::days(1),
        )
        .await
        .unwrap();
        User {
            id: signed.account.id,
            site,
            full,
            twitch_id: twitch_id.to_owned(),
        }
    }

    async fn emote(&self, owner: &User, name: &str) -> String {
        let mut conn = self.db.conn().await;
        let emote = emotes::create(
            &mut conn,
            &NewEmote {
                owner_account_id: owner.id,
                name,
                ..NewEmote::default()
            },
        )
        .await
        .unwrap();
        emotes::set_state(&mut conn, emote.id, EmoteState::Approved)
            .await
            .unwrap();
        ids::to_text(emote.id)
    }

    async fn send(&self, request: Request<Body>) -> Answer {
        let response = self.app.clone().oneshot(request).await.unwrap();
        let status = response.status();
        let location = response
            .headers()
            .get("location")
            .map(|v| v.to_str().unwrap().to_owned());
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        let body = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
        Answer {
            status,
            location,
            body,
        }
    }

    async fn gql_with(&self, bearer: Option<&str>, query: &str, variables: Value) -> Value {
        let mut request = Request::post("/v1/gql").header("content-type", "application/json");
        if let Some(token) = bearer {
            request = request.header("authorization", format!("Bearer {token}"));
        }
        let a = self
            .send(
                request
                    .body(Body::from(
                        json!({ "query": query, "variables": variables }).to_string(),
                    ))
                    .unwrap(),
            )
            .await;
        assert_eq!(a.status, StatusCode::OK, "{}", a.body);
        a.body
    }

    async fn gql(&self, token: &str, query: &str, variables: Value) -> Value {
        self.gql_with(Some(token), query, variables).await
    }

    /// `GET /v1/auth/app/authorize` with the given pairs.
    async fn authorize(&self, pairs: &[(&str, &str)]) -> Answer {
        let uri = format!("/v1/auth/app/authorize?{}", form(pairs));
        self.send(Request::get(uri).body(Body::empty()).unwrap())
            .await
    }

    /// The consent page's POST, as the website sends it (cookie and Origin).
    async fn consent(&self, user: &User, body: Value) -> Answer {
        self.send(
            Request::post("/v1/auth/app/consent")
                .header("content-type", "application/json")
                .header("origin", SITE)
                .header("cookie", format!("{}={}", cookies::SESSION, user.site))
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
    }

    async fn token(&self, pairs: &[(&str, &str)]) -> Answer {
        self.send(
            Request::post("/v1/auth/app/token")
                .header("content-type", "application/x-www-form-urlencoded")
                .header("user-agent", "cyex-deck/0.1.11")
                .body(Body::from(form(pairs)))
                .unwrap(),
        )
        .await
    }

    /// Up to the code: returns (code, verifier).
    async fn code_for(&self, user: &User) -> (String, String) {
        let verifier = tokens::random_token();
        let challenge = tokens::pkce_challenge(&verifier);
        let a = self
            .consent(
                user,
                json!({
                    "clientId": "cyex-deck",
                    "redirectUri": REDIRECT,
                    "codeChallenge": challenge,
                    "codeChallengeMethod": "S256",
                    "state": "st-1",
                    "allow": true
                }),
            )
            .await;
        assert_eq!(a.status, StatusCode::OK, "{}", a.body);
        let location = a.body["location"].as_str().unwrap().to_owned();
        assert!(location.starts_with(REDIRECT), "{location}");
        assert_eq!(param(&location, "state").as_deref(), Some("st-1"));
        (param(&location, "code").unwrap(), verifier)
    }

    async fn exchange(&self, code: &str, verifier: &str) -> Answer {
        self.token(&[
            ("grant_type", "authorization_code"),
            ("client_id", "cyex-deck"),
            ("code", code),
            ("code_verifier", verifier),
            ("redirect_uri", REDIRECT),
        ])
        .await
    }

    async fn sign_in_app(&self, user: &User) -> AppTokens {
        let (code, verifier) = self.code_for(user).await;
        let a = self.exchange(&code, &verifier).await;
        assert_eq!(a.status, StatusCode::OK, "{}", a.body);
        AppTokens {
            access: a.body["access_token"].as_str().unwrap().to_owned(),
            refresh: a.body["refresh_token"].as_str().unwrap().to_owned(),
        }
    }

    async fn refresh(&self, refresh: &str) -> Answer {
        self.token(&[
            ("grant_type", "refresh_token"),
            ("client_id", "cyex-deck"),
            ("refresh_token", refresh),
        ])
        .await
    }

    async fn signed_in(&self, access: &str) -> bool {
        let r = self.gql(access, ME, json!({})).await;
        match code(&r) {
            "" => true,
            "signed_out" => false,
            other => panic!("unexpected {other}: {r}"),
        }
    }

    async fn app_sessions(&self, user: &User) -> Vec<Value> {
        let r = self.gql(&user.full, SESSIONS, json!({})).await;
        ok(&r);
        r["data"]["mySessions"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|s| s["appClientId"] == "cyex-deck")
            .cloned()
            .collect()
    }
}

#[tokio::test]
async fn the_whole_flow_gives_an_app_session_listed_in_sessions() {
    let Some(w) = World::new().await else { return };
    let alice = w.user("1001").await;

    // The app opens authorize; the browser goes on to the consent page with the same request.
    let verifier = tokens::random_token();
    let challenge = tokens::pkce_challenge(&verifier);
    let a = w
        .authorize(&[
            ("response_type", "code"),
            ("client_id", "cyex-deck"),
            ("redirect_uri", REDIRECT),
            ("code_challenge", &challenge),
            ("code_challenge_method", "S256"),
            ("state", "xyz"),
        ])
        .await;
    assert_eq!(a.status, StatusCode::SEE_OTHER);
    let consent = a.location.unwrap();
    assert!(
        consent.starts_with("https://cyex.tv/authorize?"),
        "{consent}"
    );
    assert_eq!(param(&consent, "redirect_uri").as_deref(), Some(REDIRECT));
    assert_eq!(param(&consent, "code_challenge"), Some(challenge.clone()));
    assert_eq!(param(&consent, "state").as_deref(), Some("xyz"));

    // The consent page reads the client.
    let a = w
        .send(
            Request::get("/v1/auth/app/clients/cyex-deck")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
    assert_eq!(a.status, StatusCode::OK);
    assert_eq!(a.body["name"], "cyex deck");
    assert_eq!(
        a.body["scopes"][0]["description"],
        "Manage emotes in sets you can edit"
    );
    assert_eq!(a.body["scopes"][1]["description"], "Pick your cosmetics");
    let a = w
        .send(
            Request::get("/v1/auth/app/clients/nope")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
    assert_eq!(a.status, StatusCode::NOT_FOUND);

    // Allow, then trade the code.
    let a = w
        .consent(
            &alice,
            json!({
                "clientId": "cyex-deck", "redirectUri": REDIRECT, "codeChallenge": challenge,
                "codeChallengeMethod": "S256", "state": "xyz", "allow": true
            }),
        )
        .await;
    assert_eq!(a.status, StatusCode::OK, "{}", a.body);
    let back = a.body["location"].as_str().unwrap().to_owned();
    let code = param(&back, "code").unwrap();
    let a = w.exchange(&code, &verifier).await;
    assert_eq!(a.status, StatusCode::OK, "{}", a.body);
    assert_eq!(a.body["token_type"], "Bearer");
    assert_eq!(a.body["expires_in"], 3600);
    assert_eq!(a.body["scope"], "sets.emotes cosmetics.select");
    let access = a.body["access_token"].as_str().unwrap().to_owned();
    assert!(tokens::is_well_formed(&access));
    assert!(w.signed_in(&access).await);
    let r = w.gql(&access, ME, json!({})).await;
    assert_eq!(r["data"]["me"]["id"], ids::to_text(alice.id));

    // Listed once, named after the app, ending 60 days out; the access token ends in an hour.
    let listed = w.app_sessions(&alice).await;
    assert_eq!(listed.len(), 1, "{listed:?}");
    assert_eq!(listed[0]["kind"], "APP");
    assert_eq!(listed[0]["device"], "cyex deck");
    let ends: chrono::DateTime<Utc> = listed[0]["expiresAt"].as_str().unwrap().parse().unwrap();
    assert!(ends > Utc::now() + Duration::days(59) && ends < Utc::now() + Duration::days(61));
    let mut conn = w.db.conn().await;
    let stored = sessions::find_active(&mut conn, &tokens::hash(&access))
        .await
        .unwrap()
        .unwrap();
    let access_ends = stored.access_expires_at.unwrap();
    assert!(access_ends > Utc::now() + Duration::minutes(55));
    assert!(access_ends < Utc::now() + Duration::minutes(65));

    // Using the token doesn't stretch the session to the 90 days of other app sessions.
    sqlx::query("UPDATE sessions SET last_seen_at = now() - interval '2 hours' WHERE id = $1")
        .bind(stored.id)
        .execute(&mut *conn)
        .await
        .unwrap();
    assert!(w.signed_in(&access).await);
    let after = sessions::find_active(&mut conn, &tokens::hash(&access))
        .await
        .unwrap()
        .unwrap();
    assert!(after.expires_at < Utc::now() + Duration::days(61));
    assert!(after.last_seen_at > Utc::now() - Duration::minutes(5));

    // An expired access token stops working; the refresh token still gets a new one.
    sqlx::query(
        "UPDATE sessions SET access_expires_at = now() - interval '1 second' WHERE id = $1",
    )
    .bind(stored.id)
    .execute(&mut *conn)
    .await
    .unwrap();
    assert!(!w.signed_in(&access).await);
    let refresh = a.body["refresh_token"].as_str().unwrap();
    let a = w.refresh(refresh).await;
    assert_eq!(a.status, StatusCode::OK, "{}", a.body);
    assert!(w.signed_in(a.body["access_token"].as_str().unwrap()).await);
    drop(conn);
    w.db.close().await;
}

#[tokio::test]
async fn authorize_refuses_bad_clients_and_redirects() {
    let Some(w) = World::new().await else { return };
    let challenge = tokens::pkce_challenge(&tokens::random_token());
    let with = |client: &'static str, redirect: &'static str| {
        vec![
            ("response_type", "code"),
            ("client_id", client),
            ("redirect_uri", redirect),
            ("code_challenge_method", "S256"),
            ("state", "s"),
        ]
    };

    let mut pairs = with("someone-else", REDIRECT);
    pairs.push(("code_challenge", &challenge));
    let a = w.authorize(&pairs).await;
    assert_eq!(a.status, StatusCode::BAD_REQUEST);
    assert_eq!(a.body["error"], "unknown_client");
    assert!(a.location.is_none());

    for redirect in [
        "https://evil.example/callback",
        "http://localhost:53682/callback",
        "http://127.0.0.1.evil.example/callback",
        "http://127.0.0.1:53682/elsewhere",
        "https://127.0.0.1:53682/callback",
    ] {
        let mut pairs = with("cyex-deck", redirect);
        pairs.push(("code_challenge", &challenge));
        let a = w.authorize(&pairs).await;
        assert_eq!(a.status, StatusCode::BAD_REQUEST, "{redirect}");
        assert_eq!(a.body["error"], "bad_redirect_uri", "{redirect}");
        assert!(a.location.is_none(), "{redirect}");
    }

    // Any loopback port, IPv6 too.
    let mut pairs = with("cyex-deck", "http://[::1]:61000/callback");
    pairs.push(("code_challenge", &challenge));
    assert_eq!(w.authorize(&pairs).await.status, StatusCode::SEE_OTHER);

    // With a safe redirect, other mistakes go back to the app.
    let mut pairs = with("cyex-deck", REDIRECT);
    pairs[3] = ("code_challenge_method", "plain");
    pairs.push(("code_challenge", &challenge));
    let a = w.authorize(&pairs).await;
    assert_eq!(a.status, StatusCode::SEE_OTHER);
    let back = a.location.unwrap();
    assert!(back.starts_with(REDIRECT), "{back}");
    assert_eq!(param(&back, "error").as_deref(), Some("invalid_request"));
    assert_eq!(param(&back, "state").as_deref(), Some("s"));
    w.db.close().await;
}

#[tokio::test]
async fn consent_needs_the_site_session_and_deny_goes_back() {
    let Some(w) = World::new().await else { return };
    let alice = w.user("1101").await;
    let challenge = tokens::pkce_challenge(&tokens::random_token());
    let body = |allow: bool| {
        json!({
            "clientId": "cyex-deck", "redirectUri": REDIRECT, "codeChallenge": challenge,
            "codeChallengeMethod": "S256", "state": "s9", "allow": allow
        })
    };

    // Signed out.
    let a = w
        .send(
            Request::post("/v1/auth/app/consent")
                .header("content-type", "application/json")
                .header("origin", SITE)
                .body(Body::from(body(true).to_string()))
                .unwrap(),
        )
        .await;
    assert_eq!(a.status, StatusCode::UNAUTHORIZED);
    // The cookie from another site.
    let a = w
        .send(
            Request::post("/v1/auth/app/consent")
                .header("content-type", "application/json")
                .header("origin", "https://evil.example")
                .header("cookie", format!("{}={}", cookies::SESSION, alice.site))
                .body(Body::from(body(true).to_string()))
                .unwrap(),
        )
        .await;
    assert_eq!(a.status, StatusCode::FORBIDDEN);
    assert_eq!(a.body["error"], "bad_origin");
    // An app token can't mint itself more codes.
    let app = w.sign_in_app(&alice).await;
    let a = w
        .send(
            Request::post("/v1/auth/app/consent")
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {}", app.access))
                .body(Body::from(body(true).to_string()))
                .unwrap(),
        )
        .await;
    assert_eq!(a.status, StatusCode::UNAUTHORIZED);
    // A bad redirect is refused, not followed.
    let mut bad = body(true);
    bad["redirectUri"] = json!("https://evil.example/callback");
    let a = w.consent(&alice, bad).await;
    assert_eq!(a.status, StatusCode::BAD_REQUEST);
    assert_eq!(a.body["error"], "bad_redirect_uri");

    // Deny.
    let a = w.consent(&alice, body(false)).await;
    assert_eq!(a.status, StatusCode::OK);
    let back = a.body["location"].as_str().unwrap();
    assert_eq!(param(back, "error").as_deref(), Some("access_denied"));
    assert_eq!(param(back, "state").as_deref(), Some("s9"));
    assert!(param(back, "code").is_none());
    w.db.close().await;
}

#[tokio::test]
async fn codes_work_once_with_the_right_verifier_in_time() {
    let Some(w) = World::new().await else { return };
    let alice = w.user("1201").await;

    // Wrong verifier: refused, and the code is used up.
    let (code, verifier) = w.code_for(&alice).await;
    let a = w.exchange(&code, &tokens::random_token()).await;
    assert_eq!(a.status, StatusCode::BAD_REQUEST);
    assert_eq!(a.body["error"], "invalid_grant");
    let a = w.exchange(&code, &verifier).await;
    assert_eq!(a.body["error"], "invalid_grant");

    // Reused code: refused, and the session it gave is signed out.
    let (code, verifier) = w.code_for(&alice).await;
    let first = w.exchange(&code, &verifier).await;
    assert_eq!(first.status, StatusCode::OK);
    let access = first.body["access_token"].as_str().unwrap();
    assert!(w.signed_in(access).await);
    let again = w.exchange(&code, &verifier).await;
    assert_eq!(again.status, StatusCode::BAD_REQUEST);
    assert_eq!(again.body["error"], "invalid_grant");
    assert!(!w.signed_in(access).await);
    let refresh = first.body["refresh_token"].as_str().unwrap();
    assert_eq!(w.refresh(refresh).await.body["error"], "invalid_grant");

    // Expired code.
    let (code, verifier) = w.code_for(&alice).await;
    let mut conn = w.db.conn().await;
    sqlx::query(
        "UPDATE app_auth_codes SET expires_at = now() - interval '1 second' WHERE code_hash = $1",
    )
    .bind(&tokens::hash(&code)[..])
    .execute(&mut *conn)
    .await
    .unwrap();
    drop(conn);
    assert_eq!(
        w.exchange(&code, &verifier).await.body["error"],
        "invalid_grant"
    );

    // Another redirect_uri than at authorize.
    let (code, verifier) = w.code_for(&alice).await;
    let a = w
        .token(&[
            ("grant_type", "authorization_code"),
            ("client_id", "cyex-deck"),
            ("code", &code),
            ("code_verifier", &verifier),
            ("redirect_uri", "http://127.0.0.1:1/callback"),
        ])
        .await;
    assert_eq!(a.body["error"], "invalid_grant");

    // Unknown client, unknown grant, no verifier, not a form.
    let (code, verifier) = w.code_for(&alice).await;
    let a = w
        .token(&[
            ("grant_type", "authorization_code"),
            ("client_id", "someone-else"),
            ("code", &code),
            ("code_verifier", &verifier),
            ("redirect_uri", REDIRECT),
        ])
        .await;
    assert_eq!(a.status, StatusCode::UNAUTHORIZED);
    assert_eq!(a.body["error"], "invalid_client");
    let a = w
        .token(&[("grant_type", "password"), ("client_id", "cyex-deck")])
        .await;
    assert_eq!(a.body["error"], "unsupported_grant_type");
    let a = w
        .token(&[
            ("grant_type", "authorization_code"),
            ("client_id", "cyex-deck"),
            ("code", &code),
            ("redirect_uri", REDIRECT),
        ])
        .await;
    assert_eq!(a.body["error"], "invalid_request");
    let a = w
        .send(
            Request::post("/v1/auth/app/token")
                .header("content-type", "application/json")
                .body(Body::from("{}"))
                .unwrap(),
        )
        .await;
    assert_eq!(a.status, StatusCode::BAD_REQUEST);
    assert_eq!(a.body["error"], "invalid_request");
    // The code that wasn't used up still works.
    assert_eq!(w.exchange(&code, &verifier).await.status, StatusCode::OK);
    w.db.close().await;
}

#[tokio::test]
async fn refresh_tokens_rotate_and_a_reused_one_signs_the_session_out() {
    let Some(w) = World::new().await else { return };
    let alice = w.user("1301").await;
    let first = w.sign_in_app(&alice).await;
    let id_before = w.app_sessions(&alice).await[0]["id"].clone();

    let a = w.refresh(&first.refresh).await;
    assert_eq!(a.status, StatusCode::OK, "{}", a.body);
    let access = a.body["access_token"].as_str().unwrap().to_owned();
    let refresh = a.body["refresh_token"].as_str().unwrap().to_owned();
    assert_ne!(access, first.access);
    assert_ne!(refresh, first.refresh);
    // The old access token is gone, the new one works, and it's still one listed session.
    assert!(!w.signed_in(&first.access).await);
    assert!(w.signed_in(&access).await);
    let listed = w.app_sessions(&alice).await;
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0]["id"], id_before);

    // Replaying the first refresh token: refused, and the whole session ends.
    let a = w.refresh(&first.refresh).await;
    assert_eq!(a.status, StatusCode::BAD_REQUEST);
    assert_eq!(a.body["error"], "invalid_grant");
    assert!(!w.signed_in(&access).await);
    assert_eq!(w.refresh(&refresh).await.body["error"], "invalid_grant");
    assert!(w.app_sessions(&alice).await.is_empty());

    // Junk and unknown tokens.
    assert_eq!(w.refresh("nope").await.body["error"], "invalid_grant");
    assert_eq!(
        w.refresh(&tokens::random_token()).await.body["error"],
        "invalid_grant"
    );
    w.db.close().await;
}

#[tokio::test]
async fn signing_out_and_revoking_end_the_app_session() {
    let Some(w) = World::new().await else { return };
    let alice = w.user("1401").await;

    // Sign out with the Bearer token.
    let app = w.sign_in_app(&alice).await;
    let a = w
        .send(
            Request::post("/v1/auth/logout")
                .header("authorization", format!("Bearer {}", app.access))
                .body(Body::empty())
                .unwrap(),
        )
        .await;
    assert_eq!(a.status, StatusCode::NO_CONTENT);
    assert!(!w.signed_in(&app.access).await);
    assert_eq!(w.refresh(&app.refresh).await.body["error"], "invalid_grant");

    // RFC 7009 revoke by the refresh token; unknown tokens answer the same.
    let app = w.sign_in_app(&alice).await;
    for token in [tokens::random_token(), app.refresh.clone()] {
        let a = w
            .send(
                Request::post("/v1/auth/app/revoke")
                    .header("content-type", "application/x-www-form-urlencoded")
                    .body(Body::from(form(&[
                        ("token", &token),
                        ("client_id", "cyex-deck"),
                    ])))
                    .unwrap(),
            )
            .await;
        assert_eq!(a.status, StatusCode::OK);
    }
    assert!(!w.signed_in(&app.access).await);

    // Revoked from Settings → Sessions on the website.
    let app = w.sign_in_app(&alice).await;
    let listed = w.app_sessions(&alice).await;
    assert_eq!(listed.len(), 1);
    let r = w
        .gql(
            &alice.full,
            REVOKE_SESSION,
            json!({ "id": listed[0]["id"] }),
        )
        .await;
    assert_eq!(r["data"]["revokeSession"], true, "{r}");
    assert!(!w.signed_in(&app.access).await);
    assert_eq!(w.refresh(&app.refresh).await.body["error"], "invalid_grant");
    assert!(w.app_sessions(&alice).await.is_empty());
    w.db.close().await;
}

#[tokio::test]
async fn the_deck_calls_work_with_the_app_token_and_rights_are_unchanged() {
    let Some(w) = World::new().await else { return };
    let alice = w.user("1501").await;
    let bob = w.user("1502").await;
    let staff = w.user(STAFF).await;

    // Alice's channel shows her set.
    let r = w.gql(&alice.full, CREATE_SET, json!({ "n": "main" })).await;
    ok(&r);
    let set = r["data"]["createSet"]["id"].as_str().unwrap().to_owned();
    ok(&w
        .gql(
            &alice.full,
            ACTIVATE,
            json!({ "c": alice.twitch_id, "s": set }),
        )
        .await);
    let emote = w.emote(&alice, "peepoHi").await;

    let deck = w.sign_in_app(&bob).await;
    let owner_deck = w.sign_in_app(&alice).await;

    // The owner sees her set as hers.
    let r = w
        .gql(
            &owner_deck.access,
            EDITABLE_BY_ID,
            json!({ "c": alice.twitch_id }),
        )
        .await;
    ok(&r);
    let found = &r["data"]["channelEditableSets"];
    assert_eq!(found["role"], "OWNER");
    assert_eq!(found["canEditActiveSet"], true);
    assert_eq!(found["sets"][0]["access"], "OWNER");
    assert_eq!(found["sets"][0]["activeHere"], true);
    assert_eq!(found["channel"]["activeSetId"], set.as_str());

    // Bob may change nothing yet: an empty list, and the mutation is refused as before.
    let r = w
        .gql(
            &deck.access,
            EDITABLE_BY_ID,
            json!({ "c": alice.twitch_id }),
        )
        .await;
    ok(&r);
    let found = &r["data"]["channelEditableSets"];
    assert_eq!(found["role"], Value::Null);
    assert_eq!(found["canEditActiveSet"], false);
    assert_eq!(found["sets"], json!([]));
    let r = w
        .gql(&deck.access, ADD, json!({ "s": set, "e": emote }))
        .await;
    assert_eq!(code(&r), "forbidden", "{r}");

    // Made an editor of the set, Bob sees it and can add, rename and remove.
    let mut conn = w.db.conn().await;
    sets::add_editor(
        &mut conn,
        ids::from_text(&set).unwrap(),
        bob.id,
        Some(alice.id),
    )
    .await
    .unwrap();
    drop(conn);
    let r = w
        .gql(
            &deck.access,
            EDITABLE_BY_ID,
            json!({ "c": alice.twitch_id }),
        )
        .await;
    ok(&r);
    let found = &r["data"]["channelEditableSets"];
    assert_eq!(found["role"], "SET_EDITOR");
    assert_eq!(found["canEditActiveSet"], true);
    assert_eq!(found["sets"][0]["access"], "SET_EDITOR");
    assert_eq!(found["sets"][0]["set"]["id"], set.as_str());
    let r = w
        .gql(&deck.access, EDITABLE_BY_LOGIN, json!({ "l": "@USER1501" }))
        .await;
    ok(&r);
    assert_eq!(
        r["data"]["channelEditableSets"]["sets"][0]["set"]["id"],
        set.as_str()
    );
    let r = w
        .gql(
            &deck.access,
            EDITABLE_BY_LOGIN,
            json!({ "l": "nobody-here" }),
        )
        .await;
    assert_eq!(r["data"]["channelEditableSets"], Value::Null, "{r}");
    let r = w
        .gql(
            &deck.access,
            "{ channelEditableSets(platform: TWITCH) { role } }",
            json!({}),
        )
        .await;
    assert_eq!(code(&r), "bad_channel");

    let r = w
        .gql(&deck.access, ADD, json!({ "s": set, "e": emote }))
        .await;
    assert_eq!(r["data"]["addSetEmote"]["name"], "peepoHi", "{r}");
    let r = w
        .gql(
            &deck.access,
            RENAME,
            json!({ "s": set, "e": emote, "n": "peepoHey" }),
        )
        .await;
    assert_eq!(r["data"]["updateSetEmote"]["name"], "peepoHey", "{r}");
    let r = w
        .gql(&deck.access, REMOVE, json!({ "s": set, "e": emote }))
        .await;
    assert_eq!(r["data"]["removeSetEmote"], true, "{r}");
    // Signed out, the query is refused.
    let r = w
        .gql_with(None, EDITABLE_BY_ID, json!({ "c": alice.twitch_id }))
        .await;
    assert_eq!(code(&r), "signed_out");

    // Cosmetics: read, refused while not owned, picked once granted.
    let r = w
        .gql(
            &staff.full,
            CREATE_PAINT,
            json!({ "p": {
                "name": "sunset", "kind": "LINEAR_GRADIENT", "angle": 90, "repeat": false,
                "color": "#ff8800", "stops": [{ "at": 0.0, "color": "#ff8800" }, { "at": 1.0, "color": "#8800ff" }],
                "shadows": []
            } }),
        )
        .await;
    ok(&r);
    let paint = r["data"]["createPaint"]["id"].as_str().unwrap().to_owned();
    let r = w.gql(&deck.access, MINE, json!({})).await;
    ok(&r);
    assert_eq!(
        r["data"]["myCosmetics"]["account"]["id"],
        ids::to_text(bob.id)
    );
    assert_eq!(r["data"]["myCosmetics"]["paints"], json!([]));
    let r = w.gql(&deck.access, SELECT, json!({ "p": paint })).await;
    assert_eq!(code(&r), "not_owned", "{r}");
    ok(&w
        .gql(
            &staff.full,
            GRANT,
            json!({ "a": ids::to_text(bob.id), "c": paint }),
        )
        .await);
    let r = w.gql(&deck.access, SELECT, json!({ "p": paint })).await;
    assert_eq!(
        r["data"]["selectCosmetics"]["paintId"],
        paint.as_str(),
        "{r}"
    );
    let r = w.gql(&deck.access, MINE, json!({})).await;
    assert_eq!(r["data"]["myCosmetics"]["paintId"], paint.as_str());
    w.db.close().await;
}

#[tokio::test]
async fn app_tokens_run_only_their_scopes() {
    let Some(w) = World::new().await else { return };
    let alice = w.user("1601").await;
    let deck = w.sign_in_app(&alice).await;

    for (query, variables) in [
        (CREATE_SET, json!({ "n": "nope" })),
        ("mutation { revokeOtherSessions }", json!({})),
        ("mutation { exportMyData }", json!({})),
        ("mutation { signOutEverywhere }", json!({})),
        ("mutation { x: revokeOtherSessions }", json!({})),
        (
            "mutation { ...F } fragment F on Mutation { revokeOtherSessions }",
            json!({}),
        ),
    ] {
        let r = w.gql(&deck.access, query, variables).await;
        assert_eq!(code(&r), "insufficient_scope", "{query}: {r}");
    }
    // The unscoped session is untouched by that.
    assert!(w.signed_in(&alice.full).await);
    assert_eq!(w.app_sessions(&alice).await.len(), 1);
    // Only the listed queries run: public and own reads, nothing staff read, nothing else of
    // the account's (sessions, orders, notifications). Aliases and fragments change nothing.
    ok(&w.gql(&deck.access, ME, json!({})).await);
    ok(&w
        .gql(
            &deck.access,
            "{ a: myEmotes { total } ...F } fragment F on Query { mySets { id } globalSet { id } }",
            json!({}),
        )
        .await);
    for query in [
        SESSIONS,
        "{ auditLog { id } }",
        "{ x: reviewQueue { total } }",
        "{ ...F } fragment F on Query { roles { id } }",
        "query { me { id } ... on Query { announcements { id } } }",
        "{ myOrders { total } }",
        "{ myNotifications { id } }",
    ] {
        let r = w.gql(&deck.access, query, json!({})).await;
        assert_eq!(code(&r), "insufficient_scope", "{query}: {r}");
    }
    // The website session still runs them.
    ok(&w.gql(&alice.full, SESSIONS, json!({})).await);

    // No uploads.
    let a = w
        .send(
            Request::post("/v1/emotes")
                .header("authorization", format!("Bearer {}", deck.access))
                .header("content-type", "multipart/form-data; boundary=x")
                .body(Body::from("--x--\r\n"))
                .unwrap(),
        )
        .await;
    assert_eq!(a.status, StatusCode::FORBIDDEN);
    assert_eq!(a.body["error"], "insufficient_scope");
    w.db.close().await;
}

/// Staff grants don't count for an app token (ADR 0021): with every permission (the owner here),
/// the deck still sees and changes only sets the account itself may edit; the website and the
/// unscoped session keep `sets.edit_any`.
#[tokio::test]
async fn staff_grants_do_not_count_for_app_tokens() {
    let Some(w) = World::new().await else { return };
    let alice = w.user("1701").await;
    let staff = w.user(STAFF).await;
    let r = w.gql(&alice.full, CREATE_SET, json!({ "n": "main" })).await;
    ok(&r);
    let set = r["data"]["createSet"]["id"].as_str().unwrap().to_owned();
    ok(&w
        .gql(
            &alice.full,
            ACTIVATE,
            json!({ "c": alice.twitch_id, "s": set }),
        )
        .await);
    let emote = w.emote(&alice, "staffHi").await;

    // Unscoped, the staff account edits Alice's set through sets.edit_any.
    let r = w
        .gql(&staff.full, EDITABLE_BY_ID, json!({ "c": alice.twitch_id }))
        .await;
    ok(&r);
    assert_eq!(
        r["data"]["channelEditableSets"]["sets"][0]["access"], "STAFF",
        "{r}"
    );

    // Through the deck: nothing to edit, and every set emote mutation is refused.
    let deck = w.sign_in_app(&staff).await;
    let r = w
        .gql(
            &deck.access,
            EDITABLE_BY_ID,
            json!({ "c": alice.twitch_id }),
        )
        .await;
    ok(&r);
    let found = &r["data"]["channelEditableSets"];
    assert_eq!(found["sets"], json!([]), "{r}");
    assert_eq!(found["canEditActiveSet"], false);
    assert_eq!(found["role"], Value::Null);
    let r = w
        .gql(&deck.access, ADD, json!({ "s": set, "e": emote }))
        .await;
    assert_eq!(code(&r), "forbidden", "{r}");

    // Alice adds it; the deck can't rename or remove it either.
    ok(&w
        .gql(&alice.full, ADD, json!({ "s": set, "e": emote }))
        .await);
    let r = w
        .gql(
            &deck.access,
            RENAME,
            json!({ "s": set, "e": emote, "n": "staffHey" }),
        )
        .await;
    assert_eq!(code(&r), "forbidden", "{r}");
    let r = w
        .gql(&deck.access, REMOVE, json!({ "s": set, "e": emote }))
        .await;
    assert_eq!(code(&r), "forbidden", "{r}");
    // Staff reads are closed to it as well.
    let r = w.gql(&deck.access, "{ auditLog { id } }", json!({})).await;
    assert_eq!(code(&r), "insufficient_scope", "{r}");

    // The unscoped session still can.
    let r = w
        .gql(
            &staff.full,
            RENAME,
            json!({ "s": set, "e": emote, "n": "staffHey" }),
        )
        .await;
    assert_eq!(r["data"]["updateSetEmote"]["name"], "staffHey", "{r}");
    w.db.close().await;
}
