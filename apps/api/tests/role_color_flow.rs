//! Role colours: the `#rrggbb` is validated and stored lower case, only role managers set it, a
//! change is audited, and the profile's ring colour (`accountProfile.roleColor`) is the colour of
//! the most senior public, visible role that has one. Through the real router against a real
//! Postgres; runs when `CYEX_TV_TEST_DATABASE_URL` is set (CI sets it), skips otherwise. The owner
//! is `twitch:900`.

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
}

const CREATE: &str = "mutation($n: String!, $p: Int!, $c: String) { createRole(input: { name: $n, position: $p, permissions: [], color: $c }) { id color } }";
const UPDATE: &str =
    "mutation($id: String!, $c: String) { updateRole(id: $id, input: { color: $c }) { id color } }";
const LIST: &str = "{ roles { id name color } }";
const PROFILE: &str =
    "query($a: String!) { accountProfile(accountId: $a) { roleColor roles { name color } } }";

impl World {
    /// A role made straight in the database, shown on profiles, with an optional colour.
    async fn role(&self, name: &str, position: i32, perms: &[&str], color: Option<&str>) -> Uuid {
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
        roles::set_public(&mut conn, role.id, true).await.unwrap();
        if color.is_some() {
            roles::update(
                &mut conn,
                role.id,
                &roles::RolePatch {
                    color: Some(color),
                    ..roles::RolePatch::default()
                },
            )
            .await
            .unwrap();
        }
        role.id
    }

    async fn give(&self, user: &User, role: Uuid) {
        let mut conn = self.db.conn().await;
        roles::assign(&mut conn, user.id, role, None).await.unwrap();
    }

    async fn ring(&self, of: &User) -> Value {
        let r = self.gql(of, PROFILE, json!({ "a": of.tid() })).await;
        r["data"]["accountProfile"]["roleColor"].clone()
    }
}

#[tokio::test]
async fn colours_are_validated_and_stored_in_lower_case() {
    let Some(w) = World::new().await else { return };
    let owner = w.user("900").await;
    let made = w
        .gql(
            &owner,
            CREATE,
            json!({ "n": "Blue", "p": 10, "c": " #3B82F6 " }),
        )
        .await;
    assert_eq!(made["data"]["createRole"]["color"], "#3b82f6", "{made}");
    let id = made["data"]["createRole"]["id"]
        .as_str()
        .unwrap()
        .to_owned();
    // A role may have none.
    let plain = w.gql(&owner, CREATE, json!({ "n": "Plain", "p": 9 })).await;
    assert!(plain["data"]["createRole"]["color"].is_null(), "{plain}");
    for bad in [
        "blue",
        "#fff",
        "#12345g",
        "3b82f6a",
        "#3b82f6ff",
        "rgb(0,0,0)",
    ] {
        let r = w
            .gql(
                &owner,
                CREATE,
                json!({ "n": format!("Bad {bad}"), "p": 8, "c": bad }),
            )
            .await;
        assert_eq!(code(&r), "bad_color", "{bad}: {r}");
        let r = w.gql(&owner, UPDATE, json!({ "id": id, "c": bad })).await;
        assert_eq!(code(&r), "bad_color", "{bad}: {r}");
    }
    // Nothing was made by the refused creates, and the colour is unchanged.
    let list = w.gql(&owner, LIST, json!({})).await;
    let roles = list["data"]["roles"].as_array().unwrap();
    assert!(
        !roles
            .iter()
            .any(|r| r["name"].as_str().unwrap().starts_with("Bad "))
    );
    assert_eq!(
        roles.iter().find(|r| r["id"] == id.as_str()).unwrap()["color"],
        "#3b82f6"
    );
    // Updating changes it; leaving it out keeps it; an empty string clears it.
    let r = w
        .gql(&owner, UPDATE, json!({ "id": id, "c": "#D6409F" }))
        .await;
    assert_eq!(r["data"]["updateRole"]["color"], "#d6409f", "{r}");
    let r = w.gql(&owner, UPDATE, json!({ "id": id, "c": null })).await;
    assert_eq!(r["data"]["updateRole"]["color"], "#d6409f", "{r}");
    let r = w.gql(&owner, UPDATE, json!({ "id": id, "c": "" })).await;
    assert!(r["data"]["updateRole"]["color"].is_null(), "{r}");
    w.db.close().await;
}

#[tokio::test]
async fn only_role_managers_set_a_colour_and_it_is_audited() {
    let Some(w) = World::new().await else { return };
    let owner = w.user("900").await;
    let assigner = w.user("910").await;
    let nobody = w.user("911").await;
    let gives = w.role("Gives roles", 100, &["roles.assign"], None).await;
    w.give(&assigner, gives).await;
    let target = w.role("Target", 10, &[], None).await;
    let target_id = ids::to_text(target);
    for user in [&assigner, &nobody] {
        let r = w
            .gql(user, UPDATE, json!({ "id": target_id, "c": "#3b82f6" }))
            .await;
        assert_eq!(code(&r), "forbidden", "{r}");
        let r = w
            .gql(
                user,
                CREATE,
                json!({ "n": "Sneaky", "p": 5, "c": "#3b82f6" }),
            )
            .await;
        assert_eq!(code(&r), "forbidden", "{r}");
    }
    {
        let mut conn = w.db.conn().await;
        let role = roles::get(&mut conn, target).await.unwrap().unwrap();
        assert_eq!(role.color, None);
    }

    let r = w
        .gql(&owner, UPDATE, json!({ "id": target_id, "c": "#3b82f6" }))
        .await;
    assert_eq!(r["data"]["updateRole"]["color"], "#3b82f6", "{r}");
    let mut conn = w.db.conn().await;
    let entries = cyex_tv_db::audit::list(
        &mut conn,
        &cyex_tv_db::audit::Filter {
            target_kind: Some("role"),
            target_id: Some(&target_id),
            action: Some("role.updated"),
            ..cyex_tv_db::audit::Filter::default()
        },
        10,
    )
    .await
    .unwrap();
    let entry = entries.first().expect("the change is audited");
    assert!(entry.before_json.as_ref().unwrap()["color"].is_null());
    assert_eq!(entry.after_json.as_ref().unwrap()["color"], "#3b82f6");
    w.db.close().await;
}

#[tokio::test]
async fn the_ring_comes_from_the_highest_coloured_public_role() {
    let Some(w) = World::new().await else { return };
    let alice = w.user("920").await;
    // No roles, or none coloured: the neutral ring.
    assert!(w.ring(&alice).await.is_null());
    let plain = w.role("Plain", 90, &[], None).await;
    w.give(&alice, plain).await;
    assert!(w.ring(&alice).await.is_null());
    // The only coloured role.
    let low = w.role("Low", 10, &[], Some("#3b82f6")).await;
    w.give(&alice, low).await;
    assert_eq!(w.ring(&alice).await, "#3b82f6");
    // A more senior coloured role wins; an uncoloured one above it doesn't count.
    let mid = w.role("Mid", 50, &[], Some("#e5484d")).await;
    w.give(&alice, mid).await;
    assert_eq!(w.ring(&alice).await, "#e5484d");
    // A senior coloured role that isn't public or is invisible never colours it.
    let hidden = w.role("Hidden", 80, &[], Some("#2f9e5b")).await;
    let unseen = w.role("Unseen", 85, &[], Some("#8b5cf6")).await;
    {
        let mut conn = w.db.conn().await;
        roles::set_public(&mut conn, hidden, false).await.unwrap();
        roles::set_invisible(&mut conn, unseen, true).await.unwrap();
    }
    w.give(&alice, hidden).await;
    w.give(&alice, unseen).await;
    assert_eq!(w.ring(&alice).await, "#e5484d");
    // The public roles carry their colours, most senior first.
    let r = w.gql(&alice, PROFILE, json!({ "a": alice.tid() })).await;
    let list = r["data"]["accountProfile"]["roles"].as_array().unwrap();
    let names: Vec<_> = list.iter().map(|x| x["name"].as_str().unwrap()).collect();
    assert_eq!(names, ["Plain", "Mid", "Low"], "{r}");
    assert!(list[0]["color"].is_null());
    assert_eq!(list[1]["color"], "#e5484d");
    // Anyone sees the same ring.
    let other = w.user("921").await;
    let seen = w.gql(&other, PROFILE, json!({ "a": alice.tid() })).await;
    assert_eq!(seen["data"]["accountProfile"]["roleColor"], "#e5484d");
    w.db.close().await;
}
