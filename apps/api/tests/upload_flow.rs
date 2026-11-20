//! `POST /v1/emotes` through the real router, against a real Postgres and the in-memory store.
//! Every fixture image is made in code. Runs when `CYEX_TV_TEST_DATABASE_URL` is set (CI sets
//! it), skips otherwise.

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
use cyex_tv_db::sessions::{self, SessionKind};
use cyex_tv_db::{blocklist, emotes};
use cyex_tv_shared::model::Platform;
use cyex_tv_storage::Store;
use http_body_util::BodyExt;
use image::codecs::gif::{GifEncoder, Repeat};
use image::{Delay, Frame, ImageFormat, Rgba, RgbaImage};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use tower::ServiceExt;
use uuid::Uuid;

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

fn png(w: u32, h: u32, seed: u8) -> Vec<u8> {
    encode(&picture(w, h, seed), ImageFormat::Png)
}

fn gif(w: u32, h: u32, frames: u32) -> Vec<u8> {
    let mut out = Vec::new();
    {
        let mut enc = GifEncoder::new(&mut out);
        enc.set_repeat(Repeat::Infinite).unwrap();
        for i in 0..frames {
            enc.encode_frame(Frame::from_parts(
                picture(w, h, (i * 50) as u8),
                0,
                0,
                Delay::from_numer_denom_ms(60, 1),
            ))
            .unwrap();
        }
    }
    out
}

fn apng(w: u32, h: u32, frames: u32) -> Vec<u8> {
    let mut out = Vec::new();
    {
        let mut enc = png::Encoder::new(&mut out, w, h);
        enc.set_color(png::ColorType::Rgba);
        enc.set_depth(png::BitDepth::Eight);
        enc.set_animated(frames, 0).unwrap();
        let mut writer = enc.write_header().unwrap();
        for i in 0..frames {
            writer
                .write_image_data(picture(w, h, (i * 60) as u8).as_raw())
                .unwrap();
        }
        writer.finish().unwrap();
    }
    out
}

// The harness

struct World {
    db: TestDb,
    app: Router,
    store: Store,
}

struct User {
    id: Uuid,
    token: String,
}

struct Reply {
    status: StatusCode,
    body: Value,
}

fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xffff_ffffu32;
    for &byte in data {
        crc ^= u32::from(byte);
        for _ in 0..8 {
            crc = if crc & 1 == 1 {
                (crc >> 1) ^ 0xedb8_8320
            } else {
                crc >> 1
            };
        }
    }
    !crc
}

/// A valid small PNG padded to exactly `total` bytes with a private ancillary chunk after IHDR:
/// a big file that is still a valid (small) picture.
fn padded_png(seed: u8, total: usize) -> Vec<u8> {
    let small = png(64, 64, seed);
    let data_len = total - small.len() - 12;
    let mut chunk = Vec::with_capacity(data_len + 12);
    chunk.extend_from_slice(&u32::try_from(data_len).unwrap().to_be_bytes());
    chunk.extend_from_slice(b"prVt");
    chunk.resize(8 + data_len, 0x5a);
    let crc = crc32(&chunk[4..]);
    chunk.extend_from_slice(&crc.to_be_bytes());
    let mut out = small[..33].to_vec();
    out.extend_from_slice(&chunk);
    out.extend_from_slice(&small[33..]);
    assert_eq!(out.len(), total);
    out
}

fn jpeg(w: u32, h: u32, seed: u8) -> Vec<u8> {
    let rgb = image::DynamicImage::ImageRgba8(picture(w, h, seed)).to_rgb8();
    let mut out = Cursor::new(Vec::new());
    rgb.write_to(&mut out, ImageFormat::Jpeg).unwrap();
    out.into_inner()
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

    async fn post(&self, user: Option<&User>, body: Vec<u8>) -> Reply {
        let mut request = Request::post("/v1/emotes").header(
            "content-type",
            format!("multipart/form-data; boundary={BOUNDARY}"),
        );
        if let Some(user) = user {
            request = request.header("authorization", format!("Bearer {}", user.token));
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

    async fn count(&self, sql: &str) -> i64 {
        let mut conn = self.db.conn().await;
        sqlx::query_scalar(sql).fetch_one(&mut *conn).await.unwrap()
    }
}

fn sha_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

// The tests

#[tokio::test]
async fn a_png_becomes_a_pending_emote_with_a_job() {
    let Some(w) = World::new(true).await else {
        return;
    };
    let alice = w.user("1").await;
    let file = png(112, 112, 7);

    let r = w.upload(&alice, "alicePog", &file).await;
    assert_eq!(r.status, StatusCode::ACCEPTED, "{:?}", r.body);
    assert_eq!(r.body["name"], "alicePog");
    assert_eq!(r.body["state"], "pending");
    assert_eq!(r.body["animated"], false);
    assert_eq!(r.body["tags"], json!(["funny", "pog"]));
    assert_eq!(
        r.body["host"],
        format!("https://cdn.example.test/emotes/{}", sha_hex(&file))
    );
    assert_eq!(r.body["files"], json!([]));
    assert_eq!(r.body["owner"]["display_name"], "user1");

    // The original is stored privately, the file recorded, one transcode job queued.
    let key = format!("originals/{}", sha_hex(&file));
    assert_eq!(w.store.keys(), std::slice::from_ref(&key));
    let stored = w.store.get(&key).await.unwrap().unwrap();
    assert_eq!(stored.bytes.as_ref(), file.as_slice());
    assert_eq!(stored.content_type.as_deref(), Some("image/png"));
    assert_eq!(stored.cache_control, None);
    assert_eq!(
        w.count("SELECT count(*) FROM jobs WHERE kind = 'emote.transcode'")
            .await,
        1
    );
    assert_eq!(w.count("SELECT count(*) FROM files WHERE width = 112 AND height = 112 AND frame_count = 1 AND dhash IS NOT NULL").await, 1);
    w.db.close().await;
}

#[tokio::test]
async fn flags_can_be_set_at_upload() {
    let Some(w) = World::new(true).await else {
        return;
    };
    let alice = w.user("11").await;

    // Each flag on its own, in the spellings a form sends.
    for (field, value, key) in [
        ("zero_width", "true", "zero_width"),
        ("private", "on", "private"),
        ("nsfw", "1", "nsfw"),
    ] {
        let file = png(40, 40, field.len() as u8 + 30);
        let r = w
            .post(
                Some(&alice),
                form(&[("name", field), (field, value)], Some(&file)),
            )
            .await;
        assert_eq!(r.status, StatusCode::ACCEPTED, "{field}: {:?}", r.body);
        for other in ["zero_width", "private", "nsfw"] {
            assert_eq!(
                r.body["flags"][other],
                other == key,
                "{field} sets only {key}"
            );
        }
        let id = r.body["id"].as_str().unwrap().to_owned();
        let stored: (bool, bool, bool) = {
            let mut conn = w.db.conn().await;
            sqlx::query_as(
                "SELECT zero_width, private, nsfw FROM emotes WHERE id = (SELECT id FROM emotes WHERE name = $1)",
            )
            .bind(field)
            .fetch_one(&mut *conn)
            .await
            .unwrap()
        };
        let want = (key == "zero_width", key == "private", key == "nsfw");
        assert_eq!(stored, want, "{field} ({id}) is stored");
    }

    // All three at once; explicit false and an empty field are off.
    let r = w
        .post(
            Some(&alice),
            form(
                &[
                    ("name", "allOn"),
                    ("zero_width", "TRUE"),
                    ("private", "1"),
                    ("nsfw", "on"),
                ],
                Some(&png(40, 40, 50)),
            ),
        )
        .await;
    assert_eq!(r.status, StatusCode::ACCEPTED, "{:?}", r.body);
    assert_eq!(
        r.body["flags"],
        json!({ "zero_width": true, "private": true, "nsfw": true, "wide": false })
    );
    let r = w
        .post(
            Some(&alice),
            form(
                &[
                    ("name", "allOff"),
                    ("zero_width", "false"),
                    ("private", ""),
                    ("nsfw", "0"),
                ],
                Some(&png(40, 40, 51)),
            ),
        )
        .await;
    assert_eq!(r.status, StatusCode::ACCEPTED, "{:?}", r.body);
    assert_eq!(
        r.body["flags"],
        json!({ "zero_width": false, "private": false, "nsfw": false, "wide": false })
    );

    // Without the fields, nothing changes from before: all off.
    let r = w.upload(&alice, "plain", &png(40, 40, 52)).await;
    assert_eq!(
        r.body["flags"],
        json!({ "zero_width": false, "private": false, "nsfw": false, "wide": false })
    );
    w.db.close().await;
}

#[tokio::test]
async fn animated_and_webp_uploads() {
    let Some(w) = World::new(true).await else {
        return;
    };
    let alice = w.user("2").await;
    for (name, file, animated, mime) in [
        ("apngOne", apng(48, 48, 5), true, "image/png"),
        ("gifOne", gif(40, 32, 6), true, "image/gif"),
        (
            "webpOne",
            encode(&picture(64, 64, 3), ImageFormat::WebP),
            false,
            "image/webp",
        ),
    ] {
        let r = w.upload(&alice, name, &file).await;
        assert_eq!(r.status, StatusCode::ACCEPTED, "{name}: {:?}", r.body);
        assert_eq!(r.body["animated"], animated, "{name}");
        let key = format!("originals/{}", sha_hex(&file));
        let stored = w.store.get(&key).await.unwrap().unwrap();
        assert_eq!(stored.content_type.as_deref(), Some(mime), "{name}");
    }
    assert_eq!(
        w.count("SELECT count(*) FROM files WHERE frame_count = 5")
            .await,
        1
    );
    assert_eq!(
        w.count("SELECT count(*) FROM files WHERE frame_count = 6")
            .await,
        1
    );
    w.db.close().await;
}

#[tokio::test]
async fn a_duplicate_reuses_the_file() {
    let Some(w) = World::new(true).await else {
        return;
    };
    let alice = w.user("3").await;
    let bob = w.user("4").await;
    let file = png(64, 64, 9);
    let first = w.upload(&alice, "first", &file).await;
    let second = w.upload(&bob, "second", &file).await;
    assert_eq!(first.status, StatusCode::ACCEPTED);
    assert_eq!(second.status, StatusCode::ACCEPTED);
    assert_ne!(first.body["id"], second.body["id"]);
    assert_eq!(first.body["host"], second.body["host"]);
    assert_eq!(w.count("SELECT count(*) FROM files").await, 1);
    assert_eq!(w.count("SELECT count(*) FROM emotes").await, 2);
    assert_eq!(w.count("SELECT count(*) FROM jobs").await, 1);
    assert_eq!(w.store.keys().len(), 1);
    w.db.close().await;
}

#[tokio::test]
async fn the_bytes_of_a_purged_file_are_stored_again() {
    let Some(w) = World::new(true).await else {
        return;
    };
    let alice = w.user("41").await;
    let bob = w.user("42").await;
    let file = png(64, 64, 41);
    assert_eq!(
        w.upload(&alice, "first", &file).await.status,
        StatusCode::ACCEPTED
    );
    // Alice's account went, and its file with it (the worker deleted the objects).
    {
        let mut conn = w.db.conn().await;
        sqlx::query("UPDATE emotes SET deleted_at = now()")
            .execute(&mut *conn)
            .await
            .unwrap();
        sqlx::query("UPDATE files SET purged_at = now(), variants = '[]'")
            .execute(&mut *conn)
            .await
            .unwrap();
    }
    for key in w.store.keys() {
        w.store.delete(&key).await.unwrap();
    }
    assert!(w.store.keys().is_empty());
    // Bob uploads the same bytes: the original is stored again and transcoded again.
    assert_eq!(
        w.upload(&bob, "again", &file).await.status,
        StatusCode::ACCEPTED
    );
    assert_eq!(w.count("SELECT count(*) FROM files").await, 1);
    assert_eq!(
        w.count("SELECT count(*) FROM files WHERE purged_at IS NULL")
            .await,
        1
    );
    assert_eq!(
        w.count("SELECT count(*) FROM jobs WHERE kind = 'emote.transcode'")
            .await,
        2
    );
    assert_eq!(w.store.keys().len(), 1);
    w.db.close().await;
}

#[tokio::test]
async fn bad_files_are_refused() {
    let Some(w) = World::new(true).await else {
        return;
    };
    let alice = w.user("5").await;
    let mut corrupt = png(80, 80, 1);
    corrupt.truncate(corrupt.len() / 2);
    let svg = b"<?xml version=\"1.0\"?><svg xmlns=\"http://www.w3.org/2000/svg\" width=\"10\" height=\"10\"/>".to_vec();
    let video = b"\x00\x00\x00\x18ftypmp42\x00\x00\x00\x00isommp42".to_vec();
    let mut oversized = png(16, 16, 1);
    oversized.resize(8 * 1024 * 1024 + 1, 0);

    for (case, file, status, code) in [
        (
            "svg",
            svg,
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            "unsupported_format",
        ),
        (
            "video",
            video,
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            "unsupported_format",
        ),
        (
            "corrupt",
            corrupt,
            StatusCode::UNPROCESSABLE_ENTITY,
            "invalid_image",
        ),
        (
            "oversized",
            oversized,
            StatusCode::PAYLOAD_TOO_LARGE,
            "too_large",
        ),
        (
            "too wide",
            png(1001, 8, 1),
            StatusCode::PAYLOAD_TOO_LARGE,
            "too_large",
        ),
    ] {
        let r = w.upload(&alice, "nope", &file).await;
        assert_eq!(r.status, status, "{case}: {:?}", r.body);
        assert_eq!(r.body["error"], code, "{case}");
        if case == "oversized" {
            // Said plainly, with the limit in it.
            assert_eq!(
                r.body["message"],
                "The file is over 8 MB, the most an upload can be.",
            );
        }
        if code == "invalid_image" {
            // The decoder's own words stay in the logs.
            assert_eq!(
                r.body["message"],
                "That file isn't a valid image. Try a PNG, GIF, WebP or JPEG that opens elsewhere.",
                "{case}"
            );
        }
    }
    // Nothing was kept from any of them.
    assert!(w.store.keys().is_empty());
    assert_eq!(w.count("SELECT count(*) FROM emotes").await, 0);
    assert_eq!(w.count("SELECT count(*) FROM files").await, 0);
    w.db.close().await;
}

#[tokio::test]
async fn bad_forms_and_callers() {
    let Some(w) = World::new(true).await else {
        return;
    };
    let alice = w.user("6").await;
    let file = png(32, 32, 2);

    let r = w.post(None, form(&[("name", "x")], Some(&file))).await;
    assert_eq!(
        (r.status, r.body["error"].as_str()),
        (StatusCode::UNAUTHORIZED, Some("signed_out"))
    );
    for (fields, file, code) in [
        (vec![("tags", "a")], Some(file.as_slice()), "bad_form"),
        (vec![("name", "x")], None, "bad_form"),
        (
            vec![("name", "x"), ("colour", "red")],
            Some(file.as_slice()),
            "bad_form",
        ),
        (
            vec![("name", "x"), ("nsfw", "yes")],
            Some(file.as_slice()),
            "bad_form",
        ),
        (
            vec![("name", "two words")],
            Some(file.as_slice()),
            "bad_name",
        ),
        (
            vec![("name", "ok"), ("tags", "no spaces please")],
            Some(file.as_slice()),
            "bad_tag",
        ),
    ] {
        let r = w.post(Some(&alice), form(&fields, file)).await;
        assert_eq!(
            r.status,
            StatusCode::BAD_REQUEST,
            "{fields:?}: {:?}",
            r.body
        );
        assert_eq!(r.body["error"], code, "{fields:?}");
    }

    // A restricted account can't upload.
    let mut conn = w.db.conn().await;
    accounts::set_restricted(&mut conn, alice.id, true)
        .await
        .unwrap();
    drop(conn);
    let r = w.upload(&alice, "ok", &file).await;
    assert_eq!(
        (r.status, r.body["error"].as_str()),
        (StatusCode::FORBIDDEN, Some("restricted"))
    );
    w.db.close().await;
}

#[tokio::test]
async fn blocked_images_and_near_copies_are_refused() {
    let Some(w) = World::new(true).await else {
        return;
    };
    let alice = w.user("7").await;
    let banned = png(100, 100, 42);
    let sha: [u8; 32] = Sha256::digest(&banned).into();
    let mut conn = w.db.conn().await;
    blocklist::add(&mut conn, Some(&sha), None, "test: exact", None)
        .await
        .unwrap();
    // A near copy is blocked through the dHash of the original.
    let near_source = picture(100, 100, 200);
    let hash = cyex_tv_media::dhash(&near_source);
    blocklist::add(
        &mut conn,
        None,
        Some(i64::from_ne_bytes(hash.to_ne_bytes())),
        "test: near",
        None,
    )
    .await
    .unwrap();
    drop(conn);

    let r = w.upload(&alice, "banned", &banned).await;
    assert_eq!(
        (r.status, r.body["error"].as_str()),
        (StatusCode::UNPROCESSABLE_ENTITY, Some("blocked"))
    );
    let mut near = near_source.clone();
    near.put_pixel(50, 50, Rgba([0, 0, 0, 255]));
    let r = w
        .upload(&alice, "near", &encode(&near, ImageFormat::Png))
        .await;
    assert_eq!(
        (r.status, r.body["error"].as_str()),
        (StatusCode::UNPROCESSABLE_ENTITY, Some("blocked"))
    );
    // Something else entirely goes through: the mirror image, whose dHash is far away.
    let mut other = picture(100, 100, 200);
    image::imageops::flip_horizontal_in_place(&mut other);
    assert!(cyex_tv_media::distance(cyex_tv_media::dhash(&other), hash) > 20);
    let r = w
        .upload(&alice, "fine", &encode(&other, ImageFormat::Png))
        .await;
    assert_eq!(r.status, StatusCode::ACCEPTED, "{:?}", r.body);
    w.db.close().await;
}

#[tokio::test]
async fn uploads_are_rate_limited_per_account() {
    let Some(w) = World::new(true).await else {
        return;
    };
    let alice = w.user("8").await;
    let bob = w.user("9").await;
    let mut conn = w.db.conn().await;
    for i in 0..20 {
        emotes::create(
            &mut conn,
            &emotes::NewEmote {
                owner_account_id: alice.id,
                name: &format!("e{i}"),
                ..emotes::NewEmote::default()
            },
        )
        .await
        .unwrap();
    }
    drop(conn);
    let r = w.upload(&alice, "one_more", &png(20, 20, 1)).await;
    assert_eq!(
        (r.status, r.body["error"].as_str()),
        (StatusCode::TOO_MANY_REQUESTS, Some("rate_limited"))
    );
    // Bob has his own allowance.
    let r = w.upload(&bob, "bobs", &png(20, 20, 1)).await;
    assert_eq!(r.status, StatusCode::ACCEPTED, "{:?}", r.body);
    w.db.close().await;
}

#[tokio::test]
async fn without_storage_uploads_are_unavailable() {
    let Some(w) = World::new(false).await else {
        return;
    };
    let alice = w.user("10").await;
    let r = w.upload(&alice, "ok", &png(20, 20, 1)).await;
    assert_eq!(
        (r.status, r.body["error"].as_str()),
        (StatusCode::SERVICE_UNAVAILABLE, Some("unavailable"))
    );
    assert_eq!(w.count("SELECT count(*) FROM emotes").await, 0);
    w.db.close().await;
}

#[tokio::test]
async fn files_up_to_8_mb_and_jpeg_are_taken() {
    let Some(w) = World::new(true).await else {
        return;
    };
    let alice = w.user("81").await;

    // Over the old 5 MB, under 8 MB: taken, and the original is kept as sent.
    let six = padded_png(81, 6 * 1024 * 1024);
    let r = w.upload(&alice, "sixMegs", &six).await;
    assert_eq!(r.status, StatusCode::ACCEPTED, "{:?}", r.body);
    let stored = w
        .store
        .get(&format!("originals/{}", sha_hex(&six)))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(stored.bytes.len(), six.len());

    // Exactly 8 MB is the most; one byte more is refused before it is decoded.
    let full = padded_png(82, 8 * 1024 * 1024);
    let r = w.upload(&alice, "eightMegs", &full).await;
    assert_eq!(r.status, StatusCode::ACCEPTED, "{:?}", r.body);
    let over = padded_png(83, 8 * 1024 * 1024 + 1);
    let r = w.upload(&alice, "overEight", &over).await;
    assert_eq!(r.status, StatusCode::PAYLOAD_TOO_LARGE, "{:?}", r.body);
    assert_eq!(r.body["error"], "too_large");
    assert_eq!(
        r.body["message"],
        "The file is over 8 MB, the most an upload can be."
    );

    // JPEG: a still emote, stored as image/jpeg, transcoded like any other.
    let file = jpeg(96, 64, 84);
    let r = w.upload(&alice, "jpegOne", &file).await;
    assert_eq!(r.status, StatusCode::ACCEPTED, "{:?}", r.body);
    assert_eq!(r.body["animated"], false);
    let stored = w
        .store
        .get(&format!("originals/{}", sha_hex(&file)))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(stored.content_type.as_deref(), Some("image/jpeg"));
    assert_eq!(
        w.count(
            "SELECT count(*) FROM files WHERE mime = 'image/jpeg' AND width = 96 AND height = 64"
        )
        .await,
        1
    );
    assert_eq!(
        w.count("SELECT count(*) FROM jobs WHERE kind = 'emote.transcode'")
            .await,
        3
    );
    // Only the over-limit file left nothing behind.
    assert_eq!(w.count("SELECT count(*) FROM emotes").await, 3);
    w.db.close().await;
}

#[tokio::test]
async fn the_upload_dialog_flags_arrive_together() {
    let Some(w) = World::new(true).await else {
        return;
    };
    let alice = w.user("85").await;
    // What the website's upload dialog sends: zero-width, NSFW and unlisted at once.
    let r = w
        .post(
            Some(&alice),
            form(
                &[
                    ("name", "rainOver"),
                    ("tags", "rain,weather,overlay"),
                    ("zero_width", "true"),
                    ("nsfw", "true"),
                    ("visibility", "unlisted"),
                ],
                Some(&png(40, 40, 85)),
            ),
        )
        .await;
    assert_eq!(r.status, StatusCode::ACCEPTED, "{:?}", r.body);
    assert_eq!(r.body["flags"]["zero_width"], true);
    assert_eq!(r.body["flags"]["nsfw"], true);
    assert_eq!(r.body["flags"]["private"], false);
    assert_eq!(r.body["visibility"], "unlisted");
    assert_eq!(r.body["tags"], json!(["overlay", "rain", "weather"]));
    let stored: (bool, bool, bool, bool) = {
        let mut conn = w.db.conn().await;
        sqlx::query_as(
            "SELECT zero_width, nsfw, private, unlisted FROM emotes WHERE name = 'rainOver'",
        )
        .fetch_one(&mut *conn)
        .await
        .unwrap()
    };
    assert_eq!(stored, (true, true, false, true));
    w.db.close().await;
}

/// Uploads at the same time can't overshoot the hourly limit: with one left, of four at once
/// exactly one gets in.
#[tokio::test]
async fn concurrent_uploads_respect_the_hourly_limit() {
    let Some(w) = World::new(true).await else {
        return;
    };
    let alice = w.user("10").await;
    let mut conn = w.db.conn().await;
    for i in 0..19 {
        emotes::create(
            &mut conn,
            &emotes::NewEmote {
                owner_account_id: alice.id,
                name: &format!("e{i}"),
                ..emotes::NewEmote::default()
            },
        )
        .await
        .unwrap();
    }
    drop(conn);
    let (a, b, c, d) = (
        png(20, 20, 11),
        png(20, 20, 12),
        png(20, 20, 13),
        png(20, 20, 14),
    );
    let got = tokio::join!(
        w.upload(&alice, "racea", &a),
        w.upload(&alice, "raceb", &b),
        w.upload(&alice, "racec", &c),
        w.upload(&alice, "raced", &d),
    );
    let statuses = [got.0.status, got.1.status, got.2.status, got.3.status];
    let accepted = statuses
        .iter()
        .filter(|s| **s == StatusCode::ACCEPTED)
        .count();
    assert_eq!(accepted, 1, "{statuses:?}");
    assert!(
        statuses
            .iter()
            .all(|s| *s == StatusCode::ACCEPTED || *s == StatusCode::TOO_MANY_REQUESTS),
        "{statuses:?}"
    );
    assert_eq!(w.count("SELECT count(*) FROM emotes").await, 20);
    w.db.close().await;
}

/// A text field longer than its cap is refused, however long it is.
#[tokio::test]
async fn over_long_text_fields_are_refused() {
    let Some(w) = World::new(true).await else {
        return;
    };
    let alice = w.user("11").await;
    let file = png(20, 20, 3);
    let long = "a".repeat(200_000);
    for key in ["name", "tags", "owner_id", "visibility", "nsfw"] {
        let mut fields = vec![(key, long.as_str())];
        if key != "name" {
            fields.push(("name", "ok"));
        }
        let r = w.post(Some(&alice), form(&fields, Some(&file))).await;
        assert_eq!(
            (r.status, r.body["error"].as_str()),
            (StatusCode::BAD_REQUEST, Some("bad_form")),
            "{key}"
        );
    }
    assert_eq!(w.count("SELECT count(*) FROM emotes").await, 0);
    w.db.close().await;
}
