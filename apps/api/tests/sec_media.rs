//! Blocklist checks on every frame of an animation. Same harness as `upload_flow.rs`. Runs when
//! `CYEX_TV_TEST_DATABASE_URL` is set (CI sets it), skips otherwise.

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
use cyex_tv_db::blocklist;
use cyex_tv_db::sessions::{self, SessionKind};
use cyex_tv_shared::model::Platform;
use cyex_tv_storage::Store;
use http_body_util::BodyExt;
use image::codecs::gif::{GifEncoder, Repeat};
use image::{Delay, Frame, ImageFormat, Rgba, RgbaImage};
use serde_json::Value;
use tower::ServiceExt;

// Fixtures

fn picture(w: u32, h: u32, seed: u8) -> RgbaImage {
    RgbaImage::from_fn(w, h, |x, y| {
        Rgba([(x * 255 / w) as u8, (y * 255 / h) as u8, seed, 255])
    })
}

fn encode(img: &RgbaImage, format: ImageFormat) -> Vec<u8> {
    let mut out = Cursor::new(Vec::new());
    img.write_to(&mut out, format).unwrap();
    out.into_inner()
}

// The harness

struct World {
    db: TestDb,
    app: Router,
}

struct User {
    token: String,
}

struct Reply {
    status: StatusCode,
    body: Value,
}

const BOUNDARY: &str = "cyextvboundary7MA4YWxkTrZu0gW";

/// A multipart body: text fields, then the file if given.
fn form(fields: &[(&str, &str)], file: Option<&[u8]>) -> Vec<u8> {
    let mut body = Vec::new();
    for (name, value) in fields {
        body.extend_from_slice(
            format!(
                "--{BOUNDARY}\r\nContent-Disposition: form-data; name=\"{name}\"\r\n\r\n{value}\r\n"
            )
            .as_bytes(),
        );
    }
    if let Some(file) = file {
        body.extend_from_slice(
            format!(
                "--{BOUNDARY}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"emote.png\"\r\nContent-Type: application/octet-stream\r\n\r\n"
            )
            .as_bytes(),
        );
        body.extend_from_slice(file);
        body.extend_from_slice(b"\r\n");
    }
    body.extend_from_slice(format!("--{BOUNDARY}--\r\n").as_bytes());
    body
}

impl World {
    async fn new(with_store: bool) -> Option<Self> {
        let db = TestDb::new().await?;
        let auth = Arc::new(Auth::new(AuthConfig::from_source(|_| None).unwrap()).unwrap());
        let store = Store::memory();
        let mut state =
            AppState::new(db.pool.clone(), auth).with_cdn_url("https://cdn.example.test");
        if with_store {
            state = state.with_store(store.clone());
        }
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
        User { token }
    }

    async fn post(&self, user: Option<&User>, body: Vec<u8>) -> Reply {
        let mut request = Request::post("/v1/emotes").header(
            "content-type",
            format!("multipart/form-data; boundary={BOUNDARY}"),
        );
        if let Some(user) = user {
            request = request
                .header(
                    "cookie",
                    format!("{}={}", cyex_tv_auth::cookies::SESSION, user.token),
                )
                .header("origin", "https://cyex.tv");
        }
        let response = self
            .app
            .clone()
            .oneshot(request.body(Body::from(body)).unwrap())
            .await
            .unwrap();
        let status = response.status();
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        Reply {
            status,
            body: serde_json::from_slice(&bytes).unwrap_or(Value::Null),
        }
    }

    async fn upload(&self, user: &User, name: &str, file: &[u8]) -> Reply {
        self.post(
            Some(user),
            form(&[("name", name), ("tags", "pog, Funny")], Some(file)),
        )
        .await
    }
}

// The tests

fn frames_gif(frames: &[RgbaImage]) -> Vec<u8> {
    let mut out = Vec::new();
    {
        let mut enc = GifEncoder::new(&mut out);
        enc.set_repeat(Repeat::Infinite).unwrap();
        for f in frames {
            enc.encode_frame(Frame::from_parts(
                f.clone(),
                0,
                0,
                Delay::from_numer_denom_ms(60, 1),
            ))
            .unwrap();
        }
    }
    out
}

#[tokio::test]
async fn a_blocked_image_hidden_as_a_later_frame_is_refused() {
    let Some(w) = World::new(true).await else {
        return;
    };
    let alice = w.user("7").await;
    let blocked = picture(100, 100, 200);
    let hash = cyex_tv_media::dhash(&blocked);
    let mut conn = w.db.conn().await;
    blocklist::add(
        &mut conn,
        None,
        Some(i64::from_ne_bytes(hash.to_ne_bytes())),
        "test: later frame",
        None,
    )
    .await
    .unwrap();
    drop(conn);

    // The still itself is refused.
    let r = w
        .upload(&alice, "still", &encode(&blocked, ImageFormat::Png))
        .await;
    assert_eq!(
        (r.status, r.body["error"].as_str()),
        (StatusCode::UNPROCESSABLE_ENTITY, Some("blocked"))
    );

    // A harmless first frame, the blocked image second.
    let mut benign = picture(100, 100, 200);
    image::imageops::flip_horizontal_in_place(&mut benign);
    assert!(cyex_tv_media::distance(cyex_tv_media::dhash(&benign), hash) > 20);
    let r = w
        .upload(
            &alice,
            "sneaky",
            &frames_gif(&[benign.clone(), blocked.clone()]),
        )
        .await;
    assert_eq!(
        (r.status, r.body["error"].as_str()),
        (StatusCode::UNPROCESSABLE_ENTITY, Some("blocked")),
        "{:?}",
        r.body
    );

    // An animation of harmless frames still goes through.
    let mut other = benign.clone();
    other.put_pixel(50, 50, Rgba([0, 0, 0, 255]));
    let r = w
        .upload(&alice, "fine", &frames_gif(&[benign, other]))
        .await;
    assert_eq!(r.status, StatusCode::ACCEPTED, "{:?}", r.body);
    w.db.close().await;
}
