//! Batch verdicts and batch report dismissals (one transaction, one audit row per item), the
//! "possible duplicate" read, the uploader's same-file check, the ready reject reasons staff
//! keep, and matching pasted logins to accounts. Through the real router against a real Postgres;
//! runs when `CYEX_TV_TEST_DATABASE_URL` is set (CI sets it), skips otherwise.

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
use cyex_tv_db::files::{self, NewFile};
use cyex_tv_db::sessions::{self, SessionKind};
use cyex_tv_db::{ids, reports, roles};
use cyex_tv_shared::model::{EmoteState, Platform};
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

const REVIEW: &str = "mutation($ids: [String!]!, $v: ReviewVerdict!, $vis: EmoteVisibility, $r: String) { reviewEmotes(ids: $ids, verdict: $v, visibility: $vis, reason: $r) { done failed { id code message } } }";
const DISMISS: &str = "mutation($ids: [String!]!, $n: String) { dismissReports(ids: $ids, note: $n) { done failed { id code } } }";
const DUPES: &str = "query($id: String!) { emoteDuplicates(emoteId: $id) { sameFile similarity emote { id name state } } }";
const SAME_FILE: &str =
    "query($s: String!) { emotesWithSameFile(sha256: $s) { id name owner { displayName } } }";
const REASONS: &str = "query { reviewReasons }";
const SET_REASONS: &str = "mutation($r: [String!]!) { setReviewReasons(reasons: $r) }";
const LOGINS: &str = "query($l: [String!]!) { accountsByLogin(logins: $l) { login problem account { id displayName } } }";

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

    /// A user signed in on `platform` as `login`, with a site session and the given permissions.
    async fn user_on(
        &self,
        platform: Platform,
        platform_id: &str,
        login: &str,
        permissions: &[&str],
    ) -> User {
        let mut conn = self.db.conn().await;
        let signed = accounts::sign_in(
            &mut conn,
            Profile {
                platform,
                platform_user_id: platform_id,
                login,
                display_name: login,
                avatar: None,
            },
        )
        .await
        .unwrap();
        if !permissions.is_empty() {
            let role = roles::create(
                &mut conn,
                &roles::NewRole {
                    name: &format!("role {platform_id}"),
                    description: None,
                    position: 1,
                    permissions,
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

    async fn user(&self, twitch_id: &str, login: &str, permissions: &[&str]) -> User {
        self.user_on(Platform::Twitch, twitch_id, login, permissions)
            .await
    }

    /// A stored file whose sha256 is 32 times `byte`.
    async fn file(&self, byte: u8) -> Uuid {
        let mut conn = self.db.conn().await;
        let sha = [byte; 32];
        let key = format!("originals/{byte:02x}");
        files::insert_or_get(
            &mut conn,
            &NewFile {
                sha256: &sha,
                dhash: Some(i64::from(byte)),
                mime: "image/png",
                width: 32,
                height: 32,
                frame_count: 1,
                storage_key: &key,
            },
        )
        .await
        .unwrap()
        .0
        .id
    }

    /// An emote in `state`, from `file` when given.
    async fn emote(
        &self,
        owner: &User,
        name: &str,
        file: Option<Uuid>,
        state: EmoteState,
        unlisted: bool,
    ) -> String {
        let mut conn = self.db.conn().await;
        let emote = emotes::create(
            &mut conn,
            &NewEmote {
                owner_account_id: owner.id,
                file_id: file,
                name,
                unlisted,
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

    async fn count(&self, sql: &str, bind: &str) -> i64 {
        let mut conn = self.db.conn().await;
        sqlx::query_scalar(sql)
            .bind(bind)
            .fetch_one(&mut *conn)
            .await
            .unwrap()
    }

    async fn state_of(&self, id: &str) -> String {
        let mut conn = self.db.conn().await;
        sqlx::query_scalar("SELECT state FROM emotes WHERE id = $1")
            .bind(self.db.row(id).await)
            .fetch_one(&mut *conn)
            .await
            .unwrap()
    }
}

#[tokio::test]
async fn batch_verdicts_commit_together_with_an_audit_row_each() {
    let Some(w) = World::new().await else {
        return;
    };
    let reviewer = w.user("201", "reviewer", &["emotes.review"]).await;
    let alice = w.user("202", "alice", &[]).await;
    let a = w
        .emote(&alice, "OneA", None, EmoteState::Pending, false)
        .await;
    let b = w
        .emote(&alice, "OneB", None, EmoteState::Pending, false)
        .await;
    let c = w
        .emote(&alice, "OneC", None, EmoteState::Pending, false)
        .await;
    let done = w
        .emote(&alice, "Done", None, EmoteState::Approved, false)
        .await;

    // Deny: without emotes.review nothing changes.
    let body = w
        .gql(Some(&alice), REVIEW, json!({ "ids": [a], "v": "APPROVE" }))
        .await;
    assert_eq!(code(&body), "forbidden", "{body}");
    assert_eq!(w.state_of(&a).await, "pending");

    // Approve as unlisted: the pending ones change; the approved one and the bad ID are listed.
    let body = w
        .gql(
            Some(&reviewer),
            REVIEW,
            json!({ "ids": [a, b, done, "nope", a], "v": "APPROVE", "vis": "UNLISTED" }),
        )
        .await;
    assert!(body["errors"].is_null(), "{body}");
    let out = &body["data"]["reviewEmotes"];
    assert_eq!(out["done"], json!([a, b]));
    let failed = out["failed"].as_array().unwrap();
    assert_eq!(failed.len(), 2, "{out}");
    assert_eq!(failed[0]["id"], json!(done));
    assert_eq!(failed[0]["code"], "not_pending");
    assert_eq!(failed[1]["id"], "nope");
    assert_eq!(failed[1]["code"], "not_found");
    assert_eq!(w.state_of(&a).await, "approved");
    let mut conn = w.db.conn().await;
    let unlisted: bool = sqlx::query_scalar("SELECT unlisted FROM emotes WHERE id = $1")
        .bind(w.db.row(&b).await)
        .fetch_one(&mut *conn)
        .await
        .unwrap();
    assert!(unlisted);
    drop(conn);
    // One audit row and one notice per emote.
    assert_eq!(
        w.count(
            "SELECT count(*) FROM audit_log WHERE action = 'emote.approved' AND target_id = ANY(string_to_array($1, ','))",
            &format!("{a},{b}"),
        )
        .await,
        2
    );
    assert_eq!(
        w.count(
            "SELECT count(*) FROM notifications WHERE kind = 'emote.approved' AND account_id::text = $1",
            &alice.id.to_string(),
        )
        .await,
        2
    );

    // A rejection needs a reason, and takes no visibility.
    let body = w
        .gql(
            Some(&reviewer),
            REVIEW,
            json!({ "ids": [c], "v": "REJECT" }),
        )
        .await;
    assert_eq!(code(&body), "bad_reason", "{body}");
    let body = w
        .gql(
            Some(&reviewer),
            REVIEW,
            json!({ "ids": [c], "v": "REJECT", "r": "Blurry", "vis": "PRIVATE" }),
        )
        .await;
    assert_eq!(code(&body), "bad_visibility", "{body}");
    let body = w
        .gql(
            Some(&reviewer),
            REVIEW,
            json!({ "ids": [c], "v": "REJECT", "r": "Blurry" }),
        )
        .await;
    assert_eq!(body["data"]["reviewEmotes"]["done"], json!([c]), "{body}");
    assert_eq!(w.state_of(&c).await, "rejected");
    assert_eq!(
        w.count(
            "SELECT count(*) FROM audit_log WHERE action = 'emote.rejected' AND after_json->>'reason' = $1",
            "Blurry",
        )
        .await,
        1
    );

    // At most 100, and at least one.
    let many: Vec<String> = (0..101).map(|i| format!("id{i}")).collect();
    let body = w
        .gql(
            Some(&reviewer),
            REVIEW,
            json!({ "ids": many, "v": "APPROVE" }),
        )
        .await;
    assert_eq!(code(&body), "bad_batch", "{body}");
    let body = w
        .gql(
            Some(&reviewer),
            REVIEW,
            json!({ "ids": [], "v": "APPROVE" }),
        )
        .await;
    assert_eq!(code(&body), "bad_batch", "{body}");
    w.db.close().await;
}

#[tokio::test]
async fn reports_are_dismissed_in_one_step() {
    let Some(w) = World::new().await else {
        return;
    };
    let moderator = w.user("301", "moderator", &["reports.manage"]).await;
    let alice = w.user("302", "alice", &[]).await;
    let bob = w.user("303", "bob", &[]).await;
    let e1 = w
        .emote(&alice, "Rep1", None, EmoteState::Approved, false)
        .await;
    let e2 = w
        .emote(&alice, "Rep2", None, EmoteState::Approved, false)
        .await;
    let mut conn = w.db.conn().await;
    let r1 = reports::create(&mut conn, w.db.row(&e1).await, bob.id, "spam")
        .await
        .unwrap();
    let r2 = reports::create(&mut conn, w.db.row(&e2).await, bob.id, "spam")
        .await
        .unwrap();
    drop(conn);
    let (r1, r2) = (ids::to_text(r1.id), ids::to_text(r2.id));

    let body = w.gql(Some(&bob), DISMISS, json!({ "ids": [r1] })).await;
    assert_eq!(code(&body), "forbidden", "{body}");

    let body = w
        .gql(
            Some(&moderator),
            DISMISS,
            json!({ "ids": [r1, r2], "n": "Fine as it is" }),
        )
        .await;
    assert!(body["errors"].is_null(), "{body}");
    assert_eq!(body["data"]["dismissReports"]["done"], json!([r1, r2]));
    assert_eq!(
        w.count(
            "SELECT count(*) FROM audit_log WHERE action = 'report.dismissed' AND after_json->>'note' = $1",
            "Fine as it is",
        )
        .await,
        2
    );
    // Again: both are resolved already.
    let body = w
        .gql(Some(&moderator), DISMISS, json!({ "ids": [r1] }))
        .await;
    assert_eq!(body["data"]["dismissReports"]["done"], json!([]));
    assert_eq!(
        body["data"]["dismissReports"]["failed"][0]["code"],
        "not_open"
    );
    w.db.close().await;
}

#[tokio::test]
async fn duplicates_and_the_same_file_check() {
    let Some(w) = World::new().await else {
        return;
    };
    let reviewer = w.user("401", "reviewer", &["emotes.review"]).await;
    let alice = w.user("402", "alice", &[]).await;
    let bob = w.user("403", "bob", &[]).await;
    let file = w.file(7).await;
    let other = w.file(8).await;
    let original = w
        .emote(&alice, "catJAM", Some(file), EmoteState::Approved, false)
        .await;
    let hidden = w
        .emote(&alice, "catJamLink", Some(file), EmoteState::Approved, true)
        .await;
    let upload = w
        .emote(&bob, "catJAMM", Some(file), EmoteState::Pending, false)
        .await;
    let similar = w
        .emote(&alice, "CATJAM", Some(other), EmoteState::Rejected, false)
        .await;
    w.emote(
        &alice,
        "pepeHands",
        Some(other),
        EmoteState::Approved,
        false,
    )
    .await;

    let body = w.gql(Some(&alice), DUPES, json!({ "id": upload })).await;
    assert_eq!(code(&body), "forbidden", "{body}");

    let body = w.gql(Some(&reviewer), DUPES, json!({ "id": upload })).await;
    assert!(body["errors"].is_null(), "{body}");
    let list = body["data"]["emoteDuplicates"].as_array().unwrap();
    let ids_of: Vec<&str> = list
        .iter()
        .map(|d| d["emote"]["id"].as_str().unwrap())
        .collect();
    // The same file first (both), then the similar name; never itself, never pepeHands.
    assert_eq!(&ids_of[..2].len(), &2, "{body}");
    assert!(ids_of[..2].contains(&original.as_str()));
    assert!(ids_of[..2].contains(&hidden.as_str()));
    assert!(list[0]["sameFile"].as_bool().unwrap());
    assert!(ids_of.contains(&similar.as_str()), "{body}");
    assert!(!ids_of.contains(&upload.as_str()));
    assert!(list.iter().all(|d| d["emote"]["name"] != "pepeHands"));
    let near = list
        .iter()
        .find(|d| d["emote"]["id"] == json!(similar))
        .unwrap();
    assert_eq!(near["sameFile"], false);
    assert!(near["similarity"].as_f64().unwrap() >= 0.5);

    // The uploader's check: public and approved only (not the unlisted copy, not the pending one).
    let sha = "07".repeat(32);
    let body = w.gql(None, SAME_FILE, json!({ "s": sha })).await;
    assert!(!body["errors"].is_null(), "signed in only: {body}");
    let body = w.gql(Some(&bob), SAME_FILE, json!({ "s": sha })).await;
    assert!(body["errors"].is_null(), "{body}");
    assert_eq!(
        body["data"]["emotesWithSameFile"],
        json!([{ "id": original, "name": "catJAM", "owner": { "displayName": "alice" } }])
    );
    let body = w
        .gql(Some(&bob), SAME_FILE, json!({ "s": "09".repeat(32) }))
        .await;
    assert_eq!(body["data"]["emotesWithSameFile"], json!([]));
    let body = w.gql(Some(&bob), SAME_FILE, json!({ "s": "nothex" })).await;
    assert_eq!(code(&body), "bad_sha256", "{body}");
    w.db.close().await;
}

#[tokio::test]
async fn ready_reasons_are_kept_by_review_settings_and_audited() {
    let Some(w) = World::new().await else {
        return;
    };
    let reviewer = w.user("501", "reviewer", &["emotes.review"]).await;
    let lead = w.user("502", "lead", &["review.settings"]).await;
    let nobody = w.user("503", "nobody", &[]).await;

    let body = w.gql(Some(&reviewer), REASONS, json!({})).await;
    let seeded = body["data"]["reviewReasons"].as_array().unwrap();
    assert_eq!(seeded.len(), 6, "{body}");
    assert_eq!(seeded[0], "Low quality or blurry");
    let body = w.gql(Some(&nobody), REASONS, json!({})).await;
    assert_eq!(code(&body), "forbidden", "{body}");

    // Reviewers read them; only review.settings changes them.
    let body = w
        .gql(Some(&reviewer), SET_REASONS, json!({ "r": ["Blurry"] }))
        .await;
    assert_eq!(code(&body), "forbidden", "{body}");
    let body = w
        .gql(
            Some(&lead),
            SET_REASONS,
            json!({ "r": [" Blurry ", "Not an emote", "blurry"] }),
        )
        .await;
    assert_eq!(
        body["data"]["setReviewReasons"],
        json!(["Blurry", "Not an emote"]),
        "{body}"
    );
    let body = w.gql(Some(&reviewer), REASONS, json!({})).await;
    assert_eq!(
        body["data"]["reviewReasons"],
        json!(["Blurry", "Not an emote"])
    );
    assert_eq!(
        w.count(
            "SELECT count(*) FROM audit_log WHERE action = 'review.reasons_changed' AND actor_account_id::text = $1",
            &lead.id.to_string(),
        )
        .await,
        1
    );
    let body = w.gql(Some(&lead), SET_REASONS, json!({ "r": [""] })).await;
    assert_eq!(code(&body), "bad_reason", "{body}");
    let body = w.gql(Some(&lead), SET_REASONS, json!({ "r": [] })).await;
    assert_eq!(body["data"]["setReviewReasons"], json!([]), "{body}");
    w.db.close().await;
}

#[tokio::test]
async fn pasted_logins_match_accounts() {
    let Some(w) = World::new().await else {
        return;
    };
    let granter = w.user("601", "granter", &["cosmetics.grant"]).await;
    let alice = w.user("602", "alice", &[]).await;
    let twin_twitch = w.user("603", "twin", &[]).await;
    let twin_kick = w.user_on(Platform::Kick, "604", "twin", &[]).await;

    let body = w.gql(Some(&alice), LOGINS, json!({ "l": ["alice"] })).await;
    assert_eq!(code(&body), "forbidden", "{body}");

    let body = w
        .gql(
            Some(&granter),
            LOGINS,
            json!({ "l": ["@Alice", "ghost", "twin", "kick:twin", "bad login", "ALICE", " "] }),
        )
        .await;
    assert!(body["errors"].is_null(), "{body}");
    let list = body["data"]["accountsByLogin"].as_array().unwrap();
    assert_eq!(list.len(), 5, "{body}");
    assert_eq!(list[0]["account"]["id"], json!(w.db.public(alice.id).await));
    assert_eq!(list[1]["problem"], "not_found");
    assert_eq!(list[2]["problem"], "ambiguous");
    assert_eq!(
        list[3]["account"]["id"],
        json!(w.db.public(twin_kick.id).await)
    );
    assert_eq!(list[4]["problem"], "bad_login");
    drop(twin_twitch);
    w.db.close().await;
}

#[tokio::test]
async fn two_saves_of_the_reasons_at_once_take_turns() {
    let Some(w) = World::new().await else {
        return;
    };
    let a = w.user("701", "lead_a", &["review.settings"]).await;
    let b = w.user("702", "lead_b", &["review.settings"]).await;
    // Both at once, many times: without the lock the second DELETE + INSERT collides with the
    // first's rows and answers an internal error.
    for round in 0..5 {
        let first = json!({ "r": [format!("A{round} one"), format!("A{round} two")] });
        let second = json!({ "r": [format!("B{round} one")] });
        let (ra, rb) = tokio::join!(
            w.gql(Some(&a), SET_REASONS, first),
            w.gql(Some(&b), SET_REASONS, second)
        );
        assert!(ra["errors"].is_null(), "{ra}");
        assert!(rb["errors"].is_null(), "{rb}");
        let now = w.gql(Some(&a), REASONS, json!({})).await;
        let list = now["data"]["reviewReasons"].clone();
        assert!(
            list == ra["data"]["setReviewReasons"] || list == rb["data"]["setReviewReasons"],
            "one of the two saves wins whole: {list}"
        );
    }
    // Every save is in the audit log, each with the list it replaced.
    assert_eq!(
        w.count(
            "SELECT count(*) FROM audit_log WHERE action = 'review.reasons_changed' AND $1 <> ''",
            "x",
        )
        .await,
        10
    );
    w.db.close().await;
}

#[tokio::test]
async fn login_checks_take_from_the_account_search_limit() {
    let Some(w) = World::new().await else {
        return;
    };
    let granter = w.user("801", "granter", &["cosmetics.grant"]).await;
    let mut limited = false;
    for _ in 0..80 {
        let body = w
            .gql(Some(&granter), LOGINS, json!({ "l": ["someone"] }))
            .await;
        if code(&body) == "rate_limited" {
            limited = true;
            break;
        }
        assert!(body["errors"].is_null(), "{body}");
    }
    assert!(limited, "80 checks in a row were never limited");
    w.db.close().await;
}

#[tokio::test]
async fn the_same_file_check_hides_emotes_of_deleted_and_blocked_accounts() {
    let Some(w) = World::new().await else {
        return;
    };
    let alice = w.user("901", "alice", &[]).await;
    let leaving = w.user("902", "leaving", &[]).await;
    let file = w.file(11).await;
    w.emote(
        &leaving,
        "goneSoon",
        Some(file),
        EmoteState::Approved,
        false,
    )
    .await;
    let mut conn = w.db.conn().await;
    sqlx::query("UPDATE accounts SET deleted_at = now() WHERE id = $1")
        .bind(leaving.id)
        .execute(&mut *conn)
        .await
        .unwrap();
    drop(conn);
    let body = w
        .gql(Some(&alice), SAME_FILE, json!({ "s": "0b".repeat(32) }))
        .await;
    assert_eq!(body["data"]["emotesWithSameFile"], json!([]), "{body}");

    // Nor of blocked accounts: the rule every emote search shares.
    let blocked = w.user("903", "blocked", &[]).await;
    let file = w.file(12).await;
    w.emote(
        &blocked,
        "blockedOne",
        Some(file),
        EmoteState::Approved,
        false,
    )
    .await;
    let mut conn = w.db.conn().await;
    sqlx::query("UPDATE accounts SET blocked_at = now() WHERE id = $1")
        .bind(blocked.id)
        .execute(&mut *conn)
        .await
        .unwrap();
    drop(conn);
    let body = w
        .gql(Some(&alice), SAME_FILE, json!({ "s": "0c".repeat(32) }))
        .await;
    assert_eq!(body["data"]["emotesWithSameFile"], json!([]), "{body}");
    w.db.close().await;
}

#[tokio::test]
async fn verdicts_retried_with_their_key_answer_the_first_outcome() {
    let Some(w) = World::new().await else {
        return;
    };
    const APPROVE: &str = "mutation($id: String!, $k: String) { approveEmote(id: $id, idempotencyKey: $k) { id state } }";
    const REVIEW_KEYED: &str = "mutation($ids: [String!]!, $v: ReviewVerdict!, $r: String, $k: String) { reviewEmotes(ids: $ids, verdict: $v, reason: $r, idempotencyKey: $k) { done failed { id code } } }";
    let reviewer = w.user("991", "keyreviewer", &["emotes.review"]).await;
    let alice = w.user("992", "keyalice", &[]).await;
    let a = w
        .emote(&alice, "KeyA", None, EmoteState::Pending, false)
        .await;
    let b = w
        .emote(&alice, "KeyB", None, EmoteState::Pending, false)
        .await;
    let c = w
        .emote(&alice, "KeyC", None, EmoteState::Pending, false)
        .await;
    let audits = "SELECT count(*) FROM audit_log WHERE action = $1";

    let key = ids::new_id().to_string();
    let first = w
        .gql(Some(&reviewer), APPROVE, json!({ "id": a, "k": key }))
        .await;
    assert_eq!(
        first["data"]["approveEmote"]["state"], "APPROVED",
        "{first}"
    );
    let again = w
        .gql(Some(&reviewer), APPROVE, json!({ "id": a, "k": key }))
        .await;
    assert_eq!(again["data"], first["data"], "{again}");
    assert_eq!(w.count(audits, "emote.approved").await, 1);
    // Without it, the second approval says it isn't pending any more.
    let r = w.gql(Some(&reviewer), APPROVE, json!({ "id": a })).await;
    assert_eq!(code(&r), "not_pending", "{r}");
    // The same key for another emote: refused, and that one stays pending.
    let r = w
        .gql(Some(&reviewer), APPROVE, json!({ "id": b, "k": key }))
        .await;
    assert_eq!(code(&r), "idempotency_key_reused", "{r}");
    assert_eq!(w.state_of(&b).await, "pending");

    let key = ids::new_id().to_string();
    let vars = json!({ "ids": [b, c], "v": "REJECT", "r": "Blurry", "k": key });
    let first = w.gql(Some(&reviewer), REVIEW_KEYED, vars.clone()).await;
    assert_eq!(
        first["data"]["reviewEmotes"]["done"],
        json!([b, c]),
        "{first}"
    );
    let again = w.gql(Some(&reviewer), REVIEW_KEYED, vars).await;
    assert_eq!(again["data"], first["data"], "{again}");
    assert_eq!(w.count(audits, "emote.rejected").await, 2);
    w.db.close().await;
}
