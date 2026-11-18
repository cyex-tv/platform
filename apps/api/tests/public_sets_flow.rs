//! Public emote sets: visibility (listed, unlisted, private), the public `set` read and
//! `GET /v1/sets/{id}` honouring it, `accountSets`, `cloneSet` (capacity, private and pending
//! emotes, IDOR, the rate limit, audit rows) and the `set.created` event. Through the real router
//! against a real Postgres; runs when `CYEX_TV_TEST_DATABASE_URL` is set (CI sets it) and skips
//! otherwise.

mod common;

use std::sync::Arc;

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use chrono::Duration;
use common::TestDb;
use cyex_tv_api::limits::{Limits, Rates};
use cyex_tv_api::state::AppState;
use cyex_tv_auth::{Auth, AuthConfig, tokens};
use cyex_tv_db::accounts::{self, Profile};
use cyex_tv_db::emotes::{self, NewEmote};
use cyex_tv_db::sessions::{self, SessionKind};
use cyex_tv_db::{audit, channels, editors, ids, outbox};
use cyex_tv_shared::model::{EmoteState, Platform};
use cyex_tv_shared::ratelimit::Rate;
use http_body_util::BodyExt;
use serde_json::{Value, json};
use tower::ServiceExt;
use uuid::Uuid;

struct User {
    id: Uuid,
    token: String,
    twitch_id: String,
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

fn code(response: &Value) -> &str {
    response["errors"][0]["extensions"]["code"]
        .as_str()
        .unwrap_or_default()
}

const CREATE: &str = "mutation($n: String!, $k: SetKind!, $v: SetVisibility, $o: String) {
    createSet(input: { name: $n, kind: $k, visibility: $v, ownerId: $o }) {
        id visibility listed capacity slotsUsed createdAt
    }
}";
const UPDATE: &str = "mutation($id: String!, $v: SetVisibility) {
    updateSet(id: $id, input: { visibility: $v }) { id visibility listed }
}";
const READ: &str =
    "query($id: String!) { set(id: $id) { id name visibility listed slotsUsed emotes { name } } }";
const ACCOUNT_SETS: &str = "query($a: String!, $p: Int) {
    accountSets(accountId: $a, preview: $p) {
        id name kind visibility listed capacity slotsUsed active createdAt
        owner { id } preview { name }
    }
}";
const CLONE: &str = "mutation($id: String!, $n: String, $o: String, $v: SetVisibility) {
    cloneSet(id: $id, name: $n, ownerId: $o, visibility: $v) {
        copied skipped set { id name kind visibility owner { id } capacity slotsUsed emotes { name zeroWidth } }
    }
}";
const ADD_EMOTE: &str = "mutation($s: String!, $e: String!, $n: String) {
    addSetEmote(setId: $s, emoteId: $e, input: { name: $n }) { name }
}";
const ADD_SET_EDITOR: &str =
    "mutation($s: String!, $a: String!) { addSetEditor(setId: $s, accountId: $a) }";
const RENAME: &str = "mutation($s: String!, $e: String!, $n: String) {
    updateSetEmote(setId: $s, emoteId: $e, input: { name: $n }) { name }
}";
const ACTIVE: &str = "mutation($c: String!, $s: String) {
    updateChannelSettings(platform: TWITCH, id: $c, input: { activeSetId: $s }) { activeSetId }
}";

impl World {
    async fn new() -> Option<Self> {
        Self::with_rates(Rates::default()).await
    }

    async fn with_rates(rates: Rates) -> Option<Self> {
        let db = TestDb::new().await?;
        let auth = Arc::new(Auth::new(AuthConfig::from_source(|_| None).unwrap()).unwrap());
        let state = AppState::new(db.pool.clone(), auth).with_limits(Limits::new(rates, false));
        Some(Self {
            app: cyex_tv_api::app(state),
            db,
        })
    }

    /// A Twitch user `user<id>` with a channel and an app session.
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
            SessionKind::App,
            Duration::days(1),
        )
        .await
        .unwrap();
        User {
            id: signed.account.id,
            token,
            twitch_id: twitch_id.to_owned(),
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

    async fn gql(&self, user: &User, query: &str, variables: Value) -> Value {
        self.post(Some(user), query, variables).await
    }

    async fn rest_status(&self, path: &str) -> StatusCode {
        let request = Request::get(path).body(Body::empty()).unwrap();
        self.app.clone().oneshot(request).await.unwrap().status()
    }

    async fn create(&self, owner: &User, name: &str, kind: &str, vis: Option<&str>) -> Value {
        self.gql(
            owner,
            CREATE,
            json!({ "n": name, "k": kind, "v": vis, "o": null }),
        )
        .await
    }

    async fn set(&self, owner: &User, name: &str, vis: Option<&str>) -> String {
        let r = self.create(owner, name, "CHANNEL", vis).await;
        r["data"]["createSet"]["id"]
            .as_str()
            .unwrap_or_else(|| panic!("{r}"))
            .to_owned()
    }

    async fn add(&self, by: &User, set: &str, emote: &str) {
        let r = self
            .gql(by, ADD_EMOTE, json!({ "s": set, "e": emote, "n": null }))
            .await;
        assert!(r["errors"].is_null(), "{r}");
    }

    async fn audit_rows(&self) -> Vec<audit::Entry> {
        let mut conn = self.db.conn().await;
        let mut rows = audit::list(&mut conn, &audit::Filter::default(), 100)
            .await
            .unwrap();
        rows.reverse();
        rows
    }

    async fn events(&self, event_type: &str) -> Vec<outbox::Event> {
        let mut conn = self.db.conn().await;
        sqlx::query_as(
            "SELECT id, type, topics, set_id, payload, created_at FROM events_outbox
             WHERE type = $1 ORDER BY id",
        )
        .bind(event_type)
        .fetch_all(&mut *conn)
        .await
        .unwrap()
    }
}

#[tokio::test]
async fn visibility_defaults_and_rules() {
    let Some(w) = World::new().await else { return };
    let alice = w.user("2001").await;

    let r = w.create(&alice, "Main", "CHANNEL", None).await;
    let set = &r["data"]["createSet"];
    assert_eq!(set["visibility"], "LISTED", "{r}");
    assert_eq!(set["listed"], true);
    assert_eq!(set["capacity"], 1000);
    assert_eq!(set["slotsUsed"], 0);
    assert!(set["createdAt"].as_str().is_some_and(|t| t.ends_with('Z')));

    let r = w.create(&alice, "Mine", "PERSONAL", None).await;
    assert_eq!(r["data"]["createSet"]["visibility"], "UNLISTED", "{r}");
    let personal = r["data"]["createSet"]["id"].as_str().unwrap().to_owned();

    // Personal sets are shown in chat to everyone: never private.
    let r = w
        .gql(&alice, UPDATE, json!({ "id": personal, "v": "PRIVATE" }))
        .await;
    assert_eq!(code(&r), "bad_visibility", "{r}");
    let r = w.create(&alice, "Hidden", "CHANNEL", Some("PRIVATE")).await;
    assert_eq!(r["data"]["createSet"]["visibility"], "PRIVATE", "{r}");
    assert_eq!(r["data"]["createSet"]["listed"], false);

    // The global set stays listed.
    let r = w
        .post(None, "{ globalSet { id visibility } }", json!({}))
        .await;
    assert_eq!(r["data"]["globalSet"]["visibility"], "LISTED", "{r}");
    let global = r["data"]["globalSet"]["id"].as_str().unwrap().to_owned();
    {
        let mut conn = w.db.conn().await;
        let role = cyex_tv_db::roles::create(
            &mut conn,
            &cyex_tv_db::roles::NewRole {
                name: "Staff",
                description: None,
                position: 10,
                permissions: &["global_set.edit"],
                created_by: None,
            },
        )
        .await
        .unwrap();
        cyex_tv_db::roles::assign(&mut conn, alice.id, role.id, None)
            .await
            .unwrap();
    }
    let r = w
        .gql(&alice, UPDATE, json!({ "id": global, "v": "UNLISTED" }))
        .await;
    assert_eq!(code(&r), "bad_visibility", "{r}");

    // slotsUsed counts what's in the set.
    let main = set["id"].as_str().unwrap().to_owned();
    let ok = w
        .emote(&alice, "aliceOk", EmoteState::Approved, false)
        .await;
    let pending = w
        .emote(&alice, "alicePending", EmoteState::Pending, false)
        .await;
    w.add(&alice, &main, &ok).await;
    w.add(&alice, &main, &pending).await;
    let stranger = w.user("2002").await;
    let r = w.gql(&stranger, READ, json!({ "id": main })).await;
    // A stranger sees one emote, but both slots are used.
    assert_eq!(
        r["data"]["set"]["emotes"].as_array().unwrap().len(),
        1,
        "{r}"
    );
    assert_eq!(r["data"]["set"]["slotsUsed"], 2);

    w.db.close().await;
}

#[tokio::test]
async fn private_sets_are_never_readable_by_others() {
    let Some(w) = World::new().await else { return };
    let alice = w.user("2101").await;
    let bob = w.user("2102").await;
    let carol = w.user("2103").await;
    let dave = w.user("2104").await;
    let hidden = w.set(&alice, "Secret", Some("PRIVATE")).await;
    let unlisted = w.set(&alice, "Link only", Some("UNLISTED")).await;

    // Strangers and signed-out callers: as if it didn't exist, over GraphQL and REST.
    let r = w.gql(&bob, READ, json!({ "id": hidden })).await;
    assert!(r["data"]["set"].is_null(), "{r}");
    let r = w.post(None, READ, json!({ "id": hidden })).await;
    assert!(r["data"]["set"].is_null(), "{r}");
    assert_eq!(
        w.rest_status(&format!("/v1/sets/{hidden}")).await,
        StatusCode::NOT_FOUND
    );
    // Unlisted opens by link.
    let r = w.post(None, READ, json!({ "id": unlisted })).await;
    assert_eq!(r["data"]["set"]["visibility"], "UNLISTED", "{r}");
    assert_eq!(
        w.rest_status(&format!("/v1/sets/{unlisted}")).await,
        StatusCode::OK
    );

    // The owner, a per-set editor and an account editor see it.
    let r = w.gql(&alice, READ, json!({ "id": hidden })).await;
    assert_eq!(r["data"]["set"]["name"], "Secret", "{r}");
    let r = w
        .gql(
            &alice,
            ADD_SET_EDITOR,
            json!({ "s": hidden, "a": carol.tid() }),
        )
        .await;
    assert_eq!(r["data"]["addSetEditor"], true, "{r}");
    let r = w.gql(&carol, READ, json!({ "id": hidden })).await;
    assert_eq!(r["data"]["set"]["name"], "Secret", "{r}");
    {
        let mut conn = w.db.conn().await;
        editors::add(
            &mut conn,
            alice.id,
            dave.id,
            &["profile".to_owned()],
            Some(alice.id),
        )
        .await
        .unwrap();
    }
    let r = w.gql(&dave, READ, json!({ "id": hidden })).await;
    assert_eq!(r["data"]["set"]["name"], "Secret", "{r}");
    // A per-set editor can't change who sees it.
    let r = w
        .gql(&carol, UPDATE, json!({ "id": hidden, "v": "LISTED" }))
        .await;
    assert_eq!(code(&r), "forbidden", "{r}");

    // Never in someone's public list, and not cloneable by others.
    let r = w
        .post(None, ACCOUNT_SETS, json!({ "a": alice.tid(), "p": 0 }))
        .await;
    let listed: Vec<&str> = r["data"]["accountSets"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["id"].as_str().unwrap())
        .collect();
    assert!(!listed.contains(&hidden.as_str()), "{r}");
    assert!(!listed.contains(&unlisted.as_str()), "{r}");
    let r = w.gql(&bob, CLONE, json!({ "id": hidden })).await;
    assert_eq!(code(&r), "not_found", "{r}");

    // While a channel shows it, it is public (chat shows it to everyone).
    let r = w
        .gql(&alice, ACTIVE, json!({ "c": alice.twitch_id, "s": hidden }))
        .await;
    assert_eq!(
        r["data"]["updateChannelSettings"]["activeSetId"],
        hidden.as_str(),
        "{r}"
    );
    assert_eq!(
        w.rest_status(&format!("/v1/sets/{hidden}")).await,
        StatusCode::OK
    );
    let r = w.gql(&bob, READ, json!({ "id": hidden })).await;
    assert_eq!(r["data"]["set"]["name"], "Secret", "{r}");
    // Off the channel, private again.
    let r = w
        .gql(&alice, ACTIVE, json!({ "c": alice.twitch_id, "s": "" }))
        .await;
    assert!(r["errors"].is_null(), "{r}");
    let r = w.gql(&bob, READ, json!({ "id": hidden })).await;
    assert!(r["data"]["set"].is_null(), "{r}");

    // No event for a private or unlisted set.
    assert!(w.events("set.created").await.is_empty());
    w.db.close().await;
}

#[tokio::test]
async fn account_sets_lists_listed_sets_of_live_accounts() {
    let Some(w) = World::new().await else { return };
    let alice = w.user("2201").await;
    let main = w.set(&alice, "Main", None).await;
    let _alt = w.set(&alice, "Alt", Some("UNLISTED")).await;
    let _hidden = w.set(&alice, "Secret", Some("PRIVATE")).await;
    let mut names = Vec::new();
    for n in 0..5 {
        let e = w
            .emote(&alice, &format!("emo{n}"), EmoteState::Approved, false)
            .await;
        w.add(&alice, &main, &e).await;
        names.push(format!("emo{n}"));
    }
    let r = w
        .gql(&alice, ACTIVE, json!({ "c": alice.twitch_id, "s": main }))
        .await;
    assert!(r["errors"].is_null(), "{r}");

    let r = w
        .post(None, ACCOUNT_SETS, json!({ "a": alice.tid(), "p": 3 }))
        .await;
    let cards = r["data"]["accountSets"].as_array().unwrap();
    assert_eq!(cards.len(), 1, "{r}");
    let card = &cards[0];
    assert_eq!(card["id"], main.as_str());
    assert_eq!(card["active"], true);
    assert_eq!(card["slotsUsed"], 5);
    assert_eq!(card["capacity"], 1000);
    assert_eq!(card["owner"]["id"], alice.tid().as_str());
    // The newest first.
    assert_eq!(
        card["preview"],
        json!([{ "name": "emo4" }, { "name": "emo3" }, { "name": "emo2" }])
    );

    // Unknown, malformed, blocked and deleted accounts list nothing.
    let r = w
        .post(None, ACCOUNT_SETS, json!({ "a": "nope", "p": 1 }))
        .await;
    assert_eq!(r["data"]["accountSets"], json!([]), "{r}");
    {
        let mut conn = w.db.conn().await;
        accounts::set_blocked(&mut conn, alice.id, true)
            .await
            .unwrap();
    }
    let r = w
        .post(None, ACCOUNT_SETS, json!({ "a": alice.tid(), "p": 1 }))
        .await;
    assert_eq!(r["data"]["accountSets"], json!([]), "{r}");
    let bob = w.user("2202").await;
    let _ = w.set(&bob, "Bob's", None).await;
    {
        let mut conn = w.db.conn().await;
        sqlx::query("UPDATE accounts SET deleted_at = now() WHERE id = $1")
            .bind(bob.id)
            .execute(&mut *conn)
            .await
            .unwrap();
    }
    let r = w
        .post(None, ACCOUNT_SETS, json!({ "a": bob.tid(), "p": 1 }))
        .await;
    assert_eq!(r["data"]["accountSets"], json!([]), "{r}");
    w.db.close().await;
}

#[tokio::test]
async fn clone_copies_what_may_be_copied() {
    let Some(w) = World::new().await else { return };
    let alice = w.user("2301").await;
    let bob = w.user("2302").await;
    let source = w.set(&alice, "Alice's", None).await;
    let ok = w
        .emote(&alice, "aliceOk", EmoteState::Approved, false)
        .await;
    let private = w
        .emote(&alice, "alicePriv", EmoteState::Approved, true)
        .await;
    let pending = w
        .emote(&alice, "alicePending", EmoteState::Pending, false)
        .await;
    let bobs_private = w.emote(&bob, "bobPriv", EmoteState::Approved, true).await;
    w.add(&alice, &source, &ok).await;
    w.add(&alice, &source, &private).await;
    w.add(&alice, &source, &pending).await;
    // An alias survives the copy.
    let r = w
        .gql(
            &alice,
            ADD_EMOTE,
            json!({ "s": source, "e": bobs_private, "n": null }),
        )
        .await;
    // Bob's private emote can't go in Alice's set at all.
    assert_eq!(code(&r), "emote_private", "{r}");
    {
        let mut conn = w.db.conn().await;
        sqlx::query("UPDATE emote_set_members SET name = 'okAlias' WHERE emote_id = $1")
            .bind(ids::from_text(&ok).unwrap())
            .execute(&mut *conn)
            .await
            .unwrap();
    }

    // Bob copies: only the approved, public emote, under its alias.
    let r = w.gql(&bob, CLONE, json!({ "id": source })).await;
    let out = &r["data"]["cloneSet"];
    assert_eq!(out["copied"], 1, "{r}");
    assert_eq!(out["skipped"], 2);
    assert_eq!(out["set"]["name"], "Alice's");
    assert_eq!(out["set"]["kind"], "CHANNEL");
    assert_eq!(out["set"]["visibility"], "LISTED");
    assert_eq!(out["set"]["owner"]["id"], bob.tid().as_str());
    assert_eq!(
        out["set"]["emotes"],
        json!([{ "name": "okAlias", "zeroWidth": false }])
    );
    let bobs_copy = out["set"]["id"].as_str().unwrap().to_owned();

    // Alice copies her own: her private and pending emotes come along.
    let r = w
        .gql(
            &alice,
            CLONE,
            json!({ "id": source, "n": "Mine again", "v": "UNLISTED" }),
        )
        .await;
    let out = &r["data"]["cloneSet"];
    assert_eq!(out["copied"], 3, "{r}");
    assert_eq!(out["skipped"], 0);
    assert_eq!(out["set"]["name"], "Mine again");
    assert_eq!(out["set"]["visibility"], "UNLISTED");

    // `set.created` for listed sets only, on the owner's user topics.
    let created = w.events("set.created").await;
    let bobs: Vec<_> = created
        .iter()
        .filter(|e| e.set_id == ids::from_text(&bobs_copy))
        .collect();
    assert_eq!(bobs.len(), 1);
    assert_eq!(
        bobs[0].topics,
        vec![outbox::user_topic(Platform::Twitch, &bob.twitch_id)]
    );
    assert!(
        created
            .iter()
            .all(|e| e.payload["set_id"] != r["data"]["cloneSet"]["set"]["id"])
    );

    // Bad input and IDOR: a set for an account you don't edit.
    let r = w
        .gql(&bob, CLONE, json!({ "id": source, "o": alice.tid() }))
        .await;
    assert_eq!(code(&r), "forbidden", "{r}");
    let r = w.gql(&bob, CLONE, json!({ "id": source, "n": "  " })).await;
    assert_eq!(code(&r), "bad_name", "{r}");
    let r = w.gql(&bob, CLONE, json!({ "id": "nope" })).await;
    assert_eq!(code(&r), "not_found", "{r}");
    let r = w.post(None, CLONE, json!({ "id": source })).await;
    assert_eq!(code(&r), "signed_out", "{r}");

    // An account editor with `sets` copies for the owner: audited, marked as through the grant.
    {
        let mut conn = w.db.conn().await;
        editors::add(
            &mut conn,
            alice.id,
            bob.id,
            &["sets".to_owned()],
            Some(alice.id),
        )
        .await
        .unwrap();
    }
    let r = w
        .gql(&bob, CLONE, json!({ "id": bobs_copy, "o": alice.tid() }))
        .await;
    assert_eq!(
        r["data"]["cloneSet"]["set"]["owner"]["id"],
        alice.tid().as_str(),
        "{r}"
    );
    let rows = w.audit_rows().await;
    let row = rows
        .iter()
        .find(|e| {
            e.action == "set.created"
                && e.after_json
                    .as_ref()
                    .is_some_and(|a| a["via"] == "account_editor")
        })
        .expect("audited");
    assert_eq!(row.actor_account_id, Some(bob.id));
    let after = row.after_json.as_ref().unwrap();
    assert_eq!(after["via"], "account_editor");
    assert_eq!(after["source_id"], bobs_copy.as_str());
    w.db.close().await;
}

#[tokio::test]
async fn clone_stops_at_capacity() {
    let Some(w) = World::new().await else { return };
    let alice = w.user("2401").await;
    let bob = w.user("2402").await;
    let source = w.set(&alice, "Big", None).await;
    // 1005 approved emotes in a set raised above the default (staff could do that).
    {
        let mut conn = w.db.conn().await;
        let set_id = ids::from_text(&source).unwrap();
        sqlx::query("UPDATE emote_sets SET capacity = 2000 WHERE id = $1")
            .bind(set_id)
            .execute(&mut *conn)
            .await
            .unwrap();
        sqlx::query(
            "WITH made AS (
                 INSERT INTO emotes (id, owner_account_id, name, state)
                 SELECT gen_random_uuid(), $1, 'big' || n, 'approved' FROM generate_series(1, 1005) n
                 RETURNING id, name
             )
             INSERT INTO emote_set_members (set_id, emote_id, name)
             SELECT $2, id, name FROM made",
        )
        .bind(alice.id)
        .bind(set_id)
        .execute(&mut *conn)
        .await
        .unwrap();
    }
    let r = w.gql(&bob, CLONE, json!({ "id": source })).await;
    let out = &r["data"]["cloneSet"];
    assert_eq!(out["copied"], 1000, "{}", r["errors"]);
    assert_eq!(out["skipped"], 5);
    assert_eq!(out["set"]["capacity"], 1000);
    assert_eq!(out["set"]["slotsUsed"], 1000);
    w.db.close().await;
}

#[tokio::test]
async fn clones_are_rate_limited() {
    let Some(w) = World::with_rates(Rates {
        set_clones: Rate::per_hour(2),
        ..Rates::default()
    })
    .await
    else {
        return;
    };
    let alice = w.user("2501").await;
    let source = w.set(&alice, "Main", None).await;
    for _ in 0..2 {
        let r = w.gql(&alice, CLONE, json!({ "id": source })).await;
        assert!(r["errors"].is_null(), "{r}");
    }
    let r = w.gql(&alice, CLONE, json!({ "id": source })).await;
    assert_eq!(code(&r), "rate_limited", "{r}");
    w.db.close().await;
}

#[tokio::test]
async fn activity_shows_listed_sets_only() {
    let Some(w) = World::new().await else { return };
    let alice = w.user("2601").await;
    let listed = w.set(&alice, "Out loud", None).await;
    let unlisted = w.set(&alice, "By link", Some("UNLISTED")).await;
    let hidden = w.set(&alice, "Quiet", Some("PRIVATE")).await;
    let e = w
        .emote(&alice, "aliceOk", EmoteState::Approved, false)
        .await;
    for set in [&listed, &unlisted, &hidden] {
        w.add(&alice, set, &e).await;
    }
    // A copy is a set made: it shows when listed.
    let r = w
        .gql(&alice, CLONE, json!({ "id": listed, "n": "Copy" }))
        .await;
    let copy = r["data"]["cloneSet"]["set"]["id"]
        .as_str()
        .unwrap_or_else(|| panic!("{r}"))
        .to_owned();

    const FEED: &str = "query($a: String!) { accountActivity(accountId: $a) { kind set { id } } }";
    let r = w.post(None, FEED, json!({ "a": alice.tid() })).await;
    let lines = r["data"]["accountActivity"]
        .as_array()
        .unwrap_or_else(|| panic!("{r}"));
    let sets: Vec<&str> = lines
        .iter()
        .filter_map(|l| l["set"]["id"].as_str())
        .collect();
    assert!(sets.contains(&listed.as_str()), "{r}");
    assert!(sets.contains(&copy.as_str()), "{r}");
    assert!(!sets.contains(&unlisted.as_str()), "{r}");
    assert!(!sets.contains(&hidden.as_str()), "{r}");
    assert!(!r.to_string().contains(&hidden), "{r}");

    // Hiding a set later hides its past lines too.
    let r = w
        .gql(&alice, UPDATE, json!({ "id": listed, "v": "PRIVATE" }))
        .await;
    assert_eq!(r["data"]["updateSet"]["visibility"], "PRIVATE", "{r}");
    let r = w.post(None, FEED, json!({ "a": alice.tid() })).await;
    assert!(!r.to_string().contains(&listed), "{r}");
    w.db.close().await;
}

/// Security review M-B: every topic is public, so a private set's member events must not be
/// published on `set:<id>`; listed and unlisted sets still are.
#[tokio::test]
async fn private_sets_publish_no_set_topic() {
    let Some(w) = World::new().await else { return };
    let alice = w.user("2701").await;
    let listed = w.set(&alice, "Out loud", None).await;
    let unlisted = w.set(&alice, "By link", Some("UNLISTED")).await;
    let hidden = w.set(&alice, "Quiet", Some("PRIVATE")).await;
    let shown = w.set(&alice, "Live", Some("PRIVATE")).await;
    let e = w
        .emote(&alice, "aliceOk", EmoteState::Approved, false)
        .await;
    for set in [&listed, &unlisted, &hidden, &shown] {
        w.add(&alice, set, &e).await;
    }
    let topics_of = |set: &str| {
        let set = set.to_owned();
        async {
            let mut conn = w.db.conn().await;
            let rows: Vec<Vec<String>> = sqlx::query_scalar(
                "SELECT topics FROM events_outbox WHERE payload->>'set_id' = $1 ORDER BY id",
            )
            .bind(set)
            .fetch_all(&mut *conn)
            .await
            .unwrap();
            rows
        }
    };
    let topic = |set: &str| format!("set:{set}");

    for set in [&listed, &unlisted] {
        let rows = topics_of(set).await;
        assert_eq!(rows.len(), 1);
        assert!(rows[0].contains(&topic(set)), "{rows:?}");
    }
    assert!(topics_of(&hidden).await.is_empty());
    assert!(topics_of(&shown).await.is_empty());

    // Renames and removals of a private set stay quiet too.
    let r = w
        .gql(
            &alice,
            RENAME,
            json!({ "s": hidden, "e": e, "n": "renamed" }),
        )
        .await;
    assert!(r["errors"].is_null(), "{r}");
    assert!(topics_of(&hidden).await.is_empty());

    // A private set a channel shows is public: its channel topic still gets the change.
    let r = w
        .gql(&alice, ACTIVE, json!({ "c": alice.twitch_id, "s": shown }))
        .await;
    assert!(r["errors"].is_null(), "{r}");
    let other = w
        .emote(&alice, "aliceTwo", EmoteState::Approved, false)
        .await;
    w.add(&alice, &shown, &other).await;
    let rows = topics_of(&shown).await;
    let channel = format!("channel:twitch:{}", alice.twitch_id);
    let last = rows.iter().rev().find(|t| t.contains(&channel));
    assert!(last.is_some(), "{rows:?}");
    assert!(
        rows.iter().flatten().all(|t| t != &topic(&shown)),
        "{rows:?}"
    );
    w.db.close().await;
}
