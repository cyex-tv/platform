//! The `cosmetic.transcode` job (ADR 0015) against a real Postgres and the in-memory store: a
//! badge picture becomes 18, 36 and 72 px files under `cosmetics/badges/`, recorded on its row.
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
    assert_eq!(webp, [("1x.webp", 18), ("2x.webp", 36), ("4x.webp", 72)]);
    assert_eq!(files.len(), 6);
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
    db.close().await;
}
