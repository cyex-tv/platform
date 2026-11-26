//! Per-role limits and the cap on emotes waiting for review, through the real router against a
//! real Postgres and the in-memory store. Runs when `CYEX_TV_TEST_DATABASE_URL` is set (CI sets
//! it), skips otherwise. The owner here is `twitch:900`.

mod common;

use std::io::Cursor;
use std::sync::Arc;

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use chrono::Duration;
use common::TestDb;
use cyex_tv_api::state::AppState;
use cyex_tv_auth::{Auth, AuthConfig, tokens};
use cyex_tv_db::accounts::{self, Profile};
use cyex_tv_db::emotes::{self, NewEmote};
use cyex_tv_db::ids;
use cyex_tv_db::sessions::{self, SessionKind};
use cyex_tv_shared::model::Platform;
use cyex_tv_storage::Store;
use http_body_util::BodyExt;
use image::{ImageFormat, Rgba, RgbaImage};
use serde_json::{Value, json};
use tower::ServiceExt;
use uuid::Uuid;

const BOUNDARY: &str = "cyextvlimitsboundary";

fn png(seed: u8) -> Vec<u8> {
    let img = RgbaImage::from_fn(24, 24, |x, y| Rgba([x as u8 * 9, y as u8 * 9, seed, 255]));
    let mut out = Cursor::new(Vec::new());
    img.write_to(&mut out, ImageFormat::Png).unwrap();
    out.into_inner()
}

fn form(fields: &[(&str, &str)], file: &[u8]) -> Vec<u8> {
    let mut body = Vec::new();
    for (name, value) in fields {
        body.extend_from_slice(
            format!(
                "--{BOUNDARY}\r\nContent-Disposition: form-data; name=\"{name}\"\r\n\r\n{value}\r\n"
            )
            .as_bytes(),
        );
    }
    body.extend_from_slice(
        format!(
            "--{BOUNDARY}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"e.png\"\r\nContent-Type: image/png\r\n\r\n"
        )
        .as_bytes(),
    );
    body.extend_from_slice(file);
    body.extend_from_slice(format!("\r\n--{BOUNDARY}--\r\n").as_bytes());
    body
}

/// A flat picture `side` px square: tiny as a file, big once decoded.
fn flat_png(side: u32, seed: u8) -> Vec<u8> {
    let mut out = Cursor::new(Vec::new());
    RgbaImage::from_pixel(side, side, Rgba([seed, 90, 200, 255]))
        .write_to(&mut out, ImageFormat::Png)
        .unwrap();
    out.into_inner()
}

fn cosmetic_form(kind: &str, file: &[u8]) -> Vec<u8> {
    let mut body = format!(
        "--{BOUNDARY}\r\nContent-Disposition: form-data; name=\"kind\"\r\n\r\n{kind}\r\n--{BOUNDARY}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"art.png\"\r\nContent-Type: image/png\r\n\r\n"
    )
    .into_bytes();
    body.extend_from_slice(file);
    body.extend_from_slice(format!("\r\n--{BOUNDARY}--\r\n").as_bytes());
    body
}

fn code(r: &Value) -> &str {
    r["errors"][0]["extensions"]["code"]
        .as_str()
        .unwrap_or_default()
}

struct User {
    id: Uuid,
    token: String,
}

impl User {
    fn tid(&self) -> String {
        ids::to_text(self.id)
    }
}

struct Reply {
    status: StatusCode,
    body: Value,
}

struct World {
    db: TestDb,
    app: Router,
}

const SET_LIMITS: &str = "mutation($id: String!, $l: [RoleLimitInput!]!) {
    setRoleLimits(id: $id, limits: $l) { key group default max windowSeconds value }
}";
const ROLE_LIMITS: &str =
    "query($id: String!) { roleLimits(id: $id) { key group default windowSeconds value } }";
const QUOTA: &str = "query($o: String) {
    uploadQuota(ownerId: $o) { accountId pending pendingMax uploadsThisHour uploadsPerHour maxBytes maxSide maxFrames }
}";

impl World {
    async fn new() -> Option<Self> {
        let db = TestDb::new().await?;
        let cfg = AuthConfig::from_source(|name| {
            (name == "CYEX_TV_OWNER_IDS").then(|| "twitch:900".to_owned())
        })
        .unwrap();
        let state = AppState::new(db.pool.clone(), Arc::new(Auth::new(cfg).unwrap()))
            .with_cdn_url("https://cdn.example.test")
            .with_store(Store::memory());
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

    async fn send(&self, user: &User, request: axum::http::request::Builder, body: Body) -> Reply {
        let request = request
            .header(
                "cookie",
                format!("{}={}", cyex_tv_auth::cookies::SESSION, user.token),
            )
            .header("origin", "https://cyex.tv")
            .body(body)
            .unwrap();
        let response = self.app.clone().oneshot(request).await.unwrap();
        let status = response.status();
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        Reply {
            status,
            body: serde_json::from_slice(&bytes).unwrap_or(Value::Null),
        }
    }

    async fn gql_raw(&self, user: &User, query: &str, variables: Value) -> Reply {
        self.send(
            user,
            Request::post("/v1/gql").header("content-type", "application/json"),
            Body::from(json!({ "query": query, "variables": variables }).to_string()),
        )
        .await
    }

    async fn gql(&self, user: &User, query: &str, variables: Value) -> Value {
        let r = self.gql_raw(user, query, variables).await;
        assert_eq!(r.status, StatusCode::OK, "{:?}", r.body);
        r.body
    }

    async fn upload_file(&self, user: &User, name: &str, file: &[u8]) -> Reply {
        self.send(
            user,
            Request::post("/v1/emotes").header(
                "content-type",
                format!("multipart/form-data; boundary={BOUNDARY}"),
            ),
            Body::from(form(&[("name", name)], file)),
        )
        .await
    }

    async fn upload_picture(&self, user: &User, kind: &str, file: &[u8]) -> Reply {
        self.send(
            user,
            Request::post("/v1/cosmetics/images").header(
                "content-type",
                format!("multipart/form-data; boundary={BOUNDARY}"),
            ),
            Body::from(cosmetic_form(kind, file)),
        )
        .await
    }

    async fn upload(&self, user: &User, name: &str, seed: u8, owner: Option<&User>) -> Reply {
        let owner_id = owner.map(User::tid);
        let mut fields = vec![("name", name)];
        if let Some(o) = &owner_id {
            fields.push(("owner_id", o.as_str()));
        }
        self.send(
            user,
            Request::post("/v1/emotes").header(
                "content-type",
                format!("multipart/form-data; boundary={BOUNDARY}"),
            ),
            Body::from(form(&fields, &png(seed))),
        )
        .await
    }

    /// `n` pending emotes owned by `owner`, made two hours ago, so the hourly limit doesn't count
    /// them and only the pending cap does.
    async fn pending(&self, owner: &User, n: usize) {
        let mut conn = self.db.conn().await;
        for i in 0..n {
            let e = emotes::create(
                &mut conn,
                &NewEmote {
                    owner_account_id: owner.id,
                    name: &format!("waiting{i}"),
                    ..NewEmote::default()
                },
            )
            .await
            .unwrap();
            sqlx::query("UPDATE emotes SET created_at = now() - interval '2 hours' WHERE id = $1")
                .bind(e.id)
                .execute(&mut *conn)
                .await
                .unwrap();
        }
    }

    async fn count(&self, sql: &str) -> i64 {
        let mut conn = self.db.conn().await;
        sqlx::query_scalar(sql).fetch_one(&mut *conn).await.unwrap()
    }

    async fn create_role(&self, by: &User, name: &str, position: i32, perms: &[&str]) -> String {
        let r = self
            .gql(
                by,
                "mutation($n: String!, $p: Int!, $perms: [String!]!) { createRole(input: { name: $n, position: $p, permissions: $perms }) { id } }",
                json!({ "n": name, "p": position, "perms": perms }),
            )
            .await;
        r["data"]["createRole"]["id"]
            .as_str()
            .unwrap_or_else(|| panic!("{r}"))
            .to_owned()
    }

    async fn assign(&self, by: &User, to: &User, role: &str) {
        let r = self
            .gql(
                by,
                "mutation($a: String!, $r: String!) { assignRole(accountId: $a, roleId: $r) }",
                json!({ "a": to.tid(), "r": role }),
            )
            .await;
        assert_eq!(r["data"]["assignRole"], true, "{r}");
    }

    async fn unassign(&self, by: &User, from: &User, role: &str) {
        let r = self
            .gql(
                by,
                "mutation($a: String!, $r: String!) { unassignRole(accountId: $a, roleId: $r) }",
                json!({ "a": from.tid(), "r": role }),
            )
            .await;
        assert_eq!(r["data"]["unassignRole"], true, "{r}");
    }

    async fn set_limits(&self, by: &User, role: &str, limits: Value) -> Value {
        self.gql(by, SET_LIMITS, json!({ "id": role, "l": limits }))
            .await
    }

    async fn quota(&self, user: &User, owner: Option<&User>) -> Value {
        self.gql(user, QUOTA, json!({ "o": owner.map(User::tid) }))
            .await
    }
}

fn limit<'a>(list: &'a Value, key: &str) -> &'a Value {
    list.as_array()
        .unwrap()
        .iter()
        .find(|l| l["key"] == key)
        .unwrap_or_else(|| panic!("{key} in {list}"))
}

#[tokio::test]
async fn twenty_waiting_emotes_refuse_the_next_upload() {
    let Some(w) = World::new().await else {
        return;
    };
    let alice = w.user("8101").await;
    w.pending(&alice, 20).await;

    let q = w.quota(&alice, None).await;
    assert_eq!(q["data"]["uploadQuota"]["pending"], 20, "{q}");
    assert_eq!(q["data"]["uploadQuota"]["pendingMax"], 20, "{q}");
    // The effective file limits, the same the upload checks run with.
    assert_eq!(q["data"]["uploadQuota"]["maxBytes"], 8 * 1024 * 1024, "{q}");
    assert_eq!(q["data"]["uploadQuota"]["maxSide"], 1000, "{q}");
    assert_eq!(q["data"]["uploadQuota"]["maxFrames"], 1000, "{q}");
    assert_eq!(
        q["data"]["uploadQuota"]["accountId"],
        w.db.public(alice.id).await
    );

    let r = w.upload(&alice, "oneMore", 1, None).await;
    assert_eq!(r.status, StatusCode::CONFLICT, "{:?}", r.body);
    assert_eq!(r.body["error"], "too_many_pending");
    assert_eq!(r.body["details"]["limit"], 20, "{:?}", r.body);
    assert_eq!(r.body["details"]["current"], 20, "{:?}", r.body);
    assert_eq!(
        r.body["message"],
        "You have 20 emotes waiting for review. Upload more once some are reviewed."
    );
    assert_eq!(
        w.count("SELECT count(*) FROM emotes WHERE name = 'oneMore'")
            .await,
        0
    );

    // Reviewed, rejected or deleted emotes don't wait any more.
    let mut conn = w.db.conn().await;
    sqlx::query(
        "UPDATE emotes SET state = 'approved' WHERE id = (
           SELECT id FROM emotes WHERE owner_account_id = $1 ORDER BY id LIMIT 1)",
    )
    .bind(alice.id)
    .execute(&mut *conn)
    .await
    .unwrap();
    drop(conn);
    let r = w.upload(&alice, "nowItFits", 2, None).await;
    assert_eq!(r.status, StatusCode::ACCEPTED, "{:?}", r.body);
    // The new one is pending, still being processed: it counts too.
    let q = w.quota(&alice, None).await;
    assert_eq!(q["data"]["uploadQuota"]["pending"], 20, "{q}");
    assert_eq!(q["data"]["uploadQuota"]["uploadsThisHour"], 1, "{q}");
    let r = w.upload(&alice, "tooMany", 3, None).await;
    assert_eq!(r.body["error"], "too_many_pending", "{:?}", r.body);

    let mut conn = w.db.conn().await;
    sqlx::query(
        "UPDATE emotes SET deleted_at = now() WHERE id = (
           SELECT id FROM emotes WHERE owner_account_id = $1 AND state = 'pending'
           ORDER BY id LIMIT 1)",
    )
    .bind(alice.id)
    .execute(&mut *conn)
    .await
    .unwrap();
    drop(conn);
    let r = w.upload(&alice, "afterDelete", 4, None).await;
    assert_eq!(r.status, StatusCode::ACCEPTED, "{:?}", r.body);
    w.db.close().await;
}

/// Two uploads at once with one place left: exactly one gets it.
#[tokio::test]
async fn two_parallel_uploads_at_19_waiting_let_one_in() {
    let Some(w) = World::new().await else {
        return;
    };
    let alice = w.user("8102").await;
    w.pending(&alice, 19).await;
    let (a, b) = tokio::join!(
        w.upload(&alice, "raceA", 11, None),
        w.upload(&alice, "raceB", 12, None),
    );
    let mut statuses = [a.status, b.status];
    statuses.sort();
    assert_eq!(
        statuses,
        [StatusCode::ACCEPTED, StatusCode::CONFLICT],
        "{:?} {:?}",
        a.body,
        b.body
    );
    let refused = if a.status == StatusCode::CONFLICT {
        &a
    } else {
        &b
    };
    assert_eq!(refused.body["error"], "too_many_pending");
    assert_eq!(
        w.count("SELECT count(*) FROM emotes WHERE state = 'pending' AND deleted_at IS NULL")
            .await,
        20
    );
    w.db.close().await;
}

/// An editor uploading for an account counts against that account's waiting emotes.
#[tokio::test]
async fn uploading_for_someone_counts_against_the_owner() {
    let Some(w) = World::new().await else {
        return;
    };
    let alice = w.user("8103").await;
    let bob = w.user("8104").await;
    let carol = w.user("8105").await;
    let r = w
        .gql(
            &alice,
            "mutation($a: String, $f: [EditorPermission!]!) {
                addAccountEditor(input: { accountId: $a, permissions: $f }) { permissions }
            }",
            json!({ "a": bob.tid(), "f": ["EMOTE_UPLOAD"] }),
        )
        .await;
    assert!(r["errors"].is_null(), "{r}");
    w.pending(&alice, 20).await;

    let r = w.upload(&bob, "forAlice", 21, Some(&alice)).await;
    assert_eq!(r.status, StatusCode::CONFLICT, "{:?}", r.body);
    assert_eq!(r.body["error"], "too_many_pending");
    assert_eq!(
        r.body["message"],
        "That account has 20 emotes waiting for review. Upload more once some are reviewed."
    );
    // Bob's own uploads have their own count.
    let r = w.upload(&bob, "bobsOwn", 22, None).await;
    assert_eq!(r.status, StatusCode::ACCEPTED, "{:?}", r.body);

    // The quota of the account he uploads for, as the dialog shows it.
    let q = w.quota(&bob, Some(&alice)).await;
    assert_eq!(q["data"]["uploadQuota"]["pending"], 20, "{q}");
    assert_eq!(
        q["data"]["uploadQuota"]["accountId"],
        w.db.public(alice.id).await
    );
    // Not for an account he doesn't edit.
    let q = w.quota(&carol, Some(&alice)).await;
    assert_eq!(code(&q), "forbidden", "{q}");
    w.db.close().await;
}

#[tokio::test]
async fn a_role_raises_and_lowers_limits_for_whoever_holds_it() {
    let Some(w) = World::new().await else {
        return;
    };
    let owner = w.user("900").await;
    let alice = w.user("8106").await;
    let carol = w.user("8107").await;
    let dave = w.user("8108").await;

    let partners = w.create_role(&owner, "Partners", 10, &[]).await;
    let r = w
        .set_limits(
            &owner,
            &partners,
            json!([{ "key": "pending_emotes", "value": 25 }, { "key": "prefs", "value": null }]),
        )
        .await;
    let list = &r["data"]["setRoleLimits"];
    assert_eq!(limit(list, "pending_emotes")["value"], 25, "{r}");
    assert_eq!(limit(list, "pending_emotes")["default"], 20, "{r}");
    assert_eq!(limit(list, "pending_emotes")["group"], "UPLOADS", "{r}");
    assert!(limit(list, "prefs")["value"].is_null());
    assert_eq!(limit(list, "gql_account")["windowSeconds"], 60);
    let r = w.gql(&owner, ROLE_LIMITS, json!({ "id": partners })).await;
    assert_eq!(
        limit(&r["data"]["roleLimits"], "pending_emotes")["value"],
        25
    );

    // Raised: 20 waiting, and a Partner may upload more.
    w.pending(&alice, 20).await;
    assert_eq!(
        w.upload(&alice, "before", 31, None).await.body["error"],
        "too_many_pending"
    );
    w.assign(&owner, &alice, &partners).await;
    let r = w.upload(&alice, "raised", 32, None).await;
    assert_eq!(r.status, StatusCode::ACCEPTED, "{:?}", r.body);
    let q = w.quota(&alice, None).await;
    assert_eq!(q["data"]["uploadQuota"]["pendingMax"], 25, "{q}");
    assert_eq!(q["data"]["uploadQuota"]["pending"], 21, "{q}");

    // Lowered: a Trial role alone caps it at 2; with Partners too, the more generous one wins.
    let trial = w.create_role(&owner, "Trial", 5, &[]).await;
    w.set_limits(
        &owner,
        &trial,
        json!([{ "key": "pending_emotes", "value": 2 }]),
    )
    .await;
    w.pending(&carol, 2).await;
    w.assign(&owner, &carol, &trial).await;
    let r = w.upload(&carol, "lowered", 33, None).await;
    assert_eq!(r.status, StatusCode::CONFLICT, "{:?}", r.body);
    assert_eq!(
        r.body["message"],
        "You have 2 emotes waiting for review. Upload more once some are reviewed."
    );
    w.assign(&owner, &carol, &partners).await;
    let r = w.upload(&carol, "both", 34, None).await;
    assert_eq!(r.status, StatusCode::ACCEPTED, "{:?}", r.body);
    w.unassign(&owner, &carol, &partners).await;
    assert_eq!(
        w.upload(&carol, "backDown", 35, None).await.body["error"],
        "too_many_pending"
    );

    // A request limit, counted per account at the role's rate: 3 GraphQL calls a minute.
    let slow = w.create_role(&owner, "Slow", 4, &[]).await;
    w.set_limits(&owner, &slow, json!([{ "key": "gql_account", "value": 3 }]))
        .await;
    w.assign(&owner, &dave, &slow).await;
    for _ in 0..3 {
        let r = w.gql_raw(&dave, "{ me { id } }", json!({})).await;
        assert_eq!(r.status, StatusCode::OK, "{:?}", r.body);
    }
    let r = w.gql_raw(&dave, "{ me { id } }", json!({})).await;
    assert_eq!(r.status, StatusCode::TOO_MANY_REQUESTS, "{:?}", r.body);

    // Clearing the overrides puts the defaults back.
    let r = w.set_limits(&owner, &partners, json!([])).await;
    assert!(limit(&r["data"]["setRoleLimits"], "pending_emotes")["value"].is_null());
    let q = w.quota(&alice, None).await;
    assert_eq!(q["data"]["uploadQuota"]["pendingMax"], 20, "{q}");

    // Every change is audited, a change to nothing new is not.
    assert_eq!(
        w.count("SELECT count(*) FROM audit_log WHERE action = 'role.limits_changed'")
            .await,
        4
    );
    w.set_limits(&owner, &partners, json!([])).await;
    assert_eq!(
        w.count("SELECT count(*) FROM audit_log WHERE action = 'role.limits_changed'")
            .await,
        4
    );
    w.db.close().await;
}

#[tokio::test]
async fn changing_limits_follows_the_role_edit_rules() {
    let Some(w) = World::new().await else {
        return;
    };
    let owner = w.user("900").await;
    let manager = w.user("8110").await;
    let plain = w.user("8111").await;

    let managers = w
        .create_role(&owner, "Managers", 50, &["roles.manage"])
        .await;
    let above = w.create_role(&owner, "Seniors", 60, &[]).await;
    let below = w.create_role(&owner, "Helpers", 20, &[]).await;
    w.assign(&owner, &manager, &managers).await;
    let raise = json!([{ "key": "uploads_per_hour", "value": 500 }]);

    // Without roles.manage: neither read nor write.
    let r = w.set_limits(&plain, &below, raise.clone()).await;
    assert_eq!(code(&r), "forbidden", "{r}");
    let r = w.gql(&plain, ROLE_LIMITS, json!({ "id": below })).await;
    assert_eq!(code(&r), "forbidden", "{r}");
    let r = w.gql(&plain, "{ limitDefaults { key } }", json!({})).await;
    assert_eq!(code(&r), "forbidden", "{r}");

    // A manager: not a role above them, not one they hold.
    let r = w.set_limits(&manager, &above, raise.clone()).await;
    assert_eq!(code(&r), "not_below_you", "{r}");
    // Their own top role is not below them; a lower one they hold is still theirs.
    let r = w.set_limits(&manager, &managers, raise.clone()).await;
    assert_eq!(code(&r), "not_below_you", "{r}");
    let held = w.create_role(&owner, "Held", 10, &[]).await;
    w.assign(&owner, &manager, &held).await;
    let r = w.set_limits(&manager, &held, raise.clone()).await;
    assert_eq!(code(&r), "own_role", "{r}");
    // Below them it works.
    let r = w.set_limits(&manager, &below, raise.clone()).await;
    assert_eq!(
        limit(&r["data"]["setRoleLimits"], "uploads_per_hour")["value"],
        500,
        "{r}"
    );
    let r = w
        .gql(&manager, "{ limitDefaults { key default } }", json!({}))
        .await;
    assert_eq!(
        r["data"]["limitDefaults"].as_array().unwrap().len(),
        24,
        "{r}"
    );

    // The User role's limits are the defaults.
    let user_role: String = {
        let mut conn = w.db.conn().await;
        let id: Uuid = sqlx::query_scalar("SELECT id FROM roles WHERE system = 'user'")
            .fetch_one(&mut *conn)
            .await
            .unwrap();
        ids::to_text(id)
    };
    let r = w.set_limits(&owner, &user_role, raise.clone()).await;
    assert_eq!(code(&r), "built_in_role", "{r}");

    // Only known limits, 1 to a million.
    let r = w
        .set_limits(&owner, &below, json!([{ "key": "auth", "value": 5 }]))
        .await;
    assert_eq!(code(&r), "unknown_limit", "{r}");
    let r = w
        .set_limits(&owner, &below, json!([{ "key": "redeem", "value": 0 }]))
        .await;
    assert_eq!(code(&r), "bad_limit", "{r}");

    // Refusals change nothing.
    let stored: Value = {
        let mut conn = w.db.conn().await;
        sqlx::query_scalar("SELECT limits FROM roles WHERE id = $1")
            .bind(w.db.row(&above).await)
            .fetch_one(&mut *conn)
            .await
            .unwrap()
    };
    assert_eq!(stored, json!({}));
    w.db.close().await;
}

#[tokio::test]
async fn the_owner_isnt_capped() {
    let Some(w) = World::new().await else {
        return;
    };
    let owner = w.user("900").await;
    w.pending(&owner, 20).await;
    let q = w.quota(&owner, None).await;
    assert_eq!(q["data"]["uploadQuota"]["pending"], 20, "{q}");
    assert!(q["data"]["uploadQuota"]["pendingMax"].is_null(), "{q}");
    let r = w.upload(&owner, "ownerMore", 2, None).await;
    assert_ne!(r.status, StatusCode::CONFLICT, "{:?}", r.body);
}

const LIMITS: &str = "query { limits {
    emote { maxBytes maxSide maxFrames maxTotalPixels maxDurationMs minSide maxAspect maxServedSide }
    badge { maxBytes maxSide maxFrames maxDurationMs minSide maxAspect maxServedSide }
    paint { maxBytes maxSide minSide maxAspect }
    names { emote { min max } tooltip { min max } }
    tags { max maxLen }
    rates { key count windowSeconds }
    account { key default effective windowSeconds }
} }";

#[tokio::test]
async fn limits_lists_every_limit_and_the_callers_role_overrides() {
    let Some(w) = World::new().await else {
        return;
    };
    let owner = w.user("900").await;
    let alice = w.user("8110").await;

    // Signed in: the media numbers, the field limits, the rates, and the account's own limits.
    let r = w.gql(&alice, LIMITS, json!({})).await;
    let l = &r["data"]["limits"];
    assert_eq!(l["emote"]["maxBytes"], 8 * 1024 * 1024, "{r}");
    assert_eq!(l["emote"]["maxSide"], 1000, "{r}");
    assert_eq!(l["emote"]["maxFrames"], 1000, "{r}");
    assert!(l["emote"]["maxDurationMs"].is_null(), "{r}");
    assert_eq!(l["emote"]["maxServedSide"], 128, "{r}");
    assert_eq!(l["badge"]["maxSide"], 2048, "{r}");
    assert_eq!(l["badge"]["minSide"], 72, "{r}");
    assert_eq!(l["badge"]["maxDurationMs"], 10_000, "{r}");
    assert_eq!(l["paint"]["maxSide"], 4096, "{r}");
    assert_eq!(l["paint"]["maxAspect"], 8, "{r}");
    assert_eq!(l["names"]["emote"]["max"], 100, "{r}");
    assert_eq!(l["tags"]["max"], 10, "{r}");
    assert_eq!(l["tags"]["maxLen"], 32, "{r}");
    assert!(
        l["rates"]
            .as_array()
            .unwrap()
            .iter()
            .any(|x| x["key"] == "reads"),
        "{r}"
    );
    let pending = limit(&l["account"], "pending_emotes");
    assert_eq!(pending["default"], 20, "{r}");
    assert_eq!(pending["effective"], 20, "{r}");

    // A role that raises the cap shows in the holder's `account`, and in nobody else's.
    let partners = w.create_role(&owner, "Partners", 10, &[]).await;
    w.set_limits(
        &owner,
        &partners,
        json!([{ "key": "pending_emotes", "value": 25 }]),
    )
    .await;
    w.assign(&owner, &alice, &partners).await;
    let r = w.gql(&alice, LIMITS, json!({})).await;
    let pending = limit(&r["data"]["limits"]["account"], "pending_emotes");
    assert_eq!(pending["default"], 20, "{r}");
    assert_eq!(pending["effective"], 25, "{r}");
    let bob = w.user("8111").await;
    let r = w.gql(&bob, LIMITS, json!({})).await;
    assert_eq!(
        limit(&r["data"]["limits"]["account"], "pending_emotes")["effective"],
        20,
        "{r}"
    );
    w.db.close().await;
}

const MEDIA_QUOTA: &str = "query { uploadQuota { maxBytes maxSide maxFrames } limits {
    account { key default effective windowSeconds }
} }";

async fn last_payload(w: &World, kind: &str) -> Value {
    let mut conn = w.db.conn().await;
    sqlx::query_scalar("SELECT payload FROM jobs WHERE kind = $1 ORDER BY id DESC LIMIT 1")
        .bind(kind)
        .fetch_one(&mut *conn)
        .await
        .unwrap()
}

#[tokio::test]
async fn a_role_raises_the_badge_side_and_the_upload_passes() {
    let Some(w) = World::new().await else {
        return;
    };
    let owner = w.user("900").await;
    let plain = w.user("8120").await;
    let crew = w.user("8121").await;
    let staff = w
        .create_role(&owner, "Badge staff", 20, &["cosmetics.manage"])
        .await;
    w.assign(&owner, &plain, &staff).await;
    w.assign(&owner, &crew, &staff).await;
    let supporters = w.create_role(&owner, "Supporters", 10, &[]).await;
    let r = w
        .set_limits(
            &owner,
            &supporters,
            json!([
                { "key": "badge_max_side", "value": 2300 },
                { "key": "badge_max_bytes", "value": 12 * 1024 * 1024 },
            ]),
        )
        .await;
    let list = &r["data"]["setRoleLimits"];
    assert_eq!(limit(list, "badge_max_side")["value"], 2300, "{r}");
    assert_eq!(limit(list, "badge_max_side")["default"], 2048, "{r}");
    assert_eq!(limit(list, "badge_max_side")["max"], 4096, "{r}");
    assert_eq!(limit(list, "badge_max_side")["group"], "MEDIA", "{r}");
    w.assign(&owner, &crew, &supporters).await;

    // 2100 px is over the 2048 everyone gets, and under what the Supporters role sets.
    let big = flat_png(2100, 5);
    let r = w.upload_picture(&plain, "badge", &big).await;
    assert_eq!(r.status, StatusCode::PAYLOAD_TOO_LARGE, "{:?}", r.body);
    assert_eq!(r.body["error"], "too_large");
    assert_eq!(
        w.count("SELECT count(*) FROM cosmetic_images").await,
        0,
        "a refused picture leaves nothing behind"
    );
    let r = w.upload_picture(&crew, "badge", &big).await;
    assert_eq!(r.status, StatusCode::ACCEPTED, "{:?}", r.body);
    // Both upload routes answer with the same fields: id, state, files and limits_used.
    let emote = w.upload(&crew, "ShapeCheck", 99, None).await;
    assert_eq!(emote.status, StatusCode::ACCEPTED, "{:?}", emote.body);
    for (what, body) in [("picture", &r.body), ("emote", &emote.body)] {
        assert!(body["id"].as_str().is_some_and(|s| !s.is_empty()), "{what}");
        assert!(body["state"].as_str().is_some(), "{what}: {body}");
        assert!(body["files"].is_array(), "{what}: {body}");
        let used = &body["limits_used"];
        assert!(used["kind"].is_string(), "{what}: {body}");
        for key in ["max_bytes", "max_side", "max_frames"] {
            assert!(used[key].as_u64().is_some_and(|n| n > 0), "{what} {key}");
        }
    }
    assert_eq!(emote.body["state"], "pending");
    assert_eq!(emote.body["limits_used"]["kind"], "emote");
    assert_eq!(emote.body["limits_used"]["max_side"], 1000);
    assert_eq!(r.body["state"], "processing");
    assert_eq!(r.body["limits_used"]["kind"], "badge");
    assert_eq!(r.body["limits_used"]["max_side"], 2300);
    // And still not past what the role sets.
    let r = w.upload_picture(&crew, "badge", &flat_png(2400, 6)).await;
    assert_eq!(r.status, StatusCode::PAYLOAD_TOO_LARGE, "{:?}", r.body);

    // The worker gets the limits the picture was uploaded under, and a remake keeps them.
    let stored: Value = {
        let mut conn = w.db.conn().await;
        sqlx::query_scalar("SELECT media_limits FROM cosmetic_images")
            .fetch_one(&mut *conn)
            .await
            .unwrap()
    };
    assert_eq!(stored["max_side"], 2300, "{stored}");
    assert_eq!(stored["max_bytes"], 12 * 1024 * 1024, "{stored}");
    let payload = last_payload(&w, "cosmetic.transcode").await;
    assert_eq!(payload["limits"]["max_side"], 2300, "{payload}");
    let id: String = {
        let mut conn = w.db.conn().await;
        let id: Uuid = sqlx::query_scalar("SELECT id FROM cosmetic_images")
            .fetch_one(&mut *conn)
            .await
            .unwrap();
        ids::to_text(id)
    };
    // The role changes meanwhile: the remake still goes out under the stored limits.
    w.set_limits(&owner, &supporters, json!([])).await;
    let r = w
        .gql(
            &plain,
            "mutation($id: String!) { remakeCosmeticImage(id: $id) { id } }",
            json!({ "id": id }),
        )
        .await;
    assert!(r["errors"].is_null(), "{r}");
    let payload = last_payload(&w, "cosmetic.transcode").await;
    assert_eq!(payload["limits"]["max_side"], 2300, "{payload}");
    w.db.close().await;
}

#[tokio::test]
async fn an_emote_side_can_only_go_down_from_1000() {
    let Some(w) = World::new().await else {
        return;
    };
    let owner = w.user("900").await;
    let alice = w.user("8122").await;
    let role = w.create_role(&owner, "Emote people", 10, &[]).await;

    // 1000 x 1000 is a hard ceiling: it can't be set above it.
    for value in [1001, 2048, 5000] {
        let r = w
            .set_limits(
                &owner,
                &role,
                json!([{ "key": "emote_max_side", "value": value }]),
            )
            .await;
        assert_eq!(code(&r), "bad_limit", "{r}");
    }
    let r = w
        .set_limits(
            &owner,
            &role,
            json!([{ "key": "badge_max_side", "value": 5000 }]),
        )
        .await;
    assert_eq!(code(&r), "bad_limit", "{r}");

    // Even a value written past the API (by hand) isn't believed: the upload still stops at 1000.
    {
        let mut conn = w.db.conn().await;
        sqlx::query("UPDATE roles SET limits = $2 WHERE id = $1")
            .bind(w.db.row(&role).await)
            .bind(json!({ "emote_max_side": 5000, "emote_max_bytes": 4_194_304 }))
            .execute(&mut *conn)
            .await
            .unwrap();
    }
    w.assign(&owner, &alice, &role).await;
    let r = w.upload_file(&alice, "tooWide", &flat_png(1200, 8)).await;
    assert_eq!(r.status, StatusCode::PAYLOAD_TOO_LARGE, "{:?}", r.body);
    assert_eq!(r.body["error"], "too_large");
    let q = w.gql(&alice, MEDIA_QUOTA, json!({})).await;
    assert_eq!(q["data"]["uploadQuota"]["maxSide"], 1000, "{q}");
    assert_eq!(q["data"]["uploadQuota"]["maxBytes"], 4_194_304, "{q}");

    // Lowering works: 20 px a side refuses a 24 px emote, 30 px takes it.
    w.set_limits(
        &owner,
        &role,
        json!([{ "key": "emote_max_side", "value": 20 }]),
    )
    .await;
    let r = w.upload(&alice, "smallOnly", 9, None).await;
    assert_eq!(r.status, StatusCode::PAYLOAD_TOO_LARGE, "{:?}", r.body);
    w.set_limits(
        &owner,
        &role,
        json!([{ "key": "emote_max_side", "value": 30 }]),
    )
    .await;
    let r = w.upload(&alice, "fits", 10, None).await;
    assert_eq!(r.status, StatusCode::ACCEPTED, "{:?}", r.body);
    // The transcode job carries the numbers it was checked under.
    let payload = last_payload(&w, "emote.transcode").await;
    assert_eq!(payload["limits"]["max_side"], 30, "{payload}");
    w.db.close().await;
}

#[tokio::test]
async fn the_quota_and_limits_report_the_effective_media_numbers() {
    let Some(w) = World::new().await else {
        return;
    };
    let owner = w.user("900").await;
    let alice = w.user("8123").await;
    let bob = w.user("8124").await;

    let q = w.gql(&alice, MEDIA_QUOTA, json!({})).await;
    assert_eq!(q["data"]["uploadQuota"]["maxBytes"], 8 * 1024 * 1024, "{q}");
    assert_eq!(q["data"]["uploadQuota"]["maxFrames"], 1000, "{q}");
    let account = &q["data"]["limits"]["account"];
    for (key, default) in [
        ("emote_max_bytes", 8 * 1024 * 1024),
        ("emote_max_side", 1000),
        ("emote_max_frames", 1000),
        ("badge_max_bytes", 8 * 1024 * 1024),
        ("badge_max_side", 2048),
        ("badge_max_frames", 120),
        ("paint_max_bytes", 8 * 1024 * 1024),
        ("paint_max_side", 4096),
    ] {
        assert_eq!(limit(account, key)["default"], default, "{key} {q}");
        assert_eq!(limit(account, key)["effective"], default, "{key} {q}");
        assert!(limit(account, key)["windowSeconds"].is_null(), "{key}");
    }

    let role = w.create_role(&owner, "Big files", 10, &[]).await;
    w.set_limits(
        &owner,
        &role,
        json!([
            { "key": "emote_max_bytes", "value": 12 * 1024 * 1024 },
            { "key": "emote_max_frames", "value": 1500 },
            { "key": "paint_max_side", "value": 6144 },
            { "key": "badge_max_frames", "value": 240 },
        ]),
    )
    .await;
    w.assign(&owner, &alice, &role).await;
    let q = w.gql(&alice, MEDIA_QUOTA, json!({})).await;
    assert_eq!(
        q["data"]["uploadQuota"]["maxBytes"],
        12 * 1024 * 1024,
        "{q}"
    );
    assert_eq!(q["data"]["uploadQuota"]["maxFrames"], 1500, "{q}");
    assert_eq!(q["data"]["uploadQuota"]["maxSide"], 1000, "{q}");
    let account = &q["data"]["limits"]["account"];
    assert_eq!(limit(account, "paint_max_side")["effective"], 6144, "{q}");
    assert_eq!(limit(account, "paint_max_side")["default"], 4096, "{q}");
    assert_eq!(limit(account, "badge_max_frames")["effective"], 240, "{q}");
    // Not for anyone who doesn't hold the role.
    let q = w.gql(&bob, MEDIA_QUOTA, json!({})).await;
    assert_eq!(q["data"]["uploadQuota"]["maxBytes"], 8 * 1024 * 1024, "{q}");
    assert_eq!(
        limit(&q["data"]["limits"]["account"], "paint_max_side")["effective"],
        4096,
        "{q}"
    );
    // The public numbers are the configured ones for everybody.
    let r = w.gql(&alice, LIMITS, json!({})).await;
    assert_eq!(
        r["data"]["limits"]["emote"]["maxBytes"],
        8 * 1024 * 1024,
        "{r}"
    );
    w.db.close().await;
}
