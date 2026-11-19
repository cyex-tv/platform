//! Acting on a more senior account is refused: restricting it and taking a role from it. Runs
//! when `CYEX_TV_TEST_DATABASE_URL` is set (CI sets it), skips otherwise. The owner is
//! `twitch:900`.

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
    fn text_id(&self) -> String {
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

    async fn gql(&self, user: &User, query: &str, variables: Value) -> Value {
        let request = Request::post("/v1/gql")
            .header("content-type", "application/json")
            .header("authorization", format!("Bearer {}", user.token))
            .body(Body::from(
                json!({ "query": query, "variables": variables }).to_string(),
            ))
            .unwrap();
        let response = self.app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        serde_json::from_slice(&bytes).unwrap()
    }

    async fn unassign(&self, by: &User, from: &User, role: &str) -> Value {
        self.gql(
            by,
            "mutation($a: String!, $r: String!) { unassignRole(accountId: $a, roleId: $r) }",
            json!({ "a": from.text_id(), "r": role }),
        )
        .await
    }
}

impl World {
    /// A role made straight in the database, held by `to` if given.
    async fn role(&self, name: &str, position: i32, perms: &[&str], to: Option<&User>) -> Uuid {
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
        if let Some(u) = to {
            roles::assign(&mut conn, u.id, role.id, None).await.unwrap();
        }
        role.id
    }
}

const RESTRICT: &str = "mutation($a: String!, $b: Boolean) { restrictAccount(accountId: $a, blocked: $b) { blocked } }";

#[tokio::test]
async fn restricting_respects_seniority() {
    let Some(w) = World::new().await else { return };
    let owner = w.user("900").await;
    let moderator = w.user("901").await;
    let admin = w.user("902").await;
    let plain = w.user("903").await;
    w.role("Mods", 10, &["users.restrict"], Some(&moderator))
        .await;
    w.role("Admins", 100, &["users.restrict"], Some(&admin))
        .await;

    let r = w
        .gql(
            &moderator,
            RESTRICT,
            json!({ "a": admin.text_id(), "b": true }),
        )
        .await;
    assert_eq!(code(&r), "not_below_you", "{r}");
    let me = w.gql(&admin, "{ me { id } }", json!({})).await;
    assert_eq!(me["data"]["me"]["id"], admin.text_id(), "{me}");

    let r = w
        .gql(
            &moderator,
            RESTRICT,
            json!({ "a": plain.text_id(), "b": true }),
        )
        .await;
    assert_eq!(r["data"]["restrictAccount"]["blocked"], true, "{r}");

    let r = w
        .gql(&owner, RESTRICT, json!({ "a": admin.text_id(), "b": true }))
        .await;
    assert_eq!(r["data"]["restrictAccount"]["blocked"], true, "{r}");
    w.db.close().await;
}

#[tokio::test]
async fn unassigning_respects_seniority() {
    let Some(w) = World::new().await else { return };
    let owner = w.user("900").await;
    let manager = w.user("901").await;
    let admin = w.user("902").await;
    w.role("Managers", 50, &["roles.manage"], Some(&manager))
        .await;
    w.role("Admins", 100, &["roles.manage"], Some(&admin)).await;
    let mid = w.role("Mid", 40, &[], Some(&admin)).await;

    let r = w.unassign(&manager, &admin, &ids::to_text(mid)).await;
    assert_eq!(code(&r), "not_below_you", "{r}");

    let r = w.unassign(&owner, &admin, &ids::to_text(mid)).await;
    assert_eq!(r["data"]["unassignRole"], true, "{r}");
    w.db.close().await;
}

const ADD_EDITOR: &str = "mutation($o: String, $a: String!, $f: [EditorPermission!]!) {
    addAccountEditor(input: { ownerId: $o, accountId: $a, permissions: $f }) { account { id } }
}";
const UPDATE_EDITOR: &str = "mutation($o: String, $a: String!, $f: [EditorPermission!]!) {
    updateAccountEditor(ownerId: $o, accountId: $a, permissions: $f) { account { id } }
}";
const REMOVE_EDITOR: &str =
    "mutation($o: String, $a: String!) { removeAccountEditor(ownerId: $o, accountId: $a) }";
const LIST_EDITORS: &str = "query($o: String) { accountEditors(accountId: $o) { account { id } } }";

impl World {
    /// `by` makes `who` an editor of `of` with `SETS` and `EDITORS`.
    async fn add_editor(&self, by: &User, of: &User, who: &User) -> Value {
        let vars = json!({ "o": of.text_id(), "a": who.text_id(), "f": ["SETS", "EDITORS"] });
        self.gql(by, ADD_EDITOR, vars).await
    }
}

/// `editors.manage_any` (staff) never reaches the owner's account, and only accounts below the
/// staff member's own highest role.
#[tokio::test]
async fn staff_editor_management_respects_the_owner_and_seniority() {
    let Some(w) = World::new().await else { return };
    let owner = w.user("900").await;
    let moderator = w.user("901").await;
    let admin = w.user("902").await;
    let plain = w.user("903").await;
    w.role("Mods", 10, &["editors.manage_any"], Some(&moderator))
        .await;
    w.role("Admins", 100, &["editors.manage_any"], Some(&admin))
        .await;

    // The Admin can't make itself the owner's editor, nor read, change or remove theirs.
    let r = w.add_editor(&admin, &owner, &admin).await;
    assert_eq!(code(&r), "owner_protected", "{r}");
    let r = w
        .gql(
            &owner,
            ADD_EDITOR,
            json!({ "o": null, "a": plain.text_id(), "f": ["SETS"] }),
        )
        .await;
    assert!(r["errors"].is_null(), "{r}");
    let r = w
        .gql(&admin, LIST_EDITORS, json!({ "o": owner.text_id() }))
        .await;
    assert_eq!(code(&r), "owner_protected", "{r}");
    let r = w
        .gql(
            &admin,
            UPDATE_EDITOR,
            json!({ "o": owner.text_id(), "a": plain.text_id(), "f": ["EDITORS", "SETS"] }),
        )
        .await;
    assert_eq!(code(&r), "owner_protected", "{r}");
    let r = w
        .gql(
            &admin,
            REMOVE_EDITOR,
            json!({ "o": owner.text_id(), "a": plain.text_id() }),
        )
        .await;
    assert_eq!(code(&r), "owner_protected", "{r}");

    // On a lower account it can, with every flag; the moderator can't on the Admin's.
    let r = w.add_editor(&admin, &plain, &admin).await;
    assert!(r["errors"].is_null(), "{r}");
    let r = w.add_editor(&admin, &moderator, &admin).await;
    assert!(r["errors"].is_null(), "{r}");
    let r = w.add_editor(&moderator, &admin, &moderator).await;
    assert_eq!(code(&r), "not_below_you", "{r}");
    let r = w
        .gql(&moderator, LIST_EDITORS, json!({ "o": admin.text_id() }))
        .await;
    assert_eq!(code(&r), "not_below_you", "{r}");
    // Same rank is not below.
    let peer = w.user("904").await;
    w.role("Mods2", 10, &[], Some(&peer)).await;
    let r = w.add_editor(&moderator, &peer, &moderator).await;
    assert_eq!(code(&r), "not_below_you", "{r}");
    let r = w.add_editor(&moderator, &plain, &moderator).await;
    assert!(r["errors"].is_null(), "{r}");

    // The owner may still manage anyone's.
    let r = w.add_editor(&owner, &admin, &owner).await;
    assert!(r["errors"].is_null(), "{r}");
    w.db.close().await;
}
