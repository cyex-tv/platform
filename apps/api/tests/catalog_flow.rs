//! Channels, sets, members, editors and the chat-client reads, through the real router against a
//! real Postgres. Accounts are made directly in the database and call with app (Bearer) sessions,
//! so these tests don't depend on the OAuth flow (`auth_flow.rs` covers that). They run when
//! `CYEX_TV_TEST_DATABASE_URL` is set (CI sets it) and skip otherwise.

mod common;

use std::sync::Arc;

use axum::Router;
use axum::body::Body;
use axum::http::{HeaderMap, Request, StatusCode};
use chrono::Duration;
use common::TestDb;
use cyex_tv_api::state::AppState;
use cyex_tv_auth::{Auth, AuthConfig, tokens};
use cyex_tv_db::accounts::{self, Profile};
use cyex_tv_db::emotes::{self, NewEmote};
use cyex_tv_db::sessions::{self, SessionKind};
use cyex_tv_db::{channels, ids, outbox, roles};
use cyex_tv_shared::model::{EmoteState, Platform};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use tower::ServiceExt;
use uuid::Uuid;

struct User {
    id: Uuid,
    token: String,
}

struct Reply {
    status: StatusCode,
    headers: HeaderMap,
    body: Value,
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
        let state = AppState::new(db.pool.clone(), auth).with_cdn_url("https://cdn.example.test/");
        Some(Self {
            app: cyex_tv_api::app(state),
            db,
        })
    }

    /// A Twitch user with a channel and an app session.
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

    async fn grant(&self, user: &User, permission: &str) {
        let mut conn = self.db.conn().await;
        let role = roles::create(
            &mut conn,
            &roles::NewRole {
                name: &format!("{permission} holders {}", ids::to_text(user.id)),
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

    async fn send(&self, request: Request<Body>) -> Reply {
        let response = self.app.clone().oneshot(request).await.unwrap();
        let status = response.status();
        let headers = response.headers().clone();
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        Reply {
            status,
            headers,
            body: serde_json::from_slice(&bytes).unwrap_or(Value::Null),
        }
    }

    async fn gql(&self, user: Option<&User>, query: &str, variables: Value) -> Value {
        let mut request = Request::post("/v1/gql").header("content-type", "application/json");
        if let Some(user) = user {
            request = request.header("authorization", format!("Bearer {}", user.token));
        }
        let reply = self
            .send(
                request
                    .body(Body::from(
                        json!({ "query": query, "variables": variables }).to_string(),
                    ))
                    .unwrap(),
            )
            .await;
        assert_eq!(reply.status, StatusCode::OK, "{:?}", reply.body);
        reply.body
    }

    async fn get(&self, uri: &str, if_none_match: Option<&str>) -> Reply {
        let mut request = Request::get(uri);
        if let Some(tag) = if_none_match {
            request = request.header("if-none-match", tag);
        }
        self.send(request.body(Body::empty()).unwrap()).await
    }

    async fn create_set(&self, user: &User, name: &str, kind: &str) -> String {
        let r = self
            .gql(
                Some(user),
                "mutation($n: String!, $k: SetKind!) { createSet(input: { name: $n, kind: $k }) { id kind capacity } }",
                json!({ "n": name, "k": kind }),
            )
            .await;
        r["data"]["createSet"]["id"]
            .as_str()
            .unwrap_or_else(|| panic!("{r}"))
            .to_owned()
    }

    async fn add(&self, user: &User, set: &str, emote: &str, alias: Option<&str>) -> Value {
        self.gql(
            Some(user),
            "mutation($s: String!, $e: String!, $n: String) { addSetEmote(setId: $s, emoteId: $e, input: { name: $n }) { name emote { id name state } } }",
            json!({ "s": set, "e": emote, "n": alias }),
        )
        .await
    }

    async fn events(&self) -> Vec<outbox::Event> {
        let mut conn = self.db.conn().await;
        sqlx::query_as(
            "SELECT id, type, topics, set_id, payload, created_at FROM events_outbox ORDER BY id",
        )
        .fetch_all(&mut *conn)
        .await
        .unwrap()
    }

    /// Audit actions oldest first, without the owners' own changes, which are audited for their
    /// public activity feed (they have a subject account; see tests/activity.rs).
    async fn audit_actions(&self) -> Vec<String> {
        let mut conn = self.db.conn().await;
        sqlx::query_scalar(
            "SELECT action FROM audit_log WHERE subject_account_id IS NULL ORDER BY id",
        )
        .fetch_all(&mut *conn)
        .await
        .unwrap()
    }
}

#[tokio::test]
async fn channel_sets_pending_emotes_and_public_reads() {
    let Some(w) = World::new().await else { return };
    let alice = w.user("101").await;

    let set = w.create_set(&alice, "  Main  ", "CHANNEL").await;
    let pending = w
        .emote(&alice, "alicePending", EmoteState::Pending, false)
        .await;
    let approved = w
        .emote(&alice, "aliceOk", EmoteState::Approved, false)
        .await;
    let r = w.add(&alice, &set, &pending, None).await;
    assert_eq!(r["data"]["addSetEmote"]["emote"]["state"], "PENDING", "{r}");
    let r = w.add(&alice, &set, &approved, Some("OK")).await;
    assert_eq!(r["data"]["addSetEmote"]["name"], "OK");

    // The owner sees both; the public read shows only the approved one.
    let mine = w
        .gql(
            Some(&alice),
            "{ mySets { id name capacity emotes { name } } }",
            json!({}),
        )
        .await;
    assert_eq!(mine["data"]["mySets"][0]["name"], "Main");
    assert_eq!(mine["data"]["mySets"][0]["capacity"], 1000);
    assert_eq!(
        mine["data"]["mySets"][0]["emotes"]
            .as_array()
            .unwrap()
            .len(),
        2
    );

    let public = w.get(&format!("/v1/sets/{set}"), None).await;
    assert_eq!(public.status, StatusCode::OK);
    let names: Vec<&str> = public.body["emotes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, ["OK"]);
    assert_eq!(public.body["owner"]["display_name"], "user101");
    assert_eq!(
        public.headers["cache-control"],
        "public, max-age=30, stale-while-revalidate=60"
    );

    // ETag round trip, then a change makes a new one.
    let etag = public.headers["etag"].to_str().unwrap().to_owned();
    let again = w.get(&format!("/v1/sets/{set}"), Some(&etag)).await;
    assert_eq!(again.status, StatusCode::NOT_MODIFIED);
    assert_eq!(again.headers["etag"].to_str().unwrap(), etag);
    let mut conn = w.db.conn().await;
    emotes::set_state(
        &mut conn,
        ids::from_text(&pending).unwrap(),
        EmoteState::Approved,
    )
    .await
    .unwrap();
    drop(conn);
    let changed = w.get(&format!("/v1/sets/{set}"), Some(&etag)).await;
    assert_eq!(changed.status, StatusCode::OK);
    assert_eq!(changed.body["emotes"].as_array().unwrap().len(), 2);

    // Unknown or malformed IDs are 404 with the error body.
    for uri in ["/v1/sets/not-an-id", "/v1/sets/01HZZZZZZZZZZZZZZZZZZZZZZZ"] {
        let r = w.get(uri, None).await;
        assert_eq!(r.status, StatusCode::NOT_FOUND, "{uri}");
        assert_eq!(r.body["error"], "not_found");
    }
    w.db.close().await;
}

#[tokio::test]
async fn active_set_and_channel_reads() {
    let Some(w) = World::new().await else { return };
    let alice = w.user("201").await;
    let bob = w.user("202").await;
    let set = w.create_set(&alice, "Main", "CHANNEL").await;
    let bobs = w.create_set(&bob, "Bob's", "CHANNEL").await;
    let personal = w.create_set(&alice, "Mine", "PERSONAL").await;

    let update = "mutation($id: String!, $s: String, $p: Boolean) { updateChannelSettings(platform: TWITCH, id: $id, input: { activeSetId: $s, allowPersonalEmotes: $p }) { activeSetId allowPersonalEmotes login } }";
    let r = w
        .gql(
            Some(&alice),
            update,
            json!({ "id": "201", "s": set, "p": false }),
        )
        .await;
    assert_eq!(
        r["data"]["updateChannelSettings"]["activeSetId"],
        set.as_str(),
        "{r}"
    );
    assert_eq!(
        r["data"]["updateChannelSettings"]["allowPersonalEmotes"],
        false
    );

    // Someone else's set, a personal set, or someone else's channel.
    for (who, id, s, expected) in [
        (&alice, "201", bobs.as_str(), "invalid_active_set"),
        (&alice, "201", personal.as_str(), "invalid_active_set"),
        (&bob, "201", bobs.as_str(), "forbidden"),
        (&alice, "999", set.as_str(), "not_found"),
    ] {
        let r = w.gql(Some(who), update, json!({ "id": id, "s": s })).await;
        assert_eq!(code(&r), expected, "{r}");
    }

    let channel = w.get("/v1/channels/twitch/201", None).await;
    assert_eq!(channel.status, StatusCode::OK);
    assert_eq!(channel.body["active_set_id"], set.as_str());
    assert_eq!(channel.body["login"], "user201");
    assert_eq!(channel.body["allow_personal_emotes"], false);
    assert!(channel.headers.contains_key("etag"));
    assert_eq!(
        w.get("/v1/channels/kick/201", None).await.status,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        w.get("/v1/channels/youtube/201", None).await.status,
        StatusCode::BAD_REQUEST
    );

    let r = w
        .gql(
            None,
            r#"{ channelByLogin(platform: TWITCH, login: "USER201") { platformChannelId } }"#,
            json!({}),
        )
        .await;
    assert_eq!(r["data"]["channelByLogin"]["platformChannelId"], "201");

    // The change went out on the channel's topic.
    let events = w.events().await;
    let changed = events
        .iter()
        .find(|e| e.event_type == "channel.active_set_changed")
        .unwrap();
    assert_eq!(changed.topics, ["channel:twitch:201"]);

    // The active set can't be deleted until it is cleared.
    let delete = "mutation($id: String!) { deleteSet(id: $id) }";
    assert_eq!(
        code(&w.gql(Some(&alice), delete, json!({ "id": set })).await),
        "set_in_use"
    );
    let r = w
        .gql(Some(&alice), update, json!({ "id": "201", "s": "" }))
        .await;
    assert_eq!(
        r["data"]["updateChannelSettings"]["activeSetId"],
        Value::Null
    );
    let r = w.gql(Some(&alice), delete, json!({ "id": set })).await;
    assert_eq!(r["data"]["deleteSet"], true);
    w.db.close().await;
}

#[tokio::test]
async fn member_rules_and_events() {
    let Some(w) = World::new().await else { return };
    let alice = w.user("301").await;
    let bob = w.user("302").await;
    let set = w.create_set(&alice, "Main", "CHANNEL").await;
    let r = w
        .gql(
            Some(&alice),
            "mutation($id: String!, $s: String) { updateChannelSettings(platform: TWITCH, id: $id, input: { activeSetId: $s }) { activeSetId } }",
            json!({ "id": "301", "s": set }),
        )
        .await;
    assert!(r["errors"].is_null(), "{r}");

    let bobs_pending = w
        .emote(&bob, "bobPending", EmoteState::Pending, false)
        .await;
    let bobs_private = w
        .emote(&bob, "bobPrivate", EmoteState::Approved, true)
        .await;
    let rejected = w.emote(&alice, "nope", EmoteState::Rejected, false).await;
    let ok = w.emote(&bob, "bobOk", EmoteState::Approved, false).await;

    assert_eq!(
        code(&w.add(&alice, &set, &bobs_pending, None).await),
        "not_approved"
    );
    assert_eq!(
        code(&w.add(&alice, &set, &bobs_private, None).await),
        "emote_private"
    );
    assert_eq!(
        code(&w.add(&alice, &set, &rejected, None).await),
        "emote_unavailable"
    );
    assert_eq!(
        code(&w.add(&alice, &set, &ok, Some("two words")).await),
        "bad_name"
    );
    assert_eq!(code(&w.add(&bob, &set, &ok, None).await), "forbidden");

    let r = w.add(&alice, &set, &ok, None).await;
    assert_eq!(r["data"]["addSetEmote"]["name"], "bobOk");
    assert_eq!(
        code(&w.add(&alice, &set, &ok, None).await),
        "already_in_set"
    );

    let rename = "mutation($s: String!, $e: String!) { updateSetEmote(setId: $s, emoteId: $e, input: { name: \"Renamed\" }) { name } }";
    let r = w
        .gql(Some(&alice), rename, json!({ "s": set, "e": ok }))
        .await;
    assert_eq!(r["data"]["updateSetEmote"]["name"], "Renamed");

    let remove = "mutation($s: String!, $e: String!) { removeSetEmote(setId: $s, emoteId: $e) }";
    assert_eq!(
        w.gql(Some(&alice), remove, json!({ "s": set, "e": ok }))
            .await["data"]["removeSetEmote"],
        true
    );
    assert_eq!(
        w.gql(Some(&alice), remove, json!({ "s": set, "e": ok }))
            .await["data"]["removeSetEmote"],
        false
    );

    let kinds: Vec<(String, Vec<String>, Value)> = w
        .events()
        .await
        .into_iter()
        .filter(|e| e.event_type.starts_with("set."))
        .map(|e| (e.event_type, e.topics, e.payload["name"].clone()))
        .collect();
    let topics = vec![format!("set:{set}"), "channel:twitch:301".to_owned()];
    assert_eq!(
        kinds,
        [
            ("set.emote_added".to_owned(), topics.clone(), json!("bobOk")),
            (
                "set.emote_renamed".to_owned(),
                topics.clone(),
                json!("Renamed")
            ),
            ("set.emote_removed".to_owned(), topics, json!("Renamed")),
        ]
    );
    // Owners editing their own sets aren't audited as grants or permissions.
    assert!(w.audit_actions().await.is_empty());
    w.db.close().await;
}

#[tokio::test]
async fn editors() {
    let Some(w) = World::new().await else { return };
    let alice = w.user("401").await;
    let bob = w.user("402").await;
    let carol = w.user("403").await;
    let set = w.create_set(&alice, "Main", "CHANNEL").await;
    let emote = w
        .emote(&carol, "carolOk", EmoteState::Approved, false)
        .await;
    let bobs_pending = w
        .emote(&bob, "bobPending", EmoteState::Pending, false)
        .await;

    let add_editor =
        "mutation($s: String!, $a: String!) { addSetEditor(setId: $s, accountId: $a) }";
    let remove_editor =
        "mutation($s: String!, $a: String!) { removeSetEditor(setId: $s, accountId: $a) }";
    let bob_id = ids::to_text(bob.id);
    let carol_id = ids::to_text(carol.id);

    assert_eq!(
        code(
            &w.gql(Some(&bob), add_editor, json!({ "s": set, "a": bob_id }))
                .await
        ),
        "forbidden"
    );
    let r = w
        .gql(Some(&alice), add_editor, json!({ "s": set, "a": bob_id }))
        .await;
    assert_eq!(r["data"]["addSetEditor"], true, "{r}");
    assert_eq!(
        w.gql(Some(&alice), add_editor, json!({ "s": set, "a": bob_id }))
            .await["data"]["addSetEditor"],
        false
    );
    assert_eq!(
        code(
            &w.gql(
                Some(&alice),
                add_editor,
                json!({ "s": set, "a": ids::to_text(alice.id) })
            )
            .await
        ),
        "already_owner"
    );
    assert_eq!(
        code(
            &w.gql(
                Some(&alice),
                add_editor,
                json!({ "s": set, "a": ids::to_text(Uuid::nil()) })
            )
            .await
        ),
        "not_found"
    );

    // Bob edits; Carol can't. Bob can't add editors or delete the set.
    assert_eq!(
        w.add(&bob, &set, &emote, None).await["data"]["addSetEmote"]["name"],
        "carolOk"
    );
    assert_eq!(
        code(&w.add(&carol, &set, &emote, Some("x")).await),
        "forbidden"
    );
    assert_eq!(
        code(
            &w.gql(Some(&bob), add_editor, json!({ "s": set, "a": carol_id }))
                .await
        ),
        "forbidden"
    );
    assert_eq!(
        code(
            &w.gql(
                Some(&bob),
                "mutation($id: String!) { deleteSet(id: $id) }",
                json!({ "id": set })
            )
            .await
        ),
        "forbidden"
    );
    // A pending emote may only go in its owner's own sets, even through an editor.
    assert_eq!(
        code(&w.add(&bob, &set, &bobs_pending, None).await),
        "not_approved"
    );

    // Bob sees the set among his.
    let mine = w.gql(Some(&bob), "{ mySets { id } }", json!({})).await;
    assert_eq!(mine["data"]["mySets"][0]["id"], set.as_str());

    // An editor may leave.
    let r = w
        .gql(Some(&bob), remove_editor, json!({ "s": set, "a": bob_id }))
        .await;
    assert_eq!(r["data"]["removeSetEditor"], true);
    assert_eq!(
        code(&w.add(&bob, &set, &emote, Some("again")).await),
        "forbidden"
    );

    assert_eq!(
        w.audit_actions().await,
        ["set.editor_added", "set.editor_removed"]
    );
    w.db.close().await;
}

#[tokio::test]
async fn personal_sets_and_the_chatter_lookup() {
    let Some(w) = World::new().await else { return };
    let alice = w.user("501").await;
    // A user with no personal set: left out of the lookup.
    let _bob = w.user("502").await;
    let personal = w.create_set(&alice, "Mine", "PERSONAL").await;
    let r = w
        .gql(
            Some(&alice),
            "mutation { createSet(input: { name: \"Again\", kind: PERSONAL }) { id } }",
            json!({}),
        )
        .await;
    assert_eq!(code(&r), "personal_set_exists");
    let r = w
        .gql(
            Some(&alice),
            "mutation { createSet(input: { name: \"G\", kind: GLOBAL }) { id } }",
            json!({}),
        )
        .await;
    assert_eq!(code(&r), "invalid_set_kind");

    let pending = w.emote(&alice, "mine0", EmoteState::Pending, false).await;
    assert_eq!(
        code(&w.add(&alice, &personal, &pending, None).await),
        "not_approved"
    );
    for i in 0..5 {
        let e = w
            .emote(
                &alice,
                &format!("mine{}", i + 1),
                EmoteState::Approved,
                false,
            )
            .await;
        let r = w.add(&alice, &personal, &e, None).await;
        assert!(r["errors"].is_null(), "{r}");
    }
    let sixth = w.emote(&alice, "mine6", EmoteState::Approved, false).await;
    assert_eq!(
        code(&w.add(&alice, &personal, &sixth, None).await),
        "set_full"
    );

    // Events for a personal set reach its owner's user topic.
    let events = w.events().await;
    assert!(
        events[0].topics.contains(&"user:twitch:501".to_owned()),
        "{:?}",
        events[0].topics
    );

    let lookup = |ids: Value| {
        Request::post("/v1/personal-sets/lookup")
            .header("content-type", "application/json")
            .body(Body::from(
                json!({ "platform": "twitch", "user_ids": ids }).to_string(),
            ))
            .unwrap()
    };
    let r = w
        .send(lookup(json!(["501", "502", "501", "nope", "999"])))
        .await;
    assert_eq!(r.status, StatusCode::OK);
    let sets = r.body["sets"].as_array().unwrap();
    assert_eq!(sets.len(), 1, "{sets:?}");
    assert_eq!(sets[0]["platform_user_id"], "501");
    assert_eq!(sets[0]["set"]["emotes"].as_array().unwrap().len(), 5);

    let too_many: Vec<String> = (0..101).map(|i| i.to_string()).collect();
    let r = w.send(lookup(json!(too_many))).await;
    assert_eq!(r.status, StatusCode::BAD_REQUEST);
    assert_eq!(r.body["error"], "too_many");

    let r = w
        .gql(None, r#"{ personalSets(platform: TWITCH, userIds: ["501", "502"]) { platformUserId set { kind } } personalSet(platform: TWITCH, id: "502") { id } }"#, json!({}))
        .await;
    assert_eq!(
        r["data"]["personalSets"],
        json!([{ "platformUserId": "501", "set": { "kind": "PERSONAL" } }])
    );
    assert_eq!(r["data"]["personalSet"], Value::Null);
    w.db.close().await;
}

#[tokio::test]
async fn global_set_and_permissions() {
    let Some(w) = World::new().await else { return };
    let alice = w.user("601").await;
    let mod_user = w.user("602").await;
    let emote = w
        .emote(&alice, "globalOne", EmoteState::Approved, false)
        .await;

    let r = w.get("/v1/sets/global", None).await;
    assert_eq!(r.status, StatusCode::OK);
    assert_eq!(r.body["kind"], "global");
    assert_eq!(r.body["owner"], Value::Null);
    let global = r.body["id"].as_str().unwrap().to_owned();

    assert_eq!(
        code(&w.add(&mod_user, &global, &emote, None).await),
        "forbidden"
    );
    w.grant(&mod_user, "global_set.edit").await;
    let r = w.add(&mod_user, &global, &emote, None).await;
    assert_eq!(r["data"]["addSetEmote"]["name"], "globalOne", "{r}");
    let r = w
        .gql(None, "{ globalSet { emotes { name } } }", json!({}))
        .await;
    assert_eq!(
        r["data"]["globalSet"]["emotes"],
        json!([{ "name": "globalOne" }])
    );

    // Capacity: owners may stay at or below the default; above needs sets.edit_any.
    let set = w.create_set(&alice, "Main", "CHANNEL").await;
    let capacity = "mutation($id: String!, $c: Int!) { updateSet(id: $id, input: { capacity: $c }) { capacity } }";
    assert_eq!(
        w.gql(Some(&alice), capacity, json!({ "id": set, "c": 500 }))
            .await["data"]["updateSet"]["capacity"],
        500
    );
    assert_eq!(
        code(
            &w.gql(Some(&alice), capacity, json!({ "id": set, "c": 2000 }))
                .await
        ),
        "forbidden"
    );
    assert_eq!(
        code(
            &w.gql(Some(&alice), capacity, json!({ "id": set, "c": 0 }))
                .await
        ),
        "bad_capacity"
    );
    w.add(&alice, &set, &emote, None).await;
    let other = w.emote(&alice, "second", EmoteState::Approved, false).await;
    w.add(&alice, &set, &other, None).await;
    assert_eq!(
        code(
            &w.gql(Some(&alice), capacity, json!({ "id": set, "c": 1 }))
                .await
        ),
        "capacity_too_low"
    );

    // A moderator with sets.edit_any can change anyone's set, and that is audited.
    w.grant(&mod_user, "sets.edit_any").await;
    let r = w
        .gql(Some(&mod_user), capacity, json!({ "id": set, "c": 5000 }))
        .await;
    assert_eq!(r["data"]["updateSet"]["capacity"], 5000, "{r}");
    assert_eq!(
        w.audit_actions().await,
        ["set.emote_added", "set.capacity_changed"]
    );
    w.db.close().await;
}

#[tokio::test]
async fn reads_without_a_session_hide_pending_emotes_from_everyone_but_reviewers() {
    let Some(w) = World::new().await else { return };
    let alice = w.user("701").await;
    let reviewer = w.user("702").await;
    let stranger = w.user("703").await;
    let set = w.create_set(&alice, "Main", "CHANNEL").await;
    let pending = w.emote(&alice, "waiting", EmoteState::Pending, false).await;
    w.add(&alice, &set, &pending, None).await;
    w.grant(&reviewer, "emotes.review").await;

    let query = "query($id: String!) { set(id: $id) { emotes { name } } }";
    let count = |r: &Value| r["data"]["set"]["emotes"].as_array().unwrap().len();
    assert_eq!(count(&w.gql(None, query, json!({ "id": set })).await), 0);
    assert_eq!(
        count(&w.gql(Some(&stranger), query, json!({ "id": set })).await),
        0
    );
    assert_eq!(
        count(&w.gql(Some(&reviewer), query, json!({ "id": set })).await),
        1
    );
    assert_eq!(
        count(&w.gql(Some(&alice), query, json!({ "id": set })).await),
        1
    );
    w.db.close().await;
}

#[tokio::test]
async fn my_emotes_and_set_editors() {
    let Some(w) = World::new().await else { return };
    let alice = w.user("801").await;
    let bob = w.user("802").await;
    let stranger = w.user("803").await;
    w.emote(&alice, "first", EmoteState::Approved, false).await;
    w.emote(&alice, "second", EmoteState::Pending, false).await;
    w.emote(&alice, "third", EmoteState::Rejected, false).await;
    w.emote(&bob, "bobs", EmoteState::Approved, false).await;

    // Every state, newest first, paged; only your own.
    let mine = "query($p: Int, $n: Int) { myEmotes(page: $p, perPage: $n) { total items { name state } } }";
    let r = w.gql(Some(&alice), mine, json!({ "p": 1, "n": 2 })).await;
    assert_eq!(r["data"]["myEmotes"]["total"], 3, "{r}");
    let names: Vec<&str> = r["data"]["myEmotes"]["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, ["third", "second"]);
    let r = w.gql(Some(&alice), mine, json!({ "p": 2, "n": 2 })).await;
    assert_eq!(r["data"]["myEmotes"]["items"][0]["name"], "first", "{r}");
    assert_eq!(code(&w.gql(None, mine, json!({})).await), "signed_out");

    // Editors: visible to the owner and the editors, not to anyone else.
    let set = w.create_set(&alice, "Main", "CHANNEL").await;
    let add = "mutation($s: String!, $a: String!) { addSetEditor(setId: $s, accountId: $a) }";
    let r = w
        .gql(
            Some(&alice),
            add,
            json!({ "s": set, "a": ids::to_text(bob.id) }),
        )
        .await;
    assert_eq!(r["data"]["addSetEditor"], true, "{r}");
    let editors = "query($s: String!) { setEditors(setId: $s) { id displayName } }";
    for user in [&alice, &bob] {
        let r = w.gql(Some(user), editors, json!({ "s": set })).await;
        assert_eq!(
            r["data"]["setEditors"][0]["id"],
            ids::to_text(bob.id),
            "{r}"
        );
        assert_eq!(r["data"]["setEditors"][0]["displayName"], "user802");
    }
    assert_eq!(
        code(&w.gql(Some(&stranger), editors, json!({ "s": set })).await),
        "forbidden"
    );
    w.db.close().await;
}
