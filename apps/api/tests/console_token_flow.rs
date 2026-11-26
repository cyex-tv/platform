//! Console tokens (ADR 0028): `POST /v1/auth/token/export` and `createConsoleToken` make a
//! personal Bearer token, with the website's cookie and `Origin` only. The token reads `me`, shows
//! in `mySessions` as `Console token`, lasts 3 days without sliding, is the account's only one
//! (making another signs the last out), ends with "sign out everywhere" and `revokeSession` and
//! rotates whenever any session signs out (ADR 0029), holds
//! the account's roles but never the owner's all-permissions or website-only actions, is rate
//! limited and audited. Through the real router against a real Postgres; runs when
//! `CYEX_TV_TEST_DATABASE_URL` is set (CI sets it) and skips otherwise.

mod common;

use std::sync::Arc;

use axum::Router;
use axum::body::Body;
use axum::http::{HeaderMap, Request, StatusCode};
use chrono::{DateTime, Duration, Utc};
use common::TestDb;
use cyex_tv_api::limits::{Limits, Rates};
use cyex_tv_api::state::AppState;
use cyex_tv_auth::{Auth, AuthConfig, cookies, tokens};
use cyex_tv_db::accounts::{self, Profile};
use cyex_tv_db::sessions::{self, SessionKind};
use cyex_tv_db::{app_auth, audit, roles};
use cyex_tv_shared::model::Platform;
use cyex_tv_shared::ratelimit::Rate;
use http_body_util::BodyExt;
use serde_json::{Value, json};
use tower::ServiceExt;
use uuid::Uuid;

const SITE: &str = "https://cyex.tv";
/// Holds every permission in these tests only (`CYEX_TV_OWNER_IDS`).
const OWNER: &str = "90001";
const EXPORT: &str = "/v1/auth/token/export";
const ME: &str = "{ me { id permissions } }";
const SESSIONS: &str = "{ mySessions { id kind device expiresAt current appClientId } }";
const CURRENT: &str = "{ myConsoleToken { id kind device createdAt expiresAt } }";

struct User {
    id: Uuid,
    /// A website cookie session.
    site: String,
    /// The deck's app session (Bearer, scoped).
    app: String,
}

struct World {
    db: TestDb,
    app: Router,
}

struct Answer {
    status: StatusCode,
    headers: HeaderMap,
    body: Value,
}

/// How a request proves who it is.
#[derive(Clone, Copy)]
enum As<'a> {
    Nobody,
    /// The website: the cookie and the site's `Origin`.
    Site(&'a str),
    /// The cookie with another `Origin`, or none.
    CookieFrom(&'a str, Option<&'a str>),
    Bearer(&'a str),
}

fn code(r: &Value) -> &str {
    r["errors"][0]["extensions"]["code"]
        .as_str()
        .or_else(|| r["error"].as_str())
        .unwrap_or_default()
}

impl World {
    async fn new() -> Option<Self> {
        Self::with_rates(Rates::default()).await
    }

    async fn with_rates(rates: Rates) -> Option<Self> {
        let db = TestDb::new().await?;
        let cfg = AuthConfig::from_source(|name| {
            (name == "CYEX_TV_OWNER_IDS").then(|| format!("twitch:{OWNER}"))
        })
        .unwrap();
        let state = AppState::new(db.pool.clone(), Arc::new(Auth::new(cfg).unwrap()))
            .with_limits(Limits::new(rates, false));
        Some(Self {
            app: cyex_tv_api::app(state),
            db,
        })
    }

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
        let id = signed.account.id;
        let site = tokens::random_token();
        sessions::create(
            &mut conn,
            &tokens::hash(&site),
            id,
            SessionKind::Site,
            Duration::days(30),
        )
        .await
        .unwrap();
        let app = tokens::random_token();
        app_auth::create_session(
            &mut conn,
            &tokens::hash(&app),
            id,
            "cyex-deck",
            Duration::hours(1),
            Duration::days(60),
            None,
        )
        .await
        .unwrap();
        User { id, site, app }
    }

    fn with(
        mut request: axum::http::request::Builder,
        who: As<'_>,
    ) -> axum::http::request::Builder {
        match who {
            As::Nobody => {}
            As::Site(cookie) => {
                request = request
                    .header("cookie", format!("{}={cookie}", cookies::SESSION))
                    .header("origin", SITE);
            }
            As::CookieFrom(cookie, origin) => {
                request = request.header("cookie", format!("{}={cookie}", cookies::SESSION));
                if let Some(origin) = origin {
                    request = request.header("origin", origin);
                }
            }
            As::Bearer(token) => {
                request = request.header("authorization", format!("Bearer {token}"));
            }
        }
        request
    }

    async fn send(&self, request: Request<Body>) -> Answer {
        let response = self.app.clone().oneshot(request).await.unwrap();
        let status = response.status();
        let headers = response.headers().clone();
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        Answer {
            status,
            headers,
            body: serde_json::from_slice(&bytes).unwrap_or(Value::Null),
        }
    }

    async fn export(&self, who: As<'_>) -> Answer {
        let request = Self::with(Request::post(EXPORT), who)
            .header("user-agent", "Mozilla/5.0 (Windows NT 10.0) Firefox/140.0")
            .body(Body::empty())
            .unwrap();
        self.send(request).await
    }

    /// Mints a token as the website; panics unless it works.
    async fn mint(&self, user: &User) -> Value {
        let a = self.export(As::Site(&user.site)).await;
        assert_eq!(a.status, StatusCode::OK, "{}", a.body);
        a.body
    }

    async fn gql(&self, who: As<'_>, query: &str, variables: Value) -> Value {
        let request = Self::with(
            Request::post("/v1/gql").header("content-type", "application/json"),
            who,
        )
        .body(Body::from(
            json!({ "query": query, "variables": variables }).to_string(),
        ))
        .unwrap();
        let a = self.send(request).await;
        assert_eq!(a.status, StatusCode::OK, "{}", a.body);
        a.body
    }

    async fn console_rows(&self, account: Uuid) -> Vec<sessions::Session> {
        let mut conn = self.db.conn().await;
        sessions::list_for_account(&mut conn, account)
            .await
            .unwrap()
            .into_iter()
            .filter(|s| s.kind == SessionKind::Console)
            .collect()
    }
}

fn token_of(body: &Value) -> String {
    body["token"].as_str().expect("a token").to_owned()
}

#[tokio::test]
async fn minting_needs_the_website_cookie_and_its_origin() {
    let Some(w) = World::new().await else { return };
    let alice = w.user("5101").await;

    let a = w.export(As::Nobody).await;
    assert_eq!(a.status, StatusCode::UNAUTHORIZED, "{}", a.body);
    assert_eq!(code(&a.body), "signed_out");

    for origin in [
        None,
        Some("https://evil.example"),
        Some("https://cyex.tv.evil.example"),
    ] {
        let a = w.export(As::CookieFrom(&alice.site, origin)).await;
        assert_eq!(a.status, StatusCode::FORBIDDEN, "{origin:?}: {}", a.body);
        assert_eq!(code(&a.body), "bad_origin");
    }

    // An app's token can't make one, and neither can a console token.
    let a = w.export(As::Bearer(&alice.app)).await;
    assert_eq!(a.status, StatusCode::FORBIDDEN, "{}", a.body);
    assert_eq!(code(&a.body), "website_only");
    assert!(w.console_rows(alice.id).await.is_empty());

    let body = w.mint(&alice).await;
    let token = token_of(&body);
    assert!(tokens::is_well_formed(&token));
    assert_eq!(body["token_type"], "Bearer");
    let a = w.export(As::Bearer(&token)).await;
    assert_eq!(a.status, StatusCode::FORBIDDEN, "{}", a.body);
    assert_eq!(code(&a.body), "website_only");
    let r = w
        .gql(
            As::Bearer(&token),
            "mutation { createConsoleToken { token } }",
            json!({}),
        )
        .await;
    assert_eq!(code(&r), "website_only", "{r}");
    w.db.close().await;
}

#[tokio::test]
async fn the_token_reads_me_and_shows_as_a_console_session() {
    let Some(w) = World::new().await else { return };
    let alice = w.user("5201").await;
    let a = w.export(As::Site(&alice.site)).await;
    assert_eq!(a.status, StatusCode::OK, "{}", a.body);
    assert_eq!(a.headers["cache-control"], "no-store");
    let token = token_of(&a.body);

    // A working Bearer token for the same account.
    let site_me = w.gql(As::Site(&alice.site), ME, json!({})).await;
    let me = w.gql(As::Bearer(&token), ME, json!({})).await;
    assert!(me["errors"].is_null(), "{me}");
    assert_eq!(me["data"]["me"]["id"], site_me["data"]["me"]["id"]);

    // Listed once, as a console token, with the answer's ID and expiry, 3 days out.
    let list = w.gql(As::Site(&alice.site), SESSIONS, json!({})).await;
    let consoles: Vec<&Value> = list["data"]["mySessions"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|s| s["kind"] == "CONSOLE")
        .collect();
    assert_eq!(consoles.len(), 1, "{list}");
    assert_eq!(consoles[0]["device"], "Console token");
    assert_eq!(consoles[0]["id"], a.body["session_id"]);
    assert_eq!(consoles[0]["expiresAt"], a.body["expires_at"]);
    assert!(consoles[0]["appClientId"].is_null());
    let expires: DateTime<Utc> = a.body["expires_at"].as_str().unwrap().parse().unwrap();
    let days = (expires - Utc::now()).num_hours();
    assert!((3 * 24 - 1..=3 * 24).contains(&days), "{days} hours");

    // Used later, it doesn't slide.
    {
        let mut conn = w.db.conn().await;
        sqlx::query(
            "UPDATE sessions SET last_seen_at = now() - interval '2 hours' WHERE kind = 'console'",
        )
        .execute(&mut *conn)
        .await
        .unwrap();
    }
    let me = w.gql(As::Bearer(&token), ME, json!({})).await;
    assert!(me["errors"].is_null(), "{me}");
    let row = &w.console_rows(alice.id).await[0];
    assert!(row.last_seen_at > Utc::now() - Duration::minutes(5));
    assert_eq!(
        row.expires_at.timestamp_millis(),
        expires.timestamp_millis()
    );

    // The GraphQL mutation does the same, from the website.
    let r = w
        .gql(
            As::Site(&alice.site),
            "mutation { createConsoleToken { token tokenType expiresAt sessionId } }",
            json!({}),
        )
        .await;
    assert!(r["errors"].is_null(), "{r}");
    let again = r["data"]["createConsoleToken"]["token"].as_str().unwrap();
    assert!(w.gql(As::Bearer(again), ME, json!({})).await["errors"].is_null());

    // Audited, never with the token.
    let mut conn = w.db.conn().await;
    let rows = audit::list(&mut conn, &audit::Filter::default(), 100)
        .await
        .unwrap();
    let made: Vec<_> = rows
        .iter()
        .filter(|e| e.action == "account.console_token_created")
        .collect();
    assert_eq!(made.len(), 2);
    assert!(made.iter().all(|e| e.actor_account_id == Some(alice.id)));
    for e in &made {
        let text = format!("{:?} {:?}", e.before_json, e.after_json);
        assert!(!text.contains(&token) && !text.contains(again), "{text}");
    }
    drop(conn);
    w.db.close().await;
}

#[tokio::test]
async fn minting_again_signs_the_last_one_out() {
    let Some(w) = World::new().await else { return };
    let alice = w.user("5301").await;
    let bob = w.user("5302").await;
    let first = token_of(&w.mint(&alice).await);
    let bobs = token_of(&w.mint(&bob).await);
    let second = token_of(&w.mint(&alice).await);
    assert_ne!(first, second);

    let r = w.gql(As::Bearer(&first), ME, json!({})).await;
    assert_eq!(code(&r), "signed_out", "{r}");
    assert!(w.gql(As::Bearer(&second), ME, json!({})).await["errors"].is_null());
    assert_eq!(w.console_rows(alice.id).await.len(), 1);
    // Someone else's token is theirs alone.
    assert!(w.gql(As::Bearer(&bobs), ME, json!({})).await["errors"].is_null());
    // The website session that made them stays.
    assert!(w.gql(As::Site(&alice.site), ME, json!({})).await["errors"].is_null());
    w.db.close().await;
}

#[tokio::test]
async fn signing_out_everywhere_or_the_session_ends_it() {
    let Some(w) = World::new().await else { return };
    let alice = w.user("5401").await;

    // Signed out from the sessions list by its ID.
    let body = w.mint(&alice).await;
    let token = token_of(&body);
    let r = w
        .gql(
            As::Site(&alice.site),
            "mutation($id: String!) { revokeSession(id: $id) }",
            json!({ "id": body["session_id"] }),
        )
        .await;
    assert_eq!(r["data"]["revokeSession"], true, "{r}");
    assert_eq!(
        code(&w.gql(As::Bearer(&token), ME, json!({})).await),
        "signed_out"
    );

    // With the token itself, only the token ends.
    let token = token_of(&w.mint(&alice).await);
    let r = w
        .gql(
            As::Bearer(&token),
            "mutation { signOutEverywhere }",
            json!({}),
        )
        .await;
    assert_eq!(r["data"]["signOutEverywhere"], 1, "{r}");
    assert_eq!(
        code(&w.gql(As::Bearer(&token), ME, json!({})).await),
        "signed_out"
    );
    assert!(w.gql(As::Site(&alice.site), ME, json!({})).await["errors"].is_null());

    // Sign out everywhere on the website ends it too.
    let token = token_of(&w.mint(&alice).await);
    let r = w
        .gql(
            As::Site(&alice.site),
            "mutation { signOutEverywhere }",
            json!({}),
        )
        .await;
    assert!(r["errors"].is_null(), "{r}");
    assert_eq!(
        code(&w.gql(As::Bearer(&token), ME, json!({})).await),
        "signed_out"
    );
    assert!(w.console_rows(alice.id).await.is_empty());

    // So does POST /v1/auth/logout with the token.
    let alice = w.user("5402").await;
    let token = token_of(&w.mint(&alice).await);
    let a = w
        .send(
            World::with(Request::post("/v1/auth/logout"), As::Bearer(&token))
                .body(Body::empty())
                .unwrap(),
        )
        .await;
    assert_eq!(a.status, StatusCode::NO_CONTENT);
    assert_eq!(
        code(&w.gql(As::Bearer(&token), ME, json!({})).await),
        "signed_out"
    );
    w.db.close().await;
}

#[tokio::test]
async fn it_has_the_accounts_rights_and_no_more() {
    let Some(w) = World::new().await else { return };
    // A moderator: their role counts with the token.
    let modr = w.user("5501").await;
    {
        let mut conn = w.db.conn().await;
        let role = roles::create(
            &mut conn,
            &roles::NewRole {
                name: "console mods",
                description: None,
                position: 10,
                permissions: &["emotes.review"],
                created_by: None,
            },
        )
        .await
        .unwrap();
        roles::assign(&mut conn, modr.id, role.id, None)
            .await
            .unwrap();
    }
    let token = token_of(&w.mint(&modr).await);
    let perms = w.gql(As::Bearer(&token), ME, json!({})).await["data"]["me"]["permissions"].clone();
    assert!(
        perms
            .as_array()
            .unwrap()
            .iter()
            .any(|p| p == "emotes.review"),
        "{perms}"
    );
    assert!(
        !perms
            .as_array()
            .unwrap()
            .iter()
            .any(|p| p == "roles.manage"),
        "{perms}"
    );

    // The owner: everything on the website, only their roles with the token.
    let owner = w.user(OWNER).await;
    let site =
        w.gql(As::Site(&owner.site), ME, json!({})).await["data"]["me"]["permissions"].clone();
    assert!(
        site.as_array().unwrap().iter().any(|p| p == "roles.manage"),
        "{site}"
    );
    let token = token_of(&w.mint(&owner).await);
    let perms = w.gql(As::Bearer(&token), ME, json!({})).await["data"]["me"]["permissions"].clone();
    assert!(
        !perms
            .as_array()
            .unwrap()
            .iter()
            .any(|p| p == "roles.manage"),
        "{perms}"
    );

    // Website-only actions stay website only.
    for doc in [
        "mutation { exportMyData }",
        "mutation { revokeOtherSessions }",
        "mutation { deleteAccount(confirm: \"user5501\") }",
    ] {
        let r = w.gql(As::Bearer(&token), doc, json!({})).await;
        assert_eq!(code(&r), "website_only", "{doc}: {r}");
    }
    w.db.close().await;
}

#[tokio::test]
async fn minting_is_rate_limited_per_account() {
    let rates = Rates {
        console_tokens: Rate::per_hour(2),
        ..Rates::default()
    };
    let Some(w) = World::with_rates(rates).await else {
        return;
    };
    let alice = w.user("5601").await;
    let bob = w.user("5602").await;
    w.mint(&alice).await;
    let last = token_of(&w.mint(&alice).await);
    let a = w.export(As::Site(&alice.site)).await;
    assert_eq!(a.status, StatusCode::TOO_MANY_REQUESTS, "{}", a.body);
    assert_eq!(code(&a.body), "rate_limited");
    let r = w
        .gql(
            As::Site(&alice.site),
            "mutation { createConsoleToken { token } }",
            json!({}),
        )
        .await;
    assert_eq!(code(&r), "rate_limited", "{r}");
    // A refused try changes nothing: the last token still works.
    assert!(w.gql(As::Bearer(&last), ME, json!({})).await["errors"].is_null());
    // Another account has its own count.
    w.mint(&bob).await;
    w.db.close().await;
}

/// Whether a console token still works, over GraphQL and REST alike.
async fn works(w: &World, token: &str) -> bool {
    let r = w.gql(As::Bearer(token), ME, json!({})).await;
    let gql = r["errors"].is_null();
    // REST: a live token is refused making another (`website_only`); an ended one is nobody.
    let a = w.export(As::Bearer(token)).await;
    let rest = a.status == StatusCode::FORBIDDEN;
    assert!(rest || a.status == StatusCode::UNAUTHORIZED, "{}", a.body);
    assert_eq!(gql, rest, "GraphQL and REST disagree: {r} / {}", a.status);
    gql
}

/// A second website session for the user (another browser): its token and public ID.
async fn other_browser(w: &World, user: &User) -> (String, String) {
    let mut conn = w.db.conn().await;
    let token = tokens::random_token();
    let s = sessions::create(
        &mut conn,
        &tokens::hash(&token),
        user.id,
        SessionKind::Site,
        Duration::days(30),
    )
    .await
    .unwrap();
    (token, cyex_tv_db::ids::to_text(s.id))
}

#[tokio::test]
async fn it_stops_working_three_days_after_it_was_made() {
    let Some(w) = World::new().await else { return };
    let alice = w.user("5701").await;
    let body = w.mint(&alice).await;
    let old = token_of(&body);
    assert!(works(&w, &old).await);

    // `myConsoleToken` shows it (never the token), with the answer's ID and expiry.
    let r = w.gql(As::Site(&alice.site), CURRENT, json!({})).await;
    assert!(r["errors"].is_null(), "{r}");
    assert_eq!(r["data"]["myConsoleToken"]["id"], body["session_id"]);
    assert_eq!(r["data"]["myConsoleToken"]["expiresAt"], body["expires_at"]);
    assert_eq!(r["data"]["myConsoleToken"]["kind"], "CONSOLE");
    assert!(!r.to_string().contains(&old));

    {
        let mut conn = w.db.conn().await;
        // The database refuses a console session longer than 3 days.
        let longer = sqlx::query(
            "UPDATE sessions SET expires_at = created_at + interval '4 days' WHERE kind = 'console'",
        )
        .execute(&mut *conn)
        .await;
        assert!(longer.is_err(), "a 4-day console token was written");
        // Three days and a minute later.
        sqlx::query(
            "UPDATE sessions SET created_at = created_at - interval '3 days 1 minute',
                                 expires_at = expires_at - interval '3 days 1 minute',
                                 last_seen_at = last_seen_at - interval '3 days 1 minute'
             WHERE kind = 'console'",
        )
        .execute(&mut *conn)
        .await
        .unwrap();
    }
    assert!(!works(&w, &old).await);
    let r = w.gql(As::Site(&alice.site), CURRENT, json!({})).await;
    assert!(r["data"]["myConsoleToken"].is_null(), "{r}");

    // A new one works; the old one still doesn't.
    let new = token_of(&w.mint(&alice).await);
    assert!(works(&w, &new).await);
    assert!(!works(&w, &old).await);
    w.db.close().await;
}

#[tokio::test]
async fn signing_out_one_session_rotates_it() {
    let Some(w) = World::new().await else { return };
    let alice = w.user("5801").await;
    let bob = w.user("5802").await;
    let bobs = token_of(&w.mint(&bob).await);

    // Sign out (POST /v1/auth/logout) in another browser.
    let old = token_of(&w.mint(&alice).await);
    let (browser, _) = other_browser(&w, &alice).await;
    let a = w
        .send(
            World::with(Request::post("/v1/auth/logout"), As::Site(&browser))
                .body(Body::empty())
                .unwrap(),
        )
        .await;
    assert_eq!(a.status, StatusCode::NO_CONTENT);
    assert!(!works(&w, &old).await);
    assert!(w.console_rows(alice.id).await.is_empty());
    // This browser stays signed in, and someone else's token is theirs alone.
    assert!(w.gql(As::Site(&alice.site), ME, json!({})).await["errors"].is_null());
    assert!(works(&w, &bobs).await);
    let new = token_of(&w.mint(&alice).await);
    assert!(works(&w, &new).await);
    assert!(!works(&w, &old).await);

    // Signing one other session out by its ID (`revokeSession`).
    let old = new;
    let (_, id) = other_browser(&w, &alice).await;
    let r = w
        .gql(
            As::Site(&alice.site),
            "mutation($id: String!) { revokeSession(id: $id) }",
            json!({ "id": id }),
        )
        .await;
    assert_eq!(r["data"]["revokeSession"], true, "{r}");
    assert!(!works(&w, &old).await);
    // An ID that isn't one of yours signs nothing out, and rotates nothing.
    let new = token_of(&w.mint(&alice).await);
    let r = w
        .gql(
            As::Site(&alice.site),
            "mutation($id: String!) { revokeSession(id: $id) }",
            json!({ "id": "nope" }),
        )
        .await;
    assert_eq!(r["data"]["revokeSession"], false, "{r}");
    assert!(works(&w, &new).await);

    // An app signing itself out (RFC 7009).
    let old = new;
    let a = w
        .send(
            Request::post("/v1/auth/app/revoke")
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from(format!(
                    "token={}&client_id=cyex-deck",
                    alice.app
                )))
                .unwrap(),
        )
        .await;
    assert_eq!(a.status, StatusCode::OK);
    assert!(!works(&w, &old).await);
    let new = token_of(&w.mint(&alice).await);
    assert!(works(&w, &new).await);
    assert!(works(&w, &bobs).await);
    w.db.close().await;
}

#[tokio::test]
async fn signing_out_everywhere_rotates_it() {
    let Some(w) = World::new().await else { return };
    let alice = w.user("5901").await;
    let old = token_of(&w.mint(&alice).await);
    let r = w
        .gql(
            As::Site(&alice.site),
            "mutation { signOutEverywhere }",
            json!({}),
        )
        .await;
    assert!(r["errors"].is_null(), "{r}");
    assert!(!works(&w, &old).await);

    // Signed in again, the new token works and the old one never does.
    let alice = w.user("5901").await;
    let new = token_of(&w.mint(&alice).await);
    assert!(works(&w, &new).await);
    assert!(!works(&w, &old).await);

    // "Sign out everywhere but here" ends it too.
    let r = w
        .gql(
            As::Site(&alice.site),
            "mutation { revokeOtherSessions }",
            json!({}),
        )
        .await;
    assert!(r["errors"].is_null(), "{r}");
    assert!(!works(&w, &new).await);
    w.db.close().await;
}
