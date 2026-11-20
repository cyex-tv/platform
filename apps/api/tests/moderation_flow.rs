//! Emote reads, edits, deletion and the review gate, through the real router against a real
//! Postgres. Runs when `CYEX_TV_TEST_DATABASE_URL` is set (CI sets it), skips otherwise.

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
use cyex_tv_db::{audit, ids, roles, sets};
use cyex_tv_shared::model::{Platform, SetKind};
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

    async fn pending(&self, owner: &User, name: &str) -> String {
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
        ids::to_text(emote.id)
    }

    /// A channel set of `owner` holding `emote`.
    async fn set_with(&self, owner: &User, emote: &str) -> String {
        let mut conn = self.db.conn().await;
        let set = sets::create(&mut conn, owner.id, "Main", SetKind::Channel)
            .await
            .unwrap();
        sets::add_member(
            &mut conn,
            set.id,
            sets::NewMember {
                emote_id: ids::from_text(emote).unwrap(),
                name: None,
                zero_width: None,
                added_by: Some(owner.id),
            },
        )
        .await
        .unwrap();
        ids::to_text(set.id)
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

    async fn event_types(&self) -> Vec<String> {
        let mut conn = self.db.conn().await;
        sqlx::query_scalar("SELECT type FROM events_outbox ORDER BY id")
            .fetch_all(&mut *conn)
            .await
            .unwrap()
    }

    async fn audit(&self) -> Vec<audit::Entry> {
        let mut conn = self.db.conn().await;
        let mut out = audit::list(&mut conn, &audit::Filter::default(), 100)
            .await
            .unwrap();
        out.reverse();
        out
    }
}

const EMOTE: &str = "query($id: String!) { emote(id: $id) { id name state tags } }";

#[tokio::test]
async fn pending_emotes_are_seen_by_their_owner_and_reviewers_only() {
    let Some(w) = World::new().await else { return };
    let owner = w.user("1", None).await;
    let stranger = w.user("2", None).await;
    let reviewer = w.user("3", Some("emotes.review")).await;
    let id = w.pending(&owner, "waiting").await;

    for who in [None, Some(&stranger)] {
        assert_eq!(
            w.gql(who, EMOTE, json!({ "id": id })).await["data"]["emote"],
            Value::Null
        );
    }
    for who in [&owner, &reviewer] {
        let r = w.gql(Some(who), EMOTE, json!({ "id": id })).await;
        assert_eq!(r["data"]["emote"]["state"], "PENDING", "{r}");
    }
    let (status, body) = w.rest(&format!("/v1/emotes/{id}")).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["error"], "not_found");
    w.db.close().await;
}

#[tokio::test]
async fn the_review_queue_and_approval() {
    let Some(w) = World::new().await else { return };
    let owner = w.user("10", None).await;
    let reviewer = w.user("11", Some("emotes.review")).await;
    let first = w.pending(&owner, "first").await;
    let second = w.pending(&owner, "second").await;
    let third = w.pending(&owner, "third").await;
    let set = w.set_with(&owner, &first).await;

    let queue =
        "query($p: Int, $n: Int) { reviewQueue(page: $p, perPage: $n) { total items { name } } }";
    assert_eq!(
        code(&w.gql(Some(&owner), queue, json!({})).await),
        "forbidden"
    );
    let r = w.gql(Some(&reviewer), queue, json!({ "n": 2 })).await;
    assert_eq!(r["data"]["reviewQueue"]["total"], 3);
    assert_eq!(
        r["data"]["reviewQueue"]["items"],
        json!([{ "name": "first" }, { "name": "second" }])
    );
    let r = w
        .gql(Some(&reviewer), queue, json!({ "n": 2, "p": 2 }))
        .await;
    assert_eq!(
        r["data"]["reviewQueue"]["items"],
        json!([{ "name": "third" }])
    );

    let approve = "mutation($id: String!) { approveEmote(id: $id) { state } }";
    let reject =
        "mutation($id: String!, $r: String!) { rejectEmote(id: $id, reason: $r) { state } }";
    assert_eq!(
        code(&w.gql(Some(&owner), approve, json!({ "id": first })).await),
        "forbidden"
    );
    assert_eq!(
        w.gql(Some(&reviewer), approve, json!({ "id": first }))
            .await["data"]["approveEmote"]["state"],
        "APPROVED"
    );
    assert_eq!(
        code(
            &w.gql(Some(&reviewer), approve, json!({ "id": first }))
                .await
        ),
        "not_pending"
    );
    assert_eq!(
        code(
            &w.gql(Some(&reviewer), reject, json!({ "id": second, "r": "  " }))
                .await
        ),
        "bad_reason"
    );
    assert_eq!(
        w.gql(
            Some(&reviewer),
            reject,
            json!({ "id": second, "r": "Blurry" })
        )
        .await["data"]["rejectEmote"]["state"],
        "REJECTED"
    );
    assert_eq!(
        code(
            &w.gql(Some(&reviewer), approve, json!({ "id": second }))
                .await
        ),
        "not_pending"
    );
    let _ = third;

    // Approved: public now, and the set that held it heard about it.
    let (status, body) = w.rest(&format!("/v1/emotes/{first}")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["name"], "first");
    let (_, set_body) = w.rest(&format!("/v1/sets/{set}")).await;
    assert_eq!(set_body["emotes"][0]["name"], "first");
    assert_eq!(w.event_types().await, ["emote.updated"]);

    // Both reviews are audited, with the reason.
    let entries = w.audit().await;
    let actions: Vec<&str> = entries.iter().map(|e| e.action.as_str()).collect();
    assert_eq!(actions, ["emote.approved", "emote.rejected"]);
    assert_eq!(entries[1].after_json.as_ref().unwrap()["reason"], "Blurry");
    assert_eq!(entries[1].actor_account_id, Some(reviewer.id));
    let mut conn = w.db.conn().await;
    let stored: Option<String> =
        sqlx::query_scalar("SELECT review_reason FROM emotes WHERE id = $1")
            .bind(ids::from_text(&second).unwrap())
            .fetch_one(&mut *conn)
            .await
            .unwrap();
    assert_eq!(stored.as_deref(), Some("Blurry"));
    drop(conn);
    w.db.close().await;
}

#[tokio::test]
async fn owners_edit_and_delete_their_emotes() {
    let Some(w) = World::new().await else { return };
    let owner = w.user("20", None).await;
    let stranger = w.user("21", None).await;
    let editor = w.user("22", Some("emotes.edit_any")).await;
    let id = w.pending(&owner, "mine").await;
    let set = w.set_with(&owner, &id).await;

    let update = "mutation($id: String!, $n: String, $t: [String!], $nsfw: Boolean) { updateEmote(id: $id, input: { name: $n, tags: $t, nsfw: $nsfw }) { name tags flags { nsfw } } }";
    let r = w
        .gql(
            Some(&owner),
            update,
            json!({ "id": id, "n": "Mine2", "t": ["Pog", "pog", "funny"] }),
        )
        .await;
    assert_eq!(r["data"]["updateEmote"]["name"], "Mine2", "{r}");
    assert_eq!(r["data"]["updateEmote"]["tags"], json!(["funny", "pog"]));
    assert_eq!(
        code(
            &w.gql(
                Some(&owner),
                update,
                json!({ "id": id, "t": ["two words"] })
            )
            .await
        ),
        "bad_tag"
    );
    assert_eq!(
        code(
            &w.gql(Some(&owner), update, json!({ "id": id, "n": "has space" }))
                .await
        ),
        "bad_name"
    );
    assert_eq!(
        code(
            &w.gql(Some(&stranger), update, json!({ "id": id, "n": "x" }))
                .await
        ),
        "forbidden"
    );

    // A moderator's edit goes in the audit log; the owner's doesn't.
    let r = w
        .gql(Some(&editor), update, json!({ "id": id, "nsfw": true }))
        .await;
    assert_eq!(r["data"]["updateEmote"]["flags"]["nsfw"], true, "{r}");
    let entries = w.audit().await;
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].action, "emote.updated");
    assert_eq!(entries[0].before_json.as_ref().unwrap()["nsfw"], false);
    assert_eq!(
        entries[0].after_json.as_ref().unwrap()["tags"],
        json!(["funny", "pog"])
    );

    // Deleting: only the owner; it leaves every set.
    let delete = "mutation($id: String!) { deleteEmote(id: $id) }";
    assert_eq!(
        code(&w.gql(Some(&stranger), delete, json!({ "id": id })).await),
        "forbidden"
    );
    assert_eq!(
        w.gql(Some(&owner), delete, json!({ "id": id })).await["data"]["deleteEmote"],
        true
    );
    assert_eq!(
        w.gql(Some(&owner), delete, json!({ "id": id })).await["data"]["deleteEmote"],
        false
    );
    assert_eq!(
        w.gql(Some(&owner), EMOTE, json!({ "id": id })).await["data"]["emote"],
        Value::Null
    );
    let r = w
        .gql(
            Some(&owner),
            "query($id: String!) { set(id: $id) { emotes { name } } }",
            json!({ "id": set }),
        )
        .await;
    assert_eq!(r["data"]["set"]["emotes"], json!([]));
    // It was never approved, so no public event named it, not even on the way out.
    assert!(w.event_types().await.is_empty());
    w.db.close().await;
}
