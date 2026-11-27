//! The developer platform: API keys (ADR 0033) and channel webhooks (ADR 0034).
//!
//! Keys: made on the website only, shown once, listed without the key, read public data in their
//! own rate bucket (reported by `RateLimit-*`), never mutate or act as the account, end when
//! revoked, blocked or deleted, and count their requests per day. Webhooks: only the channel's
//! owner and its editors with `channels` manage them; URLs and event types are checked; at most 5
//! per channel; Send test queues a delivery. Through the real router against a real Postgres;
//! runs when `CYEX_TV_TEST_DATABASE_URL` is set (CI sets it) and skips otherwise.

mod common;

use std::sync::Arc;

use axum::Router;
use axum::body::Body;
use axum::http::{HeaderMap, Request, StatusCode};
use chrono::Duration;
use common::TestDb;
use cyex_tv_api::limits::{Limits, Rates};
use cyex_tv_api::state::AppState;
use cyex_tv_auth::Via;
use cyex_tv_auth::{Auth, AuthConfig, cookies, tokens};
use cyex_tv_db::accounts::{self, Profile};
use cyex_tv_db::sessions::{self, SessionKind};
use cyex_tv_db::{app_auth, channels, editors};
use cyex_tv_shared::model::{EditorPermission, Platform, RestrictionInput, RestrictionKind};
use cyex_tv_shared::ratelimit::Rate;
use http_body_util::BodyExt;
use serde_json::{Value, json};
use tower::ServiceExt;
use uuid::Uuid;

const SITE: &str = "https://cyex.tv";
const KEYS: &str = "{ myApiKeys { id name prefix createdAt lastUsedAt usage { day requests } } }";
const CREATE_KEY: &str =
    "mutation($name: String!) { createApiKey(name: $name) { key apiKey { id name prefix } } }";
const REVOKE_KEY: &str = "mutation($id: String!) { revokeApiKey(id: $id) }";
const HOOKS: &str = "query($id: String!) { channelWebhooks(platform: TWITCH, id: $id) { id url events enabled disabledReason failedDeliveries } }";
const CREATE_HOOK: &str = "mutation($id: String!, $url: String!, $events: [EventType!]!) { createWebhook(platform: TWITCH, id: $id, input: { url: $url, events: $events }) { secret webhook { id url events enabled } } }";

struct User {
    id: Uuid,
    site: String,
    app: String,
}

struct World {
    db: TestDb,
    app: Router,
    state: AppState,
}

struct Answer {
    status: StatusCode,
    headers: HeaderMap,
    body: Value,
}

#[derive(Clone, Copy)]
enum As<'a> {
    Nobody,
    Site(&'a str),
    Bearer(&'a str),
}

fn code(r: &Value) -> &str {
    r["errors"][0]["extensions"]["code"]
        .as_str()
        .or_else(|| r["error"].as_str())
        .unwrap_or_default()
}

impl World {
    async fn new(rates: Rates) -> Option<Self> {
        let db = TestDb::new().await?;
        let cfg = AuthConfig::from_source(|_| None).unwrap();
        let state = AppState::new(db.pool.clone(), Arc::new(Auth::new(cfg).unwrap()))
            .with_limits(Limits::new(rates, false));
        Some(Self {
            app: cyex_tv_api::app(state.clone()),
            state,
            db,
        })
    }

    async fn user(&self, twitch_id: &str) -> User {
        let mut conn = self.db.conn().await;
        let login = format!("user{twitch_id}");
        let id = accounts::sign_in(
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
        .unwrap()
        .account
        .id;
        channels::upsert(&mut conn, id, Platform::Twitch, twitch_id, &login)
            .await
            .unwrap();
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

    fn with(mut r: axum::http::request::Builder, who: As<'_>) -> axum::http::request::Builder {
        match who {
            As::Nobody => {}
            As::Site(cookie) => {
                r = r
                    .header("cookie", format!("{}={cookie}", cookies::SESSION))
                    .header("origin", SITE);
            }
            As::Bearer(token) => r = r.header("authorization", format!("Bearer {token}")),
        }
        r
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

    async fn post_gql(&self, who: As<'_>, query: &str, variables: Value) -> Answer {
        let request = Self::with(
            Request::post("/v1/gql").header("content-type", "application/json"),
            who,
        )
        .body(Body::from(
            json!({ "query": query, "variables": variables }).to_string(),
        ))
        .unwrap();
        self.send(request).await
    }

    async fn gql(&self, who: As<'_>, query: &str, variables: Value) -> Value {
        let a = self.post_gql(who, query, variables).await;
        assert_eq!(a.status, StatusCode::OK, "{}", a.body);
        a.body
    }

    async fn get(&self, who: As<'_>, path: &str) -> Answer {
        self.send(
            Self::with(Request::get(path), who)
                .body(Body::empty())
                .unwrap(),
        )
        .await
    }

    async fn make_key(&self, user: &User, name: &str) -> (String, String) {
        let r = self
            .gql(As::Site(&user.site), CREATE_KEY, json!({ "name": name }))
            .await;
        let made = &r["data"]["createApiKey"];
        assert!(made.is_object(), "{r}");
        (
            made["key"].as_str().unwrap().to_owned(),
            made["apiKey"]["id"].as_str().unwrap().to_owned(),
        )
    }
}

fn header(a: &Answer, name: &str) -> String {
    a.headers
        .get(name)
        .map(|v| v.to_str().unwrap().to_owned())
        .unwrap_or_default()
}

#[tokio::test]
async fn keys_are_made_on_the_website_and_shown_once() {
    let Some(w) = World::new(Rates::default()).await else {
        return;
    };
    let alice = w.user("6101").await;

    let r = w
        .gql(As::Nobody, CREATE_KEY, json!({ "name": "bot" }))
        .await;
    assert_eq!(code(&r), "signed_out");
    // An app's token may not, and neither may a key.
    let r = w
        .gql(As::Bearer(&alice.app), CREATE_KEY, json!({ "name": "bot" }))
        .await;
    assert!(!code(&r).is_empty(), "{r}");

    let (key, id) = w.make_key(&alice, "  my bot ").await;
    assert!(key.starts_with("cyex_live_") && key.len() == 53, "{key}");
    let list = w.gql(As::Site(&alice.site), KEYS, json!({})).await;
    let keys = list["data"]["myApiKeys"].as_array().unwrap().clone();
    assert_eq!(keys.len(), 1);
    assert_eq!(keys[0]["id"], id.as_str());
    assert_eq!(keys[0]["name"], "my bot");
    assert_eq!(keys[0]["prefix"], &key[..18]);
    assert!(keys[0]["lastUsedAt"].is_null());
    assert!(
        !list.to_string().contains(&key),
        "the key itself is never listed"
    );

    let r = w
        .post_gql(As::Bearer(&key), CREATE_KEY, json!({ "name": "again" }))
        .await;
    assert_eq!(code(&r.body), "api_key_read_only", "{}", r.body);

    for bad in ["", "   ", &"x".repeat(65)] {
        let r = w
            .gql(As::Site(&alice.site), CREATE_KEY, json!({ "name": bad }))
            .await;
        assert_eq!(code(&r), "bad_name", "{r}");
    }
    for n in 2..=10 {
        w.make_key(&alice, &format!("bot {n}")).await;
    }
    let r = w
        .gql(
            As::Site(&alice.site),
            CREATE_KEY,
            json!({ "name": "eleven" }),
        )
        .await;
    assert_eq!(code(&r), "too_many_api_keys", "{r}");
    w.db.close().await;
}

#[tokio::test]
async fn a_key_reads_but_never_acts() {
    let Some(w) = World::new(Rates::default()).await else {
        return;
    };
    let alice = w.user("6201").await;
    let (key, _) = w.make_key(&alice, "reader").await;

    let a = w.get(As::Bearer(&key), "/v1/sets/global").await;
    assert_eq!(a.status, StatusCode::OK, "{}", a.body);
    assert_eq!(
        header(&a, "ratelimit-limit"),
        "3000",
        "the key's own bucket"
    );
    // It carries the key's quota, so no shared cache may keep it.
    let control = header(&a, "cache-control");
    assert!(control.starts_with("private"), "{control}");
    assert!(!control.contains("public"), "{control}");
    // The same read without a key is public, and carries nobody's quota.
    let anon = w.get(As::Nobody, "/v1/sets/global").await;
    assert_eq!(anon.status, StatusCode::OK);
    assert!(header(&anon, "cache-control").starts_with("public"));
    assert!(header(&anon, "ratelimit-remaining").is_empty());
    assert!(header(&anon, "ratelimit-limit").is_empty());

    // GraphQL queries run signed out: the key is not the account.
    let r = w
        .post_gql(As::Bearer(&key), "{ me { id } }", json!({}))
        .await;
    assert_eq!(r.status, StatusCode::OK);
    assert_eq!(code(&r.body), "signed_out", "{}", r.body);
    assert_eq!(header(&r, "ratelimit-limit"), "3000");
    let r = w.post_gql(As::Bearer(&key), KEYS, json!({})).await;
    assert_eq!(code(&r.body), "signed_out");

    // No mutation runs, not even one a signed-out person may try.
    for m in [
        "mutation { signOutEverywhere }",
        "mutation { createConsoleToken { token } }",
        "mutation($id: String!) { revokeApiKey(id: $id) }",
    ] {
        let r = w.post_gql(As::Bearer(&key), m, json!({ "id": "x" })).await;
        assert_eq!(code(&r.body), "api_key_read_only", "{m}: {}", r.body);
    }
    // Not a read: refused before any handler, the cookie ignored.
    for (method, path) in [
        ("POST", "/v1/emotes"),
        ("POST", "/v1/auth/logout"),
        ("POST", "/v1/auth/token/export"),
        ("GET", "/v1/auth/twitch/login"),
    ] {
        let request = Request::builder()
            .method(method)
            .uri(path)
            .header("authorization", format!("Bearer {key}"))
            .header("cookie", format!("{}={}", cookies::SESSION, alice.site))
            .header("origin", SITE)
            .body(Body::empty())
            .unwrap();
        let a = w.send(request).await;
        assert_eq!(a.status, StatusCode::FORBIDDEN, "{path}");
        assert_eq!(code(&a.body), "api_key_read_only", "{path}");
    }
    // The website's session is untouched by any of that.
    let me = w
        .gql(As::Site(&alice.site), "{ me { id } }", json!({}))
        .await;
    assert!(me["data"]["me"]["id"].is_string(), "{me}");

    // A key that isn't one.
    for bad in [
        "cyex_live_nope",
        &format!("cyex_live_{}", tokens::random_token()),
    ] {
        let a = w.get(As::Bearer(bad), "/v1/sets/global").await;
        assert_eq!(a.status, StatusCode::UNAUTHORIZED, "{bad}");
        assert_eq!(code(&a.body), "api_key_invalid");
    }
    w.db.close().await;
}

#[tokio::test]
async fn a_key_has_its_own_bucket_and_counts_its_requests() {
    let rates = Rates {
        api_key: Rate::per_minute(3),
        ..Rates::default()
    };
    let Some(w) = World::new(rates).await else {
        return;
    };
    let alice = w.user("6301").await;
    let (key, id) = w.make_key(&alice, "busy").await;
    let (other, _) = w.make_key(&alice, "quiet").await;

    for left in [2, 1, 0] {
        let a = w.get(As::Bearer(&key), "/v1/sets/global").await;
        assert_eq!(a.status, StatusCode::OK);
        assert_eq!(header(&a, "ratelimit-limit"), "3");
        assert_eq!(header(&a, "ratelimit-remaining"), left.to_string());
    }
    let a = w.get(As::Bearer(&key), "/v1/sets/global").await;
    assert_eq!(a.status, StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(code(&a.body), "rate_limited");
    assert!(!header(&a, "retry-after").is_empty());
    assert_eq!(header(&a, "ratelimit-remaining"), "0");
    // Another key, and signed-out requests from the same address, are their own buckets.
    assert_eq!(
        w.get(As::Bearer(&other), "/v1/sets/global").await.status,
        StatusCode::OK
    );
    // (A public answer carries no `RateLimit-*`, so the status is what shows it.)
    let anon = w.get(As::Nobody, "/v1/sets/global").await;
    assert_eq!(anon.status, StatusCode::OK);

    w.state.key_usage.flush(&w.db.pool).await.unwrap();
    let list = w.gql(As::Site(&alice.site), KEYS, json!({})).await;
    let busy = list["data"]["myApiKeys"]
        .as_array()
        .unwrap()
        .iter()
        .find(|k| k["id"] == id.as_str())
        .unwrap()
        .clone();
    assert_eq!(busy["usage"].as_array().unwrap().len(), 1, "{busy}");
    assert_eq!(
        busy["usage"][0]["requests"], 3,
        "refused requests don't count"
    );
    assert_eq!(
        busy["usage"][0]["day"],
        chrono::Utc::now()
            .date_naive()
            .format("%Y-%m-%d")
            .to_string()
    );
    assert!(busy["lastUsedAt"].is_string());
    // Counted again: added to the same day.
    w.state.key_usage.flush(&w.db.pool).await.unwrap();
    w.db.close().await;
}

#[tokio::test]
async fn keys_end_when_revoked_blocked_or_deleted_but_not_on_sign_out() {
    let Some(w) = World::new(Rates::default()).await else {
        return;
    };
    let alice = w.user("6401").await;
    let (key, id) = w.make_key(&alice, "a").await;
    let (kept, _) = w.make_key(&alice, "b").await;

    // Signing out everywhere ends sessions, not keys.
    let r = w
        .gql(
            As::Site(&alice.site),
            "mutation { signOutEverywhere }",
            json!({}),
        )
        .await;
    assert!(r["errors"].is_null(), "{r}");
    assert_eq!(
        w.get(As::Bearer(&key), "/v1/sets/global").await.status,
        StatusCode::OK
    );

    let bob = w.user("6402").await;
    let r = w
        .gql(As::Site(&bob.site), REVOKE_KEY, json!({ "id": id }))
        .await;
    assert_eq!(r["data"]["revokeApiKey"], false, "only your own keys");
    let alice = w.user("6401").await; // signed in again
    let r = w
        .gql(As::Site(&alice.site), REVOKE_KEY, json!({ "id": id }))
        .await;
    assert_eq!(r["data"]["revokeApiKey"], true, "{r}");
    let a = w.get(As::Bearer(&key), "/v1/sets/global").await;
    assert_eq!(a.status, StatusCode::UNAUTHORIZED);
    assert_eq!(code(&a.body), "api_key_invalid");

    // Blocked: refused while it lasts. (Written straight to the database here, as another api
    // instance would: the lookup cache keeps it for its TTL, forgotten here to stand for that.)
    let mut conn = w.db.conn().await;
    sqlx::query("UPDATE accounts SET blocked_at = now(), blocked_until = NULL WHERE id = $1")
        .bind(alice.id)
        .execute(&mut *conn)
        .await
        .unwrap();
    w.state.key_cache.forget_account(alice.id);
    assert_eq!(
        w.get(As::Bearer(&kept), "/v1/sets/global").await.status,
        StatusCode::UNAUTHORIZED
    );
    sqlx::query("UPDATE accounts SET blocked_at = NULL WHERE id = $1")
        .bind(alice.id)
        .execute(&mut *conn)
        .await
        .unwrap();
    w.state.key_cache.forget_account(alice.id);
    assert_eq!(
        w.get(As::Bearer(&kept), "/v1/sets/global").await.status,
        StatusCode::OK
    );

    // Deleted (through the website, as people do): the keys go with it at once, the cached
    // lookup too, and stay gone after a restore.
    let r = w
        .gql(
            As::Site(&alice.site),
            "mutation($confirm: String!) { deleteAccount(confirm: $confirm) }",
            json!({ "confirm": "user6401" }),
        )
        .await;
    assert!(r["errors"].is_null(), "{r}");
    let left: i64 = sqlx::query_scalar("SELECT count(*) FROM api_keys WHERE account_id = $1")
        .bind(alice.id)
        .fetch_one(&mut *conn)
        .await
        .unwrap();
    assert_eq!(left, 0);
    drop(conn);
    assert_eq!(
        w.get(As::Bearer(&kept), "/v1/sets/global").await.status,
        StatusCode::UNAUTHORIZED
    );
    w.db.close().await;
}

#[tokio::test]
async fn webhooks_are_for_the_channel_and_its_editors() {
    let Some(w) = World::new(Rates::default()).await else {
        return;
    };
    let owner = w.user("6501").await;
    let editor = w.user("6502").await;
    let sets_only = w.user("6503").await;
    let stranger = w.user("6504").await;
    let mut conn = w.db.conn().await;
    editors::add(
        &mut conn,
        owner.id,
        editor.id,
        &[EditorPermission::Channels.as_str().to_owned()],
        None,
    )
    .await
    .unwrap();
    editors::add(
        &mut conn,
        owner.id,
        sets_only.id,
        &[EditorPermission::Sets.as_str().to_owned()],
        None,
    )
    .await
    .unwrap();
    drop(conn);
    let vars = |url: &str, events: Value| json!({ "id": "6501", "url": url, "events": events });

    let r = w
        .gql(
            As::Site(&owner.site),
            CREATE_HOOK,
            vars(
                "https://hooks.example.com/cyex",
                json!(["SET_EMOTE_ADDED", "EMOTE_UPDATED"]),
            ),
        )
        .await;
    let made = &r["data"]["createWebhook"];
    let secret = made["secret"].as_str().unwrap_or_else(|| panic!("{r}"));
    assert!(secret.starts_with("cyexhook_"));
    let hook_id = made["webhook"]["id"].as_str().unwrap().to_owned();
    assert_eq!(
        made["webhook"]["events"],
        json!(["SET_EMOTE_ADDED", "EMOTE_UPDATED"])
    );

    // The editor with `channels` sees and adds; the rest may not.
    let list = w
        .gql(As::Site(&editor.site), HOOKS, json!({ "id": "6501" }))
        .await;
    assert_eq!(
        list["data"]["channelWebhooks"].as_array().unwrap().len(),
        1,
        "{list}"
    );
    assert!(
        !list.to_string().contains(secret),
        "the secret is never listed"
    );
    let r = w
        .gql(
            As::Site(&editor.site),
            CREATE_HOOK,
            vars(
                "https://hooks.example.com/two",
                json!(["SET_EMOTE_REMOVED"]),
            ),
        )
        .await;
    assert!(r["data"]["createWebhook"].is_object(), "{r}");
    for who in [&sets_only, &stranger] {
        let r = w
            .gql(As::Site(&who.site), HOOKS, json!({ "id": "6501" }))
            .await;
        assert_eq!(code(&r), "forbidden", "{r}");
        let r = w
            .gql(
                As::Site(&who.site),
                CREATE_HOOK,
                vars("https://hooks.example.com/x", json!(["SET_EMOTE_ADDED"])),
            )
            .await;
        assert_eq!(code(&r), "forbidden", "{r}");
        let r = w
            .gql(
                As::Site(&who.site),
                "mutation($id: String!) { deleteWebhook(id: $id) }",
                json!({ "id": hook_id }),
            )
            .await;
        assert_eq!(code(&r), "not_found", "{r}");
        let r = w
            .gql(
                As::Site(&who.site),
                "query($id: String!) { webhookDeliveries(id: $id) { id } }",
                json!({ "id": hook_id }),
            )
            .await;
        assert_eq!(code(&r), "not_found", "{r}");
    }
    let r = w.gql(As::Nobody, HOOKS, json!({ "id": "6501" })).await;
    assert_eq!(code(&r), "signed_out");

    // A key never manages webhooks.
    let (key, _) = w.make_key(&owner, "k").await;
    let r = w
        .post_gql(
            As::Bearer(&key),
            CREATE_HOOK,
            vars("https://hooks.example.com/k", json!(["SET_EMOTE_ADDED"])),
        )
        .await;
    assert_eq!(code(&r.body), "api_key_read_only");

    // URLs and event types are checked.
    for url in [
        "http://hooks.example.com/",
        "https://127.0.0.1/hook",
        "https://169.254.169.254/latest",
        "https://[::1]/",
        "https://[::ffff:10.0.0.1]/",
        "https://localhost:8443/",
        "https://hooks.example.com:8443/hook",
        "https://hooks.example.com:80/",
        "https://user:pw@hooks.example.com/",
        "not a url",
    ] {
        let r = w
            .gql(
                As::Site(&owner.site),
                CREATE_HOOK,
                vars(url, json!(["SET_EMOTE_ADDED"])),
            )
            .await;
        assert_eq!(code(&r), "bad_webhook_url", "{url}: {r}");
    }
    for events in [json!([]), json!(["PERSONAL_UPDATED"]), json!(["RESYNC"])] {
        let r = w
            .gql(
                As::Site(&owner.site),
                CREATE_HOOK,
                vars("https://hooks.example.com/e", events.clone()),
            )
            .await;
        assert_eq!(code(&r), "bad_webhook_events", "{events}: {r}");
    }

    // Five per channel.
    for n in 3..=5 {
        let r = w
            .gql(
                As::Site(&owner.site),
                CREATE_HOOK,
                vars(
                    &format!("https://hooks.example.com/{n}"),
                    json!(["SET_EMOTE_ADDED"]),
                ),
            )
            .await;
        assert!(r["data"]["createWebhook"].is_object(), "{r}");
    }
    let r = w
        .gql(
            As::Site(&owner.site),
            CREATE_HOOK,
            vars("https://hooks.example.com/6", json!(["SET_EMOTE_ADDED"])),
        )
        .await;
    assert_eq!(code(&r), "too_many_webhooks", "{r}");

    // Off by hand, a test, the log, a new secret, gone.
    let r = w
        .gql(
            As::Site(&owner.site),
            "mutation($id: String!) { updateWebhook(id: $id, input: { enabled: false }) { enabled disabledReason } }",
            json!({ "id": hook_id }),
        )
        .await;
    assert_eq!(
        r["data"]["updateWebhook"],
        json!({ "enabled": false, "disabledReason": "manual" }),
        "{r}"
    );
    let r = w
        .gql(
            As::Site(&owner.site),
            "mutation($id: String!) { testWebhook(id: $id) { id eventType state } }",
            json!({ "id": hook_id }),
        )
        .await;
    assert_eq!(r["data"]["testWebhook"]["eventType"], "webhook.test", "{r}");
    assert_eq!(r["data"]["testWebhook"]["state"], "pending");
    let log = w
        .gql(
            As::Site(&editor.site),
            "query($id: String!) { webhookDeliveries(id: $id) { eventType state } }",
            json!({ "id": hook_id }),
        )
        .await;
    assert_eq!(
        log["data"]["webhookDeliveries"].as_array().unwrap().len(),
        1,
        "{log}"
    );
    let r = w
        .gql(
            As::Site(&owner.site),
            "mutation($id: String!) { rotateWebhookSecret(id: $id) { secret } }",
            json!({ "id": hook_id }),
        )
        .await;
    let new_secret = r["data"]["rotateWebhookSecret"]["secret"].as_str().unwrap();
    assert!(new_secret.starts_with("cyexhook_") && new_secret != secret);
    let r = w
        .gql(
            As::Site(&editor.site),
            "mutation($id: String!) { deleteWebhook(id: $id) }",
            json!({ "id": hook_id }),
        )
        .await;
    assert_eq!(r["data"]["deleteWebhook"], true, "{r}");
    let list = w
        .gql(As::Site(&owner.site), HOOKS, json!({ "id": "6501" }))
        .await;
    assert_eq!(list["data"]["channelWebhooks"].as_array().unwrap().len(), 4);
    w.db.close().await;
}

/// Staff acting with every permission (an owner's website session), for the block below.
fn staff(id: Uuid) -> cyex_tv_auth::Caller {
    cyex_tv_auth::Caller {
        account_id: id,
        owner: true,
        token_hash: [0; 32],
        kind: SessionKind::Site,
        via: Via::Cookie,
        app_client: None,
    }
}

#[tokio::test]
async fn made_up_keys_never_reach_the_database_past_the_ip_limit() {
    let rates = Rates {
        reads: Rate::per_minute(3),
        ..Rates::default()
    };
    let Some(w) = World::new(rates).await else {
        return;
    };
    let bad = format!("cyex_live_{}", tokens::random_token());
    // A miss costs the IP a token before the lookup, and the "no" one after.
    assert_eq!(
        w.get(As::Bearer(&bad), "/v1/sets/global").await.status,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(w.state.key_cache.lookups(), 1);
    // The same made-up key again: answered from the cache.
    assert_eq!(
        w.get(As::Bearer(&bad), "/v1/sets/global").await.status,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        w.state.key_cache.lookups(),
        1,
        "a known bad key isn't looked up again"
    );
    // The IP's allowance is spent: another made-up key is refused before any lookup.
    let other = format!("cyex_live_{}", tokens::random_token());
    let a = w.get(As::Bearer(&other), "/v1/sets/global").await;
    assert_eq!(a.status, StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(
        w.state.key_cache.lookups(),
        1,
        "no lookup past the IP limit"
    );
    w.db.close().await;
}

#[tokio::test]
async fn an_account_has_one_bucket_for_all_its_keys() {
    let rates = Rates {
        api_key: Rate::per_minute(3),
        api_key_account: Rate::per_minute(4),
        ..Rates::default()
    };
    let Some(w) = World::new(rates).await else {
        return;
    };
    let alice = w.user("6601").await;
    let (one, _) = w.make_key(&alice, "one").await;
    let (two, _) = w.make_key(&alice, "two").await;
    for _ in 0..3 {
        assert_eq!(
            w.get(As::Bearer(&one), "/v1/sets/global").await.status,
            StatusCode::OK
        );
    }
    let a = w.get(As::Bearer(&two), "/v1/sets/global").await;
    assert_eq!(a.status, StatusCode::OK);
    assert_eq!(
        header(&a, "ratelimit-limit"),
        "4",
        "the tighter bucket is the account's"
    );
    assert_eq!(header(&a, "ratelimit-remaining"), "0");
    let a = w.get(As::Bearer(&two), "/v1/sets/global").await;
    assert_eq!(
        a.status,
        StatusCode::TOO_MANY_REQUESTS,
        "a second key adds nothing"
    );
    // Another account's key has its own.
    let bob = w.user("6602").await;
    let (theirs, _) = w.make_key(&bob, "b").await;
    assert_eq!(
        w.get(As::Bearer(&theirs), "/v1/sets/global").await.status,
        StatusCode::OK
    );
    w.db.close().await;
}

#[tokio::test]
async fn a_staff_block_ends_keys_at_once_and_turns_webhooks_off() {
    let Some(w) = World::new(Rates::default()).await else {
        return;
    };
    let alice = w.user("6701").await;
    let mod_ = w.user("6702").await;
    let (key, _) = w.make_key(&alice, "k").await;
    assert_eq!(
        w.get(As::Bearer(&key), "/v1/sets/global").await.status,
        StatusCode::OK
    );
    let r = w
        .gql(
            As::Site(&alice.site),
            CREATE_HOOK,
            json!({ "id": "6701", "url": "https://hooks.example.com/a", "events": ["SET_EMOTE_ADDED"] }),
        )
        .await;
    assert!(r["data"]["createWebhook"].is_object(), "{r}");
    cyex_tv_api::abuse::place(
        &w.state,
        &staff(mod_.id),
        &cyex_tv_db::ids::to_text(alice.id),
        &RestrictionInput {
            kind: RestrictionKind::Account,
            until: None,
            reason: "spam".to_owned(),
        },
    )
    .await
    .unwrap();
    // The key was cached as good a moment ago; the block forgets it on this instance.
    let a = w.get(As::Bearer(&key), "/v1/sets/global").await;
    assert_eq!(a.status, StatusCode::UNAUTHORIZED, "{}", a.body);
    let mut conn = w.db.conn().await;
    let hooks: Vec<(bool, Option<String>)> = sqlx::query_as(
        "SELECT w.enabled, w.disabled_reason FROM webhooks w JOIN channels c ON c.id = w.channel_id
          WHERE c.account_id = $1",
    )
    .bind(alice.id)
    .fetch_all(&mut *conn)
    .await
    .unwrap();
    assert_eq!(hooks, vec![(false, Some("blocked".to_owned()))]);
    drop(conn);
    w.db.close().await;
}

#[tokio::test]
async fn editors_see_only_the_host_of_a_webhook_url() {
    let Some(w) = World::new(Rates::default()).await else {
        return;
    };
    let owner = w.user("6801").await;
    let editor = w.user("6802").await;
    let mut conn = w.db.conn().await;
    editors::add(
        &mut conn,
        owner.id,
        editor.id,
        &[EditorPermission::Channels.as_str().to_owned()],
        None,
    )
    .await
    .unwrap();
    drop(conn);
    let token_url = "https://chat.example.com/api/webhooks/1/s3cr3t-token";
    let r = w
        .gql(
            As::Site(&owner.site),
            CREATE_HOOK,
            json!({ "id": "6801", "url": token_url, "events": ["SET_EMOTE_ADDED"] }),
        )
        .await;
    assert_eq!(
        r["data"]["createWebhook"]["webhook"]["url"], token_url,
        "{r}"
    );
    let r = w
        .gql(As::Site(&editor.site), CREATE_HOOK, json!({ "id": "6801", "url": "https://hooks.example.com/editor-token", "events": ["SET_EMOTE_ADDED"] }))
        .await;
    assert!(r["data"]["createWebhook"].is_object(), "{r}");
    let urls = |list: &Value| -> Vec<String> {
        list["data"]["channelWebhooks"]
            .as_array()
            .unwrap()
            .iter()
            .map(|h| h["url"].as_str().unwrap().to_owned())
            .collect()
    };
    // The owner sees both whole; the editor sees their own whole and the owner's as its host.
    let all = w
        .gql(As::Site(&owner.site), HOOKS, json!({ "id": "6801" }))
        .await;
    assert_eq!(
        urls(&all),
        [token_url, "https://hooks.example.com/editor-token"]
    );
    let theirs = w
        .gql(As::Site(&editor.site), HOOKS, json!({ "id": "6801" }))
        .await;
    assert_eq!(
        urls(&theirs),
        [
            "https://chat.example.com/…",
            "https://hooks.example.com/editor-token"
        ]
    );
    assert!(!theirs.to_string().contains("s3cr3t"));
    // The audit log keeps the host only.
    let mut conn = w.db.conn().await;
    let logged: Vec<String> = sqlx::query_scalar(
        "SELECT coalesce(after_json::text, '') || coalesce(before_json::text, '') FROM audit_log
          WHERE action LIKE 'channel.webhook_%'",
    )
    .fetch_all(&mut *conn)
    .await
    .unwrap();
    assert!(!logged.is_empty());
    assert!(
        logged
            .iter()
            .all(|l| !l.contains("s3cr3t") && !l.contains("editor-token")),
        "{logged:?}"
    );
    drop(conn);
    w.db.close().await;
}

#[tokio::test]
async fn deleting_webhooks_counts_under_the_settings_limit() {
    let rates = Rates {
        prefs: Rate::per_hour(3),
        ..Rates::default()
    };
    let Some(w) = World::new(rates).await else {
        return;
    };
    let owner = w.user("6901").await;
    let mut ids = Vec::new();
    for n in 0..2 {
        let r = w
            .gql(As::Site(&owner.site), CREATE_HOOK, json!({ "id": "6901", "url": format!("https://hooks.example.com/{n}"), "events": ["SET_EMOTE_ADDED"] }))
            .await;
        ids.push(
            r["data"]["createWebhook"]["webhook"]["id"]
                .as_str()
                .unwrap_or_else(|| panic!("{r}"))
                .to_owned(),
        );
    }
    let del = "mutation($id: String!) { deleteWebhook(id: $id) }";
    let r = w
        .gql(As::Site(&owner.site), del, json!({ "id": ids[0] }))
        .await;
    assert_eq!(r["data"]["deleteWebhook"], true, "{r}");
    let r = w
        .gql(As::Site(&owner.site), del, json!({ "id": ids[1] }))
        .await;
    assert_eq!(code(&r), "rate_limited", "{r}");
    w.db.close().await;
}
