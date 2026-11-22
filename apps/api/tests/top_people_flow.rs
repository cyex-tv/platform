//! "Most important people" for the person filters (`activityTopActors`, `auditTopPeople`,
//! `orderTopBuyers`, `reviewTopUploaders`, `entitlementTopHolders`) and the filters they feed
//! (`emoteActivityPage(actorId:)`, `reviewQueue(filter: { uploaderId })`, `auditLog(targetId:)`
//! by Snowflake): counts, order, and that each is visible exactly as its list is. Through the
//! real router against a real Postgres; runs when `CYEX_TV_TEST_DATABASE_URL` is set (CI sets
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
use cyex_tv_db::{audit, channels, ids, roles};
use cyex_tv_shared::model::{EmoteState, Platform};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use tower::ServiceExt;
use uuid::Uuid;

struct User {
    id: Uuid,
    token: String,
}

impl User {
    fn tid(&self) -> String {
        ids::to_text(self.id)
    }
}

struct World {
    db: TestDb,
    app: Router,
}

const ADD_EMOTE: &str = "mutation($s: String!, $e: String!, $n: String) {
    addSetEmote(setId: $s, emoteId: $e, input: { name: $n }) { name }
}";
const RENAME_EMOTE: &str = "mutation($s: String!, $e: String!, $n: String!) {
    updateSetEmote(setId: $s, emoteId: $e, input: { name: $n }) { name }
}";
const CREATE_SET: &str = "mutation($n: String!) {
    createSet(input: { name: $n, kind: CHANNEL }) { id }
}";
const ADD_EDITOR: &str = "mutation($a: String!, $f: [EditorPermission!]!) {
    addAccountEditor(input: { accountId: $a, permissions: $f }) { permissions }
}";
const APPROVE: &str = "mutation($id: String!) { approveEmote(id: $id) { id } }";
const ACTIVITY_TOP: &str = "query($a: String, $e: String, $f: ActivityFilter, $l: Int) {
    activityTopActors(accountId: $a, emoteId: $e, filter: $f, limit: $l) {
        account { id displayName } identities { platform login } count
    }
}";
const EMOTE_FEED: &str = "query($e: String!, $by: String) {
    emoteActivityPage(emoteId: $e, actorId: $by) { entries { kind actor { id } } next }
}";
const AUDIT_TOP: &str = "query($by: AuditPersonRole, $k: String, $a: String) {
    auditTopPeople(by: $by, targetKind: $k, action: $a) { account { id } count }
}";
const AUDIT: &str = "query($k: String, $t: String) {
    auditLog(targetKind: $k, targetId: $t) { action targetId }
}";
const ORDERS_TOP: &str = "query { orderTopBuyers { account { id } count } }";
const QUEUE_TOP: &str = "query($f: ReviewQueueFilter) {
    reviewTopUploaders(filter: $f) { account { id } count }
}";
const QUEUE: &str =
    "query($f: ReviewQueueFilter) { reviewQueue(filter: $f) { items { name } total } }";
const HOLDERS_TOP: &str =
    "query($l: Int) { entitlementTopHolders(limit: $l) { account { id } count } }";

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
        self.db.public(emote.id).await
    }

    async fn grant(&self, user: &User, permission: &str) {
        let mut conn = self.db.conn().await;
        let role = roles::create(
            &mut conn,
            &roles::NewRole {
                name: &format!("{permission} holders {}", user.tid()),
                description: None,
                position: 10,
                permissions: &[permission],
                created_by: None,
            },
        )
        .await
        .unwrap();
        roles::assign(&mut conn, user.id, role.id, None)
            .await
            .unwrap();
    }

    /// A GraphQL call; without a user it is a signed-out one.
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

    async fn set(&self, owner: &User, name: &str) -> String {
        let r = self
            .gql(Some(owner), CREATE_SET, json!({ "n": name }))
            .await;
        r["data"]["createSet"]["id"]
            .as_str()
            .unwrap_or_else(|| panic!("{r}"))
            .to_owned()
    }

    async fn add(&self, by: &User, set: &str, emote: &str, alias: Option<&str>) -> Value {
        self.gql(
            Some(by),
            ADD_EMOTE,
            json!({ "s": set, "e": emote, "n": alias }),
        )
        .await
    }

    /// A badge pick for the feed, written as the pick path writes it.
    async fn pick(&self, actor: &User, account: &User, via_editor: bool) {
        let mut after = json!({ "badge_id": ids::to_text(ids::new_id()), "paint_id": null });
        if via_editor {
            after["via"] = json!("account_editor");
        }
        let mut conn = self.db.conn().await;
        audit::write_for(
            &mut conn,
            &audit::NewEntry {
                actor: Some(actor.id),
                action: "cosmetics.selected",
                target_kind: "account",
                target_id: &account.tid(),
                before: Some(json!({ "badge_id": null, "paint_id": null })),
                after: Some(after),
            },
            Some(account.id),
        )
        .await
        .unwrap();
    }

    async fn block(&self, user: &User, blocked: bool) {
        let mut conn = self.db.conn().await;
        accounts::set_blocked(&mut conn, user.id, blocked)
            .await
            .unwrap();
    }

    async fn ok(&self, user: Option<&User>, query: &str, variables: Value) -> Value {
        let r = self.gql(user, query, variables).await;
        assert!(r["errors"].is_null(), "{r}");
        r
    }

    /// The (public ID, count) pairs of a top list.
    async fn top(
        &self,
        user: Option<&User>,
        query: &str,
        field: &str,
        variables: Value,
    ) -> Vec<(String, u64)> {
        let r = self.ok(user, query, variables).await;
        r["data"][field]
            .as_array()
            .unwrap_or_else(|| panic!("{r}"))
            .iter()
            .map(|p| {
                (
                    p["account"]["id"].as_str().unwrap().to_owned(),
                    p["count"].as_u64().unwrap(),
                )
            })
            .collect()
    }

    async fn pub_id(&self, user: &User) -> String {
        self.db.public(user.id).await
    }
}

fn code(r: &Value) -> &str {
    r["errors"][0]["extensions"]["code"].as_str().unwrap_or("")
}

#[tokio::test]
async fn an_accounts_top_actors_follow_its_feed() {
    let Some(w) = World::new().await else { return };
    let alice = w.user("7101").await;
    let bob = w.user("7102").await;
    let set = w.set(&alice, "Main").await;
    w.ok(
        Some(&alice),
        ADD_EDITOR,
        json!({ "a": bob.tid(), "f": ["EMOTES", "PROFILE"] }),
    )
    .await;
    for name in ["one", "two"] {
        let e = w.emote(&alice, name, EmoteState::Approved, false).await;
        assert!(w.add(&bob, &set, &e, None).await["errors"].is_null());
    }
    w.pick(&bob, &alice, true).await;
    let (a, b) = (w.pub_id(&alice).await, w.pub_id(&bob).await);
    let top = |v: Value| w.top(None, ACTIVITY_TOP, "activityTopActors", v);

    // Signed out, as the feed is: Bob made three lines, Alice one (the set).
    assert_eq!(
        top(json!({ "a": alice.tid() })).await,
        [(b.clone(), 3), (a.clone(), 1)]
    );
    // The logins come along for the suggestion's hint, and the limit holds.
    let r = w
        .ok(None, ACTIVITY_TOP, json!({ "a": alice.tid(), "l": 1 }))
        .await;
    let list = r["data"]["activityTopActors"].as_array().unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0]["identities"][0]["login"], "user7102");
    // The feed's groups apply.
    assert_eq!(
        top(json!({ "a": alice.tid(), "f": { "groups": ["SETS"] } })).await,
        [(a.clone(), 1)]
    );
    // A blocked editor leaves with their lines; a blocked owner has no feed at all.
    w.block(&bob, true).await;
    assert_eq!(top(json!({ "a": alice.tid() })).await, [(a.clone(), 1)]);
    w.block(&alice, true).await;
    assert!(top(json!({ "a": alice.tid() })).await.is_empty());
    // Exactly one scope.
    assert_eq!(
        code(&w.gql(None, ACTIVITY_TOP, json!({})).await),
        "bad_scope"
    );
    assert_eq!(
        code(&w.gql(None, ACTIVITY_TOP, json!({ "a": a, "e": a })).await),
        "bad_scope"
    );
    w.db.close().await;
}

#[tokio::test]
async fn an_emotes_top_actors_and_its_by_filter() {
    let Some(w) = World::new().await else { return };
    let alice = w.user("7201").await;
    let bob = w.user("7202").await;
    let staff = w.user("7203").await;
    w.grant(&staff, "emotes.review").await;
    let emote = w.emote(&alice, "wave", EmoteState::Pending, false).await;
    let top = |v: Value| w.top(None, ACTIVITY_TOP, "activityTopActors", v);
    // Pending: no feed, nobody to offer.
    assert!(top(json!({ "e": emote })).await.is_empty());
    w.ok(Some(&staff), APPROVE, json!({ "id": emote })).await;
    let set = w.set(&bob, "Bob main").await;
    assert!(w.add(&bob, &set, &emote, None).await["errors"].is_null());
    w.ok(
        Some(&bob),
        RENAME_EMOTE,
        json!({ "s": set, "e": emote, "n": "waving" }),
    )
    .await;
    let (a, b) = (w.pub_id(&alice).await, w.pub_id(&bob).await);

    // Bob's add and rename; Alice's upload (the approval shows as hers, never the reviewer's).
    assert_eq!(
        top(json!({ "e": emote })).await,
        [(b.clone(), 2), (a.clone(), 1)]
    );

    // The feed narrows to one of them.
    let kinds = |r: &Value| -> Vec<String> {
        r["data"]["emoteActivityPage"]["entries"]
            .as_array()
            .unwrap()
            .iter()
            .map(|e| e["kind"].as_str().unwrap().to_owned())
            .collect()
    };
    let r = w.ok(None, EMOTE_FEED, json!({ "e": emote, "by": b })).await;
    assert_eq!(kinds(&r), ["SET_EMOTE_RENAMED", "SET_EMOTE_ADDED"]);
    let r = w.ok(None, EMOTE_FEED, json!({ "e": emote, "by": a })).await;
    assert_eq!(kinds(&r), ["EMOTE_UPLOADED"]);
    let s = w.pub_id(&staff).await;
    let r = w.ok(None, EMOTE_FEED, json!({ "e": emote, "by": s })).await;
    assert!(kinds(&r).is_empty());
    w.db.close().await;
}

#[tokio::test]
async fn the_audit_logs_top_people_need_audit_view() {
    let Some(w) = World::new().await else { return };
    let admin = w.user("7301").await;
    let nobody = w.user("7302").await;
    let target = w.user("7303").await;
    w.grant(&admin, "audit.view").await;
    {
        let mut conn = w.db.conn().await;
        for (actor, n) in [(&admin, 3), (&nobody, 1)] {
            for _ in 0..n {
                audit::write(
                    &mut conn,
                    &audit::NewEntry {
                        actor: Some(actor.id),
                        action: "account.restricted",
                        target_kind: "account",
                        target_id: &target.tid(),
                        before: None,
                        after: None,
                    },
                )
                .await
                .unwrap();
            }
        }
    }
    assert_eq!(
        code(&w.gql(Some(&nobody), AUDIT_TOP, json!({})).await),
        "forbidden"
    );
    assert!(!w.gql(None, AUDIT_TOP, json!({})).await["errors"].is_null());

    let top = |v: Value| w.top(Some(&admin), AUDIT_TOP, "auditTopPeople", v);
    assert_eq!(
        top(json!({ "a": "account.restricted" })).await,
        [(w.pub_id(&admin).await, 3), (w.pub_id(&nobody).await, 1)]
    );
    assert_eq!(
        top(json!({ "by": "TARGET", "a": "account.restricted" })).await,
        [(w.pub_id(&target).await, 4)]
    );

    // The log's target filter takes the account's Snowflake, as the people filter gives it.
    let r = w
        .ok(
            Some(&admin),
            AUDIT,
            json!({ "k": "account", "t": w.pub_id(&target).await }),
        )
        .await;
    assert_eq!(r["data"]["auditLog"].as_array().unwrap().len(), 4);
    w.db.close().await;
}

#[tokio::test]
async fn orders_queue_and_holders_match_their_lists_permissions() {
    let Some(w) = World::new().await else { return };
    let alice = w.user("7401").await;
    let bob = w.user("7402").await;
    let staff = w.user("7403").await;
    let nobody = w.user("7404").await;
    for p in ["shop.orders", "emotes.review", "entitlements.view"] {
        w.grant(&staff, p).await;
    }
    for q in [ORDERS_TOP, HOLDERS_TOP, QUEUE_TOP] {
        assert_eq!(code(&w.gql(Some(&nobody), q, json!({})).await), "forbidden");
    }
    assert!(
        w.top(Some(&staff), ORDERS_TOP, "orderTopBuyers", json!({}))
            .await
            .is_empty()
    );

    // The queue: Alice waits with two uploads, Bob with one.
    for (who, name) in [(&alice, "a1"), (&alice, "a2"), (&bob, "b1")] {
        w.emote(who, name, EmoteState::Pending, false).await;
    }
    let queue = |v: Value| w.top(Some(&staff), QUEUE_TOP, "reviewTopUploaders", v);
    assert_eq!(
        queue(json!({})).await,
        [(w.pub_id(&alice).await, 2), (w.pub_id(&bob).await, 1)]
    );
    assert_eq!(
        queue(json!({ "f": { "search": "b1" } })).await,
        [(w.pub_id(&bob).await, 1)]
    );
    let r = w
        .ok(
            Some(&staff),
            QUEUE,
            json!({ "f": { "uploaderId": w.pub_id(&alice).await } }),
        )
        .await;
    assert_eq!(r["data"]["reviewQueue"]["total"], 2);

    // Holders: the staff account holds three roles, Alice one.
    w.grant(&alice, "emotes.review").await;
    assert_eq!(
        w.top(
            Some(&staff),
            HOLDERS_TOP,
            "entitlementTopHolders",
            json!({ "l": 2 })
        )
        .await,
        [(w.pub_id(&staff).await, 3), (w.pub_id(&alice).await, 1)]
    );
    w.db.close().await;
}
