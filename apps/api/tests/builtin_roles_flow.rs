//! The built-in roles and the hierarchy rules added on 2026/11/19 (ADR 0010 amendment): the
//! User role everyone holds, Admin, the owners' hidden top role, "nobody edits a role they hold",
//! `roles.assign`, and the editable/assignable flags. Through the real router against a real
//! Postgres; runs when `CYEX_TV_TEST_DATABASE_URL` is set (CI sets it), skips otherwise. The
//! owner here is `twitch:900`.

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

const ROLES: &str = "{ roles { id name position permissions builtIn editable assignable } }";
const UPDATE: &str = "mutation($id: String!, $p: Int, $perms: [String!], $n: String) { updateRole(id: $id, input: { position: $p, permissions: $perms, name: $n }) { id name position permissions } }";
const ASSIGN: &str = "mutation($a: String!, $r: String!) { assignRole(accountId: $a, roleId: $r) }";
const UNASSIGN: &str =
    "mutation($a: String!, $r: String!) { unassignRole(accountId: $a, roleId: $r) }";
const CREATE: &str = "mutation($n: String!, $p: Int!, $perms: [String!]!) { createRole(input: { name: $n, position: $p, permissions: $perms }) { id } }";

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

    async fn system(&self, key: &str) -> String {
        let mut conn = self.db.conn().await;
        ids::to_text(roles::system(&mut conn, key).await.unwrap().unwrap().id)
    }
}

fn role_named<'a>(roles: &'a Value, name: &str) -> &'a Value {
    roles
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["name"] == name)
        .unwrap_or_else(|| panic!("no role {name} in {roles}"))
}

#[tokio::test]
async fn the_built_in_roles_exist_and_the_top_one_is_hidden() {
    let Some(w) = World::new().await else { return };
    let owner = w.user("900").await;
    let admin = w.user("901").await;
    let admin_role = w.system("admin").await;
    let root = w.system("root").await;
    let user_role = w.system("user").await;
    assert_eq!(
        w.gql(&owner, ASSIGN, json!({ "a": admin.tid(), "r": admin_role }))
            .await["data"]["assignRole"],
        true
    );

    // The owner sees all three, top first, with every permission on the top role.
    let r = w.gql(&owner, ROLES, json!({})).await;
    let list = &r["data"]["roles"];
    let names: Vec<&str> = list
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, ["Owner", "Admin", "User"], "{r}");
    assert_eq!(list[0]["builtIn"], "ROOT");
    assert_eq!(list[0]["position"], roles::ROOT_POSITION);
    assert_eq!(
        list[0]["permissions"].as_array().unwrap().len(),
        cyex_tv_shared::permissions::ALL.len()
    );
    assert_eq!(list[0]["editable"], false);
    assert_eq!(list[0]["assignable"], false);
    assert_eq!(list[1]["builtIn"], "ADMIN");
    assert_eq!(list[1]["editable"], true);
    assert_eq!(list[2]["builtIn"], "USER");
    assert_eq!(list[2]["permissions"], json!([]));
    assert_eq!(list[2]["assignable"], false);

    // An admin never sees the top role, not in the list and not on the owner's account.
    let r = w.gql(&admin, ROLES, json!({})).await;
    let names: Vec<&str> = r["data"]["roles"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, ["Admin", "User"], "{r}");
    let q = "query($a: String!) { accountRoles(accountId: $a) { name builtIn } }";
    let r = w.gql(&admin, q, json!({ "a": owner.tid() })).await;
    assert_eq!(r["data"]["accountRoles"], json!([]), "{r}");
    let r = w.gql(&owner, q, json!({ "a": owner.tid() })).await;
    assert_eq!(
        r["data"]["accountRoles"],
        json!([{ "name": "Owner", "builtIn": "ROOT" }]),
        "{r}"
    );
    assert!(
        !r.to_string().contains("User"),
        "User is implicit, not listed: {r}"
    );

    // The owner's own `me.roles` lists the top role, hidden, first; an admin's does not.
    let q = "{ me { roles { name public } } }";
    let r = w.gql(&owner, q, json!({})).await;
    assert_eq!(
        r["data"]["me"]["roles"][0],
        json!({ "name": "Owner", "public": false }),
        "{r}"
    );
    let r = w.gql(&admin, q, json!({})).await;
    assert!(!r.to_string().contains("Owner"), "{r}");

    // Nobody grants, edits or deletes the top role; not even the owner through the API.
    for (who, query, vars) in [
        (&owner, ASSIGN, json!({ "a": admin.tid(), "r": root })),
        (&owner, UPDATE, json!({ "id": root, "n": "Mine" })),
        (
            &owner,
            "mutation($id: String!) { deleteRole(id: $id) }",
            json!({ "id": root }),
        ),
    ] {
        assert_eq!(code(&w.gql(who, query, vars).await), "built_in_role");
    }
    // To anyone else it doesn't exist, even asked for by its ID.
    for (query, vars) in [
        (ASSIGN, json!({ "a": admin.tid(), "r": root })),
        (UNASSIGN, json!({ "a": owner.tid(), "r": root })),
        (UPDATE, json!({ "id": root, "n": "x" })),
        (
            "mutation($id: String!) { deleteRole(id: $id) }",
            json!({ "id": root }),
        ),
    ] {
        assert_eq!(
            code(&w.gql(&admin, query, vars).await),
            "not_found",
            "{query}"
        );
    }

    // User and Admin can't be deleted; User is never assigned and stays at 0.
    for id in [&admin_role, &user_role] {
        assert_eq!(
            code(
                &w.gql(
                    &owner,
                    "mutation($id: String!) { deleteRole(id: $id) }",
                    json!({ "id": id })
                )
                .await
            ),
            "built_in_role"
        );
    }
    assert_eq!(
        code(
            &w.gql(&owner, ASSIGN, json!({ "a": admin.tid(), "r": user_role }))
                .await
        ),
        "built_in_role"
    );
    assert_eq!(
        code(
            &w.gql(&owner, UPDATE, json!({ "id": user_role, "p": 5 }))
                .await
        ),
        "built_in_role"
    );
    // No custom role reaches the top role's position, or sits at the User role's.
    for p in [0, roles::ROOT_POSITION] {
        let r = w
            .gql(
                &owner,
                CREATE,
                json!({ "n": "Too far", "p": p, "perms": [] }),
            )
            .await;
        assert_eq!(code(&r), "bad_position", "{p}: {r}");
    }
    w.db.close().await;
}

#[tokio::test]
async fn the_user_role_is_everyones() {
    let Some(w) = World::new().await else { return };
    let owner = w.user("900").await;
    let plain = w.user("910").await;
    let user_role = w.system("user").await;

    let me = "{ me { permissions } }";
    assert_eq!(
        w.gql(&plain, me, json!({})).await["data"]["me"]["permissions"],
        json!([])
    );
    let r = w
        .gql(
            &owner,
            UPDATE,
            json!({ "id": user_role, "perms": ["audit.view"], "n": "Members" }),
        )
        .await;
    assert_eq!(r["data"]["updateRole"]["name"], "Members", "{r}");
    // Every account holds it, without an assignment.
    assert_eq!(
        w.gql(&plain, me, json!({})).await["data"]["me"]["permissions"],
        json!(["audit.view"])
    );
    let later = w.user("911").await;
    assert_eq!(
        w.gql(&later, me, json!({})).await["data"]["me"]["permissions"],
        json!(["audit.view"])
    );
    // And it is audited like any role change.
    let log = w
        .gql(
            &owner,
            "{ auditLog(targetKind: \"role\") { action targetId } }",
            json!({}),
        )
        .await;
    assert_eq!(
        log["data"]["auditLog"][0]["action"], "role.updated",
        "{log}"
    );
    assert_eq!(log["data"]["auditLog"][0]["targetId"], user_role.as_str());
    w.db.close().await;
}

#[tokio::test]
async fn nobody_edits_the_role_they_hold_or_grants_at_or_above_their_own() {
    let Some(w) = World::new().await else { return };
    let owner = w.user("900").await;
    let admin = w.user("920").await;
    let carol = w.user("921").await;
    let admin_role = w.system("admin").await;
    let mods = w
        .gql(
            &owner,
            CREATE,
            json!({ "n": "Mods", "p": 50, "perms": ["emotes.review", "roles.assign"] }),
        )
        .await["data"]["createRole"]["id"]
        .as_str()
        .unwrap()
        .to_owned();
    w.gql(&owner, ASSIGN, json!({ "a": admin.tid(), "r": admin_role }))
        .await;
    w.gql(&owner, ASSIGN, json!({ "a": admin.tid(), "r": mods }))
        .await;

    // Admin is their highest role: not below them.
    assert_eq!(
        code(
            &w.gql(&admin, UPDATE, json!({ "id": admin_role, "n": "Mine" }))
                .await
        ),
        "not_below_you"
    );
    // Mods is below them, but they hold it.
    assert_eq!(
        code(
            &w.gql(&admin, UPDATE, json!({ "id": mods, "n": "Mine" }))
                .await
        ),
        "own_role"
    );
    assert_eq!(
        code(
            &w.gql(
                &admin,
                "mutation($id: String!) { deleteRole(id: $id) }",
                json!({ "id": mods })
            )
            .await
        ),
        "own_role"
    );
    // The flags say the same.
    let r = w.gql(&admin, ROLES, json!({})).await;
    let list = &r["data"]["roles"];
    let m = role_named(list, "Mods");
    assert_eq!(
        (m["editable"].clone(), m["assignable"].clone()),
        (json!(false), json!(true)),
        "{r}"
    );
    let a = role_named(list, "Admin");
    assert_eq!(
        (a["editable"].clone(), a["assignable"].clone()),
        (json!(false), json!(false))
    );
    // They can't give their own highest role away, or anything above it.
    assert_eq!(
        code(
            &w.gql(&admin, ASSIGN, json!({ "a": carol.tid(), "r": admin_role }))
                .await
        ),
        "not_below_you"
    );
    // The owner is bound by the same rule for roles they hold.
    w.gql(&owner, ASSIGN, json!({ "a": owner.tid(), "r": mods }))
        .await;
    assert_eq!(
        code(
            &w.gql(&owner, UPDATE, json!({ "id": mods, "n": "Ours" }))
                .await
        ),
        "own_role"
    );
    w.gql(&owner, UNASSIGN, json!({ "a": owner.tid(), "r": mods }))
        .await;
    let r = w
        .gql(&owner, UPDATE, json!({ "id": mods, "n": "Moderators" }))
        .await;
    assert_eq!(r["data"]["updateRole"]["name"], "Moderators", "{r}");
    w.db.close().await;
}

#[tokio::test]
async fn roles_assign_gives_and_takes_without_editing() {
    let Some(w) = World::new().await else { return };
    let owner = w.user("900").await;
    let lead = w.user("930").await;
    let dan = w.user("931").await;
    let id = |r: Value| r["data"]["createRole"]["id"].as_str().unwrap().to_owned();
    let leads = id(w
        .gql(
            &owner,
            CREATE,
            json!({ "n": "Leads", "p": 100, "perms": ["roles.assign", "emotes.review"] }),
        )
        .await);
    let helpers = id(w
        .gql(
            &owner,
            CREATE,
            json!({ "n": "Helpers", "p": 10, "perms": ["emotes.review"] }),
        )
        .await);
    let auditors = id(w
        .gql(
            &owner,
            CREATE,
            json!({ "n": "Auditors", "p": 20, "perms": ["audit.view"] }),
        )
        .await);
    w.gql(&owner, ASSIGN, json!({ "a": lead.tid(), "r": leads }))
        .await;

    // Below them, granting only what they hold: yes.
    assert_eq!(
        w.gql(&lead, ASSIGN, json!({ "a": dan.tid(), "r": helpers }))
            .await["data"]["assignRole"],
        true
    );
    assert_eq!(
        w.gql(&lead, UNASSIGN, json!({ "a": dan.tid(), "r": helpers }))
            .await["data"]["unassignRole"],
        true
    );
    // A role granting what they don't hold: no.
    assert_eq!(
        code(
            &w.gql(&lead, ASSIGN, json!({ "a": dan.tid(), "r": auditors }))
                .await
        ),
        "escalation"
    );
    // Making or editing roles still needs roles.manage.
    assert_eq!(
        code(
            &w.gql(&lead, CREATE, json!({ "n": "New", "p": 5, "perms": [] }))
                .await
        ),
        "forbidden"
    );
    assert_eq!(
        code(
            &w.gql(&lead, UPDATE, json!({ "id": helpers, "n": "x" }))
                .await
        ),
        "forbidden"
    );
    // They can read the list to pick from; the editable flag is off everywhere.
    let r = w.gql(&lead, ROLES, json!({})).await;
    assert!(
        r["data"]["roles"]
            .as_array()
            .unwrap()
            .iter()
            .all(|r| r["editable"] == false),
        "{r}"
    );
    w.db.close().await;
}

/// Only the owner gives the User role a staff permission: an Admin with `roles.manage` adding
/// one there would hand it to every account. Renaming it and taking permissions away stay open.
#[tokio::test]
async fn only_the_owner_adds_staff_permissions_to_the_user_role() {
    let Some(w) = World::new().await else { return };
    let owner = w.user("900").await;
    let admin = w.user("930").await;
    let plain = w.user("931").await;
    let user_role = w.system("user").await;
    let admin_role = w.system("admin").await;
    w.gql(&owner, ASSIGN, json!({ "a": admin.tid(), "r": admin_role }))
        .await;
    let me = "{ me { permissions } }";

    let r = w
        .gql(
            &admin,
            UPDATE,
            json!({ "id": user_role, "perms": ["users.block"] }),
        )
        .await;
    assert_eq!(r["errors"][0]["extensions"]["code"], "owner_only", "{r}");
    assert_eq!(
        w.gql(&plain, me, json!({})).await["data"]["me"]["permissions"],
        json!([])
    );
    // A rename is fine.
    let r = w
        .gql(&admin, UPDATE, json!({ "id": user_role, "n": "Members" }))
        .await;
    assert_eq!(r["data"]["updateRole"]["name"], "Members", "{r}");
    // The owner may; then the Admin may take it away again.
    let r = w
        .gql(
            &owner,
            UPDATE,
            json!({ "id": user_role, "perms": ["audit.view"] }),
        )
        .await;
    assert!(r["errors"].is_null(), "{r}");
    let r = w
        .gql(&admin, UPDATE, json!({ "id": user_role, "perms": [] }))
        .await;
    assert!(r["errors"].is_null(), "{r}");
    assert_eq!(
        w.gql(&plain, me, json!({})).await["data"]["me"]["permissions"],
        json!([])
    );
    w.db.close().await;
}
