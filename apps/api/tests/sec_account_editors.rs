//! Account editors (ADR 0014): each flag gates its actions, an editor of one account can't touch
//! another (IDOR), no escalation through `editors`, removal and blocks take effect at once, the
//! limits hold. Through the real router against a real Postgres; runs when
//! `CYEX_TV_TEST_DATABASE_URL` is set (CI sets it) and skips otherwise.

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

const ADD: &str = "mutation($o: String, $a: String, $p: Platform, $l: String, $f: [EditorPermission!]!) {
    addAccountEditor(input: { ownerId: $o, accountId: $a, platform: $p, login: $l, permissions: $f }) {
        account { id } permissions grantedBy { id }
    }
}";
const UPDATE: &str = "mutation($o: String, $a: String!, $f: [EditorPermission!]!) {
    updateAccountEditor(ownerId: $o, accountId: $a, permissions: $f) { account { id } permissions }
}";
const REMOVE: &str =
    "mutation($o: String, $a: String!) { removeAccountEditor(ownerId: $o, accountId: $a) }";
const LIST: &str =
    "query($o: String) { accountEditors(accountId: $o) { account { id } permissions } }";
const ADD_EMOTE: &str = "mutation($s: String!, $e: String!, $n: String) {
    addSetEmote(setId: $s, emoteId: $e, input: { name: $n }) { name }
}";
const RENAME_EMOTE: &str = "mutation($s: String!, $e: String!, $n: String!) {
    updateSetEmote(setId: $s, emoteId: $e, input: { name: $n }) { name }
}";
const REMOVE_EMOTE: &str =
    "mutation($s: String!, $e: String!) { removeSetEmote(setId: $s, emoteId: $e) }";
const CREATE_SET: &str = "mutation($n: String!, $o: String) {
    createSet(input: { name: $n, kind: CHANNEL, ownerId: $o }) { id owner { id } }
}";
const RENAME_SET: &str =
    "mutation($id: String!, $n: String!) { updateSet(id: $id, input: { name: $n }) { name } }";
const DELETE_SET: &str = "mutation($id: String!) { deleteSet(id: $id) }";
const CHANNEL: &str = "mutation($c: String!, $s: String, $p: Boolean) {
    updateChannelSettings(platform: TWITCH, id: $c, input: { activeSetId: $s, allowPersonalEmotes: $p }) {
        activeSetId allowPersonalEmotes
    }
}";
const ADD_SET_EDITOR: &str =
    "mutation($s: String!, $a: String!) { addSetEditor(setId: $s, accountId: $a) }";

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

    /// A Twitch user `user<id>` with a channel and a site session.
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

    async fn gql(&self, user: &User, query: &str, variables: Value) -> Value {
        let request = Request::post("/v1/gql")
            .header("content-type", "application/json")
            .header(
                "cookie",
                format!("{}={}", cyex_tv_auth::cookies::SESSION, user.token),
            )
            .header("origin", "https://cyex.tv")
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
            .gql(owner, CREATE_SET, json!({ "n": name, "o": null }))
            .await;
        r["data"]["createSet"]["id"]
            .as_str()
            .unwrap_or_else(|| panic!("{r}"))
            .to_owned()
    }

    /// `by` makes `who` an editor of `owner` (or of their own account when `owner` is `None`).
    async fn add(&self, by: &User, owner: Option<&User>, who: &User, flags: &[&str]) -> Value {
        self.gql(
            by,
            ADD,
            json!({ "o": owner.map(User::tid), "a": who.tid(), "f": flags }),
        )
        .await
    }

    async fn add_emote(&self, by: &User, set: &str, emote: &str, alias: Option<&str>) -> Value {
        self.gql(by, ADD_EMOTE, json!({ "s": set, "e": emote, "n": alias }))
            .await
    }

    async fn audit_rows(&self) -> Vec<audit::Entry> {
        let mut conn = self.db.conn().await;
        let mut rows = audit::list(&mut conn, &audit::Filter::default(), 100)
            .await
            .unwrap();
        rows.reverse();
        rows
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
}

#[tokio::test]
async fn each_flag_gates_its_actions() {
    let Some(w) = World::new().await else { return };
    let alice = w.user("1001").await;
    let bob = w.user("1002").await;
    let carol = w.user("1003").await;
    let set = w.set(&alice, "Main").await;
    let ok = w
        .emote(&alice, "aliceOk", EmoteState::Approved, false)
        .await;
    let pending = w
        .emote(&alice, "alicePending", EmoteState::Pending, false)
        .await;
    let private = w
        .emote(&alice, "alicePriv", EmoteState::Approved, true)
        .await;
    let carols_private = w
        .emote(&carol, "carolPriv", EmoteState::Approved, true)
        .await;

    // Nothing before a grant.
    assert_eq!(code(&w.add_emote(&bob, &set, &ok, None).await), "forbidden");

    let r = w.add(&alice, None, &bob, &["EMOTES"]).await;
    assert_eq!(
        r["data"]["addAccountEditor"]["permissions"],
        json!(["EMOTES"]),
        "{r}"
    );
    assert_eq!(
        r["data"]["addAccountEditor"]["grantedBy"]["id"],
        w.db.public(alice.id).await.as_str()
    );

    // `emotes`: add, alias, rename, remove; the owner's pending and private emotes too.
    let r = w.add_emote(&bob, &set, &ok, Some("okAlias")).await;
    assert_eq!(r["data"]["addSetEmote"]["name"], "okAlias", "{r}");
    let r = w
        .gql(
            &bob,
            RENAME_EMOTE,
            json!({ "s": set, "e": ok, "n": "okRenamed" }),
        )
        .await;
    assert_eq!(r["data"]["updateSetEmote"]["name"], "okRenamed", "{r}");
    let r = w.add_emote(&bob, &set, &pending, None).await;
    assert_eq!(r["data"]["addSetEmote"]["name"], "alicePending", "{r}");
    let r = w.add_emote(&bob, &set, &private, None).await;
    assert_eq!(r["data"]["addSetEmote"]["name"], "alicePriv", "{r}");
    // Nobody else's private emotes: not a third account's.
    assert_eq!(
        code(&w.add_emote(&bob, &set, &carols_private, None).await),
        "emote_private"
    );
    let r = w
        .gql(&bob, REMOVE_EMOTE, json!({ "s": set, "e": private }))
        .await;
    assert_eq!(r["data"]["removeSetEmote"], true, "{r}");

    // The editor sees the owner's sets, pending emotes included, labelled with the owner.
    let r = w
        .gql(
            &bob,
            "{ editorSets { id owner { id } emotes { name } } }",
            json!({}),
        )
        .await;
    assert_eq!(r["data"]["editorSets"][0]["id"], set.as_str(), "{r}");
    assert_eq!(
        r["data"]["editorSets"][0]["owner"]["id"],
        w.db.public(alice.id).await.as_str()
    );
    let names: Vec<&str> = r["data"]["editorSets"][0]["emotes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["name"].as_str().unwrap())
        .collect();
    assert!(names.contains(&"alicePending"), "{r}");
    let r = w
        .gql(
            &bob,
            "{ editorOf { account { id } permissions } editorChannels { login } }",
            json!({}),
        )
        .await;
    assert_eq!(
        r["data"]["editorOf"][0]["account"]["id"],
        w.db.public(alice.id).await.as_str(),
        "{r}"
    );
    assert_eq!(r["data"]["editorChannels"][0]["login"], "user1001");

    // Without `sets`: no set or channel changes, no editors.
    assert_eq!(
        code(
            &w.gql(&bob, RENAME_SET, json!({ "id": set, "n": "Mine" }))
                .await
        ),
        "forbidden"
    );
    assert_eq!(
        code(
            &w.gql(&bob, CREATE_SET, json!({ "n": "New", "o": alice.tid() }))
                .await
        ),
        "forbidden"
    );
    assert_eq!(
        code(&w.gql(&bob, DELETE_SET, json!({ "id": set })).await),
        "forbidden"
    );
    assert_eq!(
        code(
            &w.gql(
                &bob,
                CHANNEL,
                json!({ "c": alice.twitch_id, "s": set, "p": null })
            )
            .await
        ),
        "forbidden"
    );
    assert_eq!(
        code(&w.gql(&bob, LIST, json!({ "o": alice.tid() })).await),
        "forbidden"
    );

    // `sets`: make a set for the owner, rename it, make it active, change settings, delete one.
    let r = w
        .gql(
            &alice,
            UPDATE,
            json!({ "a": bob.tid(), "f": ["EMOTES", "SETS"] }),
        )
        .await;
    assert_eq!(
        r["data"]["updateAccountEditor"]["permissions"],
        json!(["EMOTES", "SETS"]),
        "{r}"
    );
    let r = w
        .gql(
            &bob,
            CREATE_SET,
            json!({ "n": "Made by Bob", "o": alice.tid() }),
        )
        .await;
    assert_eq!(
        r["data"]["createSet"]["owner"]["id"],
        w.db.public(alice.id).await.as_str(),
        "{r}"
    );
    let made = r["data"]["createSet"]["id"].as_str().unwrap().to_owned();
    let r = w
        .gql(&bob, RENAME_SET, json!({ "id": made, "n": "Second" }))
        .await;
    assert_eq!(r["data"]["updateSet"]["name"], "Second", "{r}");
    let r = w
        .gql(
            &bob,
            CHANNEL,
            json!({ "c": alice.twitch_id, "s": set, "p": null }),
        )
        .await;
    assert_eq!(
        r["data"]["updateChannelSettings"]["activeSetId"],
        set.as_str(),
        "{r}"
    );
    // Channel settings are `channels`, not `sets` (ADR 0014, amendment 2026/11/22).
    assert_eq!(
        code(
            &w.gql(
                &bob,
                CHANNEL,
                json!({ "c": alice.twitch_id, "s": null, "p": false })
            )
            .await
        ),
        "forbidden"
    );
    // Only the owner's own channel sets can be made active.
    let bobs_set = w.set(&bob, "Bob's").await;
    assert_eq!(
        code(
            &w.gql(
                &bob,
                CHANNEL,
                json!({ "c": alice.twitch_id, "s": bobs_set, "p": null })
            )
            .await
        ),
        "invalid_active_set"
    );
    let r = w.gql(&bob, DELETE_SET, json!({ "id": made })).await;
    assert_eq!(r["data"]["deleteSet"], true, "{r}");

    // Editor changes on the active set publish the usual events to the channel.
    w.add_emote(&bob, &set, &private, None).await;
    let added = w
        .events()
        .await
        .into_iter()
        .rev()
        .find(|e| e.event_type == "set.emote_added")
        .unwrap();
    assert!(
        added
            .topics
            .contains(&outbox::channel_topic(Platform::Twitch, &alice.twitch_id)),
        "{:?}",
        added.topics
    );

    // Everything done through the grant is audited, and says so.
    let rows = w.audit_rows().await;
    let actions: Vec<&str> = rows.iter().map(|e| e.action.as_str()).collect();
    for action in [
        "account.editor_added",
        "set.emote_added",
        "set.emote_updated",
        "set.emote_removed",
        "account.editor_updated",
        "set.created",
        "set.renamed",
        "channel.settings_changed",
        "set.deleted",
    ] {
        assert!(
            actions.contains(&action),
            "{action} missing from {actions:?}"
        );
    }
    let via = rows.iter().find(|e| e.action == "set.emote_added").unwrap();
    assert_eq!(via.actor_account_id, Some(bob.id));
    assert_eq!(via.after_json.as_ref().unwrap()["via"], "account_editor");
    w.db.close().await;
}

#[tokio::test]
async fn an_editor_of_one_account_cant_touch_another() {
    let Some(w) = World::new().await else { return };
    let alice = w.user("1101").await;
    let bob = w.user("1102").await;
    let carol = w.user("1103").await;
    let dave = w.user("1104").await;
    let all = ["EMOTES", "SETS", "EDITORS", "PROFILE"];
    let carols_set = w.set(&carol, "Carol's").await;
    let alices_set = w.set(&alice, "Alice's").await;
    let emote = w
        .emote(&carol, "carolOk", EmoteState::Approved, false)
        .await;

    assert!(w.add(&alice, None, &bob, &all).await["errors"].is_null());
    assert!(w.add(&carol, None, &dave, &["EMOTES"]).await["errors"].is_null());

    // Bob holds everything on Alice and nothing on Carol.
    assert_eq!(
        code(&w.add_emote(&bob, &carols_set, &emote, None).await),
        "forbidden"
    );
    assert_eq!(
        code(
            &w.gql(&bob, RENAME_SET, json!({ "id": carols_set, "n": "x" }))
                .await
        ),
        "forbidden"
    );
    assert_eq!(
        code(&w.gql(&bob, DELETE_SET, json!({ "id": carols_set })).await),
        "forbidden"
    );
    assert_eq!(
        code(
            &w.gql(&bob, CREATE_SET, json!({ "n": "x", "o": carol.tid() }))
                .await
        ),
        "forbidden"
    );
    assert_eq!(
        code(
            &w.gql(
                &bob,
                CHANNEL,
                json!({ "c": carol.twitch_id, "s": null, "p": false })
            )
            .await
        ),
        "forbidden"
    );
    assert_eq!(
        code(&w.gql(&bob, LIST, json!({ "o": carol.tid() })).await),
        "forbidden"
    );
    assert_eq!(
        code(&w.add(&bob, Some(&carol), &bob, &["EMOTES"]).await),
        "forbidden"
    );
    assert_eq!(
        code(
            &w.gql(
                &bob,
                UPDATE,
                json!({ "o": carol.tid(), "a": dave.tid(), "f": all })
            )
            .await
        ),
        "forbidden"
    );
    assert_eq!(
        code(
            &w.gql(&bob, REMOVE, json!({ "o": carol.tid(), "a": dave.tid() }))
                .await
        ),
        "forbidden"
    );
    assert_eq!(
        code(
            &w.gql(
                &bob,
                ADD_SET_EDITOR,
                json!({ "s": carols_set, "a": bob.tid() })
            )
            .await
        ),
        "forbidden"
    );

    // Grants aren't passed on: Carol's editor Dave gets nothing on Alice through anyone.
    assert!(w.add(&bob, None, &dave, &all).await["errors"].is_null());
    assert_eq!(
        code(&w.add_emote(&dave, &alices_set, &emote, None).await),
        "forbidden"
    );
    assert_eq!(
        code(&w.add_emote(&carol, &alices_set, &emote, None).await),
        "forbidden"
    );

    // Carol's grant to Dave is untouched.
    let mut conn = w.db.conn().await;
    assert_eq!(
        editors::permissions_of(&mut conn, carol.id, dave.id)
            .await
            .unwrap(),
        Some(vec!["emotes".to_owned()])
    );
    drop(conn);
    w.db.close().await;
}

#[tokio::test]
async fn the_editors_flag_cant_escalate_or_touch_the_owner() {
    let Some(w) = World::new().await else { return };
    let alice = w.user("1201").await;
    let bob = w.user("1202").await;
    let carol = w.user("1203").await;
    let dave = w.user("1204").await;
    let erin = w.user("1205").await;
    let set = w.set(&alice, "Main").await;

    assert!(w.add(&alice, None, &bob, &["EMOTES", "EDITORS"]).await["errors"].is_null());
    assert!(w.add(&alice, None, &dave, &["EMOTES", "SETS"]).await["errors"].is_null());

    // Bob can't grant a flag he lacks, alone or alongside ones he has.
    assert_eq!(
        code(&w.add(&bob, Some(&alice), &carol, &["SETS"]).await),
        "forbidden"
    );
    assert_eq!(
        code(
            &w.add(&bob, Some(&alice), &carol, &["EMOTES", "PROFILE"])
                .await
        ),
        "forbidden"
    );
    let r = w
        .add(&bob, Some(&alice), &carol, &["EMOTES", "EDITORS"])
        .await;
    assert_eq!(
        r["data"]["addAccountEditor"]["grantedBy"]["id"],
        w.db.public(bob.id).await.as_str(),
        "{r}"
    );
    // ...nor raise an editor, himself included, above his own flags.
    assert_eq!(
        code(
            &w.gql(
                &bob,
                UPDATE,
                json!({ "o": alice.tid(), "a": carol.tid(), "f": ["SETS"] })
            )
            .await
        ),
        "forbidden"
    );
    assert_eq!(
        code(
            &w.gql(
                &bob,
                UPDATE,
                json!({ "o": alice.tid(), "a": bob.tid(), "f": ["EMOTES", "EDITORS", "SETS"] })
            )
            .await
        ),
        "forbidden"
    );
    // Dave holds `sets`, which Bob doesn't: Bob can't change or remove him.
    assert_eq!(
        code(
            &w.gql(
                &bob,
                UPDATE,
                json!({ "o": alice.tid(), "a": dave.tid(), "f": ["EMOTES"] })
            )
            .await
        ),
        "forbidden"
    );
    assert_eq!(
        code(
            &w.gql(&bob, REMOVE, json!({ "o": alice.tid(), "a": dave.tid() }))
                .await
        ),
        "forbidden"
    );
    // He can lower and remove editors within his own flags.
    let r = w
        .gql(
            &bob,
            UPDATE,
            json!({ "o": alice.tid(), "a": carol.tid(), "f": ["EMOTES"] }),
        )
        .await;
    assert_eq!(
        r["data"]["updateAccountEditor"]["permissions"],
        json!(["EMOTES"]),
        "{r}"
    );
    let r = w
        .gql(&bob, REMOVE, json!({ "o": alice.tid(), "a": carol.tid() }))
        .await;
    assert_eq!(r["data"]["removeAccountEditor"], true, "{r}");

    // The owner can't be added, changed or removed.
    assert_eq!(
        code(&w.add(&bob, Some(&alice), &alice, &["EMOTES"]).await),
        "already_owner"
    );
    assert_eq!(
        code(
            &w.gql(
                &bob,
                UPDATE,
                json!({ "o": alice.tid(), "a": alice.tid(), "f": ["EMOTES"] })
            )
            .await
        ),
        "not_found"
    );
    let r = w
        .gql(&bob, REMOVE, json!({ "o": alice.tid(), "a": alice.tid() }))
        .await;
    assert_eq!(r["data"]["removeAccountEditor"], false, "{r}");
    assert!(w.add(&alice, None, &erin, &["EDITORS"]).await["errors"].is_null());

    // Bob lists Alice's editors (he holds `editors`); Dave, without it, can't.
    let r = w.gql(&bob, LIST, json!({ "o": alice.tid() })).await;
    assert_eq!(
        r["data"]["accountEditors"].as_array().unwrap().len(),
        3,
        "{r}"
    );
    assert_eq!(
        code(&w.gql(&dave, LIST, json!({ "o": alice.tid() })).await),
        "forbidden"
    );

    // Per-set editors: `editors` and `emotes` together, since a set editor changes emotes.
    let r = w
        .gql(&bob, ADD_SET_EDITOR, json!({ "s": set, "a": carol.tid() }))
        .await;
    assert_eq!(r["data"]["addSetEditor"], true, "{r}");
    assert_eq!(
        code(
            &w.gql(&erin, ADD_SET_EDITOR, json!({ "s": set, "a": dave.tid() }))
                .await
        ),
        "forbidden"
    );
    // Erin, with `editors` only, can't hand herself `emotes` either.
    assert_eq!(
        code(&w.add(&erin, Some(&alice), &carol, &["EMOTES"]).await),
        "forbidden"
    );

    // An empty grant is refused.
    assert_eq!(
        code(&w.add(&alice, None, &carol, &[]).await),
        "no_permissions"
    );
    w.db.close().await;
}

#[tokio::test]
async fn a_removed_editor_loses_access_at_once_and_editors_can_leave() {
    let Some(w) = World::new().await else { return };
    let alice = w.user("1301").await;
    let bob = w.user("1302").await;
    let carol = w.user("1303").await;
    let set = w.set(&alice, "Main").await;
    let one = w.emote(&alice, "one", EmoteState::Approved, false).await;
    let two = w.emote(&alice, "two", EmoteState::Approved, false).await;

    assert!(w.add(&alice, None, &bob, &["EMOTES", "SETS"]).await["errors"].is_null());
    assert!(w.add(&alice, None, &carol, &["EMOTES"]).await["errors"].is_null());
    assert!(w.add_emote(&bob, &set, &one, None).await["errors"].is_null());

    let r = w.gql(&alice, REMOVE, json!({ "a": bob.tid() })).await;
    assert_eq!(r["data"]["removeAccountEditor"], true, "{r}");
    assert_eq!(
        code(&w.add_emote(&bob, &set, &two, None).await),
        "forbidden"
    );
    assert_eq!(
        code(
            &w.gql(&bob, RENAME_SET, json!({ "id": set, "n": "x" }))
                .await
        ),
        "forbidden"
    );
    let r = w
        .gql(
            &bob,
            "{ editorOf { account { id } } editorSets { id } }",
            json!({}),
        )
        .await;
    assert_eq!(r["data"]["editorOf"], json!([]), "{r}");
    assert_eq!(r["data"]["editorSets"], json!([]));

    // Lowering takes effect at once too.
    let r = w
        .gql(
            &alice,
            UPDATE,
            json!({ "a": carol.tid(), "f": ["PROFILE"] }),
        )
        .await;
    assert!(r["errors"].is_null(), "{r}");
    assert_eq!(
        code(&w.add_emote(&carol, &set, &two, None).await),
        "forbidden"
    );

    // Carol leaves on her own.
    let r = w
        .gql(
            &carol,
            REMOVE,
            json!({ "o": alice.tid(), "a": carol.tid() }),
        )
        .await;
    assert_eq!(r["data"]["removeAccountEditor"], true, "{r}");
    let r = w.gql(&alice, LIST, json!({})).await;
    assert_eq!(r["data"]["accountEditors"], json!([]), "{r}");

    let actions: Vec<String> = w.audit_rows().await.into_iter().map(|e| e.action).collect();
    assert_eq!(
        actions
            .iter()
            .filter(|a| a.as_str() == "account.editor_removed")
            .count(),
        2,
        "{actions:?}"
    );
    w.db.close().await;
}

#[tokio::test]
async fn blocked_accounts_cant_be_or_use_editors() {
    let Some(w) = World::new().await else { return };
    let alice = w.user("1401").await;
    let bob = w.user("1402").await;
    let carol = w.user("1403").await;
    let set = w.set(&alice, "Main").await;
    let emote = w.emote(&alice, "ok", EmoteState::Approved, false).await;

    // Adding by login works, case-insensitively and with an @.
    let r = w
        .gql(
            &alice,
            ADD,
            json!({ "p": "TWITCH", "l": "@USER1402", "f": ["EMOTES"] }),
        )
        .await;
    assert_eq!(
        r["data"]["addAccountEditor"]["account"]["id"],
        w.db.public(bob.id).await.as_str(),
        "{r}"
    );
    let r = w
        .gql(
            &alice,
            ADD,
            json!({ "p": "KICK", "l": "user1403", "f": ["EMOTES"] }),
        )
        .await;
    assert_eq!(code(&r), "not_found", "{r}");
    assert_eq!(
        code(&w.gql(&alice, ADD, json!({ "f": ["EMOTES"] })).await),
        "bad_editor"
    );

    // A blocked account can't be added, by ID or by login.
    let mut conn = w.db.conn().await;
    accounts::set_blocked(&mut conn, carol.id, true)
        .await
        .unwrap();
    drop(conn);
    assert_eq!(
        code(&w.add(&alice, None, &carol, &["EMOTES"]).await),
        "not_found"
    );
    assert_eq!(
        code(
            &w.gql(
                &alice,
                ADD,
                json!({ "p": "TWITCH", "l": "user1403", "f": ["EMOTES"] })
            )
            .await
        ),
        "not_found"
    );

    // A blocked editor's grant counts for nothing, but stays listed for the owner to remove.
    let mut conn = w.db.conn().await;
    accounts::set_blocked(&mut conn, bob.id, true)
        .await
        .unwrap();
    assert_eq!(
        editors::permissions_of(&mut conn, alice.id, bob.id)
            .await
            .unwrap(),
        None
    );
    drop(conn);
    let r = w.add_emote(&bob, &set, &emote, None).await;
    assert!(r["data"]["addSetEmote"].is_null(), "{r}");
    let r = w.gql(&alice, LIST, json!({})).await;
    assert_eq!(
        r["data"]["accountEditors"][0]["account"]["id"],
        w.db.public(bob.id).await.as_str(),
        "{r}"
    );

    // Grants on a blocked owner count for nothing either.
    let mut conn = w.db.conn().await;
    accounts::set_blocked(&mut conn, bob.id, false)
        .await
        .unwrap();
    accounts::set_blocked(&mut conn, alice.id, true)
        .await
        .unwrap();
    drop(conn);
    assert_eq!(
        code(&w.add_emote(&bob, &set, &emote, None).await),
        "forbidden"
    );
    let r = w
        .gql(&bob, "{ editorOf { account { id } } }", json!({}))
        .await;
    assert_eq!(r["data"]["editorOf"], json!([]), "{r}");
    w.db.close().await;
}

#[tokio::test]
async fn editor_changes_are_rate_limited() {
    // The 50-editor limit is `sets_editors_flow::at_most_fifty_editors`.
    let Some(w) = World::with_rates(Rates {
        editors: Rate::per_hour(24),
        ..Rates::default()
    })
    .await
    else {
        return;
    };
    let alice = w.user("1501").await;
    for n in 0..20 {
        let editor = w.user(&format!("16{n:02}")).await;
        let r = w.add(&alice, None, &editor, &["EMOTES"]).await;
        assert!(r["errors"].is_null(), "{n}: {r}");
        if n == 0 {
            assert_eq!(
                code(&w.add(&alice, None, &editor, &["SETS"]).await),
                "already_editor"
            );
        }
    }
    let extra = w.user("1700").await;
    let r = w.add(&alice, None, &extra, &["EMOTES"]).await;
    assert!(r["errors"].is_null(), "the 21st editor fits now: {r}");
    // 24 changes so far; the 25th within the hour is refused.
    w.add(&alice, None, &extra, &["EMOTES"]).await;
    w.add(&alice, None, &extra, &["EMOTES"]).await;
    assert_eq!(
        code(&w.add(&alice, None, &extra, &["EMOTES"]).await),
        "rate_limited"
    );
    w.db.close().await;
}

#[tokio::test]
async fn a_grant_without_set_flags_lists_no_sets() {
    let Some(w) = World::new().await else { return };
    let alice = w.user("1101").await;
    let bob = w.user("1102").await;
    let set = w.set(&alice, "Main").await;

    let r = w.add(&alice, None, &bob, &["PROFILE"]).await;
    assert!(r["errors"].is_null(), "{r}");
    let r = w.gql(&bob, "{ editorSets { id } }", json!({})).await;
    assert_eq!(r["data"]["editorSets"], json!([]), "{r}");

    // With a flag that works on sets, they're listed.
    let r = w
        .gql(
            &alice,
            UPDATE,
            json!({ "a": bob.tid(), "f": ["PROFILE", "EMOTES"] }),
        )
        .await;
    assert!(r["errors"].is_null(), "{r}");
    let r = w.gql(&bob, "{ editorSets { id } }", json!({})).await;
    let listed = r["data"]["editorSets"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    assert!(listed.iter().any(|s| s["id"] == set.as_str()), "{r}");
    w.db.close().await;
}

#[tokio::test]
async fn editor_changes_wait_for_the_owner_row() {
    let Some(w) = World::new().await else { return };
    let alice = w.user("1111").await;
    let bob = w.user("1112").await;

    // Another change to alice's editors holds her row. The same lock the API takes: writes that
    // only reference the account (the grant's own foreign key) don't wait for it, so only the
    // API's lock makes this wait.
    let mut held = w.db.conn().await;
    sqlx::query("BEGIN").execute(&mut *held).await.unwrap();
    sqlx::query("SELECT 1 FROM accounts WHERE id = $1 FOR NO KEY UPDATE")
        .bind(alice.id)
        .execute(&mut *held)
        .await
        .unwrap();

    let r = {
        let add = w.add(&alice, None, &bob, &["EMOTES"]);
        tokio::pin!(add);
        assert!(
            tokio::time::timeout(std::time::Duration::from_millis(500), &mut add)
                .await
                .is_err(),
            "the change waits for the lock"
        );
        sqlx::query("ROLLBACK").execute(&mut *held).await.unwrap();
        add.await
    };
    assert_eq!(
        r["data"]["addAccountEditor"]["account"]["id"],
        w.db.public(bob.id).await.as_str(),
        "{r}"
    );
    drop(held);
    w.db.close().await;
}
