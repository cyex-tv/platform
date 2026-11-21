//! Account editors limited to some sets, the new editor flags, the 50-editor limit, the public
//! editor list and default sets (ADR 0014 and 0017, amended 2026/11/19). Through the real router
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
use cyex_tv_db::{audit, channels, ids, sets};
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

fn ok(response: &Value) {
    assert!(response["errors"].is_null(), "{response}");
}

const ADD: &str = "mutation($o: String, $a: String, $f: [EditorPermission!]!, $s: [String!]) {
    addAccountEditor(input: { ownerId: $o, accountId: $a, permissions: $f, setIds: $s }) {
        account { id } permissions allSets setIds
    }
}";
const UPDATE: &str = "mutation($o: String, $a: String!, $f: [EditorPermission!]!, $s: [String!]) {
    updateAccountEditor(ownerId: $o, accountId: $a, permissions: $f, setIds: $s) {
        account { id } permissions allSets setIds
    }
}";
const REMOVE: &str =
    "mutation($o: String, $a: String!) { removeAccountEditor(ownerId: $o, accountId: $a) }";
const ADD_EMOTE: &str =
    "mutation($s: String!, $e: String!) { addSetEmote(setId: $s, emoteId: $e) { name } }";
const CREATE_SET: &str = "mutation($n: String!, $o: String) {
    createSet(input: { name: $n, kind: CHANNEL, ownerId: $o }) { id }
}";
const RENAME_SET: &str =
    "mutation($id: String!, $n: String!) { updateSet(id: $id, input: { name: $n }) { name } }";
const PRIVATE_SET: &str =
    "mutation($id: String!) { updateSet(id: $id, input: { visibility: PRIVATE }) { id } }";
const DELETE_SET: &str = "mutation($id: String!) { deleteSet(id: $id) }";
const CLONE_SET: &str =
    "mutation($id: String!, $o: String) { cloneSet(id: $id, ownerId: $o) { set { id } } }";
const CHANNEL: &str = "mutation($c: String!, $s: String) {
    updateChannelSettings(platform: TWITCH, id: $c, input: { activeSetId: $s }) { activeSetId }
}";
const SET: &str = "query($id: String!) { set(id: $id) { id } }";

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

    async fn emote(&self, owner: &User, name: &str) -> String {
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
        emotes::set_state(&mut conn, emote.id, EmoteState::Approved)
            .await
            .unwrap();
        self.db.public(emote.id).await
    }

    async fn send(&self, token: Option<&str>, query: &str, variables: Value) -> Value {
        let mut request = Request::post("/v1/gql").header("content-type", "application/json");
        if let Some(token) = token {
            request = request
                .header(
                    "cookie",
                    format!("{}={token}", cyex_tv_auth::cookies::SESSION),
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

    async fn gql(&self, user: &User, query: &str, variables: Value) -> Value {
        self.send(Some(&user.token), query, variables).await
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

    async fn add(
        &self,
        by: &User,
        owner: Option<&User>,
        who: &User,
        flags: &[&str],
        only: &[&str],
    ) -> Value {
        self.gql(
            by,
            ADD,
            json!({ "o": owner.map(User::tid), "a": who.tid(), "f": flags, "s": only }),
        )
        .await
    }
}

#[tokio::test]
async fn an_editor_limited_to_some_sets_works_on_those_only() {
    let Some(w) = World::new().await else { return };
    let alice = w.user("2001").await;
    let bob = w.user("2002").await;
    let a = w.set(&alice, "Main").await;
    let b = w.set(&alice, "Subs").await;
    let emote = w.emote(&alice, "aliceOk").await;

    let r = w
        .add(&alice, None, &bob, &["EMOTES", "SETS"], &[a.as_str()])
        .await;
    ok(&r);
    let grant = &r["data"]["addAccountEditor"];
    assert_eq!(grant["allSets"], false, "{r}");
    assert_eq!(grant["setIds"], json!([a]), "{r}");
    assert_eq!(grant["permissions"], json!(["EMOTES", "SETS"]), "{r}");

    // Inside the list: emotes, rename, the channel's active set.
    ok(&w.gql(&bob, ADD_EMOTE, json!({ "s": a, "e": emote })).await);
    ok(&w
        .gql(&bob, RENAME_SET, json!({ "id": a, "n": "Main 2" }))
        .await);
    let r = w
        .gql(&bob, CHANNEL, json!({ "c": alice.twitch_id, "s": a }))
        .await;
    assert_eq!(
        r["data"]["updateChannelSettings"]["activeSetId"],
        a.as_str(),
        "{r}"
    );

    // Outside it: every set mutation is refused, and the set can't be shown on the channel.
    for (query, vars) in [
        (ADD_EMOTE, json!({ "s": b, "e": emote })),
        (RENAME_SET, json!({ "id": b, "n": "Nope" })),
        (DELETE_SET, json!({ "id": b })),
        (PRIVATE_SET, json!({ "id": b })),
    ] {
        assert_eq!(
            code(&w.gql(&bob, query, vars).await),
            "forbidden",
            "{query}"
        );
    }
    assert_eq!(
        code(
            &w.gql(&bob, CHANNEL, json!({ "c": alice.twitch_id, "s": b }))
                .await
        ),
        "forbidden"
    );
    // A limited editor can't make new sets for the owner, by hand or by copying.
    assert_eq!(
        code(
            &w.gql(&bob, CREATE_SET, json!({ "n": "New", "o": alice.tid() }))
                .await
        ),
        "forbidden"
    );
    assert_eq!(
        code(
            &w.gql(&bob, CLONE_SET, json!({ "id": a, "o": alice.tid() }))
                .await
        ),
        "forbidden"
    );
    // editorSets lists only the covered set; a private set outside the list is invisible.
    let r = w.gql(&bob, "{ editorSets { id } }", json!({})).await;
    assert_eq!(r["data"]["editorSets"], json!([{ "id": a }]), "{r}");
    ok(&w.gql(&alice, PRIVATE_SET, json!({ "id": b })).await);
    let r = w.gql(&bob, SET, json!({ "id": b })).await;
    assert_eq!(r["data"]["set"], Value::Null, "{r}");
    let r = w
        .gql(&bob, "{ editorOf { allSets setIds } }", json!({}))
        .await;
    assert_eq!(r["data"]["editorOf"][0]["setIds"], json!([a]), "{r}");

    // Opening the grant to every set lets them in at once, new sets included.
    let r = w
        .gql(
            &alice,
            UPDATE,
            json!({ "a": bob.tid(), "f": ["EMOTES", "SETS"], "s": [] }),
        )
        .await;
    assert_eq!(r["data"]["updateAccountEditor"]["allSets"], true, "{r}");
    ok(&w.gql(&bob, ADD_EMOTE, json!({ "s": b, "e": emote })).await);
    ok(&w
        .gql(&bob, CREATE_SET, json!({ "n": "New", "o": alice.tid() }))
        .await);

    // Leaving `setIds` out keeps the scope; limiting again takes access away at once.
    let r = w
        .gql(
            &alice,
            UPDATE,
            json!({ "a": bob.tid(), "f": ["EMOTES"], "s": [b] }),
        )
        .await;
    assert_eq!(
        r["data"]["updateAccountEditor"]["setIds"],
        json!([b]),
        "{r}"
    );
    let r = w
        .gql(
            &alice,
            UPDATE,
            json!({ "a": bob.tid(), "f": ["EMOTES", "SETS"], "s": null }),
        )
        .await;
    assert_eq!(
        r["data"]["updateAccountEditor"]["setIds"],
        json!([b]),
        "{r}"
    );
    assert_eq!(
        code(
            &w.gql(&bob, RENAME_SET, json!({ "id": a, "n": "Gone" }))
                .await
        ),
        "forbidden"
    );

    // Sets that aren't the owner's are refused.
    let theirs = w.set(&bob, "Bob's").await;
    let r = w
        .gql(
            &alice,
            UPDATE,
            json!({ "a": bob.tid(), "f": ["EMOTES"], "s": [theirs] }),
        )
        .await;
    assert_eq!(code(&r), "bad_set", "{r}");
    let r = w
        .gql(
            &alice,
            UPDATE,
            json!({ "a": bob.tid(), "f": ["EMOTES"], "s": ["nonsense"] }),
        )
        .await;
    assert_eq!(code(&r), "bad_set", "{r}");

    // Deleting a listed set takes it off the list.
    ok(&w.gql(&alice, DELETE_SET, json!({ "id": b })).await);
    let r = w
        .gql(&bob, "{ editorOf { allSets setIds } }", json!({}))
        .await;
    assert_eq!(r["data"]["editorOf"][0]["allSets"], false, "{r}");
    assert_eq!(r["data"]["editorOf"][0]["setIds"], json!([]), "{r}");

    // Every change is audited with the scope before and after.
    let mut conn = w.db.conn().await;
    let rows = audit::list(&mut conn, &audit::Filter::default(), 100)
        .await
        .unwrap();
    let added = rows
        .iter()
        .find(|e| e.action == "account.editor_added")
        .expect("an add row");
    let after = added.after_json.as_ref().unwrap();
    assert_eq!(after["all_sets"], false);
    assert_eq!(after["set_ids"], json!([ids::to_text(w.db.row(&a).await)]));
    assert!(
        rows.iter()
            .filter(|e| e.action == "account.editor_updated")
            .count()
            >= 3
    );
    drop(conn);
    w.db.close().await;
}

#[tokio::test]
async fn a_limited_manager_grants_within_their_sets_only() {
    let Some(w) = World::new().await else { return };
    let alice = w.user("2101").await;
    let carol = w.user("2102").await;
    let dave = w.user("2103").await;
    let erin = w.user("2104").await;
    let a = w.set(&alice, "Main").await;
    let b = w.set(&alice, "Subs").await;
    ok(&w
        .add(&alice, None, &carol, &["EMOTES", "EDITORS"], &[a.as_str()])
        .await);
    ok(&w.add(&alice, None, &erin, &["EMOTES"], &[]).await);

    // Every set, or a set carol doesn't have, is more than she holds.
    let r = w.add(&carol, Some(&alice), &dave, &["EMOTES"], &[]).await;
    assert_eq!(code(&r), "forbidden", "{r}");
    let r = w
        .add(
            &carol,
            Some(&alice),
            &dave,
            &["EMOTES"],
            &[a.as_str(), b.as_str()],
        )
        .await;
    assert_eq!(code(&r), "forbidden", "{r}");
    let r = w
        .add(&carol, Some(&alice), &dave, &["EMOTES"], &[a.as_str()])
        .await;
    ok(&r);
    // Erin works on every set, so carol can't change or remove her.
    let r = w
        .gql(
            &carol,
            UPDATE,
            json!({ "o": alice.tid(), "a": erin.tid(), "f": ["EMOTES"], "s": [a] }),
        )
        .await;
    assert_eq!(code(&r), "forbidden", "{r}");
    let r = w
        .gql(&carol, REMOVE, json!({ "o": alice.tid(), "a": erin.tid() }))
        .await;
    assert_eq!(code(&r), "forbidden", "{r}");
    // Dave is inside carol's sets: she can.
    ok(&w
        .gql(&carol, REMOVE, json!({ "o": alice.tid(), "a": dave.tid() }))
        .await);
    w.db.close().await;
}

#[tokio::test]
async fn the_new_flags_are_stored_and_channels_gates_channel_settings() {
    let Some(w) = World::new().await else { return };
    let alice = w.user("2201").await;
    let bob = w.user("2202").await;
    let carol = w.user("2203").await;
    let a = w.set(&alice, "Main").await;

    let reserved = [
        "EMOTE_UPLOAD",
        "EMOTE_MANAGE",
        "EMOTE_TRANSFER",
        "BILLING_PAYMENT",
        "BILLING_SUBSCRIPTION",
    ];
    let r = w.add(&alice, None, &bob, &reserved, &[]).await;
    assert_eq!(
        r["data"]["addAccountEditor"]["permissions"],
        json!(reserved),
        "{r}"
    );
    // They grant nothing yet.
    assert_eq!(
        code(
            &w.gql(&bob, CHANNEL, json!({ "c": alice.twitch_id, "s": a }))
                .await
        ),
        "forbidden"
    );
    assert_eq!(
        code(&w.gql(&bob, RENAME_SET, json!({ "id": a, "n": "x" })).await),
        "forbidden"
    );

    // `channels` alone: the channel's settings, not the sets.
    let r = w
        .gql(&alice, UPDATE, json!({ "a": bob.tid(), "f": ["CHANNELS"] }))
        .await;
    assert_eq!(
        r["data"]["updateAccountEditor"]["permissions"],
        json!(["CHANNELS"]),
        "{r}"
    );
    let r = w
        .gql(&bob, CHANNEL, json!({ "c": alice.twitch_id, "s": a }))
        .await;
    assert_eq!(
        r["data"]["updateChannelSettings"]["activeSetId"],
        a.as_str(),
        "{r}"
    );
    assert_eq!(
        code(&w.gql(&bob, RENAME_SET, json!({ "id": a, "n": "x" })).await),
        "forbidden"
    );

    // `sets` keeps allowing channel settings, and is listed as stored.
    let r = w.add(&alice, None, &carol, &["SETS"], &[]).await;
    assert_eq!(
        r["data"]["addAccountEditor"]["permissions"],
        json!(["SETS"]),
        "{r}"
    );
    let r = w
        .gql(&carol, CHANNEL, json!({ "c": alice.twitch_id, "s": null }))
        .await;
    ok(&r);

    // An editor with `sets` and `editors` may hand out `channels` (sets brings it), but one with
    // `channels` and `editors` can't hand out `sets`.
    let dave = w.user("2204").await;
    ok(&w
        .gql(
            &alice,
            UPDATE,
            json!({ "a": carol.tid(), "f": ["SETS", "EDITORS"] }),
        )
        .await);
    ok(&w.add(&carol, Some(&alice), &dave, &["CHANNELS"], &[]).await);
    ok(&w
        .gql(
            &alice,
            UPDATE,
            json!({ "a": carol.tid(), "f": ["CHANNELS", "EDITORS"] }),
        )
        .await);
    let erin = w.user("2205").await;
    let r = w.add(&carol, Some(&alice), &erin, &["SETS"], &[]).await;
    assert_eq!(code(&r), "forbidden", "{r}");
    w.db.close().await;
}

#[tokio::test]
async fn at_most_fifty_editors() {
    let Some(w) = World::with_rates(Rates {
        editors: Rate::per_hour(60),
        ..Rates::default()
    })
    .await
    else {
        return;
    };
    let alice = w.user("2301").await;
    for n in 0..50 {
        let editor = w.user(&format!("24{n:02}")).await;
        let r = w.add(&alice, None, &editor, &["EMOTES"], &[]).await;
        assert!(r["errors"].is_null(), "{n}: {r}");
    }
    let extra = w.user("2500").await;
    let r = w.add(&alice, None, &extra, &["EMOTES"], &[]).await;
    assert_eq!(code(&r), "too_many_editors", "{r}");
    let message = r["errors"][0]["message"].as_str().unwrap_or_default();
    assert!(message.contains("50 editors"), "{message}");
    w.db.close().await;
}

#[tokio::test]
async fn anyone_sees_who_edits_an_account_but_not_what_they_may_do() {
    let Some(w) = World::new().await else { return };
    let alice = w.user("2601").await;
    let bob = w.user("2602").await;
    ok(&w
        .add(&alice, None, &bob, &["EMOTES", "BILLING_PAYMENT"], &[])
        .await);
    let r = w
        .send(
            None,
            "query($a: String!) { accountProfile(accountId: $a) { editors { id displayName } } }",
            json!({ "a": alice.tid() }),
        )
        .await;
    assert_eq!(
        r["data"]["accountProfile"]["editors"],
        json!([{ "id": w.db.public(bob.id).await, "displayName": "user2602" }]),
        "{r}"
    );
    // The flags aren't readable without the editors permission.
    let r = w
        .send(
            None,
            "query($a: String) { accountEditors(accountId: $a) { permissions } }",
            json!({ "a": alice.tid() }),
        )
        .await;
    assert!(!r["errors"].is_null(), "{r}");
    w.db.close().await;
}

#[tokio::test]
async fn default_sets_are_backfilled_once_for_accounts_without_one() {
    let Some(w) = World::new().await else { return };
    let mut conn = w.db.conn().await;
    let sign = |id: &'static str, name: &'static str| Profile {
        platform: Platform::Twitch,
        platform_user_id: id,
        login: name,
        display_name: name,
        avatar: None,
    };
    let plain = accounts::sign_in(&mut conn, sign("2701", "Plain"))
        .await
        .unwrap()
        .account
        .id;
    let has_one = accounts::sign_in(&mut conn, sign("2702", "HasOne"))
        .await
        .unwrap()
        .account
        .id;
    let gone = accounts::sign_in(&mut conn, sign("2703", "Gone"))
        .await
        .unwrap()
        .account
        .id;
    sets::create_default(&mut conn, has_one, "hasone")
        .await
        .unwrap();
    sqlx::query("UPDATE accounts SET deleted_at = now() WHERE id = $1")
        .bind(gone)
        .execute(&mut *conn)
        .await
        .unwrap();

    let backfill = include_str!("../../../crates/db/migrations/20261207000011_default_sets.sql");
    for _ in 0..2 {
        sqlx::raw_sql(backfill).execute(&mut *conn).await.unwrap();
    }

    let plain_sets = sets::list_for_owner(&mut conn, plain).await.unwrap();
    assert_eq!(plain_sets.len(), 1, "{plain_sets:?}");
    assert_eq!(plain_sets[0].name, "Plain");
    assert_eq!(plain_sets[0].kind, cyex_tv_shared::model::SetKind::Channel);
    assert_eq!(
        plain_sets[0].visibility,
        cyex_tv_shared::model::SetVisibility::Listed
    );
    // A set of that name (any case) already counts.
    assert_eq!(
        sets::list_for_owner(&mut conn, has_one)
            .await
            .unwrap()
            .len(),
        1
    );
    assert!(
        sets::list_for_owner(&mut conn, gone)
            .await
            .unwrap()
            .is_empty()
    );

    // Its ID sorts with the account's creation: before sets made later.
    let later = sets::create_with(
        &mut conn,
        plain,
        "Later",
        cyex_tv_shared::model::SetKind::Channel,
        None,
    )
    .await
    .unwrap();
    let ordered = sets::list_for_owner(&mut conn, plain).await.unwrap();
    assert_eq!(ordered[0].name, "Plain");
    assert_eq!(ordered[1].id, later.id);

    // A new account's default set is cut to the 100 characters a set name may have.
    let long = "x".repeat(150);
    let made = sets::create_default(&mut conn, plain, &long).await.unwrap();
    assert_eq!(made.name.chars().count(), 100);
    drop(conn);
    w.db.close().await;
}
