//! Merging emotes (ADR 0040): every set membership of the source moves to the target with its
//! name kept, a set holding both keeps the target's membership, delta sync reports the source
//! removed and the target added in every affected set, the source's ID answers the target (REST
//! `301`, GraphQL `mergedInto`), search leaves it out, `emotes.merge` is needed, chains are
//! followed and merging into itself, into an unapproved emote or an emote merged already is
//! refused. Through the real router against a real Postgres; runs when
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
use cyex_tv_storage::Store;
use http_body_util::BodyExt;
use serde_json::{Value, json};
use tower::ServiceExt;
use uuid::Uuid;

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

const MERGE: &str = "mutation($id: String!, $into: String!) { mergeEmote(id: $id, intoId: $into) { sourceId moved collapsed { setId setName keptName droppedName } target { id name } } }";
const PREVIEW: &str = "query($id: String!, $into: String!) { emoteMergePreview(id: $id, intoId: $into) { followed moves collapses { keptName droppedName } source { id name } target { id name } } }";
const EMOTE: &str = "query($id: String!) { emote(id: $id) { id name mergedInto } }";
const EMOTES: &str = "query($ids: [ID!]!) { emotes(ids: $ids) { id name mergedInto } }";
const CREATE_SET: &str =
    "mutation($n: String!, $k: SetKind!) { createSet(input: { name: $n, kind: $k }) { id } }";
const ADD: &str = "mutation($s: String!, $e: String!, $i: AddSetEmoteInput) { addSetEmote(setId: $s, emoteId: $e, input: $i) { name } }";
const REMOVE: &str =
    "mutation($s: String!, $e: String!) { removeSetEmote(setId: $s, emoteId: $e) }";
const SET: &str =
    "query($id: String!) { set(id: $id) { version emotes { name zeroWidth emote { id } } } }";
const CHANGES: &str = "query($id: String!, $since: Int!) { setChanges(id: $id, since: $since) { version full added { name emote { id } } renamed { emote { id } } updated { emote { id } } removedIds } }";
const SEARCH: &str = "query($q: String!) { searchEmotes(query: $q) { items { id } } }";
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

    /// An emote of `owner` in `state` (both approvals follow it), by its public ID.
    async fn emote_in(&self, owner: &User, name: &str, state: EmoteState) -> String {
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
        if state != EmoteState::Pending {
            emotes::set_state(&mut conn, e.id, state).await.unwrap();
        }
        drop(conn);
        self.db.public(e.id).await
    }

    async fn approved(&self, owner: &User, name: &str) -> String {
        self.emote_in(owner, name, EmoteState::Approved).await
    }

    async fn gql(&self, user: Option<&User>, query: &str, variables: Value) -> Value {
        let mut request = Request::post("/v1/gql")
            .header("content-type", "application/json")
            .header("origin", SITE);
        if let Some(user) = user {
            request = request.header("cookie", format!("{}={}", cookies::SESSION, user.token));
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
        let body = self.gql(Some(user), query, variables).await;
        assert!(body["errors"].is_null(), "{body}");
        body["data"].clone()
    }

    async fn merge(&self, user: &User, id: &str, into: &str) -> Value {
        self.gql(Some(user), MERGE, json!({ "id": id, "into": into }))
            .await
    }

    async fn set_of(&self, user: &User, name: &str) -> String {
        self.ok(user, CREATE_SET, json!({ "n": name, "k": "CHANNEL" }))
            .await["createSet"]["id"]
            .as_str()
            .unwrap()
            .to_owned()
    }

    async fn add(&self, user: &User, set: &str, emote: &str, name: Option<&str>, zw: bool) {
        self.ok(
            user,
            ADD,
            json!({ "s": set, "e": emote, "i": { "name": name, "zeroWidth": zw } }),
        )
        .await;
    }

    async fn set(&self, user: &User, id: &str) -> Value {
        self.ok(user, SET, json!({ "id": id })).await["set"].clone()
    }

    async fn version(&self, user: &User, id: &str) -> i64 {
        self.set(user, id).await["version"].as_i64().unwrap()
    }

    async fn changes(&self, user: &User, id: &str, since: i64) -> Value {
        self.ok(user, CHANGES, json!({ "id": id, "since": since }))
            .await["setChanges"]
            .clone()
    }

    async fn personal_set(&self, user: &User) -> String {
        let mut conn = self.db.conn().await;
        let set = cyex_tv_db::sets::ensure_personal(&mut conn, user.id)
            .await
            .unwrap();
        drop(conn);
        self.db.public(set.id).await
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

    async fn audited(&self, emote: &str) -> Vec<Value> {
        let emote = cyex_tv_db::ids::to_text(self.db.row(emote).await);
        let mut conn = self.db.conn().await;
        let rows: Vec<(Option<Value>,)> = sqlx::query_as(
            "SELECT after_json FROM audit_log
             WHERE action = 'emote.merged' AND target_kind = 'emote' AND target_id = $1",
        )
        .bind(&emote)
        .fetch_all(&mut *conn)
        .await
        .unwrap();
        rows.into_iter()
            .map(|(a,)| a.unwrap_or(Value::Null))
            .collect()
    }

    /// A signed-out REST read: status, `Location`, body.
    async fn rest(&self, path: &str) -> (StatusCode, Option<String>, Value) {
        let response = self
            .app
            .clone()
            .oneshot(Request::get(path).body(Body::empty()).unwrap())
            .await
            .unwrap();
        let status = response.status();
        let location = response
            .headers()
            .get("location")
            .map(|v| v.to_str().unwrap().to_owned());
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        (
            status,
            location,
            serde_json::from_slice(&bytes).unwrap_or(Value::Null),
        )
    }
}

fn members(set: &Value) -> Vec<(String, String, bool)> {
    let mut out: Vec<(String, String, bool)> = set["emotes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|m| {
            (
                m["emote"]["id"].as_str().unwrap().to_owned(),
                m["name"].as_str().unwrap().to_owned(),
                m["zeroWidth"].as_bool().unwrap(),
            )
        })
        .collect();
    out.sort();
    out
}

fn ids_in(list: &Value) -> Vec<String> {
    list.as_array()
        .unwrap()
        .iter()
        .map(|m| m["emote"]["id"].as_str().unwrap().to_owned())
        .collect()
}

const MERGER: &[&str] = &["emotes.merge"];

#[tokio::test]
async fn memberships_move_with_their_names_and_duplicates_collapse() {
    let Some(w) = World::new().await else {
        return;
    };
    let alice = w.user("1", "alice", &[]).await;
    let bob = w.user("2", "bob", &[]).await;
    let carol = w.user("3", "carol", &[]).await;
    let staff = w.user("4", "staff", MERGER).await;

    let source = w.approved(&alice, "Pogg").await;
    let target = w.approved(&bob, "Pog").await;

    // A: the source under an alias, zero-width. B: both, under different names. Personal: the
    // source in carol's personal set.
    let a = w.set_of(&carol, "A").await;
    let b = w.set_of(&carol, "B").await;
    let personal = w.personal_set(&carol).await;
    w.add(&carol, &a, &source, Some("PogAlias"), true).await;
    w.add(&carol, &b, &source, None, false).await;
    w.add(&carol, &b, &target, Some("PogTarget"), false).await;
    w.add(&carol, &personal, &source, None, false).await;
    let (va, vb, vp) = (
        w.version(&carol, &a).await,
        w.version(&carol, &b).await,
        w.version(&carol, &personal).await,
    );

    // The preview says what will happen, and changes nothing.
    let p = w
        .ok(&staff, PREVIEW, json!({ "id": source, "into": target }))
        .await["emoteMergePreview"]
        .clone();
    assert_eq!(p["moves"], 2, "{p}");
    assert_eq!(p["followed"], false);
    assert_eq!(
        p["collapses"],
        json!([{ "keptName": "PogTarget", "droppedName": "Pogg" }])
    );
    assert_eq!(p["source"]["id"], source.as_str());
    assert_eq!(p["target"]["name"], "Pog");
    assert_eq!(w.version(&carol, &a).await, va);

    let done = w.merge(&staff, &source, &target).await;
    assert!(done["errors"].is_null(), "{done}");
    let r = &done["data"]["mergeEmote"];
    assert_eq!(r["sourceId"], source.as_str());
    assert_eq!(r["target"]["id"], target.as_str());
    assert_eq!(r["moved"], 2);
    assert_eq!(
        r["collapsed"],
        json!([{ "setId": b, "setName": "B", "keptName": "PogTarget", "droppedName": "Pogg" }])
    );

    // Every membership names the target, with its own name and flag.
    assert_eq!(
        members(&w.set(&carol, &a).await),
        [(target.clone(), "PogAlias".to_owned(), true)]
    );
    assert_eq!(
        members(&w.set(&carol, &b).await),
        [(target.clone(), "PogTarget".to_owned(), false)]
    );
    assert_eq!(
        members(&w.set(&carol, &personal).await),
        [(target.clone(), "Pogg".to_owned(), false)]
    );

    // Delta sync: one version per set; the source removed, the target added where it moved.
    for (set, since, added) in [(&a, va, true), (&personal, vp, true), (&b, vb, false)] {
        let c = w.changes(&carol, set, since).await;
        assert_eq!(c["full"], false, "{c}");
        assert_eq!(c["version"].as_i64().unwrap(), since + 1, "{c}");
        assert_eq!(c["removedIds"], json!([source]), "{c}");
        if added {
            assert_eq!(ids_in(&c["added"]), std::slice::from_ref(&target), "{c}");
        } else {
            assert_eq!(c["added"], json!([]), "{c}");
        }
        assert_eq!(c["renamed"], json!([]), "{c}");
    }

    // Nothing is stuck: the channel removes the target as any emote.
    w.ok(&carol, REMOVE, json!({ "s": a, "e": target })).await;
    assert!(members(&w.set(&carol, &a).await).is_empty());

    // Audited on both, and both owners are told.
    let rows = w.audited(&source).await;
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["via"], "emotes.merge");
    assert_eq!(rows[0]["moved"], 2);
    assert_eq!(w.audited(&target).await.len(), 1);
    let told = w.notices(&alice, "emote.merged").await;
    assert_eq!(told.len(), 1);
    assert_eq!(told[0]["role"], "source");
    assert_eq!(told[0]["source_name"], "Pogg");
    assert_eq!(w.notices(&bob, "emote.merged").await[0]["role"], "target");
    w.db.close().await;
}

#[tokio::test]
async fn the_old_id_answers_the_target_and_leaves_search() {
    let Some(w) = World::new().await else {
        return;
    };
    let alice = w.user("1", "alice", &[]).await;
    let staff = w.user("2", "staff", MERGER).await;
    let source = w.approved(&alice, "mergeOld").await;
    let target = w.approved(&alice, "mergeNew").await;
    let (status, _, body) = w.rest(&format!("/v1/emotes/{source}")).await;
    assert_eq!(status, StatusCode::OK, "{body}");

    let done = w.merge(&staff, &source, &target).await;
    assert!(done["errors"].is_null(), "{done}");

    // REST: a 301 to the target, which reads as usual.
    let (status, location, _) = w.rest(&format!("/v1/emotes/{source}")).await;
    assert_eq!(status, StatusCode::MOVED_PERMANENTLY);
    assert_eq!(location.as_deref(), Some(&*format!("/v1/emotes/{target}")));
    let (status, _, body) = w.rest(&format!("/v1/emotes/{target}")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["name"], "mergeNew");
    assert!(body.get("merged_into").is_none(), "{body}");

    // GraphQL: the target's answer under the old ID, with `mergedInto`; signed out too.
    let e = w.gql(None, EMOTE, json!({ "id": source })).await["data"]["emote"].clone();
    assert_eq!(
        e,
        json!({ "id": source, "name": "mergeNew", "mergedInto": target })
    );
    let live = w.gql(None, EMOTE, json!({ "id": target })).await["data"]["emote"].clone();
    assert_eq!(live["mergedInto"], Value::Null);
    let both = w
        .gql(None, EMOTES, json!({ "ids": [source, target] }))
        .await["data"]["emotes"]
        .clone();
    assert_eq!(both[0]["mergedInto"], target.as_str());
    assert_eq!(both[1]["id"], target.as_str());

    // Gone from search; can't be added anywhere (as any deleted emote: emote_unavailable).
    let found = w.ok(&alice, SEARCH, json!({ "q": "mergeOld" })).await;
    assert!(
        !found["searchEmotes"]["items"]
            .as_array()
            .unwrap()
            .iter()
            .any(|i| i["id"] == source.as_str())
    );
    let set = w.set_of(&alice, "Chan").await;
    let added = w
        .gql(
            Some(&alice),
            ADD,
            json!({ "s": set, "e": source, "i": null }),
        )
        .await;
    assert_eq!(code(&added), "emote_unavailable", "{added}");
    w.db.close().await;
}

#[tokio::test]
async fn only_emotes_merge_may_merge_and_admin_has_it() {
    let Some(w) = World::new().await else {
        return;
    };
    let alice = w.user("1", "alice", &[]).await;
    let reviewer = w
        .user("2", "rev", &["emotes.review", "emotes.edit_any"])
        .await;
    let source = w.approved(&alice, "mine").await;
    let target = w.approved(&alice, "mine2").await;

    // Not the owner, not other staff.
    for who in [&alice, &reviewer] {
        let r = w.merge(who, &source, &target).await;
        assert_eq!(code(&r), "forbidden", "{r}");
        let r = w
            .gql(Some(who), PREVIEW, json!({ "id": source, "into": target }))
            .await;
        assert_eq!(code(&r), "forbidden", "{r}");
    }
    let signed_out = w
        .gql(None, MERGE, json!({ "id": source, "into": target }))
        .await;
    assert!(!signed_out["errors"].is_null());

    // The built-in Admin role holds emotes.merge (the migration gives it).
    let mut conn = w.db.conn().await;
    let held: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM role_permissions p JOIN roles r ON r.id = p.role_id
                        WHERE r.system = 'admin' AND p.permission = 'emotes.merge')",
    )
    .fetch_one(&mut *conn)
    .await
    .unwrap();
    assert!(held);
    drop(conn);
    w.db.close().await;
}

#[tokio::test]
async fn chains_are_followed_and_kept_one_step_long() {
    let Some(w) = World::new().await else {
        return;
    };
    let alice = w.user("1", "alice", &[]).await;
    let staff = w.user("2", "staff", MERGER).await;
    let a = w.approved(&alice, "chainA").await;
    let b = w.approved(&alice, "chainB").await;
    let c = w.approved(&alice, "chainC").await;
    let d = w.approved(&alice, "chainD").await;

    // A into B, then C into A: A was merged, so C goes to B.
    assert!(w.merge(&staff, &a, &b).await["errors"].is_null());
    let p = w.ok(&staff, PREVIEW, json!({ "id": c, "into": a })).await["emoteMergePreview"].clone();
    assert_eq!(p["followed"], true);
    assert_eq!(p["target"]["id"], b.as_str());
    let r = w.merge(&staff, &c, &a).await;
    assert_eq!(r["data"]["mergeEmote"]["target"]["id"], b.as_str(), "{r}");

    // B into D: A and C now answer D.
    assert!(w.merge(&staff, &b, &d).await["errors"].is_null());
    for old in [&a, &b, &c] {
        let e = w.gql(None, EMOTE, json!({ "id": old })).await["data"]["emote"].clone();
        assert_eq!(e["mergedInto"], d.as_str(), "{old}: {e}");
        assert_eq!(e["name"], "chainD");
    }
    let mut conn = w.db.conn().await;
    let longest: Option<i64> = sqlx::query_scalar(
        "SELECT count(*) FROM emotes e JOIN emotes t ON t.id = e.merged_into
         WHERE t.merged_into IS NOT NULL",
    )
    .fetch_one(&mut *conn)
    .await
    .unwrap();
    assert_eq!(longest, Some(0));
    drop(conn);
    w.db.close().await;
}

#[tokio::test]
async fn self_unavailable_and_repeated_merges_are_refused() {
    let Some(w) = World::new().await else {
        return;
    };
    let alice = w.user("1", "alice", &[]).await;
    let staff = w.user("2", "staff", MERGER).await;
    let x = w.approved(&alice, "refX").await;
    let y = w.approved(&alice, "refY").await;
    let z = w.approved(&alice, "refZ").await;
    let pending = w.emote_in(&alice, "refPending", EmoteState::Pending).await;
    let rejected = w
        .emote_in(&alice, "refRejected", EmoteState::Rejected)
        .await;
    let down = w.emote_in(&alice, "refDown", EmoteState::TakenDown).await;

    assert_eq!(code(&w.merge(&staff, &x, &x).await), "merge_into_self");
    for into in [&pending, &rejected, &down] {
        assert_eq!(
            code(&w.merge(&staff, &x, into).await),
            "merge_target_unavailable",
            "{into}"
        );
    }
    // A deleted target.
    let mut conn = w.db.conn().await;
    emotes::delete(&mut conn, w.db.row(&z).await).await.unwrap();
    drop(conn);
    assert_eq!(
        code(&w.merge(&staff, &x, &z).await),
        "merge_target_unavailable"
    );
    // A pending source may go into an approved target.
    let other = w.emote_in(&alice, "refPending2", EmoteState::Pending).await;
    assert!(w.merge(&staff, &other, &y).await["errors"].is_null());

    // X into Y; then Y into X follows X to Y itself; X again is merged already.
    assert!(w.merge(&staff, &x, &y).await["errors"].is_null());
    assert_eq!(code(&w.merge(&staff, &y, &x).await), "merge_into_self");
    let w2 = w.approved(&alice, "refW").await;
    assert_eq!(code(&w.merge(&staff, &x, &w2).await), "already_merged");
    // Nothing named that doesn't exist.
    assert_eq!(code(&w.merge(&staff, "123", &y).await), "not_found");
    w.db.close().await;
}
