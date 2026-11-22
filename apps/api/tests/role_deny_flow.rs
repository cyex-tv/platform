//! Allow, Deny and Inherit per role, and invisible roles (ADR 0010, amendment 2026/11/22): the
//! most senior role naming a permission decides, a Deny above beats an Allow below (the User
//! role's included), the owners can't be denied, every change is audited, and invisible roles are
//! never shown publicly. Through the real router against a real Postgres; runs when
//! `CYEX_TV_TEST_DATABASE_URL` is set (CI sets it), skips otherwise. The owner is `twitch:900`.

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

const ROLES: &str = "{ roles { id name permissions denies invisible public builtIn } }";
const CREATE: &str = "mutation($n: String!, $p: Int!, $perms: [String!]!, $d: [String!], $inv: Boolean, $pub: Boolean) { createRole(input: { name: $n, position: $p, permissions: $perms, denies: $d, invisible: $inv, public: $pub }) { id permissions denies invisible public } }";
const UPDATE: &str = "mutation($id: String!, $perms: [String!], $d: [String!], $inv: Boolean) { updateRole(id: $id, input: { permissions: $perms, denies: $d, invisible: $inv }) { id permissions denies invisible } }";
const ASSIGN: &str = "mutation($a: String!, $r: String!) { assignRole(accountId: $a, roleId: $r) }";
const DECISIONS: &str = "query($a: String!) { accountPermissions(accountId: $a) { permission granted effect roleId roleName owner } }";
const ME: &str = "{ me { permissions roles { name public invisible } } }";
const PROFILE: &str = "query($a: String!) { accountProfile(accountId: $a) { roles { name } } }";

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

impl World {
    /// A role made straight in the database: `allows` and `denies` at `position`.
    async fn role(&self, name: &str, position: i32, allows: &[&str], denies: &[&str]) -> Uuid {
        let mut conn = self.db.conn().await;
        let role = roles::create(
            &mut conn,
            &roles::NewRole {
                name,
                description: None,
                position,
                permissions: allows,
                created_by: None,
            },
        )
        .await
        .unwrap();
        roles::set_effects(&mut conn, role.id, allows, denies)
            .await
            .unwrap();
        role.id
    }

    async fn give(&self, user: &User, role: Uuid) {
        let mut conn = self.db.conn().await;
        roles::assign(&mut conn, user.id, role, None).await.unwrap();
    }

    async fn perms(&self, user: &User) -> Vec<String> {
        let r = self.gql(user, ME, json!({})).await;
        serde_json::from_value(r["data"]["me"]["permissions"].clone())
            .unwrap_or_else(|_| panic!("{r}"))
    }

    /// `accountPermissions` for `of`, as `viewer` sees it, by permission name.
    async fn decision(&self, viewer: &User, of: &User, permission: &str) -> Value {
        let r = self.gql(viewer, DECISIONS, json!({ "a": of.tid() })).await;
        r["data"]["accountPermissions"]
            .as_array()
            .unwrap_or_else(|| panic!("{r}"))
            .iter()
            .find(|d| d["permission"] == permission)
            .cloned()
            .unwrap_or_else(|| panic!("no {permission} in {r}"))
    }
}

#[tokio::test]
async fn a_deny_above_the_user_role_takes_away_what_it_allows() {
    let Some(w) = World::new().await else { return };
    let owner = w.user("900").await;
    let plain = w.user("910").await;
    let muted = w.user("911").await;
    assert!(w.perms(&plain).await.contains(&"emotes.upload".to_owned()));

    let role = w.role("Muted", 5, &[], &["emotes.upload"]).await;
    w.give(&muted, role).await;
    assert_eq!(w.perms(&muted).await, ["reports.file"]);
    // Everyone else keeps it.
    assert!(w.perms(&plain).await.contains(&"emotes.upload".to_owned()));

    let d = w.decision(&owner, &muted, "emotes.upload").await;
    assert_eq!(d["granted"], false, "{d}");
    assert_eq!(d["effect"], "DENY");
    assert_eq!(d["roleName"], "Muted");
    assert_eq!(d["roleId"], ids::to_text(role));
    let d = w.decision(&owner, &muted, "reports.file").await;
    assert_eq!(d["granted"], true, "{d}");
    assert_eq!(d["roleName"], "User");
    // A permission no role names is not granted, and nothing decided it.
    let d = w.decision(&owner, &muted, "audit.view").await;
    assert_eq!(d["granted"], false);
    assert!(d["effect"].is_null() && d["roleId"].is_null(), "{d}");
    // You may read your own.
    let d = w.decision(&muted, &muted, "emotes.upload").await;
    assert_eq!(d["roleName"], "Muted");
    w.db.close().await;
}

#[tokio::test]
async fn the_most_senior_role_naming_a_permission_decides() {
    let Some(w) = World::new().await else { return };
    let owner = w.user("900").await;
    let a = w.user("920").await;
    let b = w.user("921").await;
    let mods = w.role("Mods", 3, &["audit.view"], &[]).await;
    let muted = w.role("Muted", 5, &["users.view"], &["audit.view"]).await;
    let trusted = w.role("Trusted", 9, &["audit.view"], &[]).await;
    w.give(&a, mods).await;
    w.give(&a, muted).await;
    // A Deny above an Allow wins.
    assert!(!w.perms(&a).await.contains(&"audit.view".to_owned()));
    assert!(w.perms(&a).await.contains(&"users.view".to_owned()));
    // An Allow above that Deny wins again.
    w.give(&a, trusted).await;
    assert!(w.perms(&a).await.contains(&"audit.view".to_owned()));
    let d = w.decision(&owner, &a, "audit.view").await;
    assert_eq!(d["effect"], "ALLOW", "{d}");
    assert_eq!(d["roleName"], "Trusted");
    // Inherit falls through: Trusted and Muted say nothing about uploading, User decides.
    let d = w.decision(&owner, &a, "emotes.upload").await;
    assert_eq!(d["roleName"], "User", "{d}");
    // At equal positions a Deny wins.
    let even = w.role("Even", 3, &[], &["audit.view"]).await;
    w.give(&b, mods).await;
    w.give(&b, even).await;
    assert!(!w.perms(&b).await.contains(&"audit.view".to_owned()));
    w.db.close().await;
}

#[tokio::test]
async fn the_owner_cant_be_denied() {
    let Some(w) = World::new().await else { return };
    let owner = w.user("900").await;
    let admin = w.user("930").await;
    let role = w
        .role("Muted", 5, &[], cyex_tv_shared::permissions::ALL)
        .await;
    w.give(&owner, role).await;
    assert_eq!(
        w.perms(&owner).await.len(),
        cyex_tv_shared::permissions::ALL.len()
    );
    let d = w.decision(&owner, &owner, "emotes.upload").await;
    assert_eq!(d["granted"], true, "{d}");
    assert_eq!(d["owner"], true);
    assert_eq!(d["roleName"], "Owner");
    // Staff see it granted as an owner's, without the top role.
    let admin_role = w.system("admin").await;
    w.gql(&owner, ASSIGN, json!({ "a": admin.tid(), "r": admin_role }))
        .await;
    let d = w.decision(&admin, &owner, "emotes.upload").await;
    assert_eq!(d["owner"], true, "{d}");
    assert!(d["roleId"].is_null() && d["roleName"].is_null(), "{d}");
    w.db.close().await;
}

#[tokio::test]
async fn reading_someones_decisions_needs_a_staff_permission() {
    let Some(w) = World::new().await else { return };
    let a = w.user("940").await;
    let b = w.user("941").await;
    let r = w.gql(&a, DECISIONS, json!({ "a": b.tid() })).await;
    assert_eq!(code(&r), "forbidden", "{r}");
    let viewer = w.role("Viewer", 4, &["users.view"], &[]).await;
    w.give(&a, viewer).await;
    let r = w.gql(&a, DECISIONS, json!({ "a": b.tid() })).await;
    assert_eq!(
        r["data"]["accountPermissions"].as_array().map(Vec::len),
        Some(cyex_tv_shared::permissions::ALL.len()),
        "{r}"
    );
    // Denying users.view above takes the read away.
    let gag = w.role("Gag", 6, &[], &["users.view"]).await;
    w.give(&a, gag).await;
    let r = w.gql(&a, DECISIONS, json!({ "a": b.tid() })).await;
    assert_eq!(code(&r), "forbidden", "{r}");
    w.db.close().await;
}

#[tokio::test]
async fn roles_are_made_and_changed_with_denies_and_audited() {
    let Some(w) = World::new().await else { return };
    let owner = w.user("900").await;
    let r = w
        .gql(
            &owner,
            CREATE,
            json!({ "n": "Muted", "p": 5, "perms": ["emotes.review"], "d": ["emotes.upload"] }),
        )
        .await;
    let role = &r["data"]["createRole"];
    assert_eq!(role["permissions"], json!(["emotes.review"]), "{r}");
    assert_eq!(role["denies"], json!(["emotes.upload"]));
    assert_eq!(role["invisible"], false);
    let id = role["id"].as_str().unwrap().to_owned();

    // Both lists naming one permission is refused.
    let r = w
        .gql(
            &owner,
            CREATE,
            json!({ "n": "Both", "p": 5, "perms": ["audit.view"], "d": ["audit.view"] }),
        )
        .await;
    assert_eq!(code(&r), "bad_permissions", "{r}");
    let r = w
        .gql(
            &owner,
            UPDATE,
            json!({ "id": id, "perms": ["audit.view"], "d": ["audit.view"] }),
        )
        .await;
    assert_eq!(code(&r), "bad_permissions", "{r}");

    // Denying one it allows moves it; the other list keeps the rest.
    let r = w
        .gql(
            &owner,
            UPDATE,
            json!({ "id": id, "d": ["emotes.review", "emotes.upload"] }),
        )
        .await;
    assert_eq!(r["data"]["updateRole"]["permissions"], json!([]), "{r}");
    assert_eq!(
        r["data"]["updateRole"]["denies"],
        json!(["emotes.review", "emotes.upload"])
    );
    // Allowing one moves it back.
    let r = w
        .gql(
            &owner,
            UPDATE,
            json!({ "id": id, "perms": ["emotes.review"] }),
        )
        .await;
    assert_eq!(
        r["data"]["updateRole"]["permissions"],
        json!(["emotes.review"]),
        "{r}"
    );
    assert_eq!(r["data"]["updateRole"]["denies"], json!(["emotes.upload"]));

    // Each change is audited with what it denies.
    let log = w
        .gql(
            &owner,
            "query($t: String!) { auditLog(targetKind: \"role\", targetId: $t) { action before after } }",
            json!({ "t": id }),
        )
        .await;
    let entries = log["data"]["auditLog"]
        .as_array()
        .unwrap_or_else(|| panic!("{log}"));
    assert_eq!(entries.len(), 3, "{log}");
    assert_eq!(entries[2]["action"], "role.created");
    let after: Value = serde_json::from_str(entries[0]["after"].as_str().unwrap()).unwrap();
    assert_eq!(after["denies"], json!(["emotes.upload"]));
    assert_eq!(after["invisible"], false);
    let before: Value = serde_json::from_str(entries[0]["before"].as_str().unwrap()).unwrap();
    assert_eq!(before["denies"], json!(["emotes.review", "emotes.upload"]));
    w.db.close().await;
}

/// Denying needs the permission as much as allowing it: a manager can't deny what it doesn't
/// hold, nor lift a Deny of it.
#[tokio::test]
async fn you_can_only_deny_what_you_hold() {
    let Some(w) = World::new().await else { return };
    let owner = w.user("900").await;
    let manager = w.user("950").await;
    let lead = w
        .role("Lead", 500, &["roles.manage", "emotes.upload"], &[])
        .await;
    w.give(&manager, lead).await;
    let r = w
        .gql(
            &manager,
            CREATE,
            json!({ "n": "Gag", "p": 5, "perms": [], "d": ["audit.view"] }),
        )
        .await;
    assert_eq!(code(&r), "escalation", "{r}");
    let r = w
        .gql(
            &manager,
            CREATE,
            json!({ "n": "Muted", "p": 5, "perms": [], "d": ["emotes.upload"] }),
        )
        .await;
    assert!(r["errors"].is_null(), "{r}");
    // A role denying audit.view: the manager may not lift that Deny; the owner may.
    let gag = ids::to_text(w.role("Gag", 6, &[], &["audit.view"]).await);
    let r = w.gql(&manager, UPDATE, json!({ "id": gag, "d": [] })).await;
    assert_eq!(code(&r), "escalation", "{r}");
    let r = w.gql(&owner, UPDATE, json!({ "id": gag, "d": [] })).await;
    assert!(r["errors"].is_null(), "{r}");
    w.db.close().await;
}

/// A Deny of roles.manage above the Admin role takes its holder out of the role managers (the
/// lock-out rule counts them) and out of the role pages.
#[tokio::test]
async fn a_deny_counts_for_who_manages_roles() {
    let Some(w) = World::new().await else { return };
    let owner = w.user("900").await;
    let admin = w.user("960").await;
    let admin_role = w.system("admin").await;
    w.gql(&owner, ASSIGN, json!({ "a": admin.tid(), "r": admin_role }))
        .await;
    let mut conn = w.db.conn().await;
    assert_eq!(
        roles::holders_of(&mut conn, "roles.manage").await.unwrap(),
        [admin.id]
    );
    drop(conn);
    let cap = w.role("Cap", 2000, &[], &["roles.manage"]).await;
    w.give(&admin, cap).await;
    let mut conn = w.db.conn().await;
    assert!(
        roles::holders_of(&mut conn, "roles.manage")
            .await
            .unwrap()
            .is_empty()
    );
    drop(conn);
    // roles.assign still opens the list; denying it too closes it.
    let r = w.gql(&admin, ROLES, json!({})).await;
    assert!(r["errors"].is_null(), "{r}");
    let mut conn = w.db.conn().await;
    roles::set_effects(&mut conn, cap, &[], &["roles.manage", "roles.assign"])
        .await
        .unwrap();
    drop(conn);
    let r = w.gql(&admin, ROLES, json!({})).await;
    assert_eq!(code(&r), "forbidden", "{r}");
    w.db.close().await;
}

#[tokio::test]
async fn invisible_roles_are_never_shown_publicly() {
    let Some(w) = World::new().await else { return };
    let owner = w.user("900").await;
    let alice = w.user("970").await;
    let stranger = w.user("971").await;
    let r = w
        .gql(
            &owner,
            CREATE,
            json!({ "n": "Quiet", "p": 50, "perms": [], "inv": true, "pub": true }),
        )
        .await;
    assert_eq!(r["data"]["createRole"]["invisible"], true, "{r}");
    let quiet = r["data"]["createRole"]["id"].as_str().unwrap().to_owned();
    let r = w
        .gql(
            &owner,
            CREATE,
            json!({ "n": "Shown", "p": 40, "perms": [], "pub": true }),
        )
        .await;
    let shown = r["data"]["createRole"]["id"].as_str().unwrap().to_owned();
    w.gql(&owner, ASSIGN, json!({ "a": alice.tid(), "r": quiet }))
        .await;
    w.gql(&owner, ASSIGN, json!({ "a": alice.tid(), "r": shown }))
        .await;

    // Others see only the visible public role.
    let p = w.gql(&stranger, PROFILE, json!({ "a": alice.tid() })).await;
    assert_eq!(
        p["data"]["accountProfile"]["roles"],
        json!([{ "name": "Shown" }]),
        "{p}"
    );
    // The holder sees it, marked.
    let me = w.gql(&alice, ME, json!({})).await;
    let mine = me["data"]["me"]["roles"].as_array().unwrap();
    assert_eq!(mine[0]["name"], "Quiet", "{me}");
    assert_eq!(mine[0]["invisible"], true);
    // Staff see it in the role list, and the owners' top role is invisible too.
    let list = w.gql(&owner, ROLES, json!({})).await;
    let all = list["data"]["roles"].as_array().unwrap();
    let found = all.iter().find(|r| r["name"] == "Quiet").unwrap();
    assert_eq!(found["invisible"], true);
    assert_eq!(all[0]["builtIn"], "ROOT");
    assert_eq!(all[0]["invisible"], true);

    // Turning it off shows it again.
    let r = w
        .gql(&owner, UPDATE, json!({ "id": quiet, "inv": false }))
        .await;
    assert_eq!(r["data"]["updateRole"]["invisible"], false, "{r}");
    let p = w.gql(&stranger, PROFILE, json!({ "a": alice.tid() })).await;
    assert_eq!(
        p["data"]["accountProfile"]["roles"],
        json!([{ "name": "Quiet" }, { "name": "Shown" }]),
        "{p}"
    );
    w.db.close().await;
}
