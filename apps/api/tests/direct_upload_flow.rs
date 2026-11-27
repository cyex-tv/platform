//! Direct uploads of badge and paint pictures (ADR 0036) through the real router against a real
//! Postgres and the in-memory store: the presign's checks (sign-in, permission, type, size, the
//! open-upload cap), the URL and headers, finalize before and after the bytes arrive, a size that
//! differs from the declared one, a file that isn't a picture, someone else's upload, an expired
//! one, a second finalize, and the multipart route still working beside it. Runs when
//! `CYEX_TV_TEST_DATABASE_URL` is set (CI sets it) and skips otherwise.

mod common;

use std::io::Cursor;
use std::sync::Arc;

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use bytes::Bytes;
use chrono::Duration;
use common::TestDb;
use cyex_tv_auth::{Auth, AuthConfig, tokens};
use cyex_tv_db::accounts::{self, Profile};
use cyex_tv_db::roles::{self, NewRole};
use cyex_tv_db::sessions::{self, SessionKind};
use cyex_tv_db::{direct_uploads, ids};
use cyex_tv_shared::model::Platform;
use cyex_tv_storage::{Put, Store, upload_key};
use http_body_util::BodyExt;
use image::{ImageFormat, Rgba, RgbaImage};
use serde_json::{Value, json};
use tower::ServiceExt;
use uuid::Uuid;

struct User {
    id: Uuid,
    token: String,
}

struct World {
    db: TestDb,
    app: Router,
    store: Store,
}

struct Reply {
    status: StatusCode,
    body: Value,
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

impl World {
    async fn new() -> Option<Self> {
        let db = TestDb::new().await?;
        let cfg = AuthConfig::from_source(|_| None).unwrap();
        let store = Store::memory();
        let state =
            cyex_tv_api::state::AppState::new(db.pool.clone(), Arc::new(Auth::new(cfg).unwrap()))
                .with_store(store.clone())
                .with_cdn_url("https://cdn.example.test");
        Some(Self {
            app: cyex_tv_api::app(state),
            db,
            store,
        })
    }

    async fn user(&self, twitch_id: &str, perms: &[&str]) -> User {
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
        if !perms.is_empty() {
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
            roles::assign(&mut conn, signed.account.id, role.id, None)
                .await
                .unwrap();
        }
        User {
            id: signed.account.id,
            token,
        }
    }

    async fn post(&self, user: Option<&User>, uri: &str, body: Option<Value>) -> Reply {
        let mut request = Request::post(uri).header("content-type", "application/json");
        if let Some(user) = user {
            request = request
                .header(
                    "cookie",
                    format!("{}={}", cyex_tv_auth::cookies::SESSION, user.token),
                )
                .header("origin", "https://cyex.tv");
        }
        let body = body.map_or_else(Body::empty, |b| Body::from(b.to_string()));
        let response = self
            .app
            .clone()
            .oneshot(request.body(body).unwrap())
            .await
            .unwrap();
        let status = response.status();
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        Reply {
            status,
            body: serde_json::from_slice(&bytes).unwrap_or(Value::Null),
        }
    }

    async fn presign(&self, user: &User, kind: &str, size: usize, content_type: &str) -> Reply {
        self.post(
            Some(user),
            "/v1/cosmetics/images/uploads",
            Some(json!({ "kind": kind, "size": size, "content_type": content_type })),
        )
        .await
    }

    async fn finalize(&self, user: &User, id: &str) -> Reply {
        self.post(
            Some(user),
            &format!("/v1/cosmetics/images/uploads/{id}/finalize"),
            None,
        )
        .await
    }

    /// What the browser's `PUT` does: the bytes land at the upload's key.
    async fn put(&self, id: &str, bytes: &[u8]) {
        self.store
            .put(
                &upload_key(id),
                Bytes::copy_from_slice(bytes),
                Put {
                    content_type: "image/png",
                    cache_control: None,
                },
            )
            .await
            .unwrap();
    }
}

#[tokio::test]
async fn a_badge_goes_straight_to_storage_and_is_finalized() {
    let Some(w) = World::new().await else { return };
    let staff = w.user("1", &["cosmetics.manage"]).await;
    let file = png(300, 300);

    let r = w.presign(&staff, "badge", file.len(), "image/png").await;
    assert_eq!(r.status, StatusCode::CREATED, "{}", r.body);
    let id = r.body["id"].as_str().unwrap().to_owned();
    assert_eq!(r.body["method"], "PUT");
    assert_eq!(r.body["kind"], "badge");
    assert_eq!(r.body["url"], format!("memory:///uploads/{id}"));
    assert_eq!(r.body["headers"]["content-type"], "image/png");
    assert_eq!(r.body["headers"]["content-length"], file.len().to_string());
    assert!(r.body["expires_at"].is_string());

    // Nothing was PUT yet.
    let r = w.finalize(&staff, &id).await;
    assert_eq!(r.status, StatusCode::CONFLICT, "{}", r.body);
    assert_eq!(r.body["error"], "upload_missing");

    w.put(&id, &file).await;
    let r = w.finalize(&staff, &id).await;
    assert_eq!(r.status, StatusCode::ACCEPTED, "{}", r.body);
    assert_eq!(r.body["state"], "processing");
    assert_eq!(r.body["limits_used"]["kind"], "badge");
    let image = r.body["id"].as_str().unwrap().to_owned();
    // The picture keeps its original; the staging object is gone.
    let keys = w.store.keys();
    assert!(
        keys.iter().any(|k| k.starts_with("originals/cosmetics/")),
        "{keys:?}"
    );
    assert!(!keys.contains(&upload_key(&id)), "{keys:?}");
    let mut conn = w.db.conn().await;
    let row = direct_uploads::get(&mut conn, ids::from_text(&id).unwrap())
        .await
        .unwrap()
        .unwrap();
    assert!(row.finalized_at.is_some());
    assert_eq!(row.image_id.map(ids::to_text), Some(image.clone()));

    let r = w.finalize(&staff, &id).await;
    assert_eq!(r.status, StatusCode::CONFLICT);
    assert_eq!(r.body["error"], "already_finalized");

    // The multipart route still takes pictures, and the same bytes give the same picture.
    let boundary = "cyextvdirectboundary";
    let mut form = format!(
        "--{boundary}\r\nContent-Disposition: form-data; name=\"kind\"\r\n\r\nbadge\r\n--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"a.png\"\r\nContent-Type: image/png\r\n\r\n"
    )
    .into_bytes();
    form.extend_from_slice(&file);
    form.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
    let request = Request::post("/v1/cosmetics/images")
        .header(
            "content-type",
            format!("multipart/form-data; boundary={boundary}"),
        )
        .header(
            "cookie",
            format!("{}={}", cyex_tv_auth::cookies::SESSION, staff.token),
        )
        .header("origin", "https://cyex.tv")
        .body(Body::from(form))
        .unwrap();
    let response = w.app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::ACCEPTED);
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let body: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(body["id"], image);
    drop(conn);
    w.db.close().await;
}

#[tokio::test]
async fn the_presign_checks_what_an_upload_would() {
    let Some(w) = World::new().await else { return };
    let staff = w.user("1", &["cosmetics.manage"]).await;
    let plain = w.user("2", &[]).await;

    let r = w
        .post(
            None,
            "/v1/cosmetics/images/uploads",
            Some(json!({ "kind": "badge", "size": 10, "content_type": "image/png" })),
        )
        .await;
    assert_eq!(r.status, StatusCode::UNAUTHORIZED);
    let r = w.presign(&plain, "badge", 10, "image/png").await;
    assert_eq!(r.status, StatusCode::FORBIDDEN, "{}", r.body);

    let r = w.presign(&staff, "emote", 10, "image/png").await;
    assert_eq!(
        (r.status, r.body["error"].as_str()),
        (StatusCode::BAD_REQUEST, Some("bad_kind"))
    );
    let r = w.presign(&staff, "badge", 10, "image/svg+xml").await;
    assert_eq!(r.status, StatusCode::UNSUPPORTED_MEDIA_TYPE);
    assert_eq!(r.body["error"], "unsupported_format");
    let r = w.presign(&staff, "badge", 0, "image/png").await;
    assert_eq!(r.body["error"], "bad_form");
    let r = w
        .post(
            Some(&staff),
            "/v1/cosmetics/images/uploads",
            Some(json!({ "kind": "badge" })),
        )
        .await;
    assert_eq!(r.body["error"], "bad_form");

    // Over the badge byte cap (8 MB by default): refused with the numbers, no URL.
    let over = 8 * 1024 * 1024 + 1;
    let r = w.presign(&staff, "badge", over, "image/png").await;
    assert_eq!(r.status, StatusCode::PAYLOAD_TOO_LARGE, "{}", r.body);
    assert_eq!(r.body["error"], "too_large");
    assert_eq!(r.body["details"]["limit"], 8 * 1024 * 1024);
    assert_eq!(r.body["details"]["current"], over);

    // At most 10 open at once.
    let mut conn = w.db.conn().await;
    for _ in 0..10 {
        let id = ids::new_id();
        direct_uploads::insert(
            &mut conn,
            &direct_uploads::NewUpload {
                id,
                account_id: staff.id,
                kind: cyex_tv_shared::model::CosmeticKind::Badge,
                storage_key: &upload_key(&ids::to_text(id)),
                content_type: "image/png",
                declared_bytes: 10,
                expires_at: chrono::Utc::now() + Duration::hours(1),
            },
        )
        .await
        .unwrap();
    }
    let r = w.presign(&staff, "badge", 10, "image/png").await;
    assert_eq!(r.status, StatusCode::CONFLICT, "{}", r.body);
    assert_eq!(r.body["error"], "too_many_uploads");
    assert_eq!(r.body["details"]["limit"], 10);
    assert_eq!(r.body["details"]["current"], 10);
    drop(conn);
    w.db.close().await;
}

async fn presign_ok(w: &World, user: &User, size: usize) -> String {
    let r = w.presign(user, "badge", size, "image/png").await;
    assert_eq!(r.status, StatusCode::CREATED, "{}", r.body);
    r.body["id"].as_str().unwrap().to_owned()
}

#[tokio::test]
async fn finalize_refuses_what_it_should_and_discards_it() {
    let Some(w) = World::new().await else { return };
    let staff = w.user("1", &["cosmetics.manage"]).await;
    let other = w.user("2", &["cosmetics.manage"]).await;
    let file = png(100, 100);

    // More bytes than declared: refused, and the object and upload are gone.
    let id = presign_ok(&w, &staff, file.len()).await;
    w.put(&id, &[file.clone(), vec![0]].concat()).await;
    let r = w.finalize(&staff, &id).await;
    assert_eq!(r.status, StatusCode::UNPROCESSABLE_ENTITY, "{}", r.body);
    assert_eq!(r.body["error"], "size_mismatch");
    assert!(!w.store.keys().contains(&upload_key(&id)));
    assert_eq!(w.finalize(&staff, &id).await.status, StatusCode::NOT_FOUND);

    // The declared size, but not a picture: the API's own checks refuse it.
    let junk = vec![7u8; 64];
    let id = presign_ok(&w, &staff, junk.len()).await;
    w.put(&id, &junk).await;
    let r = w.finalize(&staff, &id).await;
    assert_eq!(r.status, StatusCode::UNSUPPORTED_MEDIA_TYPE, "{}", r.body);
    assert!(!w.store.keys().contains(&upload_key(&id)));

    // A picture of the wrong shape for a badge (not square).
    let wide = png(200, 100);
    let id = presign_ok(&w, &staff, wide.len()).await;
    w.put(&id, &wide).await;
    let r = w.finalize(&staff, &id).await;
    assert_eq!(r.status, StatusCode::UNPROCESSABLE_ENTITY, "{}", r.body);
    assert_eq!(r.body["error"], "bad_dimensions");

    // Someone else's upload reads as missing, and stays.
    let id = presign_ok(&w, &staff, file.len()).await;
    w.put(&id, &file).await;
    let r = w.finalize(&other, &id).await;
    assert_eq!(r.status, StatusCode::NOT_FOUND);
    assert!(w.store.keys().contains(&upload_key(&id)));

    // Past its expiry: gone.
    let mut conn = w.db.conn().await;
    sqlx::query("UPDATE direct_uploads SET expires_at = now() - interval '1 second' WHERE id = $1")
        .bind(ids::from_text(&id).unwrap())
        .execute(&mut *conn)
        .await
        .unwrap();
    let r = w.finalize(&staff, &id).await;
    assert_eq!(r.status, StatusCode::GONE);
    assert_eq!(r.body["error"], "upload_expired");

    assert_eq!(
        w.finalize(&staff, "not-an-id").await.status,
        StatusCode::NOT_FOUND
    );
    drop(conn);
    w.db.close().await;
}
