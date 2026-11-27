//! Personal set slots (ADR 0038): every account's personal set holds 3 emotes, or the number a
//! role it holds gives through the `personal_set_slots` role limit (a subscriber role: 10), or what
//! staff with `sets.edit_any` set by hand (an override, audited); the owner can't change it.
//! Members beyond a lowered capacity stay and no new one goes in until the set is under the limit.
//! Through the real router against a real Postgres; runs when `CYEX_TV_TEST_DATABASE_URL` is set
//! (CI sets it) and skips otherwise.

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

fn ok(response: &Value) {
    assert!(response["errors"].is_null(), "{response}");
}

const CAPACITY: &str = "mutation($id: String!, $c: Int!) {
    updateSet(id: $id, input: { capacity: $c }) { capacity }
}";
const RANGE: &str =
    "query($id: String!) { setCapacityRange(id: $id) { min max ownerMax used staff } }";
const ADD_EMOTE: &str = "mutation($s: String!, $e: String!, $n: String) {
    addSetEmote(setId: $s, emoteId: $e, input: { name: $n }) { name }
}";
const RESET: &str = "mutation($id: String!) {
    updateSet(id: $id, input: { resetCapacity: true }) { capacity capacityOverride }
}";
const MY_SETS: &str = "{ mySets { id kind capacity capacityOverride slotsUsed } }";

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

    async fn grant(&self, owner: &User, editor: &User, flags: &[&str]) {
        let mut conn = self.db.conn().await;
        let flags: Vec<String> = flags.iter().map(|f| (*f).to_owned()).collect();
        editors::add(&mut conn, owner.id, editor.id, &flags, Some(owner.id))
            .await
            .unwrap();
    }

    /// A role holding `permissions` and the role limits in `limits`, given to `user`.
    async fn role(&self, user: &User, name: &str, permissions: &[&str], limits: Value) -> Uuid {
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
        role.id
    }

    async fn capacity(&self, by: &User, set: &str, capacity: u32) -> Value {
        self.gql(by, CAPACITY, json!({ "id": set, "c": capacity }))
            .await
    }

    /// The user's personal set, made as sign-up makes it.
    async fn personal(&self, user: &User) -> String {
        let mut conn = self.db.conn().await;
        let set = sets::ensure_personal(&mut conn, user.id).await.unwrap();
        ids::to_text(set.id)
    }

    async fn add(&self, by: &User, set: &str, emote: &str, name: Option<&str>) -> Value {
        self.gql(by, ADD_EMOTE, json!({ "s": set, "e": emote, "n": name }))
            .await
    }

    /// The personal set as `mySets` shows it.
    async fn mine(&self, user: &User) -> Value {
        let r = self.gql(user, MY_SETS, json!({})).await;
        ok(&r);
        r["data"]["mySets"]
            .as_array()
            .unwrap()
            .iter()
            .find(|s| s["kind"] == "PERSONAL")
            .cloned()
            .expect("a personal set")
    }
}

#[tokio::test]
async fn a_personal_set_holds_three_and_a_full_one_answers_409_with_the_numbers() {
    let Some(w) = World::new().await else { return };
    let alice = w.user("5101").await;
    let set = w.personal(&alice).await;
    // Made once: asking again finds the same set.
    assert_eq!(w.personal(&alice).await, set);
    let mine = w.mine(&alice).await;
    assert_eq!(mine["capacity"], 3, "{mine}");
    assert_eq!(mine["capacityOverride"], Value::Null, "{mine}");

    for n in 0..3 {
        let e = w.emote(&alice, &format!("slot{n}")).await;
        ok(&w.add(&alice, &set, &e, None).await);
    }
    let fourth = w.emote(&alice, "slot3").await;
    let full = w.add(&alice, &set, &fourth, None).await;
    assert_eq!(code(&full), "set_full", "{full}");
    assert_eq!(full["errors"][0]["extensions"]["limit"], 3, "{full}");
    assert_eq!(full["errors"][0]["extensions"]["current"], 3, "{full}");
    w.db.close().await;
}

#[tokio::test]
async fn a_subscriber_role_gives_ten_and_losing_it_keeps_the_members_but_stops_adds() {
    let Some(w) = World::new().await else { return };
    let alice = w.user("5201").await;
    let set = w.personal(&alice).await;
    let role = w
        .role(
            &alice,
            "Test subscriber 5201",
            &[],
            json!({ "personal_set_slots": 10 }),
        )
        .await;
    let mine = w.mine(&alice).await;
    assert_eq!(mine["capacity"], 10, "{mine}");
    let mut added = Vec::new();
    for n in 0..5 {
        let e = w.emote(&alice, &format!("sub{n}")).await;
        ok(&w.add(&alice, &set, &e, None).await);
        added.push(e);
    }
    // The role goes: 3 slots again, five members stay, no new one goes in.
    {
        let mut conn = w.db.conn().await;
        roles::unassign(&mut conn, alice.id, role).await.unwrap();
    }
    let mine = w.mine(&alice).await;
    assert_eq!(mine["capacity"], 3, "{mine}");
    assert_eq!(mine["slotsUsed"], 5, "{mine}");
    let extra = w.emote(&alice, "extra").await;
    let full = w.add(&alice, &set, &extra, None).await;
    assert_eq!(code(&full), "set_full", "{full}");
    assert_eq!(full["errors"][0]["extensions"]["limit"], 3, "{full}");
    assert_eq!(full["errors"][0]["extensions"]["current"], 5, "{full}");
    // Down to 2 members (under 3): adds work again.
    for e in &added[..3] {
        let removed = w
            .gql(
                &alice,
                "mutation($s: String!, $e: String!) { removeSetEmote(setId: $s, emoteId: $e) }",
                json!({ "s": set, "e": e }),
            )
            .await;
        ok(&removed);
    }
    ok(&w.add(&alice, &set, &extra, None).await);
    w.db.close().await;
}

#[tokio::test]
async fn only_staff_override_the_slots_and_the_owner_cannot() {
    let Some(w) = World::new().await else { return };
    let alice = w.user("5301").await;
    let staff = w.user("5302").await;
    let set = w.personal(&alice).await;
    w.role(&staff, "Test set staff 5302", &["sets.edit_any"], json!({}))
        .await;

    // The owner can't set a number or reset one, and the range says nothing to them.
    assert_eq!(code(&w.capacity(&alice, &set, 8).await), "forbidden");
    assert_eq!(
        code(&w.gql(&alice, RESET, json!({ "id": set })).await),
        "forbidden"
    );
    let r = w.gql(&alice, RANGE, json!({ "id": set })).await;
    assert!(r["data"]["setCapacityRange"].is_null(), "{r}");
    // Neither can an account editor with `sets`, nor a stranger.
    let editor = w.user("5303").await;
    w.grant(&alice, &editor, &["sets", "emotes"]).await;
    assert_eq!(code(&w.capacity(&editor, &set, 8).await), "forbidden");
    let stranger = w.user("5304").await;
    assert_eq!(code(&w.capacity(&stranger, &set, 8).await), "forbidden");
    assert_eq!(w.mine(&alice).await["capacity"], 3);

    // Staff set an override: it beats the default and a subscriber role, and is audited.
    let r = w.gql(&staff, RANGE, json!({ "id": set })).await;
    assert_eq!(r["data"]["setCapacityRange"]["staff"], true, "{r}");
    assert_eq!(r["data"]["setCapacityRange"]["max"], 1000, "{r}");
    let r = w.capacity(&staff, &set, 7).await;
    assert_eq!(r["data"]["updateSet"]["capacity"], 7, "{r}");
    assert_eq!(code(&w.capacity(&staff, &set, 1001).await), "bad_capacity");
    w.role(
        &alice,
        "Test subscriber 5301",
        &[],
        json!({ "personal_set_slots": 10 }),
    )
    .await;
    let mine = w.mine(&alice).await;
    assert_eq!(mine["capacity"], 7, "{mine}");
    assert_eq!(mine["capacityOverride"], 7, "{mine}");
    {
        let mut conn = w.db.conn().await;
        let rows = audit::list(&mut conn, &audit::Filter::default(), 50)
            .await
            .unwrap();
        let row = rows
            .iter()
            .find(|e| e.action == "set.capacity_changed" && e.actor_account_id == Some(staff.id))
            .expect("audited");
        assert_eq!(row.after_json.as_ref().unwrap()["capacity_override"], 7);
        assert_eq!(
            row.before_json.as_ref().unwrap()["capacity_override"],
            Value::Null
        );
    }
    // Taking the override away hands the decision back to the roles: 10 with the subscriber role.
    let r = w.gql(&staff, RESET, json!({ "id": set })).await;
    assert_eq!(r["data"]["updateSet"]["capacity"], 10, "{r}");
    assert_eq!(
        r["data"]["updateSet"]["capacityOverride"],
        Value::Null,
        "{r}"
    );
    w.db.close().await;
}

#[tokio::test]
async fn personal_sets_take_aliases() {
    let Some(w) = World::new().await else { return };
    let alice = w.user("5401").await;
    let set = w.personal(&alice).await;
    let e = w.emote(&alice, "original").await;
    let r = w.add(&alice, &set, &e, Some("shortcut")).await;
    assert_eq!(r["data"]["addSetEmote"]["name"], "shortcut", "{r}");
    let r = w
        .gql(
            &alice,
            "query($id: String!) { setEmotes(id: $id) { items { name emote { name } } } }",
            json!({ "id": set }),
        )
        .await;
    let item = &r["data"]["setEmotes"]["items"][0];
    assert_eq!(item["name"], "shortcut", "{r}");
    assert_eq!(item["emote"]["name"], "original", "{r}");
    w.db.close().await;
}
