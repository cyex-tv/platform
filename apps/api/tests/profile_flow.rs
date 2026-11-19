//! Public profiles: `accountProfile` (public connections without IDs, only roles marked public,
//! each channel's active set), the role `public` flag (roles.manage only, audited) and
//! `accountCosmetics` (by account ID, only live accounts showing something, at most 100).
//! Through the real router against a real Postgres; runs when `CYEX_TV_TEST_DATABASE_URL` is set
//! (CI sets it) and skips otherwise.

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
use cyex_tv_db::{audit, channels, ids, roles};
use cyex_tv_shared::model::Platform;
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

const PROFILE: &str = "query($a: String!) {
    accountProfile(accountId: $a) {
        account { id displayName }
        identities { platform login displayName }
        roles { id name }
        activeSets { platform login setId setName }
        createdAt
    }
}";

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

    async fn role(&self, name: &str, position: i32, perms: &[&str], holder: &User) -> Uuid {
        let mut conn = self.db.conn().await;
        let role = roles::create(
            &mut conn,
            &roles::NewRole {
                name,
                description: None,
                position,
                permissions: perms,
                created_by: None,
            },
        )
        .await
        .unwrap();
        roles::assign(&mut conn, holder.id, role.id, None)
            .await
            .unwrap();
        role.id
    }
}

#[tokio::test]
async fn profiles_show_only_public_things() {
    let Some(w) = World::new().await else { return };
    let admin = w.user("6001").await;
    let alice = w.user("6002").await;
    let stranger = w.user("6003").await;
    w.role("Admins", 100, &["roles.manage"], &admin).await;
    let artist = w.role("Artist", 10, &[], &alice).await;
    let _staff = w
        .role("Trust and safety", 50, &["emotes.review"], &alice)
        .await;
    {
        let mut conn = w.db.conn().await;
        accounts::link(
            &mut conn,
            alice.id,
            Profile {
                platform: Platform::Kick,
                platform_user_id: "987654321",
                login: "alicekick",
                display_name: "AliceKick",
                avatar: None,
            },
        )
        .await
        .unwrap();
    }

    // No role is public until someone with roles.manage says so.
    let r = w.post(None, PROFILE, json!({ "a": alice.tid() })).await;
    let p = &r["data"]["accountProfile"];
    assert_eq!(p["account"]["id"], alice.tid().as_str(), "{r}");
    assert_eq!(p["roles"], json!([]));
    assert_eq!(
        p["identities"],
        json!([
            { "platform": "TWITCH", "login": "user6002", "displayName": "user6002" },
            { "platform": "KICK", "login": "alicekick", "displayName": "AliceKick" }
        ])
    );
    // Logins and names only: no platform user IDs.
    let text = r.to_string();
    assert!(
        !text.contains("987654321") && !text.contains("platformUserId"),
        "{text}"
    );
    assert_eq!(p["activeSets"], json!([]));

    // Only roles.manage can mark a role public; it is audited.
    const UPDATE: &str = "mutation($id: String!, $p: Boolean) { updateRole(id: $id, input: { public: $p }) { name public } }";
    let r = w
        .post(
            Some(&stranger),
            UPDATE,
            json!({ "id": ids::to_text(artist), "p": true }),
        )
        .await;
    assert_eq!(code(&r), "forbidden", "{r}");
    let r = w
        .post(
            Some(&admin),
            UPDATE,
            json!({ "id": ids::to_text(artist), "p": true }),
        )
        .await;
    assert_eq!(r["data"]["updateRole"]["public"], true, "{r}");
    {
        let mut conn = w.db.conn().await;
        let rows = audit::list(&mut conn, &audit::Filter::default(), 50)
            .await
            .unwrap();
        let row = rows
            .iter()
            .find(|e| e.action == "role.updated")
            .expect("audited");
        assert_eq!(row.after_json.as_ref().unwrap()["public"], true);
        assert_eq!(row.before_json.as_ref().unwrap()["public"], false);
    }
    // Created public in one go.
    let r = w
        .post(
            Some(&admin),
            "mutation { createRole(input: { name: \"Partner\", position: 5, permissions: [], public: true }) { id public } }",
            json!({}),
        )
        .await;
    assert_eq!(r["data"]["createRole"]["public"], true, "{r}");

    // The active set shows; the staff role doesn't.
    let r = w
        .post(
            Some(&alice),
            "mutation { createSet(input: { name: \"Main\", kind: CHANNEL, visibility: PRIVATE }) { id } }",
            json!({}),
        )
        .await;
    let set = r["data"]["createSet"]["id"].as_str().unwrap().to_owned();
    let r = w
        .post(
            Some(&alice),
            "mutation($c: String!, $s: String) { updateChannelSettings(platform: TWITCH, id: $c, input: { activeSetId: $s }) { activeSetId } }",
            json!({ "c": alice.twitch_id, "s": set }),
        )
        .await;
    assert!(r["errors"].is_null(), "{r}");
    let r = w.post(None, PROFILE, json!({ "a": alice.tid() })).await;
    let p = &r["data"]["accountProfile"];
    assert_eq!(
        p["roles"],
        json!([{ "id": ids::to_text(artist), "name": "Artist" }]),
        "{r}"
    );
    assert_eq!(
        p["activeSets"],
        json!([{ "platform": "TWITCH", "login": "user6002", "setId": set, "setName": "Main" }])
    );
    assert!(!r.to_string().contains("Trust and safety"));

    // Unknown, blocked and deleted accounts have no profile.
    let r = w.post(None, PROFILE, json!({ "a": "nope" })).await;
    assert!(r["data"]["accountProfile"].is_null(), "{r}");
    {
        let mut conn = w.db.conn().await;
        accounts::set_blocked(&mut conn, alice.id, true)
            .await
            .unwrap();
        sqlx::query("UPDATE accounts SET deleted_at = now() WHERE id = $1")
            .bind(stranger.id)
            .execute(&mut *conn)
            .await
            .unwrap();
    }
    for who in [&alice, &stranger] {
        let r = w.post(None, PROFILE, json!({ "a": who.tid() })).await;
        assert!(r["data"]["accountProfile"].is_null(), "{r}");
    }
    w.db.close().await;
}

#[tokio::test]
async fn account_cosmetics_by_account_id() {
    let Some(w) = World::new().await else { return };
    let staff = w.user("6101").await;
    let alice = w.user("6102").await;
    let bob = w.user("6103").await;
    let plain = w.user("6104").await;
    w.role(
        "Cosmetics",
        100,
        &["cosmetics.manage", "cosmetics.grant"],
        &staff,
    )
    .await;
    let r = w
        .post(
            Some(&staff),
            "mutation($p: PaintInput!) { createPaint(input: $p) { id } }",
            json!({ "p": {
                "name": "Dusk",
                "kind": "LINEAR_GRADIENT",
                "angle": 90,
                "repeat": false,
                "color": "#ff8800",
                "stops": [{ "at": 0.0, "color": "#ff8800" }, { "at": 1.0, "color": "#8800ff" }],
                "shadows": []
            } }),
        )
        .await;
    let paint = r["data"]["createPaint"]["id"]
        .as_str()
        .unwrap_or_else(|| panic!("{r}"))
        .to_owned();
    for who in [&alice, &bob] {
        let r = w
            .post(
                Some(&staff),
                "mutation($a: String!, $c: String!) { grantCosmetic(input: { accountId: $a, kind: PAINT, cosmeticId: $c, reason: \"test\" }) { cosmeticId } }",
                json!({ "a": who.tid(), "c": paint }),
            )
            .await;
        assert!(r["errors"].is_null(), "{r}");
        let r = w
            .post(
                Some(who),
                "mutation($p: String) { selectCosmetics(paintId: $p) { paintId } }",
                json!({ "p": paint }),
            )
            .await;
        assert_eq!(
            r["data"]["selectCosmetics"]["paintId"],
            paint.as_str(),
            "{r}"
        );
    }

    const LOOKUP: &str = "query($ids: [String!]!) { accountCosmetics(accountIds: $ids) { accountId badge { id } paint { id name } } }";
    let ids_in = json!([alice.tid(), bob.tid(), plain.tid(), "nope", alice.tid()]);
    let r = w.post(None, LOOKUP, json!({ "ids": ids_in })).await;
    let list = r["data"]["accountCosmetics"]
        .as_array()
        .unwrap_or_else(|| panic!("{r}"));
    assert_eq!(
        list.len(),
        2,
        "only those showing something, once each: {r}"
    );
    assert!(
        list.iter()
            .all(|c| c["paint"]["name"] == "Dusk" && c["badge"].is_null())
    );

    // Blocked and deleted accounts never show.
    {
        let mut conn = w.db.conn().await;
        accounts::set_blocked(&mut conn, bob.id, true)
            .await
            .unwrap();
    }
    let r = w
        .post(None, LOOKUP, json!({ "ids": [alice.tid(), bob.tid()] }))
        .await;
    let shown: Vec<&str> = r["data"]["accountCosmetics"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["accountId"].as_str().unwrap())
        .collect();
    assert_eq!(shown, [alice.tid().as_str()], "{r}");

    let many: Vec<String> = (0..101).map(|_| alice.tid()).collect();
    let r = w.post(None, LOOKUP, json!({ "ids": many })).await;
    assert_eq!(code(&r), "too_many", "{r}");
    w.db.close().await;
}
