//! Sets, amended 2026/11/22 (ADR 0014 and 0017): managing sets is its own editor permission,
//! apart from channel settings; every set is public; a set's capacity is 1 to the owner's
//! `set_capacity_max` role limit (1.000 by default), up to 1.000.000 for staff, never below what it
//! holds; and the migrations that carry old rows over. Through the real router against a real
//! Postgres; runs when `CYEX_TV_TEST_DATABASE_URL` is set (CI sets it) and skips otherwise.

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
use cyex_tv_db::{audit, channels, editors, ids, roles, sets};
use cyex_tv_shared::model::{EmoteState, Platform, SetKind, SetVisibility};
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

const CREATE_SET: &str = "mutation($n: String!, $k: SetKind!, $o: String) {
    createSet(input: { name: $n, kind: $k, ownerId: $o }) { id visibility capacity }
}";
const RENAME: &str =
    "mutation($id: String!, $n: String!) { updateSet(id: $id, input: { name: $n }) { name } }";
const CAPACITY: &str = "mutation($id: String!, $c: Int!) {
    updateSet(id: $id, input: { capacity: $c }) { capacity }
}";
const RANGE: &str =
    "query($id: String!) { setCapacityRange(id: $id) { min max ownerMax used staff } }";
const DELETE_SET: &str = "mutation($id: String!) { deleteSet(id: $id) }";
const ADD_EMOTE: &str =
    "mutation($s: String!, $e: String!) { addSetEmote(setId: $s, emoteId: $e) { name } }";
const CHANNEL: &str = "mutation($c: String!, $s: String, $p: Boolean) {
    updateChannelSettings(platform: TWITCH, id: $c, input: { activeSetId: $s, allowPersonalEmotes: $p }) {
        activeSetId allowPersonalEmotes
    }
}";

impl World {
    async fn new() -> Option<Self> {
        let db = TestDb::new().await?;
        let auth = Arc::new(Auth::new(AuthConfig::from_source(|_| None).unwrap()).unwrap());
        // Role limits are read on every request, so a role given mid-test counts at once.
        let limits =
            Limits::new(Rates::default(), false).with_account_ttl(std::time::Duration::ZERO);
        let state = AppState::new(db.pool.clone(), auth).with_limits(limits);
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

    async fn set(&self, by: &User, name: &str, kind: &str, owner: Option<&User>) -> Value {
        self.gql(
            by,
            CREATE_SET,
            json!({ "n": name, "k": kind, "o": owner.map(User::tid) }),
        )
        .await
    }

    async fn set_id(&self, owner: &User, name: &str) -> String {
        let r = self.set(owner, name, "CHANNEL", None).await;
        r["data"]["createSet"]["id"]
            .as_str()
            .unwrap_or_else(|| panic!("{r}"))
            .to_owned()
    }

    async fn grant(&self, owner: &User, editor: &User, flags: &[&str]) {
        let mut conn = self.db.conn().await;
        let flags: Vec<String> = flags.iter().map(|f| (*f).to_owned()).collect();
        editors::add(&mut conn, owner.id, editor.id, &flags, Some(owner.id))
            .await
            .unwrap();
    }

    /// A role holding `permissions` and the role limits in `limits`, given to `user`.
    async fn role(&self, user: &User, name: &str, permissions: &[&str], limits: Value) {
        let mut conn = self.db.conn().await;
        let role = roles::create(
            &mut conn,
            &roles::NewRole {
                name,
                description: None,
                position: 10,
                permissions,
                created_by: None,
            },
        )
        .await
        .unwrap();
        sqlx::query("UPDATE roles SET limits = $2 WHERE id = $1")
            .bind(role.id)
            .bind(limits)
            .execute(&mut *conn)
            .await
            .unwrap();
        roles::assign(&mut conn, user.id, role.id, None)
            .await
            .unwrap();
    }

    async fn capacity(&self, by: &User, set: &str, capacity: u32) -> Value {
        self.gql(by, CAPACITY, json!({ "id": set, "c": capacity }))
            .await
    }
}

#[tokio::test]
async fn managing_sets_and_channel_settings_are_separate_permissions() {
    let Some(w) = World::new().await else { return };
    let alice = w.user("3001").await;
    let bob = w.user("3002").await;
    let carol = w.user("3003").await;
    let main = w.set_id(&alice, "Main").await;
    w.grant(&alice, &bob, &["sets"]).await;
    w.grant(&alice, &carol, &["channels"]).await;

    // Bob, with `sets` only: makes, renames, resizes and deletes sets, and picks the active one.
    let r = w.set(&bob, "By Bob", "CHANNEL", Some(&alice)).await;
    ok(&r);
    let made = r["data"]["createSet"]["id"].as_str().unwrap().to_owned();
    ok(&w
        .gql(&bob, RENAME, json!({ "id": made, "n": "Renamed" }))
        .await);
    let r = w.capacity(&bob, &made, 300).await;
    assert_eq!(r["data"]["updateSet"]["capacity"], 300, "{r}");
    let r = w.gql(&bob, RANGE, json!({ "id": made })).await;
    assert_eq!(r["data"]["setCapacityRange"]["max"], 1000, "{r}");
    let r = w
        .gql(
            &bob,
            CHANNEL,
            json!({ "c": alice.twitch_id, "s": main, "p": null }),
        )
        .await;
    assert_eq!(
        r["data"]["updateChannelSettings"]["activeSetId"],
        main.as_str(),
        "{r}"
    );
    ok(&w.gql(&bob, DELETE_SET, json!({ "id": made })).await);
    // ...but not the channel's settings.
    let r = w
        .gql(
            &bob,
            CHANNEL,
            json!({ "c": alice.twitch_id, "s": null, "p": false }),
        )
        .await;
    assert_eq!(code(&r), "forbidden", "{r}");

    // Carol, with `channels` only: the channel's settings (and its active set, as before), but no
    // set management.
    let r = w
        .gql(
            &carol,
            CHANNEL,
            json!({ "c": alice.twitch_id, "s": null, "p": false }),
        )
        .await;
    assert_eq!(
        r["data"]["updateChannelSettings"]["allowPersonalEmotes"], false,
        "{r}"
    );
    let r = w
        .gql(
            &carol,
            CHANNEL,
            json!({ "c": alice.twitch_id, "s": main, "p": null }),
        )
        .await;
    ok(&r);
    let r = w.set(&carol, "By Carol", "CHANNEL", Some(&alice)).await;
    assert_eq!(code(&r), "forbidden", "{r}");
    for (query, vars) in [
        (RENAME, json!({ "id": main, "n": "Nope" })),
        (CAPACITY, json!({ "id": main, "c": 10 })),
        (DELETE_SET, json!({ "id": main })),
    ] {
        let r = w.gql(&carol, query, vars).await;
        assert_eq!(code(&r), "forbidden", "{query}: {r}");
    }
    let r = w.gql(&carol, RANGE, json!({ "id": main })).await;
    assert!(r["data"]["setCapacityRange"].is_null(), "{r}");
    w.db.close().await;
}

#[tokio::test]
async fn capacity_is_one_to_the_owners_limit_and_never_below_what_it_holds() {
    let Some(w) = World::new().await else { return };
    let alice = w.user("3101").await;
    let stranger = w.user("3102").await;
    let staff = w.user("3103").await;
    let set = w.set_id(&alice, "Main").await;

    // 1 to 1.000 by default.
    let r = w.gql(&alice, RANGE, json!({ "id": set })).await;
    assert_eq!(
        r["data"]["setCapacityRange"],
        json!({ "min": 1, "max": 1000, "ownerMax": 1000, "used": 0, "staff": false }),
        "{r}"
    );
    for n in [1, 1000, 250] {
        let r = w.capacity(&alice, &set, n).await;
        assert_eq!(r["data"]["updateSet"]["capacity"], n, "{r}");
    }
    assert_eq!(
        code(&w.capacity(&alice, &set, 1001).await),
        "capacity_over_limit"
    );
    assert_eq!(code(&w.capacity(&alice, &set, 0).await), "bad_capacity");
    assert_eq!(
        code(&w.capacity(&alice, &set, 1_000_001).await),
        "bad_capacity"
    );

    // Never below what it holds.
    for n in 0..3 {
        let e = w.emote(&alice, &format!("emo{n}")).await;
        ok(&w.gql(&alice, ADD_EMOTE, json!({ "s": set, "e": e })).await);
    }
    assert_eq!(code(&w.capacity(&alice, &set, 2).await), "capacity_too_low");
    let r = w.capacity(&alice, &set, 3).await;
    assert_eq!(r["data"]["updateSet"]["capacity"], 3, "{r}");
    let r = w.gql(&alice, RANGE, json!({ "id": set })).await;
    assert_eq!(r["data"]["setCapacityRange"]["min"], 3, "{r}");
    assert_eq!(r["data"]["setCapacityRange"]["used"], 3, "{r}");

    // A role raises the owner's limit (a future subscriber role: 2.500).
    w.role(
        &alice,
        "Subscriber",
        &[],
        json!({ "set_capacity_max": 2500 }),
    )
    .await;
    let r = w.capacity(&alice, &set, 2500).await;
    assert_eq!(r["data"]["updateSet"]["capacity"], 2500, "{r}");
    assert_eq!(
        code(&w.capacity(&alice, &set, 2501).await),
        "capacity_over_limit"
    );
    let r = w.gql(&alice, RANGE, json!({ "id": set })).await;
    assert_eq!(r["data"]["setCapacityRange"]["max"], 2500, "{r}");

    // Someone else can't, and learns nothing from the range.
    assert_eq!(code(&w.capacity(&stranger, &set, 10).await), "forbidden");
    let r = w.gql(&stranger, RANGE, json!({ "id": set })).await;
    assert!(r["data"]["setCapacityRange"].is_null(), "{r}");

    // Staff with sets.edit_any go up to 1.000.000, audited as staff.
    w.role(&staff, "Test set staff 3103", &["sets.edit_any"], json!({}))
        .await;
    let r = w.gql(&staff, RANGE, json!({ "id": set })).await;
    assert_eq!(r["data"]["setCapacityRange"]["max"], 1_000_000, "{r}");
    assert_eq!(r["data"]["setCapacityRange"]["staff"], true, "{r}");
    let r = w.capacity(&staff, &set, 1_000_000).await;
    assert_eq!(r["data"]["updateSet"]["capacity"], 1_000_000, "{r}");
    {
        let mut conn = w.db.conn().await;
        let rows = audit::list(&mut conn, &audit::Filter::default(), 50)
            .await
            .unwrap();
        let row = rows
            .iter()
            .find(|e| e.action == "set.capacity_changed" && e.actor_account_id == Some(staff.id))
            .expect("audited");
        assert_eq!(row.after_json.as_ref().unwrap()["capacity"], 1_000_000);
    }
    // The owner may bring it down again (not above what staff gave it).
    let r = w.capacity(&alice, &set, 600_000).await;
    assert_eq!(r["data"]["updateSet"]["capacity"], 600_000, "{r}");
    assert_eq!(
        code(&w.capacity(&alice, &set, 700_000).await),
        "capacity_over_limit"
    );

    // A personal set stays 1 to 5 for its owner.
    let r = w.set(&alice, "Mine", "PERSONAL", None).await;
    let personal = r["data"]["createSet"]["id"].as_str().unwrap().to_owned();
    let r = w.capacity(&alice, &personal, 2).await;
    assert_eq!(r["data"]["updateSet"]["capacity"], 2, "{r}");
    assert_eq!(
        code(&w.capacity(&alice, &personal, 6).await),
        "capacity_over_limit"
    );
    w.db.close().await;
}

#[tokio::test]
async fn every_new_set_is_public() {
    let Some(w) = World::new().await else { return };
    let alice = w.user("3201").await;
    for kind in ["CHANNEL", "PERSONAL"] {
        let r = w.set(&alice, kind, kind, None).await;
        assert_eq!(r["data"]["createSet"]["visibility"], "LISTED", "{r}");
    }
    w.db.close().await;
}

#[tokio::test]
async fn migrations_make_old_sets_public_and_keep_channel_settings_for_sets_grants() {
    let Some(w) = World::new().await else { return };
    let alice = w.user("3301").await;
    let bob = w.user("3302").await;
    let carol = w.user("3303").await;
    let mut conn = w.db.conn().await;

    // Rows as they could be before: an unlisted personal set, a private channel set, and grants of
    // `sets` alone (which brought `channels` with it) and `channels` alone.
    let personal = sets::create(&mut conn, alice.id, "Mine", SetKind::Personal)
        .await
        .unwrap();
    let private = sets::create(&mut conn, alice.id, "Secret", SetKind::Channel)
        .await
        .unwrap();
    sets::set_visibility(&mut conn, personal.id, SetVisibility::Unlisted)
        .await
        .unwrap();
    sets::set_visibility(&mut conn, private.id, SetVisibility::Private)
        .await
        .unwrap();
    editors::add(
        &mut conn,
        alice.id,
        bob.id,
        &["emotes".to_owned(), "sets".to_owned()],
        Some(alice.id),
    )
    .await
    .unwrap();
    editors::add(
        &mut conn,
        alice.id,
        carol.id,
        &["channels".to_owned()],
        Some(alice.id),
    )
    .await
    .unwrap();

    for sql in [
        include_str!(
            "../../../crates/db/migrations/20261215004101_editor_sets_without_channels.sql"
        ),
        include_str!("../../../crates/db/migrations/20261215004102_sets_all_public.sql"),
    ] {
        // Twice: running them again changes nothing.
        for _ in 0..2 {
            sqlx::raw_sql(sql).execute(&mut *conn).await.unwrap();
        }
    }

    for id in [personal.id, private.id] {
        let set = sets::get(&mut conn, id).await.unwrap().unwrap();
        assert_eq!(set.visibility, SetVisibility::Listed);
    }
    let perms = |editor: Uuid| {
        sqlx::query_scalar::<_, Vec<String>>(
            "SELECT permissions FROM account_editors WHERE account_id = $1 AND editor_id = $2",
        )
        .bind(alice.id)
        .bind(editor)
    };
    let bobs = perms(bob.id).fetch_one(&mut *conn).await.unwrap();
    assert_eq!(bobs, ["emotes", "sets", "channels"]);
    let carols = perms(carol.id).fetch_one(&mut *conn).await.unwrap();
    assert_eq!(carols, ["channels"]);
    drop(conn);
    w.db.close().await;
}
