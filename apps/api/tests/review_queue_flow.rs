//! The review queue's tabs, search and order, approving as unlisted or private, reviewers' notes
//! on an emote and an emote's history. Through the real router against a real Postgres; runs
//! when `CYEX_TV_TEST_DATABASE_URL` is set (CI sets it), skips otherwise.

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

fn code(response: &Value) -> &str {
    response["errors"][0]["extensions"]["code"]
        .as_str()
        .unwrap_or_default()
}

const QUEUE: &str = "query($f: ReviewQueueFilter, $p: Int, $n: Int) { reviewQueue(filter: $f, page: $p, perPage: $n) { total counts { public limited } items { id name state visibility owner { displayName } } } }";
const APPROVE: &str = "mutation($id: String!, $v: EmoteVisibility) { approveEmote(id: $id, visibility: $v) { state visibility flags { private } } }";
const REJECT: &str =
    "mutation($id: String!) { rejectEmote(id: $id, reason: \"Too blurry\") { state } }";
const NOTES: &str = "query($id: String!) { emoteNotes(emoteId: $id) { id emoteId authorId body } }";
const ADD_NOTE: &str = "mutation($id: String!, $b: String!) { addEmoteNote(emoteId: $id, body: $b) { id body authorId } }";
const DELETE_NOTE: &str = "mutation($id: String!) { deleteEmoteNote(id: $id) }";
const HISTORY: &str =
    "query($id: String!) { emoteHistory(emoteId: $id) { action actorId targetId after } }";

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

    /// A Twitch user named `login` with a site session and, optionally, one permission.
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

    /// A pending emote, uploaded `age_min` minutes ago.
    async fn emote(
        &self,
        owner: &User,
        name: &str,
        unlisted: bool,
        private: bool,
        age_min: i64,
    ) -> String {
        let mut conn = self.db.conn().await;
        let emote = emotes::create(
            &mut conn,
            &NewEmote {
                owner_account_id: owner.id,
                name,
                unlisted,
                private,
                ..NewEmote::default()
            },
        )
        .await
        .unwrap();
        sqlx::query(
            "UPDATE emotes SET created_at = now() - make_interval(mins => $2) WHERE id = $1",
        )
        .bind(emote.id)
        .bind(i32::try_from(age_min).unwrap())
        .execute(&mut *conn)
        .await
        .unwrap();
        self.db.public(emote.id).await
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

    async fn names(&self, user: &User, filter: Value) -> (Vec<String>, u64) {
        let body = self.gql(Some(user), QUEUE, json!({ "f": filter })).await;
        assert!(body["errors"].is_null(), "{body}");
        let q = &body["data"]["reviewQueue"];
        let names = q["items"]
            .as_array()
            .unwrap()
            .iter()
            .map(|e| e["name"].as_str().unwrap().to_owned())
            .collect();
        (names, q["total"].as_u64().unwrap())
    }
}

#[tokio::test]
async fn tabs_search_and_order() {
    let Some(w) = World::new().await else {
        return;
    };
    let mod_ = w.user("101", "reviewer", Some("emotes.review")).await;
    let alice = w.user("102", "alice", None).await;
    let bob = w.user("103", "Bobby", None).await;
    w.emote(&alice, "OldPublic", false, false, 30).await;
    w.emote(&bob, "NewPublic", false, false, 10).await;
    let unlisted = w.emote(&alice, "LinkOnly", true, false, 20).await;
    w.emote(&bob, "MineOnly", false, true, 5).await;

    // Without a filter: every pending one, oldest first, as before.
    let (all, total) = w.names(&mod_, Value::Null).await;
    assert_eq!(all, ["OldPublic", "LinkOnly", "NewPublic", "MineOnly"]);
    assert_eq!(total, 4);

    let body = w.gql(Some(&mod_), QUEUE, json!({})).await;
    assert_eq!(
        body["data"]["reviewQueue"]["counts"],
        json!({ "public": 2, "limited": 2 })
    );

    let (public, total) = w.names(&mod_, json!({ "tab": "PUBLIC" })).await;
    assert_eq!(
        (public, total),
        (vec!["OldPublic".to_owned(), "NewPublic".to_owned()], 2)
    );
    let (limited, _) = w
        .names(&mod_, json!({ "tab": "LIMITED", "descending": true }))
        .await;
    assert_eq!(limited, ["MineOnly", "LinkOnly"]);

    // Search: by name, by uploader login, any case; LIKE characters mean themselves.
    let (by_name, _) = w.names(&mod_, json!({ "search": "only" })).await;
    assert_eq!(by_name, ["LinkOnly", "MineOnly"]);
    let (by_uploader, total) = w.names(&mod_, json!({ "search": "BOBBY" })).await;
    assert_eq!(
        (by_uploader, total),
        (vec!["NewPublic".to_owned(), "MineOnly".to_owned()], 2)
    );
    let (none, total) = w.names(&mod_, json!({ "search": "%" })).await;
    assert!(none.is_empty() && total == 0);

    // Sorts: uploader (ties by upload time), name.
    let (by_who, _) = w.names(&mod_, json!({ "sort": "UPLOADER" })).await;
    assert_eq!(by_who, ["OldPublic", "LinkOnly", "NewPublic", "MineOnly"]);
    let (by_name_desc, _) = w
        .names(&mod_, json!({ "sort": "NAME", "descending": true }))
        .await;
    assert_eq!(
        by_name_desc,
        ["OldPublic", "NewPublic", "MineOnly", "LinkOnly"]
    );

    // Paging keeps the filter's total.
    let body = w
        .gql(
            Some(&mod_),
            QUEUE,
            json!({ "f": { "tab": "LIMITED" }, "p": 2, "n": 1 }),
        )
        .await;
    assert_eq!(body["data"]["reviewQueue"]["total"], 2);
    assert_eq!(body["data"]["reviewQueue"]["items"][0]["name"], "MineOnly");

    // Resolved: reviewed ones, the latest review first.
    let r = w.gql(Some(&mod_), REJECT, json!({ "id": unlisted })).await;
    assert_eq!(r["data"]["rejectEmote"]["state"], "REJECTED");
    let (resolved, total) = w.names(&mod_, json!({ "tab": "RESOLVED" })).await;
    assert_eq!((resolved, total), (vec!["LinkOnly".to_owned()], 1));
    let body = w.gql(Some(&mod_), QUEUE, json!({})).await;
    assert_eq!(
        body["data"]["reviewQueue"]["counts"],
        json!({ "public": 2, "limited": 1 })
    );

    // Bad input and no permission.
    let long = "x".repeat(101);
    let body = w
        .gql(Some(&mod_), QUEUE, json!({ "f": { "search": long } }))
        .await;
    assert_eq!(code(&body), "bad_search");
    let body = w
        .gql(Some(&alice), QUEUE, json!({ "f": { "tab": "RESOLVED" } }))
        .await;
    assert_eq!(code(&body), "forbidden");
    w.db.close().await;
}

#[tokio::test]
async fn approve_as_unlisted_or_private() {
    let Some(w) = World::new().await else {
        return;
    };
    let mod_ = w.user("201", "reviewer", Some("emotes.review")).await;
    let alice = w.user("202", "alice", None).await;
    let a = w.emote(&alice, "ForLinks", false, false, 3).await;
    let b = w.emote(&alice, "ForMe", false, false, 2).await;
    let c = w.emote(&alice, "AsIs", true, false, 1).await;

    let r = w
        .gql(Some(&mod_), APPROVE, json!({ "id": a, "v": "UNLISTED" }))
        .await;
    assert_eq!(
        r["data"]["approveEmote"],
        json!({ "state": "APPROVED", "visibility": "UNLISTED", "flags": { "private": false } })
    );
    let r = w
        .gql(Some(&mod_), APPROVE, json!({ "id": b, "v": "PRIVATE" }))
        .await;
    assert_eq!(r["data"]["approveEmote"]["visibility"], "PRIVATE");
    assert_eq!(r["data"]["approveEmote"]["flags"]["private"], true);
    // No visibility: it stays as the uploader chose.
    let r = w.gql(Some(&mod_), APPROVE, json!({ "id": c })).await;
    assert_eq!(r["data"]["approveEmote"]["visibility"], "UNLISTED");

    // The audit entry says what it became; the history shows it to reviewers.
    let h = w.gql(Some(&mod_), HISTORY, json!({ "id": a })).await;
    let entries = h["data"]["emoteHistory"].as_array().unwrap();
    let approved = entries
        .iter()
        .find(|e| e["action"] == "emote.approved")
        .expect("the approval is in the history");
    let after: Value = serde_json::from_str(approved["after"].as_str().unwrap()).unwrap();
    assert_eq!(after["visibility"], "unlisted");
    assert_eq!(approved["actorId"], w.db.public(mod_.id).await);

    // Approving twice is refused, and the uploader can't approve.
    let r = w
        .gql(Some(&mod_), APPROVE, json!({ "id": a, "v": "PUBLIC" }))
        .await;
    assert_eq!(code(&r), "not_pending");
    let d = w.emote(&alice, "Mine", false, false, 0).await;
    let r = w
        .gql(Some(&alice), APPROVE, json!({ "id": d, "v": "PUBLIC" }))
        .await;
    assert_eq!(code(&r), "forbidden");
    let h = w.gql(Some(&alice), HISTORY, json!({ "id": d })).await;
    assert_eq!(code(&h), "forbidden");
    w.db.close().await;
}

#[tokio::test]
async fn emote_notes() {
    let Some(w) = World::new().await else {
        return;
    };
    let one = w.user("301", "reviewer1", Some("emotes.review")).await;
    let two = w.user("302", "reviewer2", Some("emotes.review")).await;
    let alice = w.user("303", "alice", None).await;
    let e = w.emote(&alice, "Noted", false, false, 1).await;

    let r = w
        .gql(
            Some(&one),
            ADD_NOTE,
            json!({ "id": e, "b": "  Looks like a 7TV copy.\nCheck.  " }),
        )
        .await;
    assert!(r["errors"].is_null(), "{r}");
    let note = r["data"]["addEmoteNote"].clone();
    assert_eq!(note["body"], "Looks like a 7TV copy.\nCheck.");
    assert_eq!(note["authorId"], w.db.public(one.id).await);
    w.gql(Some(&two), ADD_NOTE, json!({ "id": e, "b": "Agreed." }))
        .await;

    let list = w.gql(Some(&two), NOTES, json!({ "id": e })).await;
    let bodies: Vec<&str> = list["data"]["emoteNotes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|n| n["body"].as_str().unwrap())
        .collect();
    assert_eq!(bodies, ["Agreed.", "Looks like a 7TV copy.\nCheck."]);

    // Bad bodies, an unknown emote, the uploader.
    let r = w
        .gql(Some(&one), ADD_NOTE, json!({ "id": e, "b": "   " }))
        .await;
    assert_eq!(code(&r), "bad_note");
    let r = w
        .gql(
            Some(&one),
            ADD_NOTE,
            json!({ "id": e, "b": "x".repeat(2001) }),
        )
        .await;
    assert_eq!(code(&r), "bad_note");
    let unknown = ids::to_text(ids::new_id());
    let r = w
        .gql(Some(&one), ADD_NOTE, json!({ "id": unknown, "b": "Hi" }))
        .await;
    assert_eq!(code(&r), "not_found");
    let r = w.gql(Some(&alice), NOTES, json!({ "id": e })).await;
    assert_eq!(code(&r), "forbidden");
    let r = w
        .gql(Some(&alice), ADD_NOTE, json!({ "id": e, "b": "Mine" }))
        .await;
    assert_eq!(code(&r), "forbidden");

    // Only the author deletes; a second delete is false; both are in the history.
    let id = note["id"].as_str().unwrap();
    let r = w.gql(Some(&two), DELETE_NOTE, json!({ "id": id })).await;
    assert_eq!(code(&r), "forbidden");
    let r = w.gql(Some(&one), DELETE_NOTE, json!({ "id": id })).await;
    assert_eq!(r["data"]["deleteEmoteNote"], true);
    let r = w.gql(Some(&one), DELETE_NOTE, json!({ "id": id })).await;
    assert_eq!(r["data"]["deleteEmoteNote"], false);
    let list = w.gql(Some(&one), NOTES, json!({ "id": e })).await;
    assert_eq!(list["data"]["emoteNotes"].as_array().unwrap().len(), 1);

    let h = w.gql(Some(&one), HISTORY, json!({ "id": e })).await;
    let actions: Vec<&str> = h["data"]["emoteHistory"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["action"].as_str().unwrap())
        .collect();
    assert_eq!(
        actions,
        ["emote.note_deleted", "emote.note_added", "emote.note_added"]
    );
    w.db.close().await;
}
