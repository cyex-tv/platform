//! The staff account dashboard and the entitlement graph (ADR 0010 amendment 2026/11/19): staff
//! notes, an account's sessions and subscriptions for staff, and where each cosmetic and role
//! came from. Through the real router against a real Postgres; runs when
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

    /// Signs in `twitch_id`; with `perms`, holding a role (at `position`) that grants them.
    async fn user(&self, twitch_id: &str, perms: &[&str], position: i32) -> User {
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
                    position,
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

    /// Runs one or more statements, separated by `;`.
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

const NOTES: &str = "query($a: String!) { staffNotes(accountId: $a) { id authorId body } }";
const ADD: &str = "mutation($a: String!, $b: String!) { addStaffNote(accountId: $a, body: $b) { id authorId body } }";
const DELETE: &str = "mutation($id: String!) { deleteStaffNote(id: $id) }";
const GRAPH: &str = "query($a: String!) { entitlementGraph(accountId: $a) { kind itemId itemName accountId source reason grantedBy grantedAt codeBatchName revokedAt revokedBy revokeReason active } }";
const HOLDERS: &str = "query($k: GrantKind!, $id: String!) { entitlementHolders(kind: $k, id: $id) { kind itemName accountId source grantedBy active revokedBy } }";

#[tokio::test]
async fn staff_notes_are_for_staff_and_audited() {
    let Some(w) = World::new().await else { return };
    let ann = w.user("1001", &["users.notes"], 10).await;
    let ben = w.user("1002", &["users.notes"], 10).await;
    let viewer = w.user("1003", &["users.view"], 10).await;
    let target = w.user("1004", &[], 0).await;
    let a = target.tid();

    assert_eq!(
        code(&w.gql(&viewer, NOTES, json!({ "a": a })).await),
        "forbidden"
    );
    assert_eq!(
        code(&w.gql(&target, NOTES, json!({ "a": a })).await),
        "forbidden"
    );
    let r = w.gql(&ann, ADD, json!({ "a": a, "b": "  " })).await;
    assert_eq!(code(&r), "bad_note");
    let r = w
        .gql(
            &ann,
            ADD,
            json!({ "a": a, "b": "Warned once for report spam." }),
        )
        .await;
    let note = r["data"]["addStaffNote"]["id"].as_str().unwrap().to_owned();
    assert_eq!(
        r["data"]["addStaffNote"]["authorId"],
        w.db.public(ann.id).await.as_str(),
        "{r}"
    );
    w.gql(&ben, ADD, json!({ "a": a, "b": "Second note." }))
        .await;

    let r = w.gql(&ben, NOTES, json!({ "a": a })).await;
    let bodies: Vec<&str> = r["data"]["staffNotes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|n| n["body"].as_str().unwrap())
        .collect();
    assert_eq!(bodies, ["Second note.", "Warned once for report spam."]);

    // Only the author deletes a note; it stays in the database, out of the list.
    assert_eq!(
        code(&w.gql(&ben, DELETE, json!({ "id": note })).await),
        "forbidden"
    );
    assert_eq!(
        w.gql(&ann, DELETE, json!({ "id": note })).await["data"]["deleteStaffNote"],
        true
    );
    assert_eq!(
        w.gql(&ann, DELETE, json!({ "id": note })).await["data"]["deleteStaffNote"],
        false
    );
    let r = w.gql(&ann, NOTES, json!({ "a": a })).await;
    assert_eq!(r["data"]["staffNotes"].as_array().unwrap().len(), 1);
    assert_eq!(w.count("SELECT count(*) FROM staff_notes").await, 2);
    assert_eq!(
        w.count("SELECT count(*) FROM audit_log WHERE action = 'account.note_added'")
            .await,
        2
    );
    assert_eq!(
        w.count("SELECT count(*) FROM audit_log WHERE action = 'account.note_deleted'")
            .await,
        1
    );
    w.db.close().await;
}

#[tokio::test]
async fn sessions_and_subscriptions_for_staff() {
    let Some(w) = World::new().await else { return };
    let sessions_staff = w.user("1101", &["users.sessions"], 10).await;
    let billing = w.user("1102", &["billing.view"], 10).await;
    let plain = w.user("1103", &[], 0).await;
    let a = plain.tid();

    let q = "query($a: String!) { accountSessions(accountId: $a) { id kind device current } }";
    assert_eq!(
        code(&w.gql(&billing, q, json!({ "a": a })).await),
        "forbidden"
    );
    let r = w.gql(&sessions_staff, q, json!({ "a": a })).await;
    let list = r["data"]["accountSessions"].as_array().unwrap();
    assert_eq!(list.len(), 1, "{r}");
    assert_eq!(list[0]["kind"], "SITE");
    assert_eq!(list[0]["current"], false);
    assert!(!r.to_string().contains("token"), "{r}");

    // No subscriptions exist yet: an empty list, for those who may see billing.
    let q = "query($a: String!) { accountSubscriptions(accountId: $a) { id state } }";
    assert_eq!(
        code(&w.gql(&sessions_staff, q, json!({ "a": a })).await),
        "forbidden"
    );
    let r = w.gql(&billing, q, json!({ "a": a })).await;
    assert_eq!(r["data"]["accountSubscriptions"], json!([]), "{r}");

    // billing.view reads one account's orders, not every order.
    let q = "query($a: String) { orders(accountId: $a) { total } }";
    assert_eq!(
        w.gql(&billing, q, json!({ "a": a })).await["data"]["orders"]["total"],
        0
    );
    assert_eq!(
        code(&w.gql(&billing, q, json!({ "a": null })).await),
        "forbidden"
    );
    w.db.close().await;
}

#[tokio::test]
async fn the_graph_says_where_everything_came_from() {
    let Some(w) = World::new().await else { return };
    let owner = w.user("900", &[], 0).await;
    let granter = w
        .user("1201", &["cosmetics.grant", "roles.manage"], 500)
        .await;
    let looker = w.user("1202", &["entitlements.view"], 10).await;
    let plain = w.user("1203", &[], 0).await;
    let dan = w.user("1204", &[], 0).await;
    let d = w.db.public(dan.id).await;

    // Two paints (no image needed) and a code batch that gave one of them.
    let (sunset, aurora, batch, code_id) =
        (ids::new_id(), ids::new_id(), ids::new_id(), ids::new_id());
    w.sql(&format!(
        "INSERT INTO paints (id, name, kind) VALUES ('{sunset}', 'Sunset', 'linear_gradient'), ('{aurora}', 'Aurora', 'linear_gradient');
         INSERT INTO code_batches (id, name, prefix, code_count, uses_per_code) VALUES ('{batch}', 'Launch week', 'LAUNCH', 1, 1);
         INSERT INTO redeem_codes (id, batch_id, code_hash) VALUES ('{code_id}', '{batch}', decode(repeat('ab', 32), 'hex'));
         INSERT INTO paint_entitlements (account_id, paint_id, source, code_id) VALUES ('{}', '{aurora}', 'code', '{code_id}');",
        dan.id
    ))
    .await;
    let sunset_t = ids::to_text(sunset);

    // Staff give Sunset, then take it away; and give a role.
    let r = w
        .gql(
            &granter,
            "mutation($a: String!, $c: String!) { grantCosmetic(input: { accountId: $a, kind: PAINT, cosmeticId: $c, reason: \"Contest winner\" }) { cosmeticId } }",
            json!({ "a": d, "c": sunset_t }),
        )
        .await;
    assert!(r["errors"].is_null(), "{r}");
    let mods = w
        .gql(
            &owner,
            "mutation { createRole(input: { name: \"Mods\", position: 50, permissions: [] }) { id } }",
            json!({}),
        )
        .await["data"]["createRole"]["id"]
        .as_str()
        .unwrap()
        .to_owned();
    w.gql(
        &granter,
        "mutation($a: String!, $r: String!) { assignRole(accountId: $a, roleId: $r) }",
        json!({ "a": d, "r": mods }),
    )
    .await;
    let r = w
        .gql(
            &granter,
            "mutation($a: String!, $c: String!) { revokeCosmetic(accountId: $a, kind: PAINT, cosmeticId: $c, reason: \"Wrong winner\") }",
            json!({ "a": d, "c": sunset_t }),
        )
        .await;
    assert_eq!(r["data"]["revokeCosmetic"], true, "{r}");

    assert_eq!(
        code(&w.gql(&plain, GRAPH, json!({ "a": d })).await),
        "forbidden"
    );
    let r = w.gql(&looker, GRAPH, json!({ "a": d })).await;
    let grants = r["data"]["entitlementGraph"].as_array().unwrap();
    let find = |name: &str, active: bool| {
        grants
            .iter()
            .find(|g| g["itemName"] == name && g["active"] == active)
            .unwrap_or_else(|| panic!("no {name} ({active}) in {r}"))
    };
    let code_grant = find("Aurora", true);
    assert_eq!(code_grant["source"], "CODE");
    assert_eq!(code_grant["codeBatchName"], "Launch week");
    let role = find("Mods", true);
    assert_eq!(role["source"], "STAFF");
    assert_eq!(role["grantedBy"], w.db.public(granter.id).await.as_str());
    let user_role = find("User", true);
    assert_eq!(user_role["source"], "DEFAULT");
    let gone = find("Sunset", false);
    assert_eq!(gone["kind"], "PAINT");
    assert_eq!(gone["source"], "STAFF");
    assert_eq!(gone["reason"], "Contest winner");
    assert_eq!(gone["grantedBy"], w.db.public(granter.id).await.as_str());
    assert!(gone["grantedAt"].is_string());
    assert_eq!(gone["revokedBy"], w.db.public(granter.id).await.as_str());
    assert_eq!(gone["revokeReason"], "Wrong winner");
    // Held ones come first.
    assert_eq!(grants[0]["active"], true);
    assert!(!r.to_string().contains("ROOT") && !r.to_string().contains("OWNER"));

    // The owner's graph shows the top role to the owner, and to nobody else.
    let o = owner.tid();
    let r = w.gql(&owner, GRAPH, json!({ "a": o })).await;
    assert!(
        r["data"]["entitlementGraph"]
            .as_array()
            .unwrap()
            .iter()
            .any(|g| g["source"] == "OWNER"),
        "{r}"
    );
    let r = w.gql(&looker, GRAPH, json!({ "a": o })).await;
    assert!(
        !r["data"]["entitlementGraph"]
            .as_array()
            .unwrap()
            .iter()
            .any(|g| g["source"] == "OWNER"),
        "{r}"
    );

    // The reverse: who holds or held Sunset, and the role.
    let r = w
        .gql(&looker, HOLDERS, json!({ "k": "PAINT", "id": sunset_t }))
        .await;
    let list = r["data"]["entitlementHolders"].as_array().unwrap();
    assert_eq!(list.len(), 1, "{r}");
    assert_eq!(list[0]["accountId"], d.as_str());
    assert_eq!(list[0]["active"], false);
    let r = w
        .gql(&looker, HOLDERS, json!({ "k": "ROLE", "id": mods }))
        .await;
    assert_eq!(
        r["data"]["entitlementHolders"][0]["accountId"],
        d.as_str(),
        "{r}"
    );
    // Built-in roles nobody is assigned have no list; the top role isn't there for others.
    let mut conn = w.db.conn().await;
    let user_role = ids::to_text(roles::system(&mut conn, "user").await.unwrap().unwrap().id);
    let root = ids::to_text(roles::system(&mut conn, "root").await.unwrap().unwrap().id);
    drop(conn);
    let r = w
        .gql(&looker, HOLDERS, json!({ "k": "ROLE", "id": user_role }))
        .await;
    assert_eq!(code(&r), "implicit_role");
    let r = w
        .gql(&looker, HOLDERS, json!({ "k": "ROLE", "id": root }))
        .await;
    assert_eq!(code(&r), "not_found");
    w.db.close().await;
}
