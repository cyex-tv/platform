//! The site announcement (who may set it, its time window, the audit trail), notification
//! preferences (a kind turned off is never created) and the public collection (private by
//! default; when shown, only what it is, since when and how it came). Through the real router
//! against a real Postgres; runs when `CYEX_TV_TEST_DATABASE_URL` is set (CI sets it), skips
//! otherwise. The owner is `twitch:900`.

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
use cyex_tv_db::sessions::{self, SessionKind};
use cyex_tv_db::{ids, roles};
use cyex_tv_shared::model::Platform;
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

fn code(response: &Value) -> &str {
    response["errors"][0]["extensions"]["code"]
        .as_str()
        .unwrap_or_default()
}

impl World {
    async fn new() -> Option<Self> {
        let db = TestDb::new().await?;
        let cfg = AuthConfig::from_source(|name| {
            (name == "CYEX_TV_OWNER_IDS").then(|| "twitch:900".to_owned())
        })
        .unwrap();
        let state = AppState::new(db.pool.clone(), Arc::new(Auth::new(cfg).unwrap()));
        Some(Self {
            app: cyex_tv_api::app(state),
            db,
        })
    }

    /// Signs in `twitch_id`; with `perms`, holding a role that grants them.
    async fn user(&self, twitch_id: &str, perms: &[&str]) -> User {
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
        if !perms.is_empty() {
            let role = roles::create(
                &mut conn,
                &roles::NewRole {
                    name: &format!("staff {twitch_id}"),
                    description: None,
                    position: 10,
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

    async fn send(&self, user: Option<&User>, query: &str, variables: Value) -> Value {
        let mut request = Request::post("/v1/gql").header("content-type", "application/json");
        if let Some(u) = user {
            request = request.header("authorization", format!("Bearer {}", u.token));
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
        self.send(Some(user), query, variables).await
    }

    async fn anon(&self, query: &str, variables: Value) -> Value {
        self.send(None, query, variables).await
    }

    async fn sql(&self, sql: &str) {
        let mut conn = self.db.conn().await;
        for statement in sql.split(';').map(str::trim).filter(|s| !s.is_empty()) {
            sqlx::query(statement).execute(&mut *conn).await.unwrap();
        }
    }

    async fn count(&self, sql: &str) -> i64 {
        let mut conn = self.db.conn().await;
        sqlx::query_scalar(sql).fetch_one(&mut *conn).await.unwrap()
    }
}

const ACTIVE: &str = "{ activeAnnouncement { id kind text linkUrl linkLabel badgeId startsAt endsAt live emote { id name } } }";
const LIST: &str = "{ announcements { id text live endedAt } }";
const CREATE: &str = "mutation($i: AnnouncementInput!) { createAnnouncement(input: $i) { id kind text live startsAt endsAt emote { id } } }";
const UPDATE: &str = "mutation($id: String!, $i: AnnouncementInput!) { updateAnnouncement(id: $id, input: $i) { id text live } }";
const END: &str = "mutation($id: String!) { endAnnouncement(id: $id) { id live endedAt } }";

#[tokio::test]
async fn announcements_are_for_site_announce_audited_and_follow_their_window() {
    let Some(w) = World::new().await else { return };
    let owner = w.user("900", &[]).await;
    let staff = w.user("1301", &["site.announce"]).await;
    let other_staff = w.user("1302", &["users.view", "shop.manage"]).await;
    let plain = w.user("1303", &[]).await;

    // Nothing yet, for anyone.
    let r = w.anon(ACTIVE, json!({})).await;
    assert!(r["data"]["activeAnnouncement"].is_null(), "{r}");

    // Only site.announce (or the owner) lists and writes.
    let input =
        json!({ "kind": "MAINTENANCE", "text": "  Uploads pause at 22:00 for ten minutes.  " });
    for who in [&plain, &other_staff] {
        assert_eq!(
            code(&w.gql(who, CREATE, json!({ "i": input })).await),
            "forbidden"
        );
        assert_eq!(code(&w.gql(who, LIST, json!({})).await), "forbidden");
    }
    assert_eq!(
        code(&w.anon(CREATE, json!({ "i": input })).await),
        "signed_out"
    );
    assert_eq!(w.count("SELECT count(*) FROM announcements").await, 0);

    // Bad input.
    for (bad, want) in [
        (json!({ "kind": "INFO", "text": "" }), "bad_text"),
        (json!({ "kind": "INFO", "text": "a\nb" }), "bad_text"),
        (
            json!({ "kind": "INFO", "text": "x", "linkUrl": "//evil.example" }),
            "bad_link",
        ),
        (
            json!({ "kind": "INFO", "text": "x", "linkUrl": "javascript:alert(1)" }),
            "bad_link",
        ),
        (
            json!({ "kind": "INFO", "text": "x", "linkLabel": "Go" }),
            "bad_label",
        ),
        (
            json!({ "kind": "INFO", "text": "x", "endsAt": "2001-01-01T00:00:00Z" }),
            "bad_time",
        ),
        (
            json!({ "kind": "INFO", "text": "x", "badgeId": ids::to_text(ids::new_id()) }),
            "bad_badge",
        ),
        (
            json!({ "kind": "INFO", "text": "x", "emoteId": ids::to_text(ids::new_id()) }),
            "bad_emote",
        ),
    ] {
        let r = w.gql(&staff, CREATE, json!({ "i": bad })).await;
        assert_eq!(code(&r), want, "{bad}: {r}");
    }

    // One now, with an approved emote.
    let emote = ids::new_id();
    w.sql(&format!(
        "INSERT INTO emotes (id, owner_account_id, name, state) VALUES ('{emote}', '{}', 'wave', 'approved')",
        plain.id
    ))
    .await;
    let r = w
        .gql(
            &staff,
            CREATE,
            json!({ "i": { "kind": "CAMPAIGN", "text": "Winter paints are in the shop.", "linkUrl": "/shop", "linkLabel": "See them", "emoteId": ids::to_text(emote) } }),
        )
        .await;
    let first = r["data"]["createAnnouncement"]["id"]
        .as_str()
        .unwrap_or_else(|| panic!("{r}"))
        .to_owned();
    assert_eq!(r["data"]["createAnnouncement"]["live"], true);
    let r = w.anon(ACTIVE, json!({})).await;
    let a = &r["data"]["activeAnnouncement"];
    assert_eq!(a["id"], first.as_str(), "{r}");
    assert_eq!(a["kind"], "CAMPAIGN");
    assert_eq!(a["linkUrl"], "/shop");
    assert_eq!(a["emote"]["name"], "wave");

    // A later one that hasn't started doesn't replace it; one that started does.
    let r = w
        .gql(
            &staff,
            CREATE,
            json!({ "i": { "kind": "INFO", "text": "Tomorrow.", "startsAt": "2999-01-01T00:00:00Z" } }),
        )
        .await;
    assert_eq!(r["data"]["createAnnouncement"]["live"], false, "{r}");
    let r = w.anon(ACTIVE, json!({})).await;
    assert_eq!(r["data"]["activeAnnouncement"]["id"], first.as_str());
    let r = w.gql(&owner, CREATE, json!({ "i": input })).await;
    let second = r["data"]["createAnnouncement"]["id"]
        .as_str()
        .unwrap_or_else(|| panic!("{r}"))
        .to_owned();
    let r = w.anon(ACTIVE, json!({})).await;
    let a = &r["data"]["activeAnnouncement"];
    assert_eq!(a["id"], second.as_str(), "{r}");
    assert_eq!(a["text"], "Uploads pause at 22:00 for ten minutes.");
    assert!(a["emote"].is_null());

    // A window that is over hides it (set in the database: the API refuses an end in the past).
    w.sql(&format!(
        "UPDATE announcements SET starts_at = now() - interval '2 hours', ends_at = now() - interval '1 hour' WHERE id = '{}'",
        ids::from_text(&second).unwrap()
    ))
    .await;
    let r = w.anon(ACTIVE, json!({})).await;
    assert_eq!(r["data"]["activeAnnouncement"]["id"], first.as_str(), "{r}");

    // Change, then end; an ended one can't change and ending again is a no-op.
    let r = w
        .gql(
            &staff,
            UPDATE,
            json!({ "id": first, "i": { "kind": "CAMPAIGN", "text": "Winter paints, last days." } }),
        )
        .await;
    assert_eq!(
        r["data"]["updateAnnouncement"]["text"], "Winter paints, last days.",
        "{r}"
    );
    assert_eq!(
        code(&w.gql(&plain, END, json!({ "id": first })).await),
        "forbidden"
    );
    let r = w.gql(&staff, END, json!({ "id": first })).await;
    assert_eq!(r["data"]["endAnnouncement"]["live"], false, "{r}");
    assert!(r["data"]["endAnnouncement"]["endedAt"].is_string());
    w.gql(&staff, END, json!({ "id": first })).await;
    let r = w
        .gql(
            &staff,
            UPDATE,
            json!({ "id": first, "i": { "kind": "INFO", "text": "Back?" } }),
        )
        .await;
    assert_eq!(code(&r), "ended");
    let r = w.anon(ACTIVE, json!({})).await;
    assert!(r["data"]["activeAnnouncement"].is_null(), "{r}");
    let r = w
        .gql(
            &staff,
            UPDATE,
            json!({ "id": ids::to_text(ids::new_id()), "i": { "kind": "INFO", "text": "x" } }),
        )
        .await;
    assert_eq!(code(&r), "not_found");

    // Staff see all three; every change has its audit row, with who did it.
    let r = w.gql(&staff, LIST, json!({})).await;
    assert_eq!(
        r["data"]["announcements"].as_array().map(Vec::len),
        Some(3),
        "{r}"
    );
    let actions = |a: &str| {
        format!(
            "SELECT count(*) FROM audit_log WHERE target_kind = 'announcement' AND action = '{a}'"
        )
    };
    assert_eq!(w.count(&actions("announcement.create")).await, 3);
    assert_eq!(w.count(&actions("announcement.update")).await, 1);
    assert_eq!(w.count(&actions("announcement.end")).await, 1);
    assert_eq!(
        w.count(&format!(
            "SELECT count(*) FROM audit_log WHERE action = 'announcement.end' AND actor_account_id = '{}' AND target_id = '{first}'",
            staff.id
        ))
        .await,
        1
    );
    // The built-in Admin role holds the new permission.
    assert_eq!(
        w.count("SELECT count(*) FROM role_permissions rp JOIN roles r ON r.id = rp.role_id WHERE r.system = 'admin' AND rp.permission = 'site.announce'").await,
        1
    );
    w.db.close().await;
}

const PREFS: &str = "{ myNotificationPrefs { kind notify toast required } }";
const SET_PREFS: &str = "mutation($p: [NotificationPrefInput!]!) { setNotificationPrefs(prefs: $p) { kind notify toast required } }";

fn pref<'a>(r: &'a Value, root: &str, kind: &str) -> &'a Value {
    r["data"][root]
        .as_array()
        .and_then(|a| a.iter().find(|p| p["kind"] == kind))
        .unwrap_or_else(|| panic!("{kind} in {r}"))
}

/// Another api instance on the same database with the read cache on (a minute) and its own
/// feeder, as the api binary runs.
fn other_instance(db: &TestDb) -> (Router, tokio::task::JoinHandle<()>) {
    let cfg = AuthConfig::from_source(|_| None).unwrap();
    let state = AppState::new(db.pool.clone(), Arc::new(Auth::new(cfg).unwrap()))
        .with_read_cache(std::time::Duration::from_secs(60));
    let feeder = tokio::spawn({
        let (db, hub) = (state.db.clone(), state.hub.clone());
        async move {
            let mut feeder = cyex_tv_hub::feeder::Feeder::new(0)
                .with_gap_wait(std::time::Duration::from_millis(50));
            loop {
                while let Ok((_, true)) = feeder.tick(&db, &hub).await {}
                tokio::time::sleep(std::time::Duration::from_millis(20)).await;
            }
        }
    });
    (cyex_tv_api::app(state), feeder)
}

async fn active_on(app: &Router) -> Value {
    let request = Request::post("/v1/gql")
        .header("content-type", "application/json")
        .body(Body::from(json!({ "query": ACTIVE }).to_string()))
        .unwrap();
    let response = app.clone().oneshot(request).await.unwrap();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let body: Value = serde_json::from_slice(&bytes).unwrap();
    body["data"]["activeAnnouncement"].clone()
}

/// Creating, changing and ending an announcement drops the cached one on every instance, not
/// only the one that made the change (`cache.forget` through the outbox).
#[tokio::test]
async fn announcement_changes_reach_every_instance() {
    let Some(w) = World::new().await else { return };
    let (there, feeder) = other_instance(&w.db);
    let staff = w.user("1311", &["site.announce"]).await;
    // Cached there: nothing.
    assert!(active_on(&there).await.is_null());

    let r = w
        .gql(
            &staff,
            CREATE,
            json!({ "i": { "kind": "INFO", "text": "First line." } }),
        )
        .await;
    let id = r["data"]["createAnnouncement"]["id"]
        .as_str()
        .unwrap_or_else(|| panic!("{r}"))
        .to_owned();
    common::wait_until("the other instance to show the new one", || async {
        active_on(&there).await["id"] == id.as_str()
    })
    .await;

    let r = w
        .gql(
            &staff,
            UPDATE,
            json!({ "id": id, "i": { "kind": "INFO", "text": "Second line." } }),
        )
        .await;
    assert!(r["errors"].is_null(), "{r}");
    common::wait_until("the other instance to show the change", || async {
        active_on(&there).await["text"] == "Second line."
    })
    .await;

    let r = w.gql(&staff, END, json!({ "id": id })).await;
    assert!(r["errors"].is_null(), "{r}");
    common::wait_until("the other instance to drop the ended one", || async {
        active_on(&there).await.is_null()
    })
    .await;
    feeder.abort();
    w.db.close().await;
}

#[tokio::test]
async fn a_kind_turned_off_is_never_created() {
    let Some(w) = World::new().await else { return };
    let granter = w.user("1401", &["cosmetics.grant"]).await;
    let dan = w.user("1402", &[]).await;
    let eve = w.user("1403", &[]).await;

    // Defaults: everything on and popping up; the required kinds say so.
    let r = w.gql(&dan, PREFS, json!({})).await;
    let p = pref(&r, "myNotificationPrefs", "cosmetic.granted");
    assert_eq!(
        (p["notify"].clone(), p["toast"].clone()),
        (json!(true), json!(true))
    );
    assert_eq!(
        pref(&r, "myNotificationPrefs", "emote.transfer_requested")["required"],
        true
    );
    assert_eq!(code(&w.anon(PREFS, json!({})).await), "signed_out");

    // Refused: unknown kinds and switching off a required one.
    let r = w
        .gql(
            &dan,
            SET_PREFS,
            json!({ "p": [{ "kind": "emote.everything", "notify": false, "toast": false }] }),
        )
        .await;
    assert_eq!(code(&r), "unknown_kind");
    let r = w
        .gql(
            &dan,
            SET_PREFS,
            json!({ "p": [{ "kind": "emote.taken_down", "notify": false, "toast": true }] }),
        )
        .await;
    assert_eq!(code(&r), "required_kind");

    // Dan turns grants off; Eve only stops them popping up.
    let r = w
        .gql(
            &dan,
            SET_PREFS,
            json!({ "p": [{ "kind": "cosmetic.granted", "notify": false, "toast": false }] }),
        )
        .await;
    assert_eq!(
        pref(&r, "setNotificationPrefs", "cosmetic.granted")["notify"],
        false,
        "{r}"
    );
    // Other kinds keep their setting.
    assert_eq!(
        pref(&r, "setNotificationPrefs", "cosmetic.revoked")["notify"],
        true
    );
    let r = w
        .gql(
            &eve,
            SET_PREFS,
            json!({ "p": [{ "kind": "cosmetic.granted", "notify": true, "toast": false }] }),
        )
        .await;
    assert_eq!(
        pref(&r, "setNotificationPrefs", "cosmetic.granted")["toast"],
        false,
        "{r}"
    );
    let r = w.gql(&eve, PREFS, json!({})).await;
    assert_eq!(
        pref(&r, "myNotificationPrefs", "cosmetic.granted")["toast"],
        false
    );

    // A grant to both: Eve gets the notification, Dan's is never stored.
    let paint = ids::new_id();
    w.sql(&format!(
        "INSERT INTO paints (id, name, kind) VALUES ('{paint}', 'Frost', 'linear_gradient')"
    ))
    .await;
    for who in [&dan, &eve] {
        let r = w
            .gql(
                &granter,
                "mutation($a: String!, $c: String!) { grantCosmetic(input: { accountId: $a, kind: PAINT, cosmeticId: $c, reason: \"Test\" }) { cosmeticId } }",
                json!({ "a": who.tid(), "c": ids::to_text(paint) }),
            )
            .await;
        assert!(r["errors"].is_null(), "{r}");
    }
    let stored = |id: Uuid| {
        format!(
            "SELECT count(*) FROM notifications WHERE account_id = '{id}' AND kind = 'cosmetic.granted'"
        )
    };
    assert_eq!(w.count(&stored(dan.id)).await, 0);
    assert_eq!(w.count(&stored(eve.id)).await, 1);
    // Revoking still tells Dan: only the kind he turned off is skipped.
    let r = w
        .gql(
            &granter,
            "mutation($a: String!, $c: String!) { revokeCosmetic(accountId: $a, kind: PAINT, cosmeticId: $c, reason: \"Test\") }",
            json!({ "a": dan.tid(), "c": ids::to_text(paint) }),
        )
        .await;
    assert!(r["errors"].is_null(), "{r}");
    assert_eq!(
        w.count(&format!(
            "SELECT count(*) FROM notifications WHERE account_id = '{}' AND kind = 'cosmetic.revoked'",
            dan.id
        ))
        .await,
        1
    );
    w.db.close().await;
}

const COLLECTION: &str = "query($a: String!) { accountCollection(accountId: $a) { accountId public items { kind cosmeticId name ownedSince source } } }";
const SHOW: &str =
    "mutation($p: Boolean!) { setCollectionPublic(public: $p) { public items { name } } }";
const PROFILE: &str = "query($a: String!) { accountProfile(accountId: $a) { collectionPublic } }";

#[tokio::test]
async fn the_collection_is_private_until_shown_and_then_only_says_what_and_how() {
    let Some(w) = World::new().await else { return };
    let granter = w.user("1501", &["cosmetics.grant"]).await;
    let fay = w.user("1502", &[]).await;
    let gus = w.user("1503", &[]).await;
    let f = fay.tid();

    let (frost, ember) = (ids::new_id(), ids::new_id());
    w.sql(&format!(
        "INSERT INTO paints (id, name, kind) VALUES ('{frost}', 'Frost', 'linear_gradient'), ('{ember}', 'Ember', 'linear_gradient');
         INSERT INTO paint_entitlements (account_id, paint_id, source, granted_at) VALUES ('{}', '{ember}', 'event', now() - interval '1 day')",
        fay.id
    ))
    .await;
    let r = w
        .gql(
            &granter,
            "mutation($a: String!, $c: String!) { grantCosmetic(input: { accountId: $a, kind: PAINT, cosmeticId: $c, reason: \"Secret reason\" }) { cosmeticId } }",
            json!({ "a": f, "c": ids::to_text(frost) }),
        )
        .await;
    assert!(r["errors"].is_null(), "{r}");

    // Off by default: nobody else sees it, signed in or out; the profile says so.
    for r in [
        w.anon(COLLECTION, json!({ "a": f })).await,
        w.gql(&gus, COLLECTION, json!({ "a": f })).await,
        w.gql(&granter, COLLECTION, json!({ "a": f })).await,
    ] {
        assert!(r["errors"].is_null(), "{r}");
        assert!(r["data"]["accountCollection"].is_null(), "{r}");
    }
    let r = w.anon(PROFILE, json!({ "a": f })).await;
    assert_eq!(
        r["data"]["accountProfile"]["collectionPublic"], false,
        "{r}"
    );
    // Fay sees her own, marked private.
    let r = w.gql(&fay, COLLECTION, json!({ "a": f })).await;
    assert_eq!(r["data"]["accountCollection"]["public"], false, "{r}");
    assert_eq!(
        r["data"]["accountCollection"]["items"]
            .as_array()
            .map(Vec::len),
        Some(2)
    );
    assert_eq!(
        code(&w.anon(SHOW, json!({ "p": true })).await),
        "signed_out"
    );

    // On: everyone sees what, since when and how, newest first; nothing about who or why.
    let r = w.gql(&fay, SHOW, json!({ "p": true })).await;
    assert_eq!(r["data"]["setCollectionPublic"]["public"], true, "{r}");
    let r = w.anon(PROFILE, json!({ "a": f })).await;
    assert_eq!(r["data"]["accountProfile"]["collectionPublic"], true);
    let r = w.anon(COLLECTION, json!({ "a": f })).await;
    let items = r["data"]["accountCollection"]["items"].as_array().unwrap();
    assert_eq!(items.len(), 2, "{r}");
    assert_eq!(items[0]["name"], "Frost");
    assert_eq!(items[0]["source"], "STAFF");
    assert_eq!(items[0]["kind"], "PAINT");
    assert_eq!(items[1]["name"], "Ember");
    assert_eq!(items[1]["source"], "EVENT");
    assert!(items[0]["ownedSince"].is_string());
    assert!(!r.to_string().contains("Secret reason"));
    // The type itself has nothing more to ask for.
    let r = w
        .anon(
            "{ __type(name: \"CollectionItem\") { fields { name } } }",
            json!({}),
        )
        .await;
    let mut fields: Vec<&str> = r["data"]["__type"]["fields"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["name"].as_str().unwrap())
        .collect();
    fields.sort_unstable();
    assert_eq!(
        fields,
        ["cosmeticId", "kind", "name", "ownedSince", "source"]
    );

    // A blocked account's collection is gone for everyone else; off again hides it.
    w.sql(&format!(
        "UPDATE accounts SET blocked_at = now() WHERE id = '{}'",
        fay.id
    ))
    .await;
    let r = w.gql(&gus, COLLECTION, json!({ "a": f })).await;
    assert!(r["data"]["accountCollection"].is_null(), "{r}");
    w.sql(&format!(
        "UPDATE accounts SET blocked_at = NULL WHERE id = '{}'",
        fay.id
    ))
    .await;
    w.gql(&fay, SHOW, json!({ "p": false })).await;
    let r = w.gql(&gus, COLLECTION, json!({ "a": f })).await;
    assert!(r["data"]["accountCollection"].is_null(), "{r}");
    w.db.close().await;
}
