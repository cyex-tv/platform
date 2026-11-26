//! The `cosmetic.transcode` job (ADR 0015) against a real Postgres and the in-memory store: a
//! badge picture becomes the size ladder (18, 36, 72, 128, 256 and 512 px) under
//! `cosmetics/badges/`, recorded on its row, and running the job again remakes a picture made
//! before the larger sizes.
//! Runs when `CYEX_TV_TEST_DATABASE_URL` is set (CI sets it), skips otherwise.

use std::io::Cursor;
use std::time::Duration;

use cyex_tv_db::cosmetics::{self, NewImage};
use cyex_tv_db::testing::TestDb;
use cyex_tv_db::{ids, jobs};
use cyex_tv_shared::model::{CosmeticKind, EmoteFile, ImageFormat};
use cyex_tv_storage::{IMMUTABLE, Put, Store, cosmetic_original_key, sigv4};
use cyex_tv_worker::{Ctx, Handlers, Limits, Outcome, cosmetics as job, run_one};
use image::{ImageFormat as Fmt, Rgba, RgbaImage};
use serde_json::json;
use sha2::{Digest, Sha256};

fn png(side: u32) -> Vec<u8> {
    let mut out = Cursor::new(Vec::new());
    RgbaImage::from_fn(side, side, |x, y| Rgba([x as u8, y as u8, 200, 255]))
        .write_to(&mut out, Fmt::Png)
        .unwrap();
    out.into_inner()
}

fn limits() -> Limits {
    Limits {
        lease: Duration::from_secs(60),
        timeout: Duration::from_secs(60),
        poll: Duration::from_millis(50),
    }
}

#[tokio::test]
async fn a_badge_picture_is_transcoded_under_cosmetics() {
    let Some(db) = TestDb::new().await else {
        return;
    };
    let store = Store::memory();
    let bytes = png(96);
    let sha: [u8; 32] = Sha256::digest(&bytes).into();
    let hex = sigv4::hex(&sha);
    let key = cosmetic_original_key(&hex);
    store
        .put(
            &key,
            bytes.into(),
            Put {
                content_type: "image/png",
                cache_control: None,
            },
        )
        .await
        .unwrap();
    let mut conn = db.conn().await;
    let (image, created) = cosmetics::insert_image(
        &mut conn,
        &NewImage {
            kind: CosmeticKind::Badge,
            sha256: &sha,
            mime: "image/png",
            width: 96,
            height: 96,
            frame_count: 1,
            animated: false,
            storage_key: &key,
            created_by: None,
        },
    )
    .await
    .unwrap();
    assert!(created);
    jobs::enqueue(
        &mut conn,
        job::KIND,
        &json!({ "image_id": ids::to_text(image.id) }),
        3,
    )
    .await
    .unwrap();
    drop(conn);

    let ctx = Ctx {
        db: db.pool.clone(),
        store: Some(store.clone()),
        purge: None,
        paddle: None,
    };
    let handlers = Handlers::default().on(job::KIND, job::run);
    let (_, outcome) = run_one(&ctx, &handlers, limits()).await.unwrap().unwrap();
    assert_eq!(outcome, Outcome::Done);

    let mut conn = db.conn().await;
    let row = cosmetics::get_image(&mut conn, image.id)
        .await
        .unwrap()
        .unwrap();
    let files: Vec<EmoteFile> = serde_json::from_value(row.variants).unwrap();
    let webp: Vec<_> = files
        .iter()
        .filter(|f| f.format == ImageFormat::Webp)
        .map(|f| (f.name.as_str(), f.width))
        .collect();
    assert_eq!(
        webp,
        [
            ("1x.webp", 18),
            ("2x.webp", 36),
            ("4x.webp", 72),
            ("128.webp", 128),
            ("256.webp", 256),
            ("512.webp", 512),
            // The upload's own size, not scaled up to the master.
            ("original.webp", 96)
        ]
    );
    assert_eq!(files.len(), 14);
    for f in &files {
        let object = store
            .get(&format!("cosmetics/badges/{hex}/{}", f.name))
            .await
            .unwrap()
            .unwrap_or_else(|| panic!("{} missing", f.name));
        assert_eq!(object.cache_control.as_deref(), Some(IMMUTABLE));
    }
    // Nothing went to the emote prefix.
    assert!(!store.keys().iter().any(|k| k.starts_with("emotes/")));

    // A picture made before the ladder (only 1x, 2x and 4x recorded): the job again remakes it.
    let old: Vec<&EmoteFile> = files
        .iter()
        .filter(|f| ["1x", "2x", "4x"].iter().any(|p| f.name.starts_with(p)))
        .collect();
    assert_eq!(old.len(), 6);
    cosmetics::set_image_variants(&mut conn, image.id, &serde_json::to_value(&old).unwrap())
        .await
        .unwrap();
    jobs::enqueue(
        &mut conn,
        job::KIND,
        &json!({ "image_id": ids::to_text(image.id) }),
        3,
    )
    .await
    .unwrap();
    drop(conn);
    let (_, outcome) = run_one(&ctx, &handlers, limits()).await.unwrap().unwrap();
    assert_eq!(outcome, Outcome::Done);
    let mut conn = db.conn().await;
    let row = cosmetics::get_image(&mut conn, image.id)
        .await
        .unwrap()
        .unwrap();
    let again: Vec<EmoteFile> = serde_json::from_value(row.variants).unwrap();
    let names = |f: &[EmoteFile]| -> Vec<(String, u16)> {
        f.iter().map(|f| (f.name.clone(), f.width)).collect()
    };
    assert_eq!(names(&again), names(&files));
    drop(conn);
    db.close().await;
}

/// A badge picture of `side` px stored and queued with `extra` (the limits) on its job.
async fn queued_badge(
    db: &TestDb,
    store: &Store,
    side: u32,
    extra: serde_json::Value,
) -> uuid::Uuid {
    let mut out = Cursor::new(Vec::new());
    RgbaImage::from_pixel(side, side, Rgba([30, 60, 200, 255]))
        .write_to(&mut out, Fmt::Png)
        .unwrap();
    let bytes = out.into_inner();
    let sha: [u8; 32] = Sha256::digest(&bytes).into();
    let key = cosmetic_original_key(&sigv4::hex(&sha));
    store
        .put(
            &key,
            bytes.into(),
            Put {
                content_type: "image/png",
                cache_control: None,
            },
        )
        .await
        .unwrap();
    let mut conn = db.conn().await;
    let (image, _) = cosmetics::insert_image(
        &mut conn,
        &NewImage {
            kind: CosmeticKind::Badge,
            sha256: &sha,
            mime: "image/png",
            width: i32::try_from(side).unwrap(),
            height: i32::try_from(side).unwrap(),
            frame_count: 1,
            animated: false,
            storage_key: &key,
            created_by: None,
        },
    )
    .await
    .unwrap();
    let mut payload = extra;
    payload["image_id"] = json!(ids::to_text(image.id));
    jobs::enqueue(&mut conn, job::KIND, &payload, 3)
        .await
        .unwrap();
    image.id
}

#[tokio::test]
async fn a_picture_is_made_under_the_limits_stored_on_its_job() {
    let Some(db) = TestDb::new().await else {
        return;
    };
    let store = Store::memory();
    let ctx = Ctx {
        db: db.pool.clone(),
        store: Some(store.clone()),
        purge: None,
        paddle: None,
    };
    let handlers = Handlers::default().on(job::KIND, job::run);
    let run = || async { run_one(&ctx, &handlers, limits()).await.unwrap().unwrap().1 };

    // 2100 px is over the 2048 of a badge, so a job without limits fails it ...
    queued_badge(&db, &store, 2100, json!({})).await;
    assert_eq!(run().await, Outcome::Failed);
    // ... and a job whose uploader's roles allowed 2300 px makes it.
    let id = queued_badge(&db, &store, 2101, json!({ "limits": { "max_side": 2300 } })).await;
    assert_eq!(run().await, Outcome::Done);
    let mut conn = db.conn().await;
    let row = cosmetics::get_image(&mut conn, id).await.unwrap().unwrap();
    assert!(!row.variants.as_array().unwrap().is_empty());
    drop(conn);
    // A lowered limit fails a picture the defaults take.
    queued_badge(&db, &store, 96, json!({ "limits": { "max_side": 64 } })).await;
    assert_eq!(run().await, Outcome::Failed);
    // A job can't ask for more than the ceiling (4096 px for a badge).
    queued_badge(&db, &store, 4200, json!({ "limits": { "max_side": 9000 } })).await;
    assert_eq!(run().await, Outcome::Failed);
    db.close().await;
}
