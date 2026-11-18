//! Emote activity (`emoteActivity`, gap list G2): uploads and set changes about one emote, built
//! from the activity store (ADR 0016), with only public-safe lines: listed sets, no moderation,
//! nothing for pending, private or deleted emotes, nothing by or about blocked accounts. Through
//! the real router against a real Postgres; runs when `CYEX_TV_TEST_DATABASE_URL` is set (CI sets
//! it) and skips otherwise.

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
use cyex_tv_db::{ids, roles};
use cyex_tv_shared::model::Platform;
use http_body_util::BodyExt;
use serde_json::{Value, json};
use tower::ServiceExt;
use uuid::Uuid;

struct User {
    id: Uuid,
    token: String,
}

struct World {
    db: TestDb,
    app: Router,
}

const FEED: &str = "query($e: String!, $b: String, $l: Int) {
    emoteActivity(emoteId: $e, before: $b, limit: $l) {
        id kind actor { id } set { id name } emote { id name } fromName toName
    }
}";

impl World {
    async fn new() -> Option<Self> {
        let db = TestDb::new().await?;
        let auth = Arc::new(Auth::new(AuthConfig::from_source(|_| None).unwrap()).unwrap());
        let state = AppState::new(db.pool.clone(), auth);
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
        let token = tokens::random_token();
        sessions::create(
            &mut conn,
            &tokens::hash(&token),
            signed.account.id,
            SessionKind::App,
            Duration::days(1),
        )
        .await
        .unwrap();
        User {
            id: signed.account.id,
            token,
        }
    }

    async fn post(&self, user: Option<&User>, query: &str, variables: Value) -> Value {
        let mut request = Request::post("/v1/gql").header("content-type", "application/json");
        if let Some(user) = user {
            request = request.header("authorization", format!("Bearer {}", user.token));
        }
        let request = request
            .body(Body::from(
                json!({ "query": query, "variables": variables }).to_string(),
            ))
            .unwrap();
        let response = self.app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        serde_json::from_slice(&bytes).unwrap()
    }

    async fn ok(&self, user: &User, query: &str, variables: Value) -> Value {
        let r = self.post(Some(user), query, variables).await;
        assert!(r["errors"].is_null(), "{r}");
        r
    }

    async fn set(&self, owner: &User, name: &str, vis: &str) -> String {
        let r = self
            .ok(
                owner,
                "mutation($n: String!, $v: SetVisibility) { createSet(input: { name: $n, kind: CHANNEL, visibility: $v }) { id } }",
                json!({ "n": name, "v": vis }),
            )
            .await;
        r["data"]["createSet"]["id"].as_str().unwrap().to_owned()
    }

    async fn feed(&self, emote: &str) -> Vec<Value> {
        let r = self.post(None, FEED, json!({ "e": emote })).await;
        r["data"]["emoteActivity"]
            .as_array()
            .unwrap_or_else(|| panic!("{r}"))
            .clone()
    }
}

const ADD: &str = "mutation($s: String!, $e: String!, $n: String) {
    addSetEmote(setId: $s, emoteId: $e, input: { name: $n }) { name }
}";
const RENAME: &str = "mutation($s: String!, $e: String!, $n: String!) {
    updateSetEmote(setId: $s, emoteId: $e, input: { name: $n }) { name }
}";
const REMOVE: &str =
    "mutation($s: String!, $e: String!) { removeSetEmote(setId: $s, emoteId: $e) }";

#[tokio::test]
async fn an_emotes_public_history() {
    let Some(w) = World::new().await else { return };
    let alice = w.user("5001").await;
    let bob = w.user("5002").await;
    let staff = w.user("5003").await;
    {
        let mut conn = w.db.conn().await;
        let role = roles::create(
            &mut conn,
            &roles::NewRole {
                name: "Reviewer",
                description: None,
                position: 10,
                permissions: &["emotes.review"],
                created_by: None,
            },
        )
        .await
        .unwrap();
        roles::assign(&mut conn, staff.id, role.id, None)
            .await
            .unwrap();
    }
    let emote = {
        let mut conn = w.db.conn().await;
        let e = emotes::create(
            &mut conn,
            &NewEmote {
                owner_account_id: alice.id,
                name: "aliceWave",
                ..NewEmote::default()
            },
        )
        .await
        .unwrap();
        ids::to_text(e.id)
    };
    // Pending: nothing at all, even after it went in Alice's own set.
    let alices = w.set(&alice, "Alice main", "LISTED").await;
    w.ok(&alice, ADD, json!({ "s": alices, "e": emote, "n": null }))
        .await;
    assert!(w.feed(&emote).await.is_empty());

    w.ok(
        &staff,
        "mutation($id: String!) { approveEmote(id: $id) { state } }",
        json!({ "id": emote }),
    )
    .await;
    let bobs = w.set(&bob, "Bob main", "LISTED").await;
    let bobs_secret = w.set(&bob, "Bob secret", "PRIVATE").await;
    let bobs_link = w.set(&bob, "Bob link", "UNLISTED").await;
    for s in [&bobs, &bobs_secret, &bobs_link] {
        w.ok(&bob, ADD, json!({ "s": s, "e": emote, "n": null }))
            .await;
    }
    w.ok(&bob, RENAME, json!({ "s": bobs, "e": emote, "n": "wave" }))
        .await;
    w.ok(&bob, REMOVE, json!({ "s": bobs, "e": emote })).await;

    let lines = w.feed(&emote).await;
    let kinds: Vec<&str> = lines.iter().map(|l| l["kind"].as_str().unwrap()).collect();
    assert_eq!(
        kinds,
        [
            "SET_EMOTE_REMOVED",
            "SET_EMOTE_RENAMED",
            "SET_EMOTE_ADDED",
            "EMOTE_UPLOADED",
            "SET_EMOTE_ADDED"
        ],
        "{lines:?}"
    );
    // Uploaded shows the owner, never the reviewer.
    assert_eq!(lines[3]["actor"]["id"], ids::to_text(alice.id).as_str());
    assert_eq!(lines[1]["fromName"], "aliceWave");
    assert_eq!(lines[1]["toName"], "wave");
    let text = serde_json::to_string(&lines).unwrap();
    assert!(!text.contains(&ids::to_text(staff.id)), "no reviewer");
    assert!(
        !text.contains(&bobs_secret) && !text.contains(&bobs_link),
        "{text}"
    );

    // Paging.
    let r = w
        .post(
            None,
            FEED,
            json!({ "e": emote, "b": lines[1]["id"], "l": 2 }),
        )
        .await;
    let page = r["data"]["emoteActivity"].as_array().unwrap();
    assert_eq!(page.len(), 2, "{r}");
    assert_eq!(page[0]["id"], lines[2]["id"]);

    // A blocked account's lines go; a private emote has no feed at all.
    {
        let mut conn = w.db.conn().await;
        accounts::set_blocked(&mut conn, bob.id, true)
            .await
            .unwrap();
    }
    let kinds: Vec<String> = w
        .feed(&emote)
        .await
        .iter()
        .map(|l| l["kind"].as_str().unwrap().to_owned())
        .collect();
    assert_eq!(kinds, ["EMOTE_UPLOADED", "SET_EMOTE_ADDED"]);
    w.ok(
        &alice,
        "mutation($id: String!) { updateEmote(id: $id, input: { private: true }) { id } }",
        json!({ "id": emote }),
    )
    .await;
    assert!(w.feed(&emote).await.is_empty());
    assert!(w.feed("nope").await.is_empty());
    w.db.close().await;
}
