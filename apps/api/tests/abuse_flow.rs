//! Reports, takedowns and restrictions end to end, against a real Postgres, the in-memory store
//! and the live-update hub. Runs when `CYEX_TV_TEST_DATABASE_URL` is set (CI sets it), skips
//! otherwise.

mod common;

use std::io::Cursor;
use std::sync::Arc;
use std::time::Duration;

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use common::TestDb;
use cyex_tv_api::state::AppState;
use cyex_tv_auth::{Auth, AuthConfig, tokens};
use cyex_tv_db::accounts::{self, Profile};
use cyex_tv_db::sessions::{self, SessionKind};
use cyex_tv_db::{channels, ids, roles};
use cyex_tv_hub::Message;
use cyex_tv_shared::model::Platform;
use cyex_tv_storage::Store;
use http_body_util::BodyExt;
use image::{ImageFormat, Rgba, RgbaImage};
use serde_json::{Value, json};
use tokio::sync::watch;
use tower::ServiceExt;
use uuid::Uuid;

struct User {
    id: Uuid,
    token: String,
    twitch: String,
}

struct World {
    db: TestDb,
    app: Router,
    state: AppState,
    _stop: watch::Sender<bool>,
}

const BOUNDARY: &str = "abuseboundary0Xz";

fn png(seed: u8) -> Vec<u8> {
    let img = RgbaImage::from_fn(64, 64, |x, y| {
        Rgba([(x * 4) as u8, (y * 4) as u8, seed, 255])
    });
    let mut out = Cursor::new(Vec::new());
    img.write_to(&mut out, ImageFormat::Png).unwrap();
    out.into_inner()
}

fn code(v: &Value) -> &str {
    v["errors"][0]["extensions"]["code"]
        .as_str()
        .unwrap_or_default()
}

impl World {
    async fn new() -> Option<Self> {
        let db = TestDb::new().await?;
        let cfg = AuthConfig::from_source(|name| {
            (name == "CYEX_TV_OWNER_IDS").then(|| "twitch:9000".to_owned())
        })
        .unwrap();
        let state = AppState::new(db.pool.clone(), Arc::new(Auth::new(cfg).unwrap()))
            .with_store(Store::memory());
        let (stop, mut rx) = watch::channel(false);
        tokio::spawn(cyex_tv_hub::feeder::run(
            db.pool.clone(),
            state.hub.clone(),
            async move {
                let _ = rx.wait_for(|s| *s).await;
            },
        ));
        tokio::time::sleep(Duration::from_millis(150)).await;
        Some(Self {
            app: cyex_tv_api::app(state.clone()),
            state,
            db,
            _stop: stop,
        })
    }

    async fn user(&self, twitch_id: &str, permissions: &[&str]) -> User {
        let mut conn = self.db.conn().await;
        let login = format!("user{twitch_id}");
        let id = accounts::sign_in(
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
        .unwrap()
        .account
        .id;
        channels::upsert(&mut conn, id, Platform::Twitch, twitch_id, &login)
            .await
            .unwrap();
        if !permissions.is_empty() {
            let role = roles::create(
                &mut conn,
                &roles::NewRole {
                    name: &format!("mods {twitch_id}"),
                    description: None,
                    position: 1,
                    permissions,
                    created_by: None,
                },
            )
            .await
            .unwrap();
            roles::assign(&mut conn, id, role.id, None).await.unwrap();
        }
        let token = tokens::random_token();
        sessions::create(
            &mut conn,
            &tokens::hash(&token),
            id,
            SessionKind::App,
            chrono::Duration::days(1),
        )
        .await
        .unwrap();
        User {
            id,
            token,
            twitch: twitch_id.to_owned(),
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
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        serde_json::from_slice(&bytes).unwrap()
    }

    /// Uploads `file` as `name`; returns the reply status and body.
    async fn upload(&self, user: &User, name: &str, file: &[u8]) -> (StatusCode, Value) {
        let mut body = format!(
            "--{BOUNDARY}\r\nContent-Disposition: form-data; name=\"name\"\r\n\r\n{name}\r\n--{BOUNDARY}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"e.png\"\r\n\r\n"
        )
        .into_bytes();
        body.extend_from_slice(file);
        body.extend_from_slice(format!("\r\n--{BOUNDARY}--\r\n").as_bytes());
        let request = Request::post("/v1/emotes")
            .header(
                "content-type",
                format!("multipart/form-data; boundary={BOUNDARY}"),
            )
            .header("authorization", format!("Bearer {}", user.token))
            .body(Body::from(body))
            .unwrap();
        let response = self.app.clone().oneshot(request).await.unwrap();
        let status = response.status();
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        (
            status,
            serde_json::from_slice(&bytes).unwrap_or(Value::Null),
        )
    }

    /// Uploads, approves (as `reviewer`) and puts the emote in its owner's active channel set.
    async fn live_emote(
        &self,
        owner: &User,
        reviewer: &User,
        name: &str,
        file: &[u8],
    ) -> (String, String) {
        let (status, body) = self.upload(owner, name, file).await;
        assert_eq!(status, StatusCode::ACCEPTED, "{body}");
        let emote = body["id"].as_str().unwrap().to_owned();
        let r = self
            .gql(
                reviewer,
                "mutation($id: String!) { approveEmote(id: $id) { state } }",
                json!({ "id": emote }),
            )
            .await;
        assert_eq!(r["data"]["approveEmote"]["state"], "APPROVED", "{r}");
        let set = self
            .gql(
                owner,
                "mutation { createSet(input: { name: \"Main\", kind: CHANNEL }) { id } }",
                json!({}),
            )
            .await["data"]["createSet"]["id"]
            .as_str()
            .unwrap()
            .to_owned();
        let r = self
            .gql(
                owner,
                "mutation($s: String!, $e: String!) { addSetEmote(setId: $s, emoteId: $e) { name } }",
                json!({ "s": set, "e": emote }),
            )
            .await;
        assert!(r["errors"].is_null(), "{r}");
        let r = self
            .gql(
                owner,
                "mutation($id: String!, $s: String) { updateChannelSettings(platform: TWITCH, id: $id, input: { activeSetId: $s }) { activeSetId } }",
                json!({ "id": owner.twitch, "s": set }),
            )
            .await;
        assert!(r["errors"].is_null(), "{r}");
        (emote, set)
    }

    async fn count(&self, sql: &str) -> i64 {
        let mut conn = self.db.conn().await;
        sqlx::query_scalar(sql).fetch_one(&mut *conn).await.unwrap()
    }
}

const REPORT: &str = "mutation($id: String!, $r: String!) { reportEmote(id: $id, reason: $r) { id state reason emote { name } } }";
const RESOLVE: &str = "mutation($id: String!, $a: ReportAction!, $n: String) { resolveReport(id: $id, action: $a, note: $n) { state resolution emote { state } } }";

#[tokio::test]
async fn report_then_dismiss() {
    let Some(w) = World::new().await else { return };
    let alice = w.user("1", &[]).await;
    let bob = w.user("2", &[]).await;
    let reviewer = w.user("3", &["emotes.review"]).await;
    let (emote, _) = w.live_emote(&alice, &reviewer, "aliceOk", &png(1)).await;

    let r = w
        .gql(&bob, REPORT, json!({ "id": emote, "r": "  spam  " }))
        .await;
    assert_eq!(r["data"]["reportEmote"]["state"], "OPEN", "{r}");
    assert_eq!(r["data"]["reportEmote"]["reason"], "spam");
    let report = r["data"]["reportEmote"]["id"].as_str().unwrap().to_owned();
    assert_eq!(
        code(
            &w.gql(&bob, REPORT, json!({ "id": emote, "r": "again" }))
                .await
        ),
        "already_reported"
    );
    assert_eq!(
        code(&w.gql(&bob, REPORT, json!({ "id": emote, "r": "" })).await),
        "bad_reason"
    );

    // A pending emote someone else can't see can't be reported by them.
    let (_, pending) = w.upload(&alice, "alicePending", &png(2)).await;
    let r = w
        .gql(&bob, REPORT, json!({ "id": pending["id"], "r": "x" }))
        .await;
    assert_eq!(code(&r), "not_found");

    let queue = "{ reportQueue { total items { id reason emote { name } } } }";
    assert_eq!(code(&w.gql(&bob, queue, json!({})).await), "forbidden");
    let q = w.gql(&reviewer, queue, json!({})).await;
    assert_eq!(q["data"]["reportQueue"]["total"], 1, "{q}");
    assert_eq!(
        q["data"]["reportQueue"]["items"][0]["emote"]["name"],
        "aliceOk"
    );

    // A reviewer can dismiss, but taking down needs emotes.takedown.
    assert_eq!(
        code(
            &w.gql(
                &reviewer,
                RESOLVE,
                json!({ "id": report, "a": "TAKE_DOWN" })
            )
            .await
        ),
        "forbidden"
    );
    let r = w
        .gql(
            &reviewer,
            RESOLVE,
            json!({ "id": report, "a": "DISMISS", "n": "fine" }),
        )
        .await;
    assert_eq!(r["data"]["resolveReport"]["state"], "DISMISSED", "{r}");
    assert_eq!(
        code(
            &w.gql(&reviewer, RESOLVE, json!({ "id": report, "a": "DISMISS" }))
                .await
        ),
        "not_open"
    );
    assert_eq!(
        w.count("SELECT count(*) FROM audit_log WHERE action = 'report.dismissed'")
            .await,
        1
    );
    w.db.close().await;
}

#[tokio::test]
async fn reports_are_limited_per_account() {
    let Some(w) = World::new().await else { return };
    let alice = w.user("11", &[]).await;
    let bob = w.user("12", &[]).await;
    let reviewer = w.user("13", &["emotes.review"]).await;
    let mut emotes = Vec::new();
    for i in 0..11u8 {
        let (status, body) = w.upload(&alice, &format!("e{i}"), &png(100 + i)).await;
        assert_eq!(status, StatusCode::ACCEPTED);
        let id = body["id"].as_str().unwrap().to_owned();
        w.gql(
            &reviewer,
            "mutation($id: String!) { approveEmote(id: $id) { state } }",
            json!({ "id": id }),
        )
        .await;
        emotes.push(id);
    }
    for id in &emotes[..10] {
        let r = w.gql(&bob, REPORT, json!({ "id": id, "r": "x" })).await;
        assert!(r["errors"].is_null(), "{r}");
    }
    assert_eq!(
        code(
            &w.gql(&bob, REPORT, json!({ "id": emotes[10], "r": "x" }))
                .await
        ),
        "rate_limited"
    );
    w.db.close().await;
}

#[tokio::test]
async fn a_takedown_removes_the_image_everywhere_and_reaches_clients() {
    let Some(w) = World::new().await else { return };
    let alice = w.user("21", &[]).await;
    let bob = w.user("22", &[]).await;
    let carol = w.user("23", &[]).await;
    let moderator = w.user("24", &["emotes.review", "emotes.takedown"]).await;
    let image = png(7);
    let (alice_emote, alice_set) = w.live_emote(&alice, &moderator, "alicePic", &image).await;
    // Bob uploaded the same image: it shares the file.
    let (bob_emote, _) = w.live_emote(&bob, &moderator, "bobPic", &image).await;

    let report = w
        .gql(
            &carol,
            REPORT,
            json!({ "id": alice_emote, "r": "not theirs" }),
        )
        .await;
    let report = report["data"]["reportEmote"]["id"]
        .as_str()
        .unwrap()
        .to_owned();

    // A client watching Alice's channel, from now on (earlier events have been delivered).
    tokio::time::sleep(Duration::from_millis(400)).await;
    let mut watcher = w.state.hub.subscriber();
    watcher.subscribe([format!("channel:twitch:{}", alice.twitch).as_str()]);

    let r = w
        .gql(
            &moderator,
            RESOLVE,
            json!({ "id": report, "a": "TAKE_DOWN", "n": "copyright claim" }),
        )
        .await;
    assert_eq!(r["data"]["resolveReport"]["state"], "ACTIONED", "{r}");
    assert_eq!(r["data"]["resolveReport"]["emote"]["state"], "TAKEN_DOWN");

    // The client hears that it changed and that it left the set, within a second.
    let mut types = Vec::new();
    while types.len() < 2 {
        match tokio::time::timeout(Duration::from_secs(1), watcher.recv()).await {
            Ok(Some(Message::Event(d))) => types.push(d.event.event_type.clone()),
            other => panic!("no event in time: {other:?}"),
        }
    }
    assert_eq!(types, ["emote.updated", "set.emote_removed"]);

    // Both uploads of the image are down and out of their sets.
    for id in [&alice_emote, &bob_emote] {
        let state: String = {
            let mut conn = w.db.conn().await;
            sqlx::query_scalar("SELECT state FROM emotes WHERE id = $1")
                .bind(ids::from_text(id).unwrap())
                .fetch_one(&mut *conn)
                .await
                .unwrap()
        };
        assert_eq!(state, "taken_down");
    }
    assert_eq!(w.count("SELECT count(*) FROM emote_set_members").await, 0);
    let public = w
        .app
        .clone()
        .oneshot(
            Request::get(format!("/v1/sets/{alice_set}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let body: Value =
        serde_json::from_slice(&public.into_body().collect().await.unwrap().to_bytes()).unwrap();
    assert_eq!(body["emotes"], json!([]));

    // Blocked, queued for deletion, owners told, audited.
    assert_eq!(w.count("SELECT count(*) FROM blocked_images").await, 1);
    assert_eq!(
        w.count("SELECT count(*) FROM jobs WHERE kind = 'files.delete'")
            .await,
        1
    );
    assert_eq!(
        w.count("SELECT count(*) FROM audit_log WHERE action = 'emote.taken_down'")
            .await,
        1
    );
    let notes = w
        .gql(
            &alice,
            "{ myNotifications { kind payload read } }",
            json!({}),
        )
        .await;
    assert_eq!(
        notes["data"]["myNotifications"][0]["kind"], "emote.taken_down",
        "{notes}"
    );
    let payload: Value = serde_json::from_str(
        notes["data"]["myNotifications"][0]["payload"]
            .as_str()
            .unwrap(),
    )
    .unwrap();
    assert_eq!(payload["reason"], "copyright claim");
    let marked = w
        .gql(&alice, "mutation { markNotificationsRead }", json!({}))
        .await;
    assert_eq!(marked["data"]["markNotificationsRead"], 1);
    let unread = w
        .gql(
            &alice,
            "{ myNotifications(unreadOnly: true) { id } }",
            json!({}),
        )
        .await;
    assert_eq!(unread["data"]["myNotifications"], json!([]));
    assert_eq!(w.count("SELECT count(*) FROM notifications").await, 2);

    // The image can't come back, and a second takedown has nothing to do.
    let (status, body) = w.upload(&carol, "again", &image).await;
    assert_eq!(
        (status, body["error"].as_str()),
        (StatusCode::UNPROCESSABLE_ENTITY, Some("blocked"))
    );
    let again = w
        .gql(
            &moderator,
            "mutation($id: String!) { takedownEmote(id: $id, reason: \"x\") { state } }",
            json!({ "id": bob_emote }),
        )
        .await;
    assert_eq!(code(&again), "already_taken_down");
    w.db.close().await;
}

#[tokio::test]
async fn restricting_and_blocking_accounts() {
    let Some(w) = World::new().await else { return };
    let owner = w.user("9000", &[]).await;
    let admin = w.user("31", &["users.restrict"]).await;
    let dave = w.user("32", &[]).await;
    let restrict = "mutation($a: String!, $u: Boolean, $b: Boolean) { restrictAccount(accountId: $a, uploadsBlocked: $u, blocked: $b) { uploadsBlocked blocked } }";

    assert_eq!(
        code(
            &w.gql(
                &dave,
                restrict,
                json!({ "a": ids::to_text(admin.id), "u": true })
            )
            .await
        ),
        "forbidden"
    );
    assert_eq!(
        code(
            &w.gql(
                &admin,
                restrict,
                json!({ "a": ids::to_text(owner.id), "b": true })
            )
            .await
        ),
        "owner_protected"
    );

    let r = w
        .gql(
            &admin,
            restrict,
            json!({ "a": ids::to_text(dave.id), "u": true }),
        )
        .await;
    assert_eq!(
        r["data"]["restrictAccount"],
        json!({ "uploadsBlocked": true, "blocked": false }),
        "{r}"
    );
    let (status, body) = w.upload(&dave, "nope", &png(50)).await;
    assert_eq!(
        (status, body["error"].as_str()),
        (StatusCode::FORBIDDEN, Some("restricted"))
    );

    let r = w
        .gql(
            &admin,
            restrict,
            json!({ "a": ids::to_text(dave.id), "u": false, "b": true }),
        )
        .await;
    assert_eq!(
        r["data"]["restrictAccount"],
        json!({ "uploadsBlocked": false, "blocked": true })
    );
    // Signed out at once, and can't come back in.
    assert_eq!(
        code(&w.gql(&dave, "{ me { id } }", json!({})).await),
        "signed_out"
    );
    assert_eq!(
        w.count("SELECT count(*) FROM audit_log WHERE action = 'account.restricted'")
            .await,
        2
    );
    w.db.close().await;
}
