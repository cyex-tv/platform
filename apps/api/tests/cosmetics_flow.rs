//! Chat cosmetics (ADR 0015) end to end through the real router against a real Postgres: staff
//! create, change and delete badges and paints; grants and revokes; a selection only of owned
//! cosmetics, cleared by a revoke or a delete; permissions (non-staff, a manage-only role,
//! account editors with and without `profile`); the batch lookups (GraphQL and REST); the events;
//! validation; the picture upload and its badge rules, and making old badges again. Runs when
//! `CYEX_TV_TEST_DATABASE_URL` is set (CI sets it) and skips otherwise.

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
use cyex_tv_db::roles::{self, NewRole};
use cyex_tv_db::sessions::{self, SessionKind};
use cyex_tv_db::{cosmetics, editors, ids, outbox};
use cyex_tv_shared::model::Platform;
use cyex_tv_storage::Store;
use http_body_util::BodyExt;
use image::{ImageFormat, Rgba, RgbaImage};
use serde_json::{Value, json};
use tower::ServiceExt;
use uuid::Uuid;

/// The owner's Twitch ID in this test (`CYEX_TV_OWNER_IDS`); a made-up number.
const OWNER: &str = "900";

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
    store: Store,
}

struct Reply {
    status: StatusCode,
    body: Value,
    etag: Option<String>,
}

fn code(response: &Value) -> &str {
    response["errors"][0]["extensions"]["code"]
        .as_str()
        .unwrap_or_default()
}

fn png(w: u32, h: u32) -> Vec<u8> {
    let mut out = Cursor::new(Vec::new());
    RgbaImage::from_fn(w, h, |x, y| {
        Rgba([(x % 256) as u8, (y % 256) as u8, 90, 255])
    })
    .write_to(&mut out, ImageFormat::Png)
    .unwrap();
    out.into_inner()
}

const BOUNDARY: &str = "cyextvcosmeticsboundary42";

fn form(kind: &str, file: &[u8]) -> Vec<u8> {
    let mut body = format!(
        "--{BOUNDARY}\r\nContent-Disposition: form-data; name=\"kind\"\r\n\r\n{kind}\r\n--{BOUNDARY}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"art.png\"\r\nContent-Type: application/octet-stream\r\n\r\n"
    )
    .into_bytes();
    body.extend_from_slice(file);
    body.extend_from_slice(format!("\r\n--{BOUNDARY}--\r\n").as_bytes());
    body
}

const CREATE_BADGE: &str = "mutation($n: String!, $t: String!, $i: String!) {
    createBadge(input: { name: $n, tooltip: $t, imageId: $i }) { id name tooltip image { id host ready files { name } } }
}";
const UPDATE_BADGE: &str = "mutation($id: String!, $n: String) {
    updateBadge(id: $id, input: { name: $n }) { id name tooltip }
}";
const DELETE_BADGE: &str = "mutation($id: String!) { deleteBadge(id: $id) }";
const CREATE_PAINT: &str = "mutation($p: PaintInput!) {
    createPaint(input: $p) { id name kind angle repeat shape color stops { at color } shadows { x y radius color } image { id } }
}";
const UPDATE_PAINT: &str = "mutation($id: String!, $p: PaintInput!) {
    updatePaint(id: $id, input: $p) { id name kind angle stops { at color } }
}";
const DELETE_PAINT: &str = "mutation($id: String!) { deletePaint(id: $id) }";
const GRANT: &str = "mutation($a: String!, $k: CosmeticKind!, $c: String!, $r: String!) {
    grantCosmetic(input: { accountId: $a, kind: $k, cosmeticId: $c, reason: $r }) {
        account { id } kind cosmeticId source reason grantedBy { id }
    }
}";
const REVOKE: &str = "mutation($a: String!, $k: CosmeticKind!, $c: String!) {
    revokeCosmetic(accountId: $a, kind: $k, cosmeticId: $c, reason: \"test\")
}";
const SELECT_BOTH: &str = "mutation($b: String, $p: String, $o: String) {
    selectCosmetics(badgeId: $b, paintId: $p, ownerId: $o) { account { id } badgeId paintId badges { id } paints { id } }
}";
const SELECT_BADGE: &str = "mutation($b: String, $o: String) {
    selectCosmetics(badgeId: $b, ownerId: $o) { badgeId paintId }
}";
const MINE: &str = "query($a: String) {
    myCosmetics(accountId: $a) { account { id } badgeId paintId badges { id } paints { id } }
}";
const LOOKUP: &str = "query($ids: [String!]!) {
    userCosmetics(platform: TWITCH, userIds: $ids) { platformUserId badgeId paintId }
}";
const CATALOG: &str = "{ cosmetics { badges { id name } paints { id name kind } } }";
const ENTITLEMENTS: &str = "query($k: CosmeticKind!, $c: String) {
    cosmeticEntitlements(kind: $k, cosmeticId: $c) { total items { account { id } cosmeticId reason } }
}";
const IMAGE: &str = "query($id: String!) { cosmeticImage(id: $id) { id ready kind } }";

fn gradient(name: &str) -> Value {
    json!({
        "name": name,
        "kind": "LINEAR_GRADIENT",
        "angle": 45,
        "repeat": false,
        "color": "#ff8800",
        "stops": [{ "at": 0.0, "color": "#ff8800" }, { "at": 1.0, "color": "#8800ffcc" }],
        "shadows": [{ "x": 0.0, "y": 1.0, "radius": 2.0, "color": "#00000080" }]
    })
}

impl World {
    async fn new() -> Option<Self> {
        let db = TestDb::new().await?;
        let cfg = AuthConfig::from_source(|name| {
            (name == "CYEX_TV_OWNER_IDS").then(|| format!("twitch:{OWNER}"))
        })
        .unwrap();
        let store = Store::memory();
        let state = AppState::new(db.pool.clone(), Arc::new(Auth::new(cfg).unwrap()))
            .with_store(store.clone())
            .with_cdn_url("https://cdn.example.test");
        Some(Self {
            app: cyex_tv_api::app(state),
            db,
            store,
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
            twitch_id: twitch_id.to_owned(),
        }
    }

    /// A user holding exactly `perms` through a role.
    async fn staff(&self, twitch_id: &str, perms: &[&str]) -> User {
        let user = self.user(twitch_id).await;
        let mut conn = self.db.conn().await;
        let role = roles::create(
            &mut conn,
            &NewRole {
                name: &format!("role{twitch_id}"),
                description: None,
                position: 10,
                permissions: perms,
                created_by: None,
            },
        )
        .await
        .unwrap();
        roles::assign(&mut conn, user.id, role.id, None)
            .await
            .unwrap();
        user
    }

    async fn gql(&self, user: Option<&User>, query: &str, variables: Value) -> Value {
        let mut request = Request::post("/v1/gql").header("content-type", "application/json");
        if let Some(user) = user {
            request = request
                .header(
                    "cookie",
                    format!("{}={}", cyex_tv_auth::cookies::SESSION, user.token),
                )
                .header("origin", "https://cyex.tv");
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

    async fn get(&self, uri: &str, if_none_match: Option<&str>) -> Reply {
        self.get_as(uri, if_none_match, None).await
    }

    async fn get_as(&self, uri: &str, if_none_match: Option<&str>, user: Option<&User>) -> Reply {
        let mut request = Request::get(uri);
        if let Some(tag) = if_none_match {
            request = request.header("if-none-match", tag);
        }
        if let Some(user) = user {
            // A site session (unscoped app tokens are retired, ADR 0021).
            request = request.header(
                "cookie",
                format!("{}={}", cyex_tv_auth::cookies::SESSION, user.token),
            );
        }
        let response = self
            .app
            .clone()
            .oneshot(request.body(Body::empty()).unwrap())
            .await
            .unwrap();
        let status = response.status();
        let etag = response
            .headers()
            .get("etag")
            .and_then(|v| v.to_str().ok())
            .map(str::to_owned);
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        Reply {
            status,
            body: serde_json::from_slice(&bytes).unwrap_or(Value::Null),
            etag,
        }
    }

    async fn upload(&self, user: &User, kind: &str, file: &[u8]) -> Reply {
        let request = Request::post("/v1/cosmetics/images")
            .header(
                "content-type",
                format!("multipart/form-data; boundary={BOUNDARY}"),
            )
            .header(
                "cookie",
                format!("{}={}", cyex_tv_auth::cookies::SESSION, user.token),
            )
            .header("origin", "https://cyex.tv")
            .body(Body::from(form(kind, file)))
            .unwrap();
        let response = self.app.clone().oneshot(request).await.unwrap();
        let status = response.status();
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        Reply {
            status,
            body: serde_json::from_slice(&bytes).unwrap_or(Value::Null),
            etag: None,
        }
    }

    /// What the worker does when it finished a picture: its files are recorded.
    async fn finish(&self, image_id: &str) {
        let mut conn = self.db.conn().await;
        cosmetics::set_image_variants(
            &mut conn,
            ids::from_text(image_id).unwrap(),
            &json!([
                { "name": "1x.webp", "format": "webp", "scale": 1, "width": 18, "height": 18, "size": 100, "is_static": false },
                { "name": "2x.webp", "format": "webp", "scale": 2, "width": 36, "height": 36, "size": 200, "is_static": false },
                { "name": "4x.webp", "format": "webp", "scale": 4, "width": 72, "height": 72, "size": 400, "is_static": false }
            ]),
        )
        .await
        .unwrap();
    }

    /// An uploaded, finished picture.
    async fn picture(&self, by: &User, kind: &str, side: u32) -> String {
        let (w, h) = if kind == "badge" {
            (side, side)
        } else {
            (side * 4, side)
        };
        let r = self.upload(by, kind, &png(w, h)).await;
        assert_eq!(r.status, StatusCode::ACCEPTED, "{}", r.body);
        let id = r.body["id"].as_str().unwrap().to_owned();
        self.finish(&id).await;
        id
    }

    async fn badge(&self, by: &User, name: &str) -> String {
        let image = self.picture(by, "badge", 72 + name.len() as u32).await;
        let r = self
            .gql(
                Some(by),
                CREATE_BADGE,
                json!({ "n": name, "t": format!("{name} badge"), "i": image }),
            )
            .await;
        r["data"]["createBadge"]["id"]
            .as_str()
            .unwrap_or_else(|| panic!("{r}"))
            .to_owned()
    }

    async fn paint(&self, by: &User, name: &str) -> String {
        let r = self
            .gql(Some(by), CREATE_PAINT, json!({ "p": gradient(name) }))
            .await;
        r["data"]["createPaint"]["id"]
            .as_str()
            .unwrap_or_else(|| panic!("{r}"))
            .to_owned()
    }

    async fn grant(&self, by: &User, to: &User, kind: &str, id: &str) -> Value {
        self.gql(
            Some(by),
            GRANT,
            json!({ "a": to.tid(), "k": kind, "c": id, "r": "for testing" }),
        )
        .await
    }

    async fn events(&self) -> Vec<outbox::Event> {
        let mut conn = self.db.conn().await;
        outbox::after(&mut conn, 0, 1000).await.unwrap()
    }

    async fn audit_actions(&self) -> Vec<String> {
        let mut conn = self.db.conn().await;
        sqlx::query_scalar("SELECT action FROM audit_log ORDER BY id")
            .fetch_all(&mut *conn)
            .await
            .unwrap()
    }
}

#[tokio::test]
async fn staff_create_change_and_delete_badges_and_paints() {
    let Some(w) = World::new().await else {
        return;
    };
    let owner = w.user(OWNER).await;

    let badge = w.badge(&owner, "Founder").await;
    let paint = w.paint(&owner, "Sunset").await;

    // Unreleased (nobody holds them, not in the shop): the owner, as staff, sees them.
    let catalog = w.gql(Some(&owner), CATALOG, json!({})).await;
    assert_eq!(
        catalog["data"]["cosmetics"]["badges"][0]["id"],
        badge.as_str()
    );
    assert_eq!(
        catalog["data"]["cosmetics"]["paints"][0]["kind"],
        "LINEAR_GRADIENT"
    );

    // The REST catalog, with its ETag.
    let rest = w.get_as("/v1/cosmetics", None, Some(&owner)).await;
    assert_eq!(rest.status, StatusCode::OK);
    assert_eq!(rest.body["badges"][0]["name"], "Founder");
    let host = rest.body["badges"][0]["image"]["host"].as_str().unwrap();
    assert!(
        host.starts_with("https://cdn.example.test/cosmetics/badges/"),
        "{host}"
    );
    assert_eq!(rest.body["paints"][0]["stops"][1]["color"], "#8800FFCC");
    assert_eq!(rest.body["paints"][0]["shadows"][0]["radius"], 2.0);
    let tag = rest.etag.clone().unwrap();
    assert_eq!(
        w.get_as("/v1/cosmetics", Some(&tag), Some(&owner))
            .await
            .status,
        StatusCode::NOT_MODIFIED
    );

    // Names are unique per kind, ignoring case.
    let image = w.picture(&owner, "badge", 80).await;
    let dup = w
        .gql(
            Some(&owner),
            CREATE_BADGE,
            json!({ "n": "founder", "t": "x", "i": image }),
        )
        .await;
    assert_eq!(code(&dup), "name_taken");

    let renamed = w
        .gql(
            Some(&owner),
            UPDATE_BADGE,
            json!({ "id": badge, "n": "Pioneer" }),
        )
        .await;
    assert_eq!(renamed["data"]["updateBadge"]["name"], "Pioneer");
    assert_eq!(renamed["data"]["updateBadge"]["tooltip"], "Founder badge");
    assert_ne!(
        w.get_as("/v1/cosmetics", Some(&tag), Some(&owner))
            .await
            .status,
        StatusCode::NOT_MODIFIED
    );

    let mut changed = gradient("Sunrise");
    changed["kind"] = json!("RADIAL_GRADIENT");
    changed["shape"] = json!("ELLIPSE");
    let updated = w
        .gql(
            Some(&owner),
            UPDATE_PAINT,
            json!({ "id": paint, "p": changed }),
        )
        .await;
    assert_eq!(
        updated["data"]["updatePaint"]["kind"], "RADIAL_GRADIENT",
        "{updated}"
    );

    // An image paint takes a ready paint picture.
    let paint_image = w.picture(&owner, "paint", 64).await;
    let image_paint = w
        .gql(
            Some(&owner),
            CREATE_PAINT,
            json!({ "p": { "name": "Galaxy", "kind": "IMAGE", "imageId": paint_image, "repeat": true } }),
        )
        .await;
    assert_eq!(
        image_paint["data"]["createPaint"]["image"]["id"],
        paint_image.as_str(),
        "{image_paint}"
    );
    // A badge picture isn't a paint picture.
    let wrong = w
        .gql(
            Some(&owner),
            CREATE_PAINT,
            json!({ "p": { "name": "Wrong", "kind": "IMAGE", "imageId": image } }),
        )
        .await;
    assert_eq!(code(&wrong), "bad_image");

    assert_eq!(
        w.gql(Some(&owner), DELETE_BADGE, json!({ "id": badge }))
            .await["data"]["deleteBadge"],
        true
    );
    assert_eq!(
        w.gql(Some(&owner), DELETE_BADGE, json!({ "id": badge }))
            .await["data"]["deleteBadge"],
        false
    );
    assert_eq!(
        w.gql(Some(&owner), DELETE_PAINT, json!({ "id": paint }))
            .await["data"]["deletePaint"],
        true
    );
    let after = w.gql(Some(&owner), CATALOG, json!({})).await;
    assert_eq!(after["data"]["cosmetics"]["badges"], json!([]));
    assert_eq!(
        after["data"]["cosmetics"]["paints"]
            .as_array()
            .unwrap()
            .len(),
        1
    );

    // Every staff action is audited.
    let actions = w.audit_actions().await;
    for action in [
        "cosmetic_image.uploaded",
        "badge.created",
        "badge.updated",
        "badge.deleted",
        "paint.created",
        "paint.updated",
        "paint.deleted",
    ] {
        assert!(
            actions.iter().any(|a| a == action),
            "{action} in {actions:?}"
        );
    }

    // The public `cosmetics` topic tells only about released cosmetics: these were never held or
    // for sale, so nothing about their creation or changes is there, only the deletes.
    let events = w.events().await;
    let on_topic: Vec<&str> = events
        .iter()
        .filter(|e| e.topics == ["cosmetics"])
        .map(|e| e.event_type.as_str())
        .collect();
    assert_eq!(on_topic, ["cosmetic.deleted", "cosmetic.deleted"]);
    let w_event = cyex_tv_hub::wire(&events[0], "cosmetics");
    assert_eq!(w_event["cosmetic_id"], badge.as_str());
    w.db.close().await;
}

/// The `name`s in a catalog answer, for GraphQL (`data.cosmetics`) or REST (the body itself).
fn names(catalog: &Value, kind: &str) -> Vec<String> {
    let list = if catalog["data"].is_null() {
        &catalog[kind]
    } else {
        &catalog["data"]["cosmetics"][kind]
    };
    let mut out: Vec<String> = list
        .as_array()
        .unwrap_or_else(|| panic!("{catalog}"))
        .iter()
        .map(|c| c["name"].as_str().unwrap().to_owned())
        .collect();
    out.sort();
    out
}

#[tokio::test]
async fn unreleased_cosmetics_are_for_staff_only() {
    let Some(w) = World::new().await else {
        return;
    };
    let owner = w.user(OWNER).await;
    let viewer = w.user("910").await;
    let granter = w.staff("911", &["cosmetics.grant"]).await;
    let other_staff = w.staff("912", &["users.view"]).await;

    let secret_badge = w.badge(&owner, "Secret").await;
    let _secret_paint = w.paint(&owner, "Hidden").await;
    let held_badge = w.badge(&owner, "Held").await;
    let held_paint = w.paint(&owner, "Worn").await;
    let sale_badge = w.badge(&owner, "Sale").await;
    let later_paint = w.paint(&owner, "Later").await;
    let after_events = w.events().await.len();

    // Held by an account.
    let r = w.grant(&owner, &viewer, "BADGE", &held_badge).await;
    assert!(r["errors"].is_null(), "{r}");
    let r = w.grant(&owner, &viewer, "PAINT", &held_paint).await;
    assert!(r["errors"].is_null(), "{r}");
    // In a shop item: one on sale (visible), and one that is not (hidden).
    let item = |name: &str, badges: &[&str], paints: &[&str], visible: bool| {
        json!({
            "name": name, "description": "x", "priceCents": 499, "paddlePriceId": "pri_01testprice",
            "badgeIds": badges, "paintIds": paints, "visible": visible,
        })
    };
    for input in [
        item("On sale", &[&sale_badge], &[], true),
        item("Hidden", &[], &[&later_paint], false),
    ] {
        let r = w
            .gql(
                Some(&owner),
                "mutation($input: ShopItemInput!) { createShopItem(input: $input) { id } }",
                json!({ "input": input }),
            )
            .await;
        assert!(r["errors"].is_null(), "{r}");
    }

    let public_badges = ["Held", "Sale"];
    let public_paints = ["Worn"];
    let all_badges = ["Held", "Sale", "Secret"];
    let all_paints = ["Hidden", "Later", "Worn"];
    // The shape is the same either way; only the list is shorter.
    let signed_out = w.gql(None, CATALOG, json!({})).await;
    assert_eq!(names(&signed_out, "badges"), public_badges);
    assert_eq!(names(&signed_out, "paints"), public_paints);
    let rest = w.get("/v1/cosmetics", None).await;
    assert_eq!(rest.status, StatusCode::OK);
    assert_eq!(names(&rest.body, "badges"), public_badges);
    assert_eq!(names(&rest.body, "paints"), public_paints);
    assert!(rest.body["badges"][0]["image"]["host"].is_string());
    // A signed-in user without a cosmetics permission gets the public list.
    for who in [&viewer, &other_staff] {
        let g = w.gql(Some(who), CATALOG, json!({})).await;
        assert_eq!(names(&g, "badges"), public_badges);
        let r = w.get_as("/v1/cosmetics", None, Some(who)).await;
        assert_eq!(names(&r.body, "paints"), public_paints);
    }
    // Staff with any cosmetics permission see all of it, over both reads.
    for who in [&owner, &granter] {
        let g = w.gql(Some(who), CATALOG, json!({})).await;
        assert_eq!(names(&g, "badges"), all_badges);
        assert_eq!(names(&g, "paints"), all_paints);
        let r = w.get_as("/v1/cosmetics", None, Some(who)).await;
        assert_eq!(names(&r.body, "badges"), all_badges);
        assert_eq!(names(&r.body, "paints"), all_paints);
    }

    // The first grant of the secret badge releases it: it is public, and chat clients are told.
    let r = w.grant(&owner, &viewer, "BADGE", &secret_badge).await;
    assert!(r["errors"].is_null(), "{r}");
    let g = w.gql(None, CATALOG, json!({})).await;
    assert_eq!(names(&g, "badges"), all_badges);
    assert_eq!(names(&g, "paints"), public_paints);
    let events = w.events().await;
    let told: Vec<(&str, &str)> = events[after_events..]
        .iter()
        .filter(|e| e.topics == ["cosmetics"])
        .map(|e| {
            (
                e.event_type.as_str(),
                e.payload["cosmetic"]["name"].as_str().unwrap_or(""),
            )
        })
        .collect();
    // Released ones were announced as they were (held, or put on sale); unreleased ones never.
    assert!(told.contains(&("cosmetic.updated", "Secret")), "{told:?}");
    assert!(told.contains(&("cosmetic.updated", "Held")), "{told:?}");
    assert!(told.contains(&("cosmetic.updated", "Sale")), "{told:?}");
    assert!(
        told.iter()
            .all(|(_, name)| *name != "Hidden" && *name != "Later"),
        "{told:?}"
    );
    w.db.close().await;
}

#[tokio::test]
async fn validation_is_enforced() {
    let Some(w) = World::new().await else {
        return;
    };
    let owner = w.user(OWNER).await;
    let mut bad = gradient("Bad");
    bad["stops"] = json!([{ "at": 0.0, "color": "#ff0000" }]);
    assert_eq!(
        code(&w.gql(Some(&owner), CREATE_PAINT, json!({ "p": bad })).await),
        "bad_stops"
    );
    let mut bad = gradient("Bad");
    bad["stops"] = json!([{ "at": 0.7, "color": "#ff0000" }, { "at": 0.2, "color": "#00ff00" }]);
    assert_eq!(
        code(&w.gql(Some(&owner), CREATE_PAINT, json!({ "p": bad })).await),
        "bad_stops"
    );
    let mut bad = gradient("Bad");
    bad["color"] = json!("orange");
    assert_eq!(
        code(&w.gql(Some(&owner), CREATE_PAINT, json!({ "p": bad })).await),
        "bad_color"
    );
    let mut bad = gradient("Bad");
    bad["angle"] = json!(400);
    assert_eq!(
        code(&w.gql(Some(&owner), CREATE_PAINT, json!({ "p": bad })).await),
        "bad_angle"
    );
    let mut bad = gradient("Bad");
    bad["shadows"] = json!(vec![
        json!({ "x": 0, "y": 0, "radius": 1, "color": "#000000" });
        5
    ]);
    assert_eq!(
        code(&w.gql(Some(&owner), CREATE_PAINT, json!({ "p": bad })).await),
        "bad_shadows"
    );
    let bad = gradient("Na\u{202E}me");
    assert_eq!(
        code(&w.gql(Some(&owner), CREATE_PAINT, json!({ "p": bad })).await),
        "bad_name"
    );
    let bad = gradient(&"x".repeat(33));
    assert_eq!(
        code(&w.gql(Some(&owner), CREATE_PAINT, json!({ "p": bad })).await),
        "bad_name"
    );

    // Pictures: a badge must be square and at least 72 px.
    assert_eq!(
        w.upload(&owner, "badge", &png(36, 36)).await.status,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    assert_eq!(
        w.upload(&owner, "badge", &png(96, 72)).await.body["error"],
        "bad_dimensions"
    );
    assert_eq!(
        w.upload(&owner, "hat", &png(72, 72)).await.body["error"],
        "bad_kind"
    );
    assert_eq!(
        w.upload(&owner, "badge", b"<svg/>").await.status,
        StatusCode::UNSUPPORTED_MEDIA_TYPE
    );

    // An upload stores the original out of public reach and queues the files.
    let r = w.upload(&owner, "badge", &png(72, 72)).await;
    assert_eq!(r.status, StatusCode::ACCEPTED, "{}", r.body);
    assert_eq!(r.body["ready"], false);
    let image = r.body["id"].as_str().unwrap().to_owned();
    assert!(
        w.store
            .keys()
            .iter()
            .any(|k| k.starts_with("originals/cosmetics/"))
    );
    let mut conn = w.db.conn().await;
    let queued: i64 =
        sqlx::query_scalar("SELECT count(*) FROM jobs WHERE kind = 'cosmetic.transcode'")
            .fetch_one(&mut *conn)
            .await
            .unwrap();
    drop(conn);
    assert_eq!(queued, 1);
    // The same bytes again: the same picture.
    let again = w.upload(&owner, "badge", &png(72, 72)).await;
    assert_eq!(again.body["id"], image.as_str());
    // Not usable until its files are made.
    let early = w
        .gql(
            Some(&owner),
            CREATE_BADGE,
            json!({ "n": "Early", "t": "x", "i": image }),
        )
        .await;
    assert_eq!(code(&early), "image_not_ready");
    assert_eq!(
        w.gql(Some(&owner), IMAGE, json!({ "id": image })).await["data"]["cosmeticImage"]["ready"],
        false
    );
    w.finish(&image).await;
    assert_eq!(
        w.gql(Some(&owner), IMAGE, json!({ "id": image })).await["data"]["cosmeticImage"]["ready"],
        true
    );
    w.db.close().await;
}

#[tokio::test]
async fn permissions_are_checked() {
    let Some(w) = World::new().await else {
        return;
    };
    let owner = w.user(OWNER).await;
    let nobody = w.user("11").await;
    let maker = w.staff("12", &["cosmetics.manage"]).await;
    let granter = w.staff("13", &["cosmetics.grant"]).await;

    // No permission: no uploads, no creating, no granting, no entitlement reads.
    assert_eq!(
        w.upload(&nobody, "badge", &png(72, 72)).await.status,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        code(
            &w.gql(
                Some(&nobody),
                CREATE_PAINT,
                json!({ "p": gradient("Mine") })
            )
            .await
        ),
        "forbidden"
    );
    assert_eq!(
        code(
            &w.gql(None, CREATE_PAINT, json!({ "p": gradient("Mine") }))
                .await
        ),
        "signed_out"
    );

    // cosmetics.manage makes, but doesn't grant.
    let paint = w.paint(&maker, "Maker").await;
    assert_eq!(
        code(&w.grant(&maker, &nobody, "PAINT", &paint).await),
        "forbidden"
    );
    assert_eq!(
        code(&w.grant(&nobody, &nobody, "PAINT", &paint).await),
        "forbidden"
    );
    assert_eq!(
        code(
            &w.gql(Some(&maker), ENTITLEMENTS, json!({ "k": "PAINT" }))
                .await
        ),
        "forbidden"
    );

    // cosmetics.grant grants, but doesn't make.
    assert_eq!(
        code(
            &w.gql(
                Some(&granter),
                CREATE_PAINT,
                json!({ "p": gradient("Nope") })
            )
            .await
        ),
        "forbidden"
    );
    let granted = w.grant(&granter, &nobody, "PAINT", &paint).await;
    assert_eq!(
        granted["data"]["grantCosmetic"]["source"], "GRANT",
        "{granted}"
    );
    assert_eq!(
        granted["data"]["grantCosmetic"]["grantedBy"]["id"],
        granter.tid().as_str()
    );
    assert_eq!(
        code(&w.grant(&granter, &nobody, "PAINT", &paint).await),
        "already_owned"
    );
    assert_eq!(
        code(&w.grant(&granter, &nobody, "BADGE", &paint).await),
        "not_found"
    );
    let list = w
        .gql(
            Some(&granter),
            ENTITLEMENTS,
            json!({ "k": "PAINT", "c": paint }),
        )
        .await;
    assert_eq!(list["data"]["cosmeticEntitlements"]["total"], 1);
    assert_eq!(
        list["data"]["cosmeticEntitlements"]["items"][0]["reason"],
        "for testing"
    );

    // The owner holds both.
    let badge = w.badge(&owner, "Owner").await;
    assert!(w.grant(&owner, &nobody, "BADGE", &badge).await["errors"].is_null());
    w.db.close().await;
}

#[tokio::test]
async fn selections_are_only_of_owned_cosmetics() {
    let Some(w) = World::new().await else {
        return;
    };
    let owner = w.user(OWNER).await;
    let fan = w.user("21").await;
    let badge = w.badge(&owner, "Fan").await;
    let other = w.badge(&owner, "Other").await;
    let paint = w.paint(&owner, "Glow").await;

    // Not owned yet.
    let r = w
        .gql(Some(&fan), SELECT_BOTH, json!({ "b": badge, "p": null }))
        .await;
    assert_eq!(code(&r), "not_owned");

    w.grant(&owner, &fan, "BADGE", &badge).await;
    w.grant(&owner, &fan, "PAINT", &paint).await;
    let r = w
        .gql(Some(&fan), SELECT_BOTH, json!({ "b": badge, "p": paint }))
        .await;
    assert_eq!(
        r["data"]["selectCosmetics"]["badgeId"],
        badge.as_str(),
        "{r}"
    );
    assert_eq!(r["data"]["selectCosmetics"]["paintId"], paint.as_str());
    assert_eq!(
        r["data"]["selectCosmetics"]["badges"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        code(&w.gql(Some(&fan), SELECT_BADGE, json!({ "b": other })).await),
        "not_owned"
    );

    // Leaving a field out keeps it; null clears it.
    let r = w.gql(Some(&fan), SELECT_BADGE, json!({ "b": null })).await;
    assert_eq!(r["data"]["selectCosmetics"]["badgeId"], Value::Null);
    assert_eq!(r["data"]["selectCosmetics"]["paintId"], paint.as_str());
    w.gql(Some(&fan), SELECT_BADGE, json!({ "b": badge })).await;

    // The selection went out on the fan's user topic.
    let topic = format!("user:twitch:{}", fan.twitch_id);
    let updates: Vec<outbox::Event> = w
        .events()
        .await
        .into_iter()
        .filter(|e| e.event_type == "user.cosmetics_updated")
        .collect();
    assert_eq!(updates.len(), 3);
    assert!(updates.iter().all(|e| e.topics == [topic.clone()]));
    assert_eq!(updates[0].payload["badge_id"], badge.as_str());
    let model = cyex_tv_hub::model_event(&updates[1], &topic);
    assert_eq!(
        (model.badge_id, model.paint_id.as_deref()),
        (None, Some(paint.as_str()))
    );

    // Revoking what's shown clears it, and says so.
    let r = w
        .gql(
            Some(&owner),
            REVOKE,
            json!({ "a": fan.tid(), "k": "BADGE", "c": badge }),
        )
        .await;
    assert_eq!(r["data"]["revokeCosmetic"], true, "{r}");
    let mine = w.gql(Some(&fan), MINE, json!({})).await;
    assert_eq!(mine["data"]["myCosmetics"]["badgeId"], Value::Null);
    assert_eq!(mine["data"]["myCosmetics"]["badges"], json!([]));
    assert_eq!(mine["data"]["myCosmetics"]["paintId"], paint.as_str());
    let last = w.events().await.pop().unwrap();
    assert_eq!(last.event_type, "user.cosmetics_updated");
    assert!(last.payload["badge_id"].is_null());
    assert_eq!(
        w.gql(
            Some(&owner),
            REVOKE,
            json!({ "a": fan.tid(), "k": "BADGE", "c": badge })
        )
        .await["data"]["revokeCosmetic"],
        false
    );

    // Deleting a paint clears every selection of it.
    w.gql(Some(&owner), DELETE_PAINT, json!({ "id": paint }))
        .await;
    let mine = w.gql(Some(&fan), MINE, json!({})).await;
    assert_eq!(mine["data"]["myCosmetics"]["paintId"], Value::Null);
    assert_eq!(mine["data"]["myCosmetics"]["paints"], json!([]));

    let actions = w.audit_actions().await;
    assert!(actions.iter().any(|a| a == "cosmetic.granted"));
    assert!(actions.iter().any(|a| a == "cosmetic.revoked"));
    // An account's own selection is audited too, for its public activity feed (tests/activity.rs).
    assert!(actions.iter().any(|a| a == "cosmetics.selected"));
    w.db.close().await;
}

#[tokio::test]
async fn account_editors_with_profile_pick_for_the_owner() {
    let Some(w) = World::new().await else {
        return;
    };
    let owner = w.user(OWNER).await;
    let streamer = w.user("31").await;
    let manager = w.user("32").await;
    let modder = w.user("33").await;
    let mut conn = w.db.conn().await;
    editors::add(
        &mut conn,
        streamer.id,
        manager.id,
        &["profile".to_owned()],
        None,
    )
    .await
    .unwrap();
    editors::add(
        &mut conn,
        streamer.id,
        modder.id,
        &["emotes".to_owned(), "sets".to_owned(), "editors".to_owned()],
        None,
    )
    .await
    .unwrap();
    drop(conn);
    let badge = w.badge(&owner, "Streamer").await;
    w.grant(&owner, &streamer, "BADGE", &badge).await;

    // With `profile`: reads and picks for the owner.
    let r = w
        .gql(Some(&manager), MINE, json!({ "a": streamer.tid() }))
        .await;
    assert_eq!(
        r["data"]["myCosmetics"]["account"]["id"],
        streamer.tid().as_str(),
        "{r}"
    );
    let r = w
        .gql(
            Some(&manager),
            SELECT_BADGE,
            json!({ "b": badge, "o": streamer.tid() }),
        )
        .await;
    assert_eq!(
        r["data"]["selectCosmetics"]["badgeId"],
        badge.as_str(),
        "{r}"
    );
    let actions = w.audit_actions().await;
    assert!(actions.iter().any(|a| a == "cosmetics.selected"));

    // The editor's grant covers the owner only: their own pick needs their own badge.
    assert_eq!(
        code(
            &w.gql(Some(&manager), SELECT_BADGE, json!({ "b": badge }))
                .await
        ),
        "not_owned"
    );
    // Editors never grant.
    assert_eq!(
        code(&w.grant(&manager, &manager, "BADGE", &badge).await),
        "forbidden"
    );

    // Without `profile`: nothing.
    assert_eq!(
        code(
            &w.gql(
                Some(&modder),
                SELECT_BADGE,
                json!({ "b": null, "o": streamer.tid() })
            )
            .await
        ),
        "forbidden"
    );
    assert_eq!(
        code(
            &w.gql(Some(&modder), MINE, json!({ "a": streamer.tid() }))
                .await
        ),
        "forbidden"
    );
    // And someone who isn't an editor at all.
    assert_eq!(
        code(
            &w.gql(Some(&modder), MINE, json!({ "a": owner.tid() }))
                .await
        ),
        "forbidden"
    );

    // Removing the grant takes effect at once.
    let mut conn = w.db.conn().await;
    editors::remove(&mut conn, streamer.id, manager.id)
        .await
        .unwrap();
    drop(conn);
    assert_eq!(
        code(
            &w.gql(
                Some(&manager),
                SELECT_BADGE,
                json!({ "b": null, "o": streamer.tid() })
            )
            .await
        ),
        "forbidden"
    );
    w.db.close().await;
}

#[tokio::test]
async fn chatters_are_looked_up_in_batches() {
    let Some(w) = World::new().await else {
        return;
    };
    let owner = w.user(OWNER).await;
    let a = w.user("41").await;
    let b = w.user("42").await;
    let quiet = w.user("43").await;
    let badge = w.badge(&owner, "Lookup").await;
    let paint = w.paint(&owner, "Lookup").await;
    for user in [&a, &b] {
        w.grant(&owner, user, "BADGE", &badge).await;
    }
    w.grant(&owner, &b, "PAINT", &paint).await;
    w.gql(Some(&a), SELECT_BADGE, json!({ "b": badge })).await;
    w.gql(Some(&b), SELECT_BOTH, json!({ "b": badge, "p": paint }))
        .await;

    let ids = json!(["41", "42", "43", "999", "41", "not-a-number"]);
    let r = w.gql(None, LOOKUP, json!({ "ids": ids })).await;
    assert_eq!(
        r["data"]["userCosmetics"],
        json!([
            { "platformUserId": "41", "badgeId": badge, "paintId": null },
            { "platformUserId": "42", "badgeId": badge, "paintId": paint },
        ])
    );
    let _ = quiet;

    let rest = w
        .get("/v1/cosmetics/users?platform=twitch&ids=42,41,43", None)
        .await;
    assert_eq!(rest.status, StatusCode::OK);
    assert_eq!(rest.body["users"][0]["platform_user_id"], "41");
    assert_eq!(rest.body["users"][1]["paint_id"], paint.as_str());
    let tag = rest.etag.unwrap();
    assert_eq!(
        w.get(
            "/v1/cosmetics/users?platform=twitch&ids=42,41,43",
            Some(&tag)
        )
        .await
        .status,
        StatusCode::NOT_MODIFIED
    );
    assert_eq!(
        w.get("/v1/cosmetics/users?platform=kick&ids=41", None)
            .await
            .body["users"],
        json!([])
    );
    let many: Vec<String> = (0..101).map(|i| i.to_string()).collect();
    let r = w
        .get(
            &format!("/v1/cosmetics/users?platform=twitch&ids={}", many.join(",")),
            None,
        )
        .await;
    assert_eq!(r.status, StatusCode::BAD_REQUEST);
    assert_eq!(
        code(&w.gql(None, LOOKUP, json!({ "ids": many })).await),
        "too_many"
    );

    // A blocked account shows nothing.
    let mut conn = w.db.conn().await;
    accounts::set_blocked(&mut conn, a.id, true).await.unwrap();
    drop(conn);
    let r = w.gql(None, LOOKUP, json!({ "ids": ["41", "42"] })).await;
    assert_eq!(r["data"]["userCosmetics"].as_array().unwrap().len(), 1);
    // And can't be granted anything.
    assert_eq!(
        code(&w.grant(&owner, &a, "PAINT", &paint).await),
        "not_found"
    );
    w.db.close().await;
}

#[tokio::test]
async fn known_pictures_whose_original_is_gone_are_stored_and_made_again() {
    let Some(w) = World::new().await else {
        return;
    };
    let owner = w.user(OWNER).await;
    // A finished picture, then the storage moved and took everything with it.
    let id = w.picture(&owner, "badge", 72).await;
    for key in w.store.keys() {
        w.store.delete(&key).await.unwrap();
    }
    // The same bytes again: the same picture, not ready, its original back and a new job queued.
    let r = w.upload(&owner, "badge", &png(72, 72)).await;
    assert_eq!(r.status, StatusCode::ACCEPTED, "{}", r.body);
    assert_eq!(r.body["id"], id.as_str());
    assert_eq!(r.body["ready"], false);
    assert!(
        w.store
            .keys()
            .iter()
            .any(|k| k.starts_with("originals/cosmetics/"))
    );
    let mut conn = w.db.conn().await;
    let queued: i64 =
        sqlx::query_scalar("SELECT count(*) FROM jobs WHERE kind = 'cosmetic.transcode'")
            .fetch_one(&mut *conn)
            .await
            .unwrap();
    assert_eq!(queued, 2);
    // A picture whose original is still there stays as it was.
    w.finish(&id).await;
    drop(conn);
    let same = w.upload(&owner, "badge", &png(72, 72)).await;
    assert_eq!(same.body["ready"], true, "{}", same.body);
}

/// A 16-bit RGBA PNG of noise: it doesn't compress, so it is about `side² × 8` bytes.
fn noise_png(side: u32) -> Vec<u8> {
    let mut seed = 0x2545_f491_4f6c_dd1du64;
    let mut next = move || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed as u16
    };
    let picture: image::ImageBuffer<Rgba<u16>, Vec<u16>> =
        image::ImageBuffer::from_fn(side, side, |_, _| Rgba([next(), next(), next(), 65535]));
    let mut out = Cursor::new(Vec::new());
    picture.write_to(&mut out, ImageFormat::Png).unwrap();
    out.into_inner()
}

fn gif(side: u32, frames: u32, delay_ms: u32) -> Vec<u8> {
    use image::codecs::gif::{GifEncoder, Repeat};
    use image::{Delay, Frame};
    let mut out = Vec::new();
    {
        let mut enc = GifEncoder::new(&mut out);
        enc.set_repeat(Repeat::Infinite).unwrap();
        for i in 0..frames {
            let frame = RgbaImage::from_fn(side, side, |x, y| {
                Rgba([(x * 3) as u8, (y * 3) as u8, (i * 2) as u8, 255])
            });
            enc.encode_frame(Frame::from_parts(
                frame,
                0,
                0,
                Delay::from_numer_denom_ms(delay_ms, 1),
            ))
            .unwrap();
        }
    }
    out
}

fn jpeg(side: u32) -> Vec<u8> {
    let mut out = Cursor::new(Vec::new());
    image::DynamicImage::ImageRgba8(RgbaImage::from_fn(side, side, |x, y| {
        Rgba([x as u8, y as u8, 40, 255])
    }))
    .to_rgb8()
    .write_to(&mut out, ImageFormat::Jpeg)
    .unwrap();
    out.into_inner()
}

const REMAKE: &str = "mutation($id: String!) { remakeCosmeticImage(id: $id) { id ready } }";

impl World {
    /// What the worker records for a badge made on the size ladder.
    async fn finish_ladder(&self, image_id: &str) {
        let files: Vec<Value> = [
            ("1x", 1, 18),
            ("2x", 2, 36),
            ("4x", 4, 72),
            ("128", 7, 128),
            ("256", 14, 256),
            ("512", 28, 512),
        ]
        .iter()
        .map(|(stem, scale, px)| {
            json!({ "name": format!("{stem}.webp"), "format": "webp", "scale": scale, "width": px, "height": px, "size": 100, "is_static": false })
        })
        .collect();
        let mut conn = self.db.conn().await;
        cosmetics::set_image_variants(&mut conn, ids::from_text(image_id).unwrap(), &json!(files))
            .await
            .unwrap();
    }

    async fn transcode_jobs(&self) -> i64 {
        let mut conn = self.db.conn().await;
        sqlx::query_scalar("SELECT count(*) FROM jobs WHERE kind = 'cosmetic.transcode'")
            .fetch_one(&mut *conn)
            .await
            .unwrap()
    }
}

#[tokio::test]
async fn badge_pictures_follow_the_badge_upload_rules() {
    let Some(w) = World::new().await else {
        return;
    };
    let owner = w.user(OWNER).await;

    // Up to 8 MB for a badge; a paint keeps its 5 MB.
    let big = noise_png(900);
    assert!(big.len() > 5 * 1024 * 1024 && big.len() < 8 * 1024 * 1024);
    let r = w.upload(&owner, "badge", &big).await;
    assert_eq!(r.status, StatusCode::ACCEPTED, "{}", r.body);
    let r = w.upload(&owner, "paint", &big).await;
    assert_eq!(r.status, StatusCode::PAYLOAD_TOO_LARGE, "{}", r.body);
    assert_eq!(
        r.body["message"],
        "The file is over 5 MB, the most an upload can be."
    );
    let over = noise_png(1060);
    assert!(over.len() > 8 * 1024 * 1024);
    let r = w.upload(&owner, "badge", &over).await;
    assert_eq!(r.status, StatusCode::PAYLOAD_TOO_LARGE, "{}", r.body);
    assert_eq!(r.body["error"], "too_large");

    // JPEG is a badge picture too; still square and at least 72 px.
    let r = w.upload(&owner, "badge", &jpeg(96)).await;
    assert_eq!(r.status, StatusCode::ACCEPTED, "{}", r.body);
    assert_eq!(
        w.upload(&owner, "badge", &jpeg(64)).await.body["error"],
        "bad_dimensions"
    );

    // Animated: at most 120 frames and 10 seconds.
    let r = w.upload(&owner, "badge", &gif(72, 120, 50)).await;
    assert_eq!(r.status, StatusCode::ACCEPTED, "{}", r.body);
    assert_eq!(r.body["animated"], true);
    let r = w.upload(&owner, "badge", &gif(72, 121, 50)).await;
    assert_eq!(r.status, StatusCode::PAYLOAD_TOO_LARGE, "{}", r.body);
    assert_eq!(
        r.body["message"],
        "The animation has more than 120 frames, the most it can have."
    );
    let r = w.upload(&owner, "badge", &gif(72, 6, 2000)).await;
    assert_eq!(r.status, StatusCode::PAYLOAD_TOO_LARGE, "{}", r.body);
    assert_eq!(
        r.body["message"],
        "The animation runs longer than 10 seconds, the most it can."
    );
    // Paints keep their own rules: no 120-frame or 10-second cap.
    let r = w.upload(&owner, "paint", &gif(72, 6, 2000)).await;
    assert_eq!(r.status, StatusCode::ACCEPTED, "{}", r.body);
    w.db.close().await;
}

#[tokio::test]
async fn badges_made_before_the_ladder_are_made_again() {
    let Some(w) = World::new().await else {
        return;
    };
    let owner = w.user(OWNER).await;
    let nobody = w.user("11").await;
    // Made with 1x, 2x and 4x only (`finish`): the same bytes again queue the larger sizes.
    let id = w.picture(&owner, "badge", 80).await;
    let before = w.transcode_jobs().await;
    let r = w.upload(&owner, "badge", &png(80, 80)).await;
    assert_eq!(r.body["id"], id.as_str());
    assert_eq!(r.body["ready"], true, "its files stay in use meanwhile");
    assert_eq!(w.transcode_jobs().await, before + 1);
    // Made on the ladder: nothing to do.
    w.finish_ladder(&id).await;
    w.upload(&owner, "badge", &png(80, 80)).await;
    assert_eq!(w.transcode_jobs().await, before + 1);

    // Staff can ask for it outright, audited; others can't.
    assert_eq!(
        code(&w.gql(Some(&nobody), REMAKE, json!({ "id": id })).await),
        "forbidden"
    );
    let r = w.gql(Some(&owner), REMAKE, json!({ "id": id })).await;
    assert_eq!(r["data"]["remakeCosmeticImage"]["id"], id.as_str(), "{r}");
    assert_eq!(r["data"]["remakeCosmeticImage"]["ready"], true);
    assert_eq!(w.transcode_jobs().await, before + 2);
    assert!(
        w.audit_actions()
            .await
            .contains(&"cosmetic_image.remade".to_owned())
    );
    assert_eq!(
        code(
            &w.gql(
                Some(&owner),
                REMAKE,
                json!({ "id": ids::to_text(ids::new_id()) })
            )
            .await
        ),
        "not_found"
    );
    // The original gone: say so, queue nothing.
    for key in w.store.keys() {
        w.store.delete(&key).await.unwrap();
    }
    assert_eq!(
        code(&w.gql(Some(&owner), REMAKE, json!({ "id": id })).await),
        "original_missing"
    );
    assert_eq!(w.transcode_jobs().await, before + 2);
    w.db.close().await;
}
