//! Moderator emote flags (`setEmoteFlags`, `emotes.flags`) and the review queue's requeue on undo
//! (`requeueEmotes`), ADR 0024. Through the real router against a real Postgres; runs when
//! `CYEX_TV_TEST_DATABASE_URL` is set (CI sets it), skips otherwise.

mod common;

use std::sync::Arc;

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use chrono::Duration;
use common::TestDb;
use cyex_tv_api::state::AppState;
use cyex_tv_auth::{Auth, AuthConfig, cookies, tokens};
use cyex_tv_db::accounts::{self, Profile};
use cyex_tv_db::emotes::{self, NewEmote};
use cyex_tv_db::roles;
use cyex_tv_db::sessions::{self, SessionKind};
use cyex_tv_shared::model::{EmoteState, Platform};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use tower::ServiceExt;
use uuid::Uuid;

/// The site in these tests (the default `CYEX_TV_SITE_URL`).
const SITE: &str = "https://cyex.tv";

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

const FLAGS: &str = "mutation($id: String!, $i: EmoteFlagsInput!) { setEmoteFlags(id: $id, input: $i) { id visibility flags { private nsfw zeroWidth } } }";
const REQUEUE: &str =
    "mutation($ids: [String!]!) { requeueEmotes(ids: $ids) { done failed { id code } } }";
const QUEUE: &str = "query($f: ReviewQueueFilter) { reviewQueue(filter: $f) { items { name } } }";
const APPROVE: &str = "mutation($id: String!) { approveEmote(id: $id) { state } }";
const NOTICES: &str = "query { myNotifications { kind payload } }";

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

    /// A Twitch user named `login` with a website session and, optionally, one permission.
    async fn user(&self, twitch_id: &str, login: &str, permission: Option<&str>) -> User {
        let mut conn = self.db.conn().await;
        let signed = accounts::sign_in(
            &mut conn,
            Profile {
                platform: Platform::Twitch,
                platform_user_id: twitch_id,
                login,
                display_name: login,
                avatar: None,
            },
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

    /// An emote by `owner`, in `state`, uploaded `age_min` minutes ago.
    async fn emote(&self, owner: &User, name: &str, state: EmoteState, age_min: i32) -> String {
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
        sqlx::query(
            "UPDATE emotes SET created_at = now() - make_interval(mins => $2) WHERE id = $1",
        )
        .bind(emote.id)
        .bind(age_min)
        .execute(&mut *conn)
        .await
        .unwrap();
        self.db.public(emote.id).await
    }

    async fn gql(&self, user: &User, query: &str, variables: Value) -> Value {
        let request = Request::post("/v1/gql")
            .header("content-type", "application/json")
            .header("origin", SITE)
            .header("cookie", format!("{}={}", cookies::SESSION, user.token))
            .body(Body::from(
                json!({ "query": query, "variables": variables }).to_string(),
            ))
            .unwrap();
        let response = self.app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        serde_json::from_slice(&bytes).unwrap()
    }

    /// Who wrote each audit row of `action` on the emote, and its `after`.
    async fn audited(&self, action: &str, emote: &str) -> Vec<(Option<Uuid>, Option<Value>)> {
        // The audit log keeps the row's ULID.
        let emote = cyex_tv_db::ids::to_text(self.db.row(emote).await);
        let mut conn = self.db.conn().await;
        sqlx::query_as(
            "SELECT actor_account_id, after_json FROM audit_log
             WHERE action = $1 AND target_kind = 'emote' AND target_id = $2
             ORDER BY created_at, id",
        )
        .bind(action)
        .bind(&emote)
        .fetch_all(&mut *conn)
        .await
        .unwrap()
    }

    /// The pending queue's names, in its default order (oldest upload first).
    async fn queue(&self, user: &User) -> Vec<String> {
        let body = self.gql(user, QUEUE, json!({ "f": null })).await;
        assert!(body["errors"].is_null(), "{body}");
        body["data"]["reviewQueue"]["items"]
            .as_array()
            .unwrap()
            .iter()
            .map(|e| e["name"].as_str().unwrap().to_owned())
            .collect()
    }

    /// The owner's `emote.visibility_changed` notifications' payloads, newest first.
    async fn told(&self, owner: &User) -> Vec<Value> {
        let n = self.gql(owner, NOTICES, json!({})).await;
        assert!(n["errors"].is_null(), "{n}");
        n["data"]["myNotifications"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|x| x["kind"] == "emote.visibility_changed")
            .map(|x| match &x["payload"] {
                Value::String(s) => serde_json::from_str(s).unwrap(),
                other => other.clone(),
            })
            .collect()
    }
}

macro_rules! world {
    () => {
        match World::new().await {
            Some(w) => w,
            None => return,
        }
    };
}

#[tokio::test]
async fn flags_need_the_permission() {
    let w = world!();
    let owner = w.user("1", "owner", None).await;
    let nobody = w.user("2", "nobody", None).await;
    let reviewer = w.user("3", "reviewer", Some("emotes.review")).await;
    let id = w.emote(&owner, "Wave", EmoteState::Approved, 5).await;
    for who in [&nobody, &reviewer, &owner] {
        let r = w
            .gql(
                who,
                FLAGS,
                json!({ "id": id, "i": { "visibility": "UNLISTED", "nsfw": true } }),
            )
            .await;
        assert_eq!(code(&r), "forbidden", "{r}");
    }
    assert!(w.audited("emote.flags_changed", &id).await.is_empty());
    assert!(w.told(&owner).await.is_empty());
    w.db.close().await;
}

#[tokio::test]
async fn moderator_unlists_and_flags_after_review_audited_and_owner_told() {
    let w = world!();
    let owner = w.user("1", "owner", None).await;
    let moderator = w.user("2", "moderator", Some("emotes.flags")).await;
    let admin = w.user("3", "admin", Some("emotes.edit_any")).await;
    let id = w.emote(&owner, "Wave", EmoteState::Approved, 5).await;

    // Off the public listings: unlisted again.
    let r = w
        .gql(
            &moderator,
            FLAGS,
            json!({ "id": id, "i": { "visibility": "UNLISTED" } }),
        )
        .await;
    assert!(r["errors"].is_null(), "{r}");
    assert_eq!(r["data"]["setEmoteFlags"]["visibility"], "UNLISTED");
    let rows = w.audited("emote.flags_changed", &id).await;
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].0, Some(moderator.id));
    let after = rows[0].1.clone().unwrap();
    assert_eq!(after["visibility"], "unlisted");
    assert_eq!(after["via"], "emotes.flags");

    // The owner hears of the visibility change.
    let told = w.told(&owner).await;
    assert_eq!(told.len(), 1);
    assert_eq!(told[0]["visibility"], "unlisted");
    assert_eq!(told[0]["was"], "public");
    assert_eq!(told[0]["emote_id"], id.as_str());

    // NSFW and zero-width (emotes.edit_any covers it too): no notice, the visibility stays.
    let r = w
        .gql(
            &admin,
            FLAGS,
            json!({ "id": id, "i": { "nsfw": true, "zeroWidth": true } }),
        )
        .await;
    assert!(r["errors"].is_null(), "{r}");
    let out = &r["data"]["setEmoteFlags"];
    assert_eq!(out["flags"]["nsfw"], true);
    assert_eq!(out["flags"]["zeroWidth"], true);
    assert_eq!(out["visibility"], "UNLISTED");
    assert_eq!(w.told(&owner).await.len(), 1);

    // Private.
    let r = w
        .gql(
            &moderator,
            FLAGS,
            json!({ "id": id, "i": { "visibility": "PRIVATE" } }),
        )
        .await;
    assert_eq!(r["data"]["setEmoteFlags"]["flags"]["private"], true, "{r}");
    assert_eq!(w.audited("emote.flags_changed", &id).await.len(), 3);
    assert_eq!(w.told(&owner).await.len(), 2);

    // Nothing changed: nothing written.
    let r = w
        .gql(
            &moderator,
            FLAGS,
            json!({ "id": id, "i": { "visibility": "PRIVATE" } }),
        )
        .await;
    assert!(r["errors"].is_null(), "{r}");
    assert_eq!(w.audited("emote.flags_changed", &id).await.len(), 3);
    w.db.close().await;
}

#[tokio::test]
async fn undone_approval_goes_to_the_end_for_everyone() {
    let w = world!();
    let owner = w.user("1", "owner", None).await;
    let reviewer = w.user("2", "reviewer", Some("emotes.review")).await;
    let other = w.user("3", "other", Some("emotes.review")).await;
    let nobody = w.user("4", "nobody", None).await;
    let a = w.emote(&owner, "Alpha", EmoteState::Pending, 30).await;
    w.emote(&owner, "Bravo", EmoteState::Pending, 20).await;
    w.emote(&owner, "Charlie", EmoteState::Pending, 10).await;
    let done = w.emote(&owner, "Delta", EmoteState::Approved, 40).await;
    assert_eq!(w.queue(&reviewer).await, ["Alpha", "Bravo", "Charlie"]);

    // Needs emotes.review.
    let r = w.gql(&nobody, REQUEUE, json!({ "ids": [a] })).await;
    assert_eq!(code(&r), "forbidden", "{r}");

    // The oldest upload's approval was taken back: it goes last, for every reviewer.
    let r = w
        .gql(&reviewer, REQUEUE, json!({ "ids": [a, done, "nope"] }))
        .await;
    assert!(r["errors"].is_null(), "{r}");
    assert_eq!(r["data"]["requeueEmotes"]["done"], json!([a]));
    let failed = r["data"]["requeueEmotes"]["failed"].as_array().unwrap();
    assert_eq!(failed.len(), 2);
    assert_eq!(failed[0]["code"], "not_pending");
    assert_eq!(failed[1]["code"], "not_found");
    assert_eq!(w.queue(&reviewer).await, ["Bravo", "Charlie", "Alpha"]);
    assert_eq!(w.queue(&other).await, ["Bravo", "Charlie", "Alpha"]);
    let rows = w.audited("emote.requeued", &a).await;
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].0, Some(reviewer.id));

    // A verdict clears the mark.
    let r = w.gql(&reviewer, APPROVE, json!({ "id": a })).await;
    assert_eq!(r["data"]["approveEmote"]["state"], "APPROVED", "{r}");
    let mut conn = w.db.conn().await;
    let cleared: Option<chrono::DateTime<chrono::Utc>> =
        sqlx::query_scalar("SELECT requeued_at FROM emotes WHERE id = $1")
            .bind(w.db.row(&a).await)
            .fetch_one(&mut *conn)
            .await
            .unwrap();
    assert!(cleared.is_none());
    drop(conn);
    w.db.close().await;
}
