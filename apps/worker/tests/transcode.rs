//! The `emote.transcode` job against a real Postgres and the in-memory store. Runs when
//! `CYEX_TV_TEST_DATABASE_URL` is set (CI sets it), skips otherwise.

use std::io::Cursor;
use std::time::Duration;

use cyex_tv_db::files::{self, NewFile};
use cyex_tv_db::testing::TestDb;
use cyex_tv_db::{ids, jobs};
use cyex_tv_storage::{IMMUTABLE, Put, Store, original_key, sigv4};
use cyex_tv_worker::{Ctx, Handlers, Limits, Outcome, run_one, transcode};
use image::codecs::gif::{GifEncoder, Repeat};
use image::{Delay, Frame, ImageFormat, Rgba, RgbaImage};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

fn picture(w: u32, h: u32, seed: u8) -> RgbaImage {
    RgbaImage::from_fn(w, h, |x, y| {
        Rgba([(x * 255 / w) as u8, (y * 255 / h) as u8, seed, 255])
    })
}

fn png(w: u32, h: u32) -> Vec<u8> {
    let mut out = Cursor::new(Vec::new());
    picture(w, h, 1)
        .write_to(&mut out, ImageFormat::Png)
        .unwrap();
    out.into_inner()
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
                Delay::from_numer_denom_ms(70, 1),
            ))
            .unwrap();
        }
    }
    out
}

fn limits() -> Limits {
    Limits {
        lease: Duration::from_secs(60),
        timeout: Duration::from_secs(60),
        poll: Duration::from_millis(50),
    }
}

/// What the upload does: the file row, the original in storage, the job.
async fn uploaded(db: &TestDb, store: &Store, bytes: &[u8], mime: &str) -> (uuid::Uuid, String) {
    let sha: [u8; 32] = Sha256::digest(bytes).into();
    let hex = sigv4::hex(&sha);
    let key = original_key(&hex);
    let mut conn = db.conn().await;
    let (file, _) = files::insert_or_get(
        &mut conn,
        &NewFile {
            sha256: &sha,
            dhash: None,
            mime,
            width: 1,
            height: 1,
            frame_count: 1,
            storage_key: &key,
        },
    )
    .await
    .unwrap();
    store
        .put(
            &key,
            bytes.to_vec().into(),
            Put {
                content_type: mime,
                cache_control: None,
            },
        )
        .await
        .unwrap();
    jobs::enqueue(
        &mut conn,
        transcode::KIND,
        &json!({ "file_id": ids::to_text(file.id) }),
        3,
    )
    .await
    .unwrap();
    (file.id, hex)
}

fn handlers() -> Handlers {
    Handlers::default().on(transcode::KIND, transcode::run)
}

async fn variants(db: &TestDb, id: uuid::Uuid) -> Vec<Value> {
    let mut conn = db.conn().await;
    let file = files::get(&mut conn, id).await.unwrap().unwrap();
    file.variants.as_array().unwrap().clone()
}

#[tokio::test]
async fn a_still_upload_gets_eight_files() {
    let Some(db) = TestDb::new().await else {
        return;
    };
    let store = Store::memory();
    let ctx = Ctx {
        db: db.pool.clone(),
        store: Some(store.clone()),
        purge: None,
    };
    let (file, hex) = uploaded(&db, &store, &png(100, 100), "image/png").await;

    let (_, outcome) = run_one(&ctx, &handlers(), limits()).await.unwrap().unwrap();
    assert_eq!(outcome, Outcome::Done);

    let v = variants(&db, file).await;
    let names: Vec<&str> = v.iter().map(|e| e["name"].as_str().unwrap()).collect();
    assert_eq!(
        names,
        [
            "1x.webp", "2x.webp", "3x.webp", "4x.webp", "1x.avif", "2x.avif", "3x.avif", "4x.avif"
        ]
    );
    assert_eq!(v[0]["format"], "webp");
    assert_eq!(
        (v[0]["width"].as_u64(), v[0]["height"].as_u64()),
        (Some(32), Some(32))
    );
    // 100 px tall: 3x and 4x keep the original size.
    assert_eq!(v[3]["height"], 100);
    assert_eq!(v[4]["format"], "avif");
    for entry in &v {
        let key = format!("emotes/{hex}/{}", entry["name"].as_str().unwrap());
        let object = store
            .get(&key)
            .await
            .unwrap()
            .unwrap_or_else(|| panic!("{key}"));
        assert_eq!(object.cache_control.as_deref(), Some(IMMUTABLE));
        assert_eq!(entry["size"].as_u64(), Some(object.bytes.len() as u64));
        let mime = if key.ends_with(".webp") {
            "image/webp"
        } else {
            "image/avif"
        };
        assert_eq!(object.content_type.as_deref(), Some(mime));
    }
    db.close().await;
}

#[tokio::test]
async fn an_animated_upload_gets_animated_webp_and_static_frames() {
    let Some(db) = TestDb::new().await else {
        return;
    };
    let store = Store::memory();
    let ctx = Ctx {
        db: db.pool.clone(),
        store: Some(store.clone()),
        purge: None,
    };
    let (file, _) = uploaded(&db, &store, &gif(64, 64, 4), "image/gif").await;
    assert_eq!(
        run_one(&ctx, &handlers(), limits())
            .await
            .unwrap()
            .unwrap()
            .1,
        Outcome::Done
    );
    let v = variants(&db, file).await;
    assert_eq!(v.len(), 12);
    let statics = v.iter().filter(|e| e["is_static"] == true).count();
    assert_eq!(statics, 8);
    assert!(v.iter().any(|e| e["name"] == "4x_static.avif"));
    db.close().await;
}

#[tokio::test]
async fn bad_originals_fail_for_good() {
    let Some(db) = TestDb::new().await else {
        return;
    };
    let store = Store::memory();
    let ctx = Ctx {
        db: db.pool.clone(),
        store: Some(store.clone()),
        purge: None,
    };
    // The original went missing.
    let (missing, _) = uploaded(&db, &store, &png(20, 20), "image/png").await;
    let mut conn = db.conn().await;
    let file = files::get(&mut conn, missing).await.unwrap().unwrap();
    drop(conn);
    store.delete(&file.storage_key).await.unwrap();
    assert_eq!(
        run_one(&ctx, &handlers(), limits())
            .await
            .unwrap()
            .unwrap()
            .1,
        Outcome::Failed
    );
    // The original doesn't decode.
    let (_, _) = uploaded(&db, &store, b"\x89PNG\r\n\x1a\nnot really", "image/png").await;
    assert_eq!(
        run_one(&ctx, &handlers(), limits())
            .await
            .unwrap()
            .unwrap()
            .1,
        Outcome::Failed
    );
    // Without storage the job waits for it instead of failing.
    let no_store = Ctx {
        db: db.pool.clone(),
        store: None,
        purge: None,
    };
    uploaded(&db, &store, &png(21, 21), "image/png").await;
    assert_eq!(
        run_one(&no_store, &handlers(), limits())
            .await
            .unwrap()
            .unwrap()
            .1,
        Outcome::Retrying
    );
    db.close().await;
}
