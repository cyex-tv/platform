//! A takedown that lands while a transcode is queued or running: the file's `variants` is still
//! empty, but the transcode goes on to write `emotes/{sha}/*`. Purge must remove those names
//! anyway, and a transcode that finds the file blocked must clean up after itself. Runs when
//! `CYEX_TV_TEST_DATABASE_URL` is set (CI sets it), skips otherwise.

use std::io::Cursor;
use std::time::Duration;

use cyex_tv_db::files::{self, NewFile};
use cyex_tv_db::testing::TestDb;
use cyex_tv_db::{blocklist, ids, jobs};
use cyex_tv_media::encode;
use cyex_tv_storage::{Put, Store, emote_key};
use cyex_tv_worker::{Ctx, Handlers, Limits, Outcome, purge, run_one, transcode};
use image::{ImageFormat, Rgba, RgbaImage};
use serde_json::json;
use sha2::Digest;

fn limits() -> Limits {
    Limits {
        lease: Duration::from_secs(60),
        timeout: Duration::from_secs(30),
        poll: Duration::from_millis(50),
    }
}

async fn put(store: &Store, key: &str) {
    store
        .put(
            key,
            b"x".to_vec().into(),
            Put {
                content_type: "image/webp",
                cache_control: None,
            },
        )
        .await
        .unwrap();
}

#[tokio::test]
async fn purge_removes_every_possible_name_even_when_variants_is_empty() {
    let Some(db) = TestDb::new().await else {
        return;
    };
    let sha = [9u8; 32];
    let hex: String = sha.iter().map(|b| format!("{b:02x}")).collect();
    let mut conn = db.conn().await;
    let (file, _) = files::insert_or_get(
        &mut conn,
        &NewFile {
            sha256: &sha,
            dhash: None,
            mime: "image/png",
            width: 32,
            height: 32,
            frame_count: 1,
            storage_key: &format!("originals/{hex}"),
        },
    )
    .await
    .unwrap();
    // No variants recorded: the transcode hadn't finished when the takedown ran.
    let store = Store::memory();
    put(&store, &format!("originals/{hex}")).await;
    for name in encode::all_names() {
        put(&store, &emote_key(&hex, &name)).await;
    }
    jobs::enqueue(
        &mut conn,
        purge::KIND,
        &json!({ "file_id": ids::to_text(file.id) }),
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
    let handlers = Handlers::default().on(purge::KIND, purge::run);
    assert_eq!(
        run_one(&ctx, &handlers, limits()).await.unwrap().unwrap().1,
        Outcome::Done
    );
    assert!(store.keys().is_empty(), "left: {:?}", store.keys());
    db.close().await;
}

#[tokio::test]
async fn a_transcode_of_a_blocked_file_leaves_nothing_behind() {
    let Some(db) = TestDb::new().await else {
        return;
    };
    let mut bytes = Cursor::new(Vec::new());
    RgbaImage::from_pixel(32, 32, Rgba([200, 10, 10, 255]))
        .write_to(&mut bytes, ImageFormat::Png)
        .unwrap();
    let bytes = bytes.into_inner();
    let sha: [u8; 32] = sha2::Sha256::digest(&bytes).into();
    let hex: String = sha.iter().map(|b| format!("{b:02x}")).collect();
    let key = format!("originals/{hex}");
    let mut conn = db.conn().await;
    let (file, _) = files::insert_or_get(
        &mut conn,
        &NewFile {
            sha256: &sha,
            dhash: None,
            mime: "image/png",
            width: 32,
            height: 32,
            frame_count: 1,
            storage_key: &key,
        },
    )
    .await
    .unwrap();
    // The takedown has already blocked it.
    blocklist::add(&mut conn, Some(&sha), None, "takedown: test", None)
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
    drop(conn);
    let store = Store::memory();
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

    let ctx = Ctx {
        db: db.pool.clone(),
        store: Some(store.clone()),
        purge: None,
    };
    let handlers = Handlers::default().on(transcode::KIND, transcode::run);
    assert_eq!(
        run_one(&ctx, &handlers, limits()).await.unwrap().unwrap().1,
        Outcome::Done
    );
    assert_eq!(store.keys(), vec![key]);
    let mut conn = db.conn().await;
    assert_eq!(
        files::get(&mut conn, file.id)
            .await
            .unwrap()
            .unwrap()
            .variants,
        json!([])
    );
    drop(conn);
    db.close().await;
}
