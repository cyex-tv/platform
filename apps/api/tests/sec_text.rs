//! Bidi controls, invisible characters and NUL are refused in names and reasons, through the real
//! router against a real Postgres. Runs when `CYEX_TV_TEST_DATABASE_URL` is set (CI sets it) and
//! skips otherwise.

mod common;

use std::sync::Arc;

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use chrono::Duration;
use common::TestDb;
use cyex_tv_api::state::AppState;
use cyex_tv_auth::{Auth, AuthConfig, tokens};
use cyex_tv_db::accounts::{self, Profile};
use cyex_tv_db::emotes::{self, NewEmote};
use cyex_tv_db::sessions::{self, SessionKind};
use cyex_tv_db::{channels, ids};
use cyex_tv_shared::model::{EmoteState, Platform};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use tower::ServiceExt;
use uuid::Uuid;

struct User {
    id: Uuid,
    token: String,
}

struct Reply {
    status: StatusCode,
    body: Value,
}

struct World {
    db: TestDb,
    app: Router,
}

fn code(response: &Value) -> &str {
    response["errors"][0]["extensions"]["code"]
        .as_str()
        .unwrap_or_default()
}

impl World {
    async fn new() -> Option<Self> {
        let db = TestDb::new().await?;
        let auth = Arc::new(Auth::new(AuthConfig::from_source(|_| None).unwrap()).unwrap());
        let state = AppState::new(db.pool.clone(), auth).with_cdn_url("https://cdn.example.test/");
        Some(Self {
            app: cyex_tv_api::app(state),
            db,
        })
    }

    /// A Twitch user with a channel and a site session.
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
        let token = tokens::random_token();
        sessions::create(
            &mut conn,
            &tokens::hash(&token),
            signed.account.id,
            SessionKind::Site,
            Duration::days(1),
        )
        .await
        .unwrap();
        User {
            id: signed.account.id,
            token,
        }
    }

    async fn emote(&self, owner: &User, name: &str, state: EmoteState, private: bool) -> String {
        let mut conn = self.db.conn().await;
        let emote = emotes::create(
            &mut conn,
            &NewEmote {
                owner_account_id: owner.id,
                name,
                private,
                ..NewEmote::default()
            },
        )
        .await
        .unwrap();
        if state != EmoteState::Pending {
            emotes::set_state(&mut conn, emote.id, state).await.unwrap();
        }
        ids::to_text(emote.id)
    }

    async fn send(&self, request: Request<Body>) -> Reply {
        let response = self.app.clone().oneshot(request).await.unwrap();
        let status = response.status();
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        Reply {
            status,
            body: serde_json::from_slice(&bytes).unwrap_or(Value::Null),
        }
    }

    async fn gql(&self, user: Option<&User>, query: &str, variables: Value) -> Value {
        let mut request = Request::post("/v1/gql").header("content-type", "application/json");
        if let Some(user) = user {
            request = request
                .header(
                    "cookie",
                    format!("{}={}", cyex_tv_auth::cookies::SESSION, user.token),
                )
                .header("origin", "https://cyex.tv");
        }
        let reply = self
            .send(
                request
                    .body(Body::from(
                        json!({ "query": query, "variables": variables }).to_string(),
                    ))
                    .unwrap(),
            )
            .await;
        assert_eq!(reply.status, StatusCode::OK, "{:?}", reply.body);
        reply.body
    }

    async fn create_set(&self, user: &User, name: &str, kind: &str) -> String {
        let r = self
            .gql(
                Some(user),
                "mutation($n: String!, $k: SetKind!) { createSet(input: { name: $n, kind: $k }) { id kind capacity } }",
                json!({ "n": name, "k": kind }),
            )
            .await;
        r["data"]["createSet"]["id"]
            .as_str()
            .unwrap_or_else(|| panic!("{r}"))
            .to_owned()
    }

    async fn add(&self, user: &User, set: &str, emote: &str, alias: Option<&str>) -> Value {
        self.gql(
            Some(user),
            "mutation($s: String!, $e: String!, $n: String) { addSetEmote(setId: $s, emoteId: $e, input: { name: $n }) { name emote { id name state } } }",
            json!({ "s": set, "e": emote, "n": alias }),
        )
        .await
    }
}

#[tokio::test]
async fn hidden_characters_are_refused() {
    let Some(w) = World::new().await else { return };
    let alice = w.user("101").await;
    let bob = w.user("102").await;

    let create = "mutation($n: String!) { createSet(input: { name: $n, kind: CHANNEL }) { id } }";
    let r = w
        .gql(Some(&alice), create, json!({ "n": "Main\u{202E}" }))
        .await;
    assert_eq!(code(&r), "bad_name", "{r}");
    // A normal name still works.
    let set = w.create_set(&alice, "Main", "CHANNEL").await;

    let emote = w
        .emote(&alice, "aliceOk", EmoteState::Approved, false)
        .await;
    let r = w.add(&alice, &set, &emote, Some("KEKW\u{200B}")).await;
    assert_eq!(code(&r), "bad_name", "{r}");
    let r = w.add(&alice, &set, &emote, Some("KEKW")).await;
    assert_eq!(r["data"]["addSetEmote"]["name"], "KEKW", "{r}");

    let r = w
        .gql(
            Some(&alice),
            "mutation($s: String!, $e: String!, $n: String) { updateSetEmote(setId: $s, emoteId: $e, input: { name: $n }) { name } }",
            json!({ "s": set, "e": emote, "n": "KE\u{200B}KW" }),
        )
        .await;
    assert_eq!(code(&r), "bad_name", "{r}");

    let report = "mutation($id: String!, $r: String!) { reportEmote(id: $id, reason: $r) { id } }";
    let r = w
        .gql(
            Some(&bob),
            report,
            json!({ "id": emote, "r": "bad\u{0}reason" }),
        )
        .await;
    assert_eq!(code(&r), "bad_reason", "{r}");
    let r = w
        .gql(
            Some(&bob),
            report,
            json!({ "id": emote, "r": "spam\u{202E}" }),
        )
        .await;
    assert_eq!(code(&r), "bad_reason", "{r}");

    w.db.close().await;
}
