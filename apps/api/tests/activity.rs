//! The public account activity feed (ADR 0016): kinds, order, paging, the limit clamp and the
//! privacy filters, through the real router against a real Postgres. Runs when
//! `CYEX_TV_TEST_DATABASE_URL` is set (CI sets it) and skips otherwise.

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
use cyex_tv_db::{audit, channels, ids, roles, sets};
use cyex_tv_shared::model::{EmoteState, Platform};
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

const ACTIVITY: &str = "query($a: String!, $b: String, $l: Int) {
    accountActivity(accountId: $a, before: $b, limit: $l) {
        id kind actor { id } set { id name } emote { id name host files { name } }
        fromName toName badge { fromId fromName toId toName } paint { fromId toId } createdAt
    }
}";
const ADD_EMOTE: &str = "mutation($s: String!, $e: String!, $n: String) {
    addSetEmote(setId: $s, emoteId: $e, input: { name: $n }) { name }
}";
const RENAME_EMOTE: &str = "mutation($s: String!, $e: String!, $n: String!) {
    updateSetEmote(setId: $s, emoteId: $e, input: { name: $n }) { name }
}";
const REMOVE_EMOTE: &str =
    "mutation($s: String!, $e: String!) { removeSetEmote(setId: $s, emoteId: $e) }";
const CREATE_SET: &str = "mutation($n: String!) {
    createSet(input: { name: $n, kind: CHANNEL }) { id }
}";
const RENAME_SET: &str =
    "mutation($id: String!, $n: String!) { updateSet(id: $id, input: { name: $n }) { name } }";
const CHANNEL: &str = "mutation($c: String!, $s: String) {
    updateChannelSettings(platform: TWITCH, id: $c, input: { activeSetId: $s }) { activeSetId }
}";
const ADD_EDITOR: &str = "mutation($a: String!, $f: [EditorPermission!]!) {
    addAccountEditor(input: { accountId: $a, permissions: $f }) { permissions }
}";
const APPROVE: &str = "mutation($id: String!) { approveEmote(id: $id) { id } }";

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

    /// The feed of `account`, read signed out.
    async fn feed(&self, account: &str, before: Option<&str>, limit: Option<u32>) -> Vec<Value> {
        let r = self
            .gql(
                None,
                ACTIVITY,
                json!({ "a": account, "b": before, "l": limit }),
            )
            .await;
        r["data"]["accountActivity"]
            .as_array()
            .unwrap_or_else(|| panic!("{r}"))
            .clone()
    }

    async fn kinds(&self, account: &str) -> Vec<String> {
        self.feed(account, None, Some(100))
            .await
            .iter()
            .map(|e| e["kind"].as_str().unwrap().to_owned())
            .collect()
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
}

#[tokio::test]
async fn kinds_appear_newest_first_with_their_details() {
    let Some(w) = World::new().await else { return };
    let alice = w.user("2001").await;
    let reviewer = w.user("2002").await;
    w.grant(&reviewer, "emotes.review").await;
    let ok = w
        .emote(&alice, "aliceOk", EmoteState::Approved, false)
        .await;
    let pending = w
        .emote(&alice, "alicePending", EmoteState::Pending, false)
        .await;

    let set = w.set(&alice, "Main").await;
    assert!(w.add(&alice, &set, &ok, Some("okAlias")).await["errors"].is_null());
    let r = w
        .gql(
            Some(&alice),
            RENAME_EMOTE,
            json!({ "s": set, "e": ok, "n": "okRenamed" }),
        )
        .await;
    assert!(r["errors"].is_null(), "{r}");
    let r = w
        .gql(
            Some(&alice),
            RENAME_SET,
            json!({ "id": set, "n": "Primary" }),
        )
        .await;
    assert!(r["errors"].is_null(), "{r}");
    let r = w
        .gql(
            Some(&alice),
            CHANNEL,
            json!({ "c": alice.twitch_id, "s": set }),
        )
        .await;
    assert!(r["errors"].is_null(), "{r}");
    let r = w
        .gql(Some(&alice), REMOVE_EMOTE, json!({ "s": set, "e": ok }))
        .await;
    assert_eq!(r["data"]["removeSetEmote"], true, "{r}");
    w.pick(&alice, &alice, false).await;
    // Approved by staff: it shows as the owner's upload, without the reviewer.
    let r = w
        .gql(Some(&reviewer), APPROVE, json!({ "id": pending }))
        .await;
    assert!(r["errors"].is_null(), "{r}");

    let feed = w.feed(&alice.tid(), None, None).await;
    let kinds: Vec<&str> = feed.iter().map(|e| e["kind"].as_str().unwrap()).collect();
    assert_eq!(
        kinds,
        [
            "EMOTE_UPLOADED",
            "COSMETICS_SELECTED",
            "SET_EMOTE_REMOVED",
            "ACTIVE_SET_CHANGED",
            "SET_RENAMED",
            "SET_EMOTE_RENAMED",
            "SET_EMOTE_ADDED",
            "SET_CREATED",
        ]
    );
    // Everything is the owner's own doing; the reviewer isn't named.
    let alice_public = w.db.public(alice.id).await;
    assert!(
        feed.iter()
            .all(|e| e["actor"]["id"] == alice_public.as_str())
    );

    assert_eq!(feed[0]["emote"]["name"], "alicePending");
    assert_eq!(feed[1]["badge"]["fromId"], Value::Null);
    assert!(feed[1]["badge"]["toId"].is_string());
    assert_eq!(feed[1]["paint"], Value::Null);
    assert_eq!(feed[2]["emote"]["name"], "okRenamed");
    assert_eq!(feed[2]["set"]["name"], "Primary");
    assert_eq!(feed[3]["set"]["id"], set.as_str());
    assert_eq!(feed[3]["toName"], "Primary");
    assert_eq!(feed[3]["fromName"], Value::Null);
    assert_eq!(feed[4]["fromName"], "Main");
    assert_eq!(feed[4]["toName"], "Primary");
    assert_eq!(feed[5]["fromName"], "okAlias");
    assert_eq!(feed[5]["toName"], "okRenamed");
    assert_eq!(feed[6]["emote"]["name"], "okAlias");
    assert!(feed[6]["emote"]["files"].is_array());
    assert!(feed[6]["createdAt"].as_str().unwrap().ends_with('Z'));
    w.db.close().await;
}

#[tokio::test]
async fn paging_and_the_limit_clamp() {
    let Some(w) = World::new().await else { return };
    let alice = w.user("2101").await;
    let set = w.set(&alice, "Main").await;
    for i in 0..5 {
        let emote = w
            .emote(&alice, &format!("emote{i}"), EmoteState::Approved, false)
            .await;
        assert!(w.add(&alice, &set, &emote, None).await["errors"].is_null());
    }
    let all = w.feed(&alice.tid(), None, Some(100)).await;
    assert_eq!(all.len(), 6);

    let mut seen: Vec<String> = Vec::new();
    let mut before: Option<String> = None;
    loop {
        let page = w.feed(&alice.tid(), before.as_deref(), Some(2)).await;
        assert!(page.len() <= 2);
        if page.is_empty() {
            break;
        }
        before = page.last().map(|e| e["id"].as_str().unwrap().to_owned());
        seen.extend(page.iter().map(|e| e["id"].as_str().unwrap().to_owned()));
    }
    let expected: Vec<String> = all
        .iter()
        .map(|e| e["id"].as_str().unwrap().to_owned())
        .collect();
    assert_eq!(seen, expected);

    // 0 is raised to 1.
    assert_eq!(w.feed(&alice.tid(), None, Some(0)).await.len(), 1);
    // A bad cursor is an error, not a silent first page.
    let r = w
        .gql(
            None,
            ACTIVITY,
            json!({ "a": alice.tid(), "b": "nope", "l": null }),
        )
        .await;
    assert!(!r["errors"].is_null(), "{r}");

    // 105 more lines: the default page is 30 and the most one page gives is 100.
    for _ in 0..105 {
        w.pick(&alice, &alice, false).await;
    }
    assert_eq!(w.feed(&alice.tid(), None, None).await.len(), 30);
    assert_eq!(w.feed(&alice.tid(), None, Some(1000)).await.len(), 100);
    w.db.close().await;
}

#[tokio::test]
async fn private_and_moderation_things_stay_out() {
    let Some(w) = World::new().await else { return };
    let alice = w.user("2201").await;
    let bob = w.user("2202").await;
    let moderator = w.user("2203").await;
    w.grant(&moderator, "sets.edit_any").await;
    w.grant(&moderator, "emotes.review").await;
    let set = w.set(&alice, "Main").await;
    assert_eq!(w.kinds(&alice.tid()).await, ["SET_CREATED"]);

    // A pending emote's add is hidden until it is approved, then it shows (and the upload).
    let pending = w.emote(&alice, "pend", EmoteState::Pending, false).await;
    assert!(w.add(&alice, &set, &pending, None).await["errors"].is_null());
    assert_eq!(w.kinds(&alice.tid()).await, ["SET_CREATED"]);

    // Private emotes never show.
    let private = w.emote(&alice, "priv", EmoteState::Approved, true).await;
    assert!(w.add(&alice, &set, &private, None).await["errors"].is_null());
    assert_eq!(w.kinds(&alice.tid()).await, ["SET_CREATED"]);

    // Staff acting on their own permissions: hidden, whether a set change or a review.
    let ok = w.emote(&alice, "ok", EmoteState::Approved, false).await;
    assert!(w.add(&moderator, &set, &ok, None).await["errors"].is_null());
    let rejected = w.emote(&alice, "bad", EmoteState::Pending, false).await;
    let r = w
        .gql(
            Some(&moderator),
            "mutation($id: String!) { rejectEmote(id: $id, reason: \"Blurry\") { id } }",
            json!({ "id": rejected }),
        )
        .await;
    assert!(r["errors"].is_null(), "{r}");
    assert_eq!(w.kinds(&alice.tid()).await, ["SET_CREATED"]);
    let raw = serde_json::to_string(&w.feed(&alice.tid(), None, None).await).unwrap();
    assert!(!raw.contains("Blurry"));

    // A per-set editor isn't an account editor: not on the feed either.
    {
        let mut conn = w.db.conn().await;
        sets::add_editor(&mut conn, w.db.row(&set).await, bob.id, Some(alice.id))
            .await
            .unwrap();
    }
    let ok2 = w.emote(&alice, "ok2", EmoteState::Approved, false).await;
    assert!(w.add(&bob, &set, &ok2, None).await["errors"].is_null());
    assert_eq!(w.kinds(&alice.tid()).await, ["SET_CREATED"]);

    // Approving the pending emote makes its add and its upload public.
    let r = w
        .gql(Some(&moderator), APPROVE, json!({ "id": pending }))
        .await;
    assert!(r["errors"].is_null(), "{r}");
    assert_eq!(
        w.kinds(&alice.tid()).await,
        ["EMOTE_UPLOADED", "SET_EMOTE_ADDED", "SET_CREATED"]
    );
    // A takedown hides everything about the emote.
    {
        let mut conn = w.db.conn().await;
        emotes::set_state(&mut conn, w.db.row(&pending).await, EmoteState::TakenDown)
            .await
            .unwrap();
    }
    assert_eq!(w.kinds(&alice.tid()).await, ["SET_CREATED"]);

    // A deleted set takes its lines with it.
    let gone = w.set(&alice, "Gone").await;
    assert_eq!(w.kinds(&alice.tid()).await, ["SET_CREATED", "SET_CREATED"]);
    let r = w
        .gql(
            Some(&alice),
            "mutation($id: String!) { deleteSet(id: $id) }",
            json!({ "id": gone }),
        )
        .await;
    assert_eq!(r["data"]["deleteSet"], true, "{r}");
    assert_eq!(w.kinds(&alice.tid()).await, ["SET_CREATED"]);

    // Unknown and malformed accounts have no feed.
    assert!(
        w.feed(&ids::to_text(ids::new_id()), None, None)
            .await
            .is_empty()
    );
    assert!(w.feed("nope", None, None).await.is_empty());

    // A blocked owner's feed is empty, and comes back when the block is lifted.
    w.block(&alice, true).await;
    assert!(w.feed(&alice.tid(), None, None).await.is_empty());
    w.block(&alice, false).await;
    assert_eq!(w.kinds(&alice.tid()).await, ["SET_CREATED"]);
    w.db.close().await;
}

#[tokio::test]
async fn editor_actions_are_attributed_to_the_editor() {
    let Some(w) = World::new().await else { return };
    let alice = w.user("2301").await;
    let bob = w.user("2302").await;
    let set = w.set(&alice, "Main").await;
    let r = w
        .gql(
            Some(&alice),
            ADD_EDITOR,
            json!({ "a": bob.tid(), "f": ["EMOTES", "PROFILE"] }),
        )
        .await;
    assert!(r["errors"].is_null(), "{r}");
    let ok = w.emote(&alice, "ok", EmoteState::Approved, false).await;
    assert!(w.add(&bob, &set, &ok, None).await["errors"].is_null());
    w.pick(&bob, &alice, true).await;

    let feed = w.feed(&alice.tid(), None, None).await;
    let kinds: Vec<&str> = feed.iter().map(|e| e["kind"].as_str().unwrap()).collect();
    assert_eq!(
        kinds,
        ["COSMETICS_SELECTED", "SET_EMOTE_ADDED", "SET_CREATED"]
    );
    assert_eq!(feed[0]["actor"]["id"], w.db.public(bob.id).await.as_str());
    assert_eq!(feed[1]["actor"]["id"], w.db.public(bob.id).await.as_str());
    assert_eq!(feed[2]["actor"]["id"], w.db.public(alice.id).await.as_str());
    // It is on Alice's feed, not on the editor's own.
    assert!(w.feed(&bob.tid(), None, None).await.is_empty());

    // A blocked editor's lines go away with them.
    w.block(&bob, true).await;
    assert_eq!(w.kinds(&alice.tid()).await, ["SET_CREATED"]);
    w.db.close().await;
}

#[tokio::test]
async fn the_feed_is_public_and_needs_no_session() {
    let Some(w) = World::new().await else { return };
    let alice = w.user("2401").await;
    let _ = w.set(&alice, "Main").await;
    // `feed` reads without a session; a signed-in read gives the same answer.
    let public = w.feed(&alice.tid(), None, None).await;
    let r = w
        .gql(
            Some(&alice),
            ACTIVITY,
            json!({ "a": alice.tid(), "b": null, "l": null }),
        )
        .await;
    assert_eq!(public.len(), 1);
    assert_eq!(r["data"]["accountActivity"], json!(public));
    w.db.close().await;
}
