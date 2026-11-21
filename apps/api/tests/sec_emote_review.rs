//! An owner's edit can't slip an unreviewed name past review, and unreviewed names never reach
//! the public event topics. Through the real router against a real Postgres; runs when
//! `CYEX_TV_TEST_DATABASE_URL` is set (CI sets it), skips otherwise.

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
use cyex_tv_db::{audit, channels, roles, sets};
use cyex_tv_shared::model::{EmoteState, Platform, SetKind};
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

fn code(response: &Value) -> &str {
    response["errors"][0]["extensions"]["code"]
        .as_str()
        .unwrap_or_default()
}

const UPDATE: &str = "mutation($id: String!, $n: String, $t: [String!], $p: Boolean) { updateEmote(id: $id, input: { name: $n, tags: $t, private: $p }) { name state tags flags { private } } }";
const SEARCH: &str = "query($q: String!) { searchEmotes(query: $q) { total items { name } } }";

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

    /// A Twitch user with a channel, a site session and, optionally, one permission.
    async fn user(&self, twitch_id: &str, permission: Option<&str>) -> User {
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
        if let Some(permission) = permission {
            let role = roles::create(
                &mut conn,
                &roles::NewRole {
                    name: &format!("{permission} {twitch_id}"),
                    description: None,
                    position: 1,
                    permissions: &[permission],
                    created_by: None,
                },
            )
            .await
            .unwrap();
            roles::assign(&mut conn, signed.account.id, role.id, None)
                .await
                .unwrap();
        }
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

    async fn emote(&self, owner: &User, name: &str, state: EmoteState) -> String {
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
        if state != EmoteState::Pending {
            emotes::set_state(&mut conn, emote.id, state).await.unwrap();
        }
        self.db.public(emote.id).await
    }

    /// A channel set of `owner` holding `emote`, made directly (no events).
    async fn set_with(&self, owner: &User, emote: &str) -> String {
        let mut conn = self.db.conn().await;
        let set = sets::create(&mut conn, owner.id, "Main", SetKind::Channel)
            .await
            .unwrap();
        sets::add_member(
            &mut conn,
            set.id,
            sets::NewMember {
                emote_id: self.db.row(emote).await,
                name: None,
                zero_width: None,
                added_by: Some(owner.id),
            },
        )
        .await
        .unwrap();
        self.db.public(set.id).await
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
        let response = self
            .app
            .clone()
            .oneshot(
                request
                    .body(Body::from(
                        json!({ "query": query, "variables": variables }).to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        serde_json::from_slice(&bytes).unwrap()
    }

    async fn rest(&self, uri: &str) -> (StatusCode, Value) {
        let response = self
            .app
            .clone()
            .oneshot(Request::get(uri).body(Body::empty()).unwrap())
            .await
            .unwrap();
        let status = response.status();
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        (
            status,
            serde_json::from_slice(&bytes).unwrap_or(Value::Null),
        )
    }

    /// Every outbox event as (type, payload).
    async fn events(&self) -> Vec<(String, Value)> {
        let mut conn = self.db.conn().await;
        sqlx::query_as("SELECT type, payload FROM events_outbox ORDER BY id")
            .fetch_all(&mut *conn)
            .await
            .unwrap()
    }

    async fn public_set_names(&self, set: &str) -> Vec<String> {
        let (status, body) = self.rest(&format!("/v1/sets/{set}")).await;
        assert_eq!(status, StatusCode::OK, "{body}");
        body["emotes"]
            .as_array()
            .unwrap()
            .iter()
            .map(|e| e["name"].as_str().unwrap().to_owned())
            .collect()
    }

    async fn search_total(&self, query: &str) -> u64 {
        let r = self.gql(None, SEARCH, json!({ "q": query })).await;
        r["data"]["searchEmotes"]["total"]
            .as_u64()
            .unwrap_or_else(|| panic!("{r}"))
    }
}

#[tokio::test]
async fn renaming_an_approved_emote_sends_it_back_to_review() {
    let Some(w) = World::new().await else { return };
    let owner = w.user("9001", None).await;
    let id = w.emote(&owner, "Reviewed", EmoteState::Approved).await;
    let set = w.set_with(&owner, &id).await;
    assert_eq!(w.search_total("Reviewed").await, 1);
    assert_eq!(w.public_set_names(&set).await, ["Reviewed"]);

    let r = w
        .gql(Some(&owner), UPDATE, json!({ "id": id, "n": "Sneaky" }))
        .await;
    assert_eq!(r["data"]["updateEmote"]["name"], "Sneaky", "{r}");
    assert_eq!(r["data"]["updateEmote"]["state"], "PENDING");

    // Gone from the public until a reviewer looks at it again.
    assert_eq!(w.search_total("Sneaky").await, 0);
    assert_eq!(w.search_total("Reviewed").await, 0);
    assert!(w.public_set_names(&set).await.is_empty());
    assert_eq!(
        w.gql(
            None,
            "query($id: String!) { emote(id: $id) { id } }",
            json!({ "id": id })
        )
        .await["data"]["emote"],
        Value::Null
    );

    // Audited, the old review cleared, and back in the queue.
    let mut conn = w.db.conn().await;
    let entries = audit::list(&mut conn, &audit::Filter::default(), 100)
        .await
        .unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].action, "emote.resubmitted");
    assert_eq!(entries[0].before_json.as_ref().unwrap()["name"], "Reviewed");
    assert_eq!(entries[0].after_json.as_ref().unwrap()["state"], "pending");
    drop(conn);

    // Clients heard that it changed, but never the new name.
    let events = w.events().await;
    assert_eq!(events.len(), 1, "{events:?}");
    assert_eq!(events[0].0, "emote.updated");
    assert!(
        !events.iter().any(|(_, p)| p.to_string().contains("Sneaky")),
        "{events:?}"
    );

    // Retagging does the same once approved again.
    let reviewer = w.user("9002", Some("emotes.review")).await;
    let r = w
        .gql(
            Some(&reviewer),
            "mutation($id: String!) { approveEmote(id: $id) { state } }",
            json!({ "id": id }),
        )
        .await;
    assert_eq!(r["data"]["approveEmote"]["state"], "APPROVED", "{r}");
    assert_eq!(w.public_set_names(&set).await, ["Reviewed"]);
    let r = w
        .gql(Some(&owner), UPDATE, json!({ "id": id, "t": ["slur"] }))
        .await;
    assert_eq!(r["data"]["updateEmote"]["state"], "PENDING", "{r}");

    // Setting the same name or tags again changes nothing that needs review.
    w.gql(
        Some(&reviewer),
        "mutation($id: String!) { approveEmote(id: $id) { state } }",
        json!({ "id": id }),
    )
    .await;
    let r = w
        .gql(
            Some(&owner),
            UPDATE,
            json!({ "id": id, "n": "Sneaky", "t": ["slur"] }),
        )
        .await;
    assert_eq!(r["data"]["updateEmote"]["state"], "APPROVED", "{r}");

    // A reviewer's or emotes.edit_any holder's own edit keeps it approved.
    let editor = w.user("9003", Some("emotes.edit_any")).await;
    let r = w
        .gql(Some(&editor), UPDATE, json!({ "id": id, "n": "Fixed" }))
        .await;
    assert_eq!(r["data"]["updateEmote"]["state"], "APPROVED", "{r}");
    w.db.close().await;
}

#[tokio::test]
async fn toggling_flags_keeps_an_emote_approved() {
    let Some(w) = World::new().await else { return };
    let owner = w.user("9101", None).await;
    let id = w.emote(&owner, "Flaggy", EmoteState::Approved).await;
    let r = w
        .gql(Some(&owner), UPDATE, json!({ "id": id, "p": true }))
        .await;
    assert_eq!(r["data"]["updateEmote"]["flags"]["private"], true, "{r}");
    assert_eq!(r["data"]["updateEmote"]["state"], "APPROVED");
    let r = w
        .gql(Some(&owner), UPDATE, json!({ "id": id, "p": false }))
        .await;
    assert_eq!(r["data"]["updateEmote"]["state"], "APPROVED", "{r}");
    assert_eq!(w.search_total("Flaggy").await, 1);
    w.db.close().await;
}

#[tokio::test]
async fn a_restricted_owner_cannot_edit() {
    let Some(w) = World::new().await else { return };
    let owner = w.user("9201", None).await;
    let id = w.emote(&owner, "Locked", EmoteState::Approved).await;
    let mut conn = w.db.conn().await;
    accounts::set_restricted(&mut conn, owner.id, true)
        .await
        .unwrap();
    drop(conn);
    for input in [
        json!({ "id": id, "n": "Other" }),
        json!({ "id": id, "p": true }),
    ] {
        let r = w.gql(Some(&owner), UPDATE, input).await;
        assert_eq!(code(&r), "restricted", "{r}");
    }
    let r = w
        .gql(
            Some(&owner),
            "query($id: String!) { emote(id: $id) { name state flags { private } } }",
            json!({ "id": id }),
        )
        .await;
    assert_eq!(
        r["data"]["emote"],
        json!({ "name": "Locked", "state": "APPROVED", "flags": { "private": false } })
    );
    w.db.close().await;
}

#[tokio::test]
async fn pending_emotes_in_a_channel_set_publish_no_events() {
    let Some(w) = World::new().await else { return };
    let owner = w.user("9301", None).await;
    let pending = w.emote(&owner, "Unreviewed", EmoteState::Pending).await;
    let r = w
        .gql(
            Some(&owner),
            "mutation { createSet(input: { name: \"Main\", kind: CHANNEL }) { id } }",
            json!({}),
        )
        .await;
    let set = r["data"]["createSet"]["id"]
        .as_str()
        .unwrap_or_else(|| panic!("{r}"))
        .to_owned();
    let before = w.events().await.len();

    let add = "mutation($s: String!, $e: String!, $n: String) { addSetEmote(setId: $s, emoteId: $e, input: { name: $n }) { name } }";
    let r = w
        .gql(
            Some(&owner),
            add,
            json!({ "s": set, "e": pending, "n": "AliasToo" }),
        )
        .await;
    assert_eq!(r["data"]["addSetEmote"]["name"], "AliasToo", "{r}");
    let rename = "mutation($s: String!, $e: String!) { updateSetEmote(setId: $s, emoteId: $e, input: { name: \"Renamed\" }) { name } }";
    let r = w
        .gql(Some(&owner), rename, json!({ "s": set, "e": pending }))
        .await;
    assert_eq!(r["data"]["updateSetEmote"]["name"], "Renamed", "{r}");
    let r = w
        .gql(
            Some(&owner),
            UPDATE,
            json!({ "id": pending, "n": "StillUnreviewed" }),
        )
        .await;
    assert_eq!(r["data"]["updateEmote"]["state"], "PENDING", "{r}");

    let events = w.events().await;
    assert_eq!(events.len(), before, "{events:?}");

    // Approval tells the set's watchers, so they fetch it and learn the name then.
    let reviewer = w.user("9302", Some("emotes.review")).await;
    let r = w
        .gql(
            Some(&reviewer),
            "mutation($id: String!) { approveEmote(id: $id) { state } }",
            json!({ "id": pending }),
        )
        .await;
    assert_eq!(r["data"]["approveEmote"]["state"], "APPROVED", "{r}");
    let events = w.events().await;
    assert_eq!(events.len(), before + 1, "{events:?}");
    assert_eq!(events[before].0, "emote.updated");
    assert_eq!(events[before].1["set_id"], set);
    assert_eq!(w.public_set_names(&set).await, ["Renamed"]);
    w.db.close().await;
}
