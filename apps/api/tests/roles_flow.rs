//! Roles, assignment, the owner and the audit log (ADR 0010), through the real router against a
//! real Postgres. Runs when `CYEX_TV_TEST_DATABASE_URL` is set (CI sets it), skips otherwise.
//! The owner here is `twitch:900`; the pure rules have unit tests in `cyex_tv_auth::rbac`.

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
use cyex_tv_db::ids;
use cyex_tv_db::sessions::{self, SessionKind};
use cyex_tv_shared::model::Platform;
use cyex_tv_shared::permissions;
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

    /// Creates a role as `by`. Returns its ID, or the error code.
    async fn create_role(
        &self,
        by: &User,
        name: &str,
        position: i32,
        perms: &[&str],
    ) -> Result<String, String> {
        let r = self
            .gql(
                by,
                "mutation($n: String!, $p: Int!, $perms: [String!]!) { createRole(input: { name: $n, position: $p, permissions: $perms }) { id permissions } }",
                json!({ "n": name, "p": position, "perms": perms }),
            )
            .await;
        match r["data"]["createRole"]["id"].as_str() {
            Some(id) => Ok(id.to_owned()),
            None => Err(code(&r).to_owned()),
        }
    }

    async fn assign(&self, by: &User, to: &User, role: &str) -> Value {
        self.gql(
            by,
            "mutation($a: String!, $r: String!) { assignRole(accountId: $a, roleId: $r) }",
            json!({ "a": to.text_id(), "r": role }),
        )
        .await
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

#[tokio::test]
async fn the_owner_holds_everything() {
    let Some(w) = World::new().await else { return };
    let owner = w.user("900").await;
    let plain = w.user("901").await;

    let me = w.gql(&owner, "{ me { permissions } }", json!({})).await;
    assert_eq!(me["data"]["me"]["permissions"], json!(permissions::ALL));
    let me = w.gql(&plain, "{ me { permissions } }", json!({})).await;
    assert_eq!(me["data"]["me"]["permissions"], json!([]));

    assert_eq!(
        code(&w.gql(&plain, "{ roles { id } }", json!({})).await),
        "forbidden"
    );
    // The owner needs no role to create one at any position, with any permission.
    let top = w
        .create_role(&owner, "Admins", 1000, permissions::ALL)
        .await
        .unwrap();
    let r = w
        .gql(&owner, "{ roles { name position permissions } }", json!({}))
        .await;
    // Beside the built-in roles (the owners' top role first).
    assert_eq!(r["data"]["roles"][0]["name"], "Owner", "{r}");
    let admins = r["data"]["roles"]
        .as_array()
        .unwrap()
        .iter()
        .find(|x| x["name"] == "Admins")
        .unwrap();
    assert_eq!(admins["position"], 1000);
    assert_eq!(
        admins["permissions"].as_array().unwrap().len(),
        permissions::ALL.len()
    );

    assert_eq!(
        w.create_role(&owner, "admins", 1, &[]).await.unwrap_err(),
        "role_name_taken"
    );
    assert_eq!(
        w.create_role(&owner, "Bad", 1, &["emotes.delete_everything"])
            .await
            .unwrap_err(),
        "unknown_permission"
    );
    let r = w
        .gql(
            &owner,
            "mutation($a: String!, $r: String!) { assignRole(accountId: $a, roleId: $r) }",
            json!({ "a": ids::to_text(Uuid::nil()), "r": top }),
        )
        .await;
    assert_eq!(code(&r), "not_found");
    w.db.close().await;
}

#[tokio::test]
async fn no_escalation() {
    let Some(w) = World::new().await else { return };
    let owner = w.user("900").await;
    let admin = w.user("910").await;
    let carol = w.user("911").await;

    let admins = w
        .create_role(&owner, "Admins", 100, &["roles.manage", "emotes.review"])
        .await
        .unwrap();
    assert_eq!(
        w.assign(&owner, &admin, &admins).await["data"]["assignRole"],
        true
    );
    assert_eq!(
        w.assign(&owner, &admin, &admins).await["data"]["assignRole"],
        false
    );

    // Below their highest role, with permissions they hold: fine.
    let mods = w
        .create_role(&admin, "Mods", 50, &["emotes.review"])
        .await
        .unwrap();
    assert_eq!(
        w.assign(&admin, &carol, &mods).await["data"]["assignRole"],
        true
    );

    // Permissions they don't hold, or at or above their highest role: refused.
    assert_eq!(
        w.create_role(&admin, "Auditors", 10, &["audit.view"])
            .await
            .unwrap_err(),
        "escalation"
    );
    assert_eq!(
        w.create_role(&admin, "Peers", 100, &[]).await.unwrap_err(),
        "not_below_you"
    );
    assert_eq!(
        w.create_role(&admin, "Bosses", 500, &[]).await.unwrap_err(),
        "not_below_you"
    );
    assert_eq!(
        code(&w.assign(&admin, &carol, &admins).await),
        "not_below_you"
    );

    let update = "mutation($id: String!, $p: Int, $perms: [String!], $n: String) { updateRole(id: $id, input: { position: $p, permissions: $perms, name: $n }) { name position permissions } }";
    assert_eq!(
        code(
            &w.gql(&admin, update, json!({ "id": admins, "n": "Mine" }))
                .await
        ),
        "not_below_you"
    );
    assert_eq!(
        code(&w.gql(&admin, update, json!({ "id": mods, "p": 100 })).await),
        "not_below_you"
    );
    assert_eq!(
        code(
            &w.gql(
                &admin,
                update,
                json!({ "id": mods, "perms": ["emotes.review", "audit.view"] })
            )
            .await
        ),
        "escalation"
    );
    let r = w
        .gql(
            &admin,
            update,
            json!({ "id": mods, "n": "Reviewers", "perms": ["roles.manage"] }),
        )
        .await;
    assert_eq!(r["data"]["updateRole"]["name"], "Reviewers", "{r}");
    assert_eq!(
        r["data"]["updateRole"]["permissions"],
        json!(["roles.manage"])
    );

    // Carol now manages roles through Reviewers, but only below position 50.
    assert_eq!(
        w.create_role(&carol, "Helpers", 49, &["roles.manage"])
            .await
            .map(|_| ()),
        Ok(())
    );
    assert_eq!(
        w.create_role(&carol, "Equal", 50, &[]).await.unwrap_err(),
        "not_below_you"
    );
    assert_eq!(
        code(&w.unassign(&carol, &admin, &admins).await),
        "not_below_you"
    );

    // Someone without roles.manage can't touch roles at all.
    let plain = w.user("912").await;
    assert_eq!(
        w.create_role(&plain, "x", -5, &[]).await.unwrap_err(),
        "forbidden"
    );
    w.db.close().await;
}

#[tokio::test]
async fn the_owner_cant_be_demoted_by_others() {
    let Some(w) = World::new().await else { return };
    let owner = w.user("900").await;
    let admin = w.user("920").await;
    let admins = w
        .create_role(&owner, "Admins", 100, &["roles.manage"])
        .await
        .unwrap();
    let mods = w.create_role(&owner, "Mods", 10, &[]).await.unwrap();
    w.assign(&owner, &admin, &admins).await;
    w.assign(&owner, &owner, &mods).await;

    assert_eq!(
        code(&w.unassign(&admin, &owner, &mods).await),
        "owner_protected"
    );
    assert_eq!(
        w.unassign(&owner, &owner, &mods).await["data"]["unassignRole"],
        true
    );
    // Without any role, the owner still holds everything.
    let me = w.gql(&owner, "{ me { permissions } }", json!({})).await;
    assert_eq!(me["data"]["me"]["permissions"], json!(permissions::ALL));
    w.db.close().await;
}

#[tokio::test]
async fn the_last_manager_stays() {
    let Some(w) = World::new().await else { return };
    let owner = w.user("900").await;
    let admin = w.user("930").await;
    let top = w
        .create_role(&owner, "Top", 100, &["emotes.review"])
        .await
        .unwrap();
    let managers = w
        .create_role(&owner, "Managers", 10, &["roles.manage"])
        .await
        .unwrap();
    w.assign(&owner, &admin, &top).await;
    w.assign(&owner, &admin, &managers).await;

    // The admin is the only non-owner with roles.manage: none of these may go through. Deleting
    // or emptying the role they hold is refused before that: nobody edits their own role.
    assert_eq!(
        code(&w.unassign(&admin, &admin, &managers).await),
        "last_manager"
    );
    assert_eq!(
        code(
            &w.gql(
                &admin,
                "mutation($id: String!) { deleteRole(id: $id) }",
                json!({ "id": managers })
            )
            .await
        ),
        "own_role"
    );
    assert_eq!(
        code(
            &w.gql(
                &admin,
                "mutation($id: String!) { updateRole(id: $id, input: { permissions: [] }) { id } }",
                json!({ "id": managers })
            )
            .await
        ),
        "own_role"
    );
    // Nothing changed.
    let r = w.gql(&admin, "{ me { permissions } }", json!({})).await;
    assert_eq!(
        r["data"]["me"]["permissions"],
        json!(["emotes.review", "roles.manage"])
    );

    // With a second manager, the first may step down.
    let second = w.user("931").await;
    w.assign(&owner, &second, &managers).await;
    assert_eq!(
        w.unassign(&admin, &admin, &managers).await["data"]["unassignRole"],
        true
    );

    // The owner may leave nobody but themselves.
    assert_eq!(
        w.unassign(&owner, &second, &managers).await["data"]["unassignRole"],
        true
    );
    w.db.close().await;
}

#[tokio::test]
async fn every_role_change_is_audited() {
    let Some(w) = World::new().await else { return };
    let owner = w.user("900").await;
    let carol = w.user("940").await;
    let mods = w
        .create_role(&owner, "Mods", 10, &["emotes.review"])
        .await
        .unwrap();
    w.assign(&owner, &carol, &mods).await;
    w.gql(
        &owner,
        "mutation($id: String!) { updateRole(id: $id, input: { description: \"Review queue\" }) { id } }",
        json!({ "id": mods }),
    )
    .await;
    w.unassign(&owner, &carol, &mods).await;
    w.gql(
        &owner,
        "mutation($id: String!) { deleteRole(id: $id) }",
        json!({ "id": mods }),
    )
    .await;

    let query = "query($b: String, $l: Int, $k: String) { auditLog(before: $b, limit: $l, targetKind: $k) { id actorId action targetKind targetId before after } }";
    assert_eq!(code(&w.gql(&carol, query, json!({})).await), "forbidden");

    let r = w.gql(&owner, query, json!({})).await;
    let entries = r["data"]["auditLog"].as_array().unwrap();
    let actions: Vec<&str> = entries
        .iter()
        .map(|e| e["action"].as_str().unwrap())
        .collect();
    assert_eq!(
        actions,
        [
            "role.deleted",
            "role.unassigned",
            "role.updated",
            "role.assigned",
            "role.created"
        ]
    );
    assert!(
        entries
            .iter()
            .all(|e| e["actorId"] == owner.text_id().as_str())
    );
    let updated = &entries[2];
    let before: Value = serde_json::from_str(updated["before"].as_str().unwrap()).unwrap();
    let after: Value = serde_json::from_str(updated["after"].as_str().unwrap()).unwrap();
    assert_eq!(before["description"], Value::Null);
    assert_eq!(after["description"], "Review queue");
    assert_eq!(entries[1]["targetKind"], "account");
    assert_eq!(entries[1]["targetId"], carol.text_id().as_str());

    // Paging and filtering.
    let page = w.gql(&owner, query, json!({ "l": 2 })).await;
    let page = page["data"]["auditLog"].as_array().unwrap().clone();
    assert_eq!(page.len(), 2);
    let next = w.gql(&owner, query, json!({ "b": page[1]["id"] })).await;
    assert_eq!(next["data"]["auditLog"].as_array().unwrap().len(), 3);
    let roles_only = w.gql(&owner, query, json!({ "k": "role" })).await;
    assert_eq!(roles_only["data"]["auditLog"].as_array().unwrap().len(), 3);

    // accountRoles: your own, or with roles.manage / users.view.
    let own = w
        .gql(
            &carol,
            "query($a: String!) { accountRoles(accountId: $a) { id } }",
            json!({ "a": carol.text_id() }),
        )
        .await;
    assert_eq!(own["data"]["accountRoles"], json!([]));
    let other = w
        .gql(
            &carol,
            "query($a: String!) { accountRoles(accountId: $a) { id } }",
            json!({ "a": owner.text_id() }),
        )
        .await;
    assert_eq!(code(&other), "forbidden");
    w.db.close().await;
}
