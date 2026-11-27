//! Two approvals per emote (ADR 0039): uploads land unlisted with both approvals pending, owners
//! can't choose listed or unlisted, `reviewApprovals` (Approve both, each part, only-pending),
//! the personal queue, personal sets taking approved-for-personal-use emotes only, the
//! `PERSONAL_USE` search flag, staff-only transfers and the retired request flow. Through the real
//! router against a real Postgres; runs when `CYEX_TV_TEST_DATABASE_URL` is set (CI sets it),
//! skips otherwise.

mod common;

use std::io::Cursor;
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
use cyex_tv_storage::Store;
use http_body_util::BodyExt;
use image::{ImageFormat, Rgba, RgbaImage};
use serde_json::{Value, json};
use tower::ServiceExt;
use uuid::Uuid;

const SITE: &str = "https://cyex.tv";
const BOUNDARY: &str = "cyextvboundaryApprovals42";

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

fn png(seed: u8) -> Vec<u8> {
    let img = RgbaImage::from_fn(64, 64, |x, y| {
        Rgba([
            (x as u8).wrapping_mul(3) ^ seed,
            (y as u8).wrapping_mul(5),
            seed,
            255,
        ])
    });
    let mut out = Cursor::new(Vec::new());
    img.write_to(&mut out, ImageFormat::Png).unwrap();
    out.into_inner()
}

fn form(fields: &[(&str, &str)], file: &[u8]) -> Vec<u8> {
    let mut body = Vec::new();
    for (name, value) in fields {
        body.extend_from_slice(
            format!(
                "--{BOUNDARY}\r\nContent-Disposition: form-data; name=\"{name}\"\r\n\r\n{value}\r\n"
            )
            .as_bytes(),
        );
    }
    body.extend_from_slice(
        format!(
            "--{BOUNDARY}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"e.png\"\r\nContent-Type: application/octet-stream\r\n\r\n"
        )
        .as_bytes(),
    );
    body.extend_from_slice(file);
    body.extend_from_slice(format!("\r\n--{BOUNDARY}--\r\n").as_bytes());
    body
}

const EMOTE: &str = "query($id: String!) { emote(id: $id) { id state visibility flags { private zeroWidth } approvals { publicListing personalUse } } }";
const REVIEW: &str = "mutation($ids: [String!]!, $i: ApprovalsInput!) { reviewApprovals(ids: $ids, input: $i) { done failed { id code } } }";
const QUEUE: &str = "query($f: ReviewQueueFilter) { reviewQueue(filter: $f) { total items { id } counts { public limited personal } } }";
const UPDATE: &str = "mutation($id: String!, $i: UpdateEmoteInput!) { updateEmote(id: $id, input: $i) { visibility flags { private } approvals { publicListing } } }";
const CREATE_SET: &str =
    "mutation($n: String!, $k: SetKind!) { createSet(input: { name: $n, kind: $k }) { id } }";
const ADD: &str =
    "mutation($s: String!, $e: String!) { addSetEmote(setId: $s, emoteId: $e) { name } }";
const SEARCH: &str =
    "query($q: String!, $f: [EmoteFlag!]) { searchEmotes(query: $q, flags: $f) { items { id } } }";
const TRANSFER: &str = "mutation($id: String!, $to: String!) { transferEmote(id: $id, toAccountId: $to) { owner { id } } }";
const ACCEPT: &str = "mutation($id: String!) { acceptEmoteTransfer(id: $id) { id } }";
const DECLINE: &str = "mutation($id: String!) { declineEmoteTransfer(id: $id) { id } }";
const REQUESTS: &str = "query { emoteTransferRequests { id } }";
const NOTICES: &str = "query { myNotifications { kind payload } }";

impl World {
    async fn new() -> Option<Self> {
        let db = TestDb::new().await?;
        let auth = Arc::new(Auth::new(AuthConfig::from_source(|_| None).unwrap()).unwrap());
        let state = AppState::new(db.pool.clone(), auth)
            .with_cdn_url("https://cdn.example.test")
            .with_store(Store::memory());
        Some(Self {
            app: cyex_tv_api::app(state),
            db,
        })
    }

    /// A Twitch user named `login` with a website session and the given permissions (one role).
    async fn user(&self, twitch_id: &str, login: &str, perms: &[&str]) -> User {
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
        if !perms.is_empty() {
            let role = roles::create(
                &mut conn,
                &roles::NewRole {
                    name: &format!("staff {twitch_id}"),
                    description: None,
                    position: 1,
                    permissions: perms,
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

    /// A pending emote of `owner` made in the database (as an upload leaves it).
    async fn pending(&self, owner: &User, name: &str) -> String {
        let mut conn = self.db.conn().await;
        let e = emotes::create(
            &mut conn,
            &NewEmote {
                owner_account_id: owner.id,
                name,
                ..NewEmote::default()
            },
        )
        .await
        .unwrap();
        self.db.public(e.id).await
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

    async fn ok(&self, user: &User, query: &str, variables: Value) -> Value {
        let body = self.gql(user, query, variables).await;
        assert!(body["errors"].is_null(), "{body}");
        body["data"].clone()
    }

    async fn emote(&self, user: &User, id: &str) -> Value {
        self.ok(user, EMOTE, json!({ "id": id })).await["emote"].clone()
    }

    async fn upload(&self, user: &User, fields: &[(&str, &str)], seed: u8) -> (StatusCode, Value) {
        let request = Request::post("/v1/emotes")
            .header(
                "content-type",
                format!("multipart/form-data; boundary={BOUNDARY}"),
            )
            .header("cookie", format!("{}={}", cookies::SESSION, user.token))
            .header("origin", SITE)
            .body(Body::from(form(fields, &png(seed))))
            .unwrap();
        let response = self.app.clone().oneshot(request).await.unwrap();
        let status = response.status();
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        (
            status,
            serde_json::from_slice(&bytes).unwrap_or(Value::Null),
        )
    }

    async fn get(&self, path: &str) -> Value {
        let response = self
            .app
            .clone()
            .oneshot(Request::get(path).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK, "{path}");
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        serde_json::from_slice(&bytes).unwrap()
    }

    async fn audited(&self, action: &str, emote: &str) -> Vec<Value> {
        let emote = cyex_tv_db::ids::to_text(self.db.row(emote).await);
        let mut conn = self.db.conn().await;
        let rows: Vec<(Option<Value>,)> = sqlx::query_as(
            "SELECT after_json FROM audit_log
             WHERE action = $1 AND target_kind = 'emote' AND target_id = $2
             ORDER BY created_at, id",
        )
        .bind(action)
        .bind(&emote)
        .fetch_all(&mut *conn)
        .await
        .unwrap();
        rows.into_iter()
            .map(|(a,)| a.unwrap_or(Value::Null))
            .collect()
    }

    async fn notices(&self, user: &User, kind: &str) -> Vec<Value> {
        self.ok(user, NOTICES, json!({})).await["myNotifications"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|n| n["kind"] == kind)
            .map(|n| serde_json::from_str(n["payload"].as_str().unwrap()).unwrap())
            .collect()
    }

    /// The account's personal set (every account has one, ADR 0038).
    async fn personal_set(&self, user: &User) -> String {
        let mut conn = self.db.conn().await;
        let set = cyex_tv_db::sets::ensure_personal(&mut conn, user.id)
            .await
            .unwrap();
        drop(conn);
        self.db.public(set.id).await
    }
}

const BOTH: &[&str] = &["emotes.review", "emotes.review_personal"];

#[tokio::test]
async fn an_upload_lands_unlisted_with_both_approvals_pending_and_in_the_queue() {
    let Some(w) = World::new().await else {
        return;
    };
    let alice = w.user("1", "alice", &[]).await;
    let mod_ = w.user("2", "mod", BOTH).await;

    // The form's old `visibility=public` still means "not private": the emote is unlisted.
    let (status, body) = w
        .upload(
            &alice,
            &[("name", "aliceWave"), ("visibility", "public")],
            1,
        )
        .await;
    assert_eq!(status, StatusCode::ACCEPTED, "{body}");
    assert_eq!(body["state"], "pending");
    assert_eq!(body["visibility"], "unlisted");
    assert_eq!(
        body["approvals"],
        json!({ "public_listing": "pending", "personal_use": "pending" })
    );
    let id = body["id"].as_str().unwrap().to_owned();

    // A private upload stays private (never both private and unlisted).
    let (status, body) = w
        .upload(&alice, &[("name", "aliceHush"), ("private", "true")], 2)
        .await;
    assert_eq!(status, StatusCode::ACCEPTED, "{body}");
    assert_eq!(body["visibility"], "private");

    // Both wait in the main queue at once: one not private, one private.
    let q = w.ok(&mod_, QUEUE, json!({ "f": null })).await["reviewQueue"].clone();
    assert_eq!(q["total"], 2);
    assert_eq!(
        q["counts"],
        json!({ "public": 1, "limited": 1, "personal": 0 })
    );
    assert!(q["items"].as_array().unwrap().iter().any(|e| e["id"] == id));

    // Not on the listings before review.
    let found = w
        .ok(&alice, SEARCH, json!({ "q": "aliceWave", "f": null }))
        .await;
    assert_eq!(found["searchEmotes"]["items"], json!([]));
    w.db.close().await;
}

#[tokio::test]
async fn owners_cant_choose_unlisted_or_listed() {
    let Some(w) = World::new().await else {
        return;
    };
    let alice = w.user("1", "alice", &[]).await;
    let mod_ = w.user("2", "mod", BOTH).await;

    let (status, body) = w
        .upload(
            &alice,
            &[("name", "aliceShy"), ("visibility", "unlisted")],
            3,
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    assert_eq!(body["error"], "bad_visibility");

    let id = w.pending(&alice, "aliceOk").await;
    let r = w
        .gql(
            &alice,
            UPDATE,
            json!({ "id": id, "i": { "visibility": "UNLISTED" } }),
        )
        .await;
    assert_eq!(code(&r), "bad_visibility", "{r}");

    // Asking for PUBLIC (or private: false) only means "not private": still unlisted until review.
    let r = w
        .ok(
            &alice,
            UPDATE,
            json!({ "id": id, "i": { "visibility": "PUBLIC" } }),
        )
        .await;
    assert_eq!(r["updateEmote"]["visibility"], "UNLISTED");
    assert_eq!(r["updateEmote"]["approvals"]["publicListing"], "PENDING");

    // Once the listing is approved, private and back keeps the approval: listed again.
    w.ok(
        &mod_,
        REVIEW,
        json!({ "ids": [id], "i": { "publicListing": "APPROVE" } }),
    )
    .await;
    let r = w
        .ok(
            &alice,
            UPDATE,
            json!({ "id": id, "i": { "private": true } }),
        )
        .await;
    assert_eq!(r["updateEmote"]["visibility"], "PRIVATE");
    let r = w
        .ok(
            &alice,
            UPDATE,
            json!({ "id": id, "i": { "private": false } }),
        )
        .await;
    assert_eq!(r["updateEmote"]["visibility"], "PUBLIC");

    // A denied listing can't be undone by the owner.
    let other = w.pending(&alice, "aliceNope").await;
    w.ok(
        &mod_,
        REVIEW,
        json!({ "ids": [other], "i": { "publicListing": "DENY", "reason": "Too blurry for listings" } }),
    )
    .await;
    let r = w
        .ok(
            &alice,
            UPDATE,
            json!({ "id": other, "i": { "visibility": "PUBLIC" } }),
        )
        .await;
    assert_eq!(r["updateEmote"]["visibility"], "UNLISTED");
    assert_eq!(r["updateEmote"]["approvals"]["publicListing"], "DENIED");
    w.db.close().await;
}

#[tokio::test]
async fn approve_both_lists_it_and_allows_personal_use_once() {
    let Some(w) = World::new().await else {
        return;
    };
    let alice = w.user("1", "alice", &[]).await;
    let mod_ = w.user("2", "mod", BOTH).await;
    let late = w.user("3", "late", BOTH).await;
    let id = w.pending(&alice, "aliceBoth").await;

    // A denial needs a reason; nothing asked is refused too.
    let r = w
        .gql(
            &mod_,
            REVIEW,
            json!({ "ids": [id], "i": { "publicListing": "DENY" } }),
        )
        .await;
    assert_eq!(code(&r), "bad_approvals", "{r}");
    let r = w.gql(&mod_, REVIEW, json!({ "ids": [id], "i": {} })).await;
    assert_eq!(code(&r), "bad_approvals", "{r}");

    let r = w
        .ok(
            &mod_,
            REVIEW,
            json!({ "ids": [id], "i": { "publicListing": "APPROVE", "personalUse": "APPROVE" } }),
        )
        .await;
    assert_eq!(r["reviewApprovals"]["done"], json!([id]));
    let e = w.emote(&alice, &id).await;
    assert_eq!(e["state"], "APPROVED");
    assert_eq!(e["visibility"], "PUBLIC");
    assert_eq!(
        e["approvals"],
        json!({ "publicListing": "APPROVED", "personalUse": "APPROVED" })
    );

    // A second reviewer with the stale card can't overturn it (only pending by default).
    let r = w
        .ok(
            &late,
            REVIEW,
            json!({ "ids": [id], "i": { "publicListing": "DENY", "personalUse": "DENY", "reason": "no" } }),
        )
        .await;
    assert_eq!(r["reviewApprovals"]["done"], json!([]));
    assert_eq!(r["reviewApprovals"]["failed"][0]["code"], "not_pending");
    assert_eq!(w.emote(&alice, &id).await["visibility"], "PUBLIC");

    // One audit row (the approval), one notice to the owner.
    let rows = w.audited("emote.approved", &id).await;
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["public_listing"], "approved");
    assert_eq!(rows[0]["personal_use"], "approved");
    let notes = w.notices(&alice, "emote.approved").await;
    assert_eq!(notes.len(), 1);
    assert_eq!(notes[0]["personal_use"], "approved");

    // Listed now, and found by the personal-use flag.
    let found = w
        .ok(
            &alice,
            SEARCH,
            json!({ "q": "aliceBoth", "f": ["PERSONAL_USE"] }),
        )
        .await;
    assert_eq!(found["searchEmotes"]["items"][0]["id"], id);

    // The edit dialog changes a decision on purpose (onlyPending: false), audited and told.
    w.ok(
        &mod_,
        REVIEW,
        json!({ "ids": [id], "i": { "personalUse": "DENY", "reason": "Too loud", "onlyPending": false } }),
    )
    .await;
    assert_eq!(
        w.emote(&alice, &id).await["approvals"]["personalUse"],
        "DENIED"
    );
    assert_eq!(w.audited("emote.approvals_changed", &id).await.len(), 1);
    let changed = w.notices(&alice, "emote.approvals_changed").await;
    assert_eq!(changed[0]["reason"], "Too loud");
    let found = w
        .ok(
            &alice,
            SEARCH,
            json!({ "q": "aliceBoth", "f": ["PERSONAL_USE"] }),
        )
        .await;
    assert_eq!(found["searchEmotes"]["items"], json!([]));
    w.db.close().await;
}

#[tokio::test]
async fn the_personal_queue_and_its_permission() {
    let Some(w) = World::new().await else {
        return;
    };
    let alice = w.user("1", "alice", &[]).await;
    let lister = w.user("2", "lister", &["emotes.review"]).await;
    let helper = w.user("3", "helper", &["emotes.review_personal"]).await;
    let id = w.pending(&alice, "aliceSolo").await;

    // Personal use needs its own permission; the listing needs emotes.review.
    let r = w
        .gql(
            &lister,
            REVIEW,
            json!({ "ids": [id], "i": { "publicListing": "APPROVE", "personalUse": "APPROVE" } }),
        )
        .await;
    assert_eq!(code(&r), "forbidden", "{r}");
    let r = w
        .gql(
            &helper,
            REVIEW,
            json!({ "ids": [id], "i": { "publicListing": "APPROVE" } }),
        )
        .await;
    assert_eq!(code(&r), "forbidden", "{r}");
    // The main queue is not the helper's; the personal one is.
    let r = w.gql(&helper, QUEUE, json!({ "f": null })).await;
    assert_eq!(code(&r), "forbidden", "{r}");

    // The listing alone: the emote moves to the personal queue.
    w.ok(
        &lister,
        REVIEW,
        json!({ "ids": [id], "i": { "publicListing": "APPROVE" } }),
    )
    .await;
    let q = w
        .ok(&helper, QUEUE, json!({ "f": { "tab": "PERSONAL" } }))
        .await["reviewQueue"]
        .clone();
    assert_eq!(q["items"][0]["id"], id);
    assert_eq!(q["counts"]["personal"], 1);

    w.ok(
        &helper,
        REVIEW,
        json!({ "ids": [id], "i": { "personalUse": "APPROVE" } }),
    )
    .await;
    let q = w
        .ok(&helper, QUEUE, json!({ "f": { "tab": "PERSONAL" } }))
        .await["reviewQueue"]
        .clone();
    assert_eq!(q["total"], 0);
    let q = w
        .ok(
            &lister,
            QUEUE,
            json!({ "f": { "tab": "RESOLVED", "personalUse": "APPROVED" } }),
        )
        .await["reviewQueue"]
        .clone();
    assert_eq!(q["items"][0]["id"], id);
    w.db.close().await;
}

#[tokio::test]
async fn a_personal_set_takes_only_emotes_approved_for_personal_use() {
    let Some(w) = World::new().await else {
        return;
    };
    let alice = w.user("1", "alice", &[]).await;
    let bob = w.user("2", "bob", &[]).await;
    let mod_ = w.user("3", "mod", BOTH).await;
    let id = w.pending(&alice, "alicePers").await;
    w.ok(
        &mod_,
        REVIEW,
        json!({ "ids": [id], "i": { "publicListing": "APPROVE" } }),
    )
    .await;
    let mine = w.personal_set(&bob).await;

    let r = w.gql(&bob, ADD, json!({ "s": mine, "e": id })).await;
    assert_eq!(code(&r), "personal_use_not_approved", "{r}");
    assert!(
        r["errors"][0]["message"]
            .as_str()
            .unwrap()
            .contains("isn't approved for personal use yet")
    );
    // A channel set takes it.
    let channel = w
        .ok(&bob, CREATE_SET, json!({ "n": "Chan", "k": "CHANNEL" }))
        .await["createSet"]["id"]
        .as_str()
        .unwrap()
        .to_owned();
    w.ok(&bob, ADD, json!({ "s": channel, "e": id })).await;

    w.ok(
        &mod_,
        REVIEW,
        json!({ "ids": [id], "i": { "personalUse": "APPROVE" } }),
    )
    .await;
    w.ok(&bob, ADD, json!({ "s": mine, "e": id })).await;

    // Denied later: the member stays (flagged by its approvals), new adds are refused.
    w.ok(
        &mod_,
        REVIEW,
        json!({ "ids": [id], "i": { "personalUse": "DENY", "reason": "Not for everywhere", "onlyPending": false } }),
    )
    .await;
    let mut conn = w.db.conn().await;
    let kept: i64 = sqlx::query_scalar("SELECT count(*) FROM emote_set_members WHERE set_id = $1")
        .bind(w.db.row(&mine).await)
        .fetch_one(&mut *conn)
        .await
        .unwrap();
    assert_eq!(kept, 1);
    drop(conn);
    let carol = w.user("4", "carol", &[]).await;
    let hers = w.personal_set(&carol).await;
    let r = w.gql(&carol, ADD, json!({ "s": hers, "e": id })).await;
    assert_eq!(code(&r), "personal_use_not_approved", "{r}");
    assert!(
        r["errors"][0]["message"]
            .as_str()
            .unwrap()
            .contains("was not approved")
    );
    w.db.close().await;
}

#[tokio::test]
async fn reject_denies_both_and_rejected_emotes_cant_be_reviewed() {
    let Some(w) = World::new().await else {
        return;
    };
    let alice = w.user("1", "alice", &[]).await;
    let mod_ = w.user("2", "mod", BOTH).await;
    let id = w.pending(&alice, "aliceBad").await;
    w.ok(
        &mod_,
        "mutation($id: String!) { rejectEmote(id: $id, reason: \"Not an emote\") { state } }",
        json!({ "id": id }),
    )
    .await;
    let e = w.emote(&alice, &id).await;
    assert_eq!(
        e["approvals"],
        json!({ "publicListing": "DENIED", "personalUse": "DENIED" })
    );
    let r = w
        .ok(
            &mod_,
            REVIEW,
            json!({ "ids": [id], "i": { "personalUse": "APPROVE", "onlyPending": false } }),
        )
        .await;
    assert_eq!(r["reviewApprovals"]["failed"][0]["code"], "not_pending");

    // The old approveEmote decides the listing only.
    let other = w.pending(&alice, "aliceOld").await;
    w.ok(
        &mod_,
        "mutation($id: String!) { approveEmote(id: $id) { state } }",
        json!({ "id": other }),
    )
    .await;
    let e = w.emote(&alice, &other).await;
    assert_eq!(e["visibility"], "PUBLIC");
    assert_eq!(
        e["approvals"],
        json!({ "publicListing": "APPROVED", "personalUse": "PENDING" })
    );
    w.db.close().await;
}

#[tokio::test]
async fn only_staff_with_emotes_transfer_move_an_emote() {
    let Some(w) = World::new().await else {
        return;
    };
    let alice = w.user("1", "alice", &[]).await;
    let bob = w.user("2", "bob", &[]).await;
    let staff = w.user("3", "staff", &["emotes.transfer"]).await;
    let editor_only = w.user("4", "edits", &["emotes.edit_any"]).await;
    let id = w.pending(&alice, "aliceGift").await;
    {
        let mut conn = w.db.conn().await;
        emotes::set_state(&mut conn, w.db.row(&id).await, EmoteState::Approved)
            .await
            .unwrap();
    }
    let to = cyex_tv_db::ids::to_text(bob.id);

    // The owner can't give it away; edit_any doesn't imply it.
    for who in [&alice, &editor_only] {
        let r = w.gql(who, TRANSFER, json!({ "id": id, "to": to })).await;
        assert_eq!(code(&r), "transfer_staff_only", "{r}");
    }

    let r = w.ok(&staff, TRANSFER, json!({ "id": id, "to": to })).await;
    assert_eq!(
        r["transferEmote"]["owner"]["id"],
        w.db.public(bob.id).await.as_str()
    );
    let rows = w.audited("emote.transferred", &id).await;
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["via"], "emotes.transfer");
    assert_eq!(
        w.notices(&alice, "emote.transferred").await[0]["role"],
        "from"
    );
    assert_eq!(w.notices(&bob, "emote.transferred").await[0]["role"], "to");

    // To its owner again: refused.
    let r = w.gql(&staff, TRANSFER, json!({ "id": id, "to": to })).await;
    assert_eq!(code(&r), "same_owner", "{r}");
    w.db.close().await;
}

#[tokio::test]
async fn the_request_flow_is_retired() {
    let Some(w) = World::new().await else {
        return;
    };
    let alice = w.user("1", "alice", &[]).await;
    let any = cyex_tv_db::ids::to_text(alice.id);
    for q in [ACCEPT, DECLINE] {
        let r = w.gql(&alice, q, json!({ "id": any })).await;
        assert_eq!(code(&r), "transfer_requests_retired", "{r}");
    }
    let r = w.ok(&alice, REQUESTS, json!({})).await;
    assert_eq!(r["emoteTransferRequests"], json!([]));
    let meta = w.get("/v1/meta").await;
    assert!(
        meta["deprecations"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["id"] == "emote_transfer_requests")
    );
    w.db.close().await;
}

#[tokio::test]
async fn edit_permissions_owner_fields_and_staff_listing() {
    let Some(w) = World::new().await else {
        return;
    };
    let alice = w.user("1", "alice", &[]).await;
    let stranger = w.user("2", "stranger", &[]).await;
    let flags = w.user("3", "flags", &["emotes.flags"]).await;
    let id = w.pending(&alice, "aliceEdit").await;
    {
        let mut conn = w.db.conn().await;
        emotes::set_state(&mut conn, w.db.row(&id).await, EmoteState::Approved)
            .await
            .unwrap();
    }
    // Strangers can't edit; the owner changes zero-width and private.
    let r = w
        .gql(
            &stranger,
            UPDATE,
            json!({ "id": id, "i": { "zeroWidth": true } }),
        )
        .await;
    assert_eq!(code(&r), "forbidden", "{r}");
    w.ok(
        &alice,
        UPDATE,
        json!({ "id": id, "i": { "zeroWidth": true } }),
    )
    .await;
    assert_eq!(w.emote(&alice, &id).await["flags"]["zeroWidth"], true);
    // Strangers can't decide approvals either.
    let r = w
        .gql(
            &stranger,
            REVIEW,
            json!({ "ids": [id], "i": { "publicListing": "APPROVE" } }),
        )
        .await;
    assert_eq!(code(&r), "forbidden", "{r}");

    // Staff with emotes.flags taking it off the listings denies the listing (and back approves it).
    let r = w
        .ok(
            &flags,
            "mutation($id: String!) { setEmoteFlags(id: $id, input: { visibility: UNLISTED }) { visibility approvals { publicListing } } }",
            json!({ "id": id }),
        )
        .await;
    assert_eq!(r["setEmoteFlags"]["visibility"], "UNLISTED");
    assert_eq!(r["setEmoteFlags"]["approvals"]["publicListing"], "DENIED");
    let r = w
        .ok(
            &flags,
            "mutation($id: String!) { setEmoteFlags(id: $id, input: { visibility: PUBLIC }) { visibility approvals { publicListing } } }",
            json!({ "id": id }),
        )
        .await;
    assert_eq!(r["setEmoteFlags"]["visibility"], "PUBLIC");
    assert_eq!(r["setEmoteFlags"]["approvals"]["publicListing"], "APPROVED");
    w.db.close().await;
}

/// The admin rail's counts (`AdminCounts`): each part is gated by its own permission, so someone
/// holding only `emotes.review_personal` can read the personal-use count and nothing more.
const ADMIN_COUNTS: &str = "query($review: Boolean!, $personal: Boolean!) {
    reviewQueue(page: 1, perPage: 1) @include(if: $review) { total counts { personal } }
    personalQueue: reviewQueue(page: 1, perPage: 1, filter: { tab: PERSONAL }) @include(if: $personal) { total }
}";

#[tokio::test]
async fn the_rail_count_for_personal_use_follows_its_permission() {
    let Some(w) = World::new().await else {
        return;
    };
    let alice = w.user("1", "alice", &[]).await;
    let lister = w.user("2", "lister", &["emotes.review"]).await;
    let helper = w.user("3", "helper", &["emotes.review_personal"]).await;
    let both = w.user("4", "both", BOTH).await;
    let id = w.pending(&alice, "aliceRail").await;
    w.ok(
        &lister,
        REVIEW,
        json!({ "ids": [id], "i": { "publicListing": "APPROVE" } }),
    )
    .await;

    // Only the personal permission: the personal count, and the main queue stays closed.
    let r = w
        .ok(
            &helper,
            ADMIN_COUNTS,
            json!({ "review": false, "personal": true }),
        )
        .await;
    assert_eq!(r["personalQueue"]["total"], 1);
    let r = w
        .gql(
            &helper,
            ADMIN_COUNTS,
            json!({ "review": true, "personal": false }),
        )
        .await;
    assert_eq!(code(&r), "forbidden", "{r}");
    // Someone with neither permission gets nothing.
    let nobody = w.user("5", "nobody", &[]).await;
    let r = w
        .gql(
            &nobody,
            ADMIN_COUNTS,
            json!({ "review": false, "personal": true }),
        )
        .await;
    assert_eq!(code(&r), "forbidden", "{r}");
    // Both: the main queue's answer carries the personal count to add to it.
    let r = w
        .ok(
            &both,
            ADMIN_COUNTS,
            json!({ "review": true, "personal": false }),
        )
        .await;
    assert_eq!(r["reviewQueue"]["total"], 0);
    assert_eq!(r["reviewQueue"]["counts"]["personal"], 1);
    w.db.close().await;
}
