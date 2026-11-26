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
    uploaded_under(db, store, bytes, mime, json!({})).await
}

/// An upload whose job carries `extra` (the `limits` the API stored on it) next to the file.
async fn uploaded_under(
    db: &TestDb,
    store: &Store,
    bytes: &[u8],
    mime: &str,
    extra: Value,
) -> (uuid::Uuid, String) {
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
        &{
            let mut payload = extra;
            payload["file_id"] = json!(ids::to_text(file.id));
            payload
        },
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
        paddle: None,
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
        paddle: None,
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
        paddle: None,
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
        paddle: None,
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

async fn uploader(db: &TestDb, file_id: uuid::Uuid, name: &str) -> (uuid::Uuid, uuid::Uuid) {
    use cyex_tv_db::accounts::{self, Profile};
    use cyex_tv_db::emotes::{self, NewEmote};
    let mut conn = db.conn().await;
    let owner = accounts::sign_in(
        &mut conn,
        Profile {
            platform: cyex_tv_shared::model::Platform::Twitch,
            platform_user_id: "9001",
            login: "failer",
            display_name: "failer",
            avatar: None,
        },
    )
    .await
    .unwrap()
    .account
    .id;
    let emote = emotes::create(
        &mut conn,
        &NewEmote {
            owner_account_id: owner,
            file_id: Some(file_id),
            name,
            ..NewEmote::default()
        },
    )
    .await
    .unwrap();
    (owner, emote.id)
}

#[tokio::test]
async fn a_transcode_that_runs_out_of_attempts_marks_the_file_failed_and_tells_the_uploader_once() {
    use cyex_tv_db::notifications;
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
    // An original that never decodes: the job fails for good on its first attempt.
    let (file_id, _) = uploaded(&db, &store, b"\x89PNG\r\n\x1a\nnot really", "image/png").await;
    let (owner, emote_id) = uploader(&db, file_id, "broken").await;
    {
        let mut conn = db.conn().await;
        let before = files::get(&mut conn, file_id).await.unwrap().unwrap();
        assert!(before.processing_failed_at.is_none());
    }

    assert_eq!(
        run_one(&ctx, &handlers(), limits())
            .await
            .unwrap()
            .unwrap()
            .1,
        Outcome::Failed
    );
    let mut conn = db.conn().await;
    let file = files::get(&mut conn, file_id).await.unwrap().unwrap();
    assert!(file.processing_failed_at.is_some());
    let notes = notifications::list(&mut conn, owner, false).await.unwrap();
    assert_eq!(notes.len(), 1);
    assert_eq!(notes[0].kind, "emote.failed");
    assert_eq!(notes[0].payload["emote_id"], ids::to_text(emote_id));
    assert_eq!(notes[0].payload["name"], "broken");

    // Marking again (a sweep of the same job, a second call) tells nobody twice.
    assert!(!files::mark_failed(&mut conn, file_id).await.unwrap());
    drop(conn);
    let mut conn = db.conn().await;
    assert_eq!(
        notifications::list(&mut conn, owner, false)
            .await
            .unwrap()
            .len(),
        1
    );

    // An upload of the same bytes tries again: the mark is cleared.
    assert!(files::clear_failed(&mut conn, file_id).await.unwrap());
    assert!(!files::clear_failed(&mut conn, file_id).await.unwrap());
    drop(conn);
    db.close().await;
}

#[tokio::test]
async fn a_swept_transcode_fails_the_file_and_a_finished_one_is_left_alone() {
    use cyex_tv_db::notifications;
    let Some(db) = TestDb::new().await else {
        return;
    };
    let store = Store::memory();
    let (file_id, _) = uploaded(&db, &store, &png(22, 22), "image/png").await;
    let (owner, _) = uploader(&db, file_id, "swept").await;
    // The worker died during the last attempt: the lease expires with no attempts left.
    sqlx::query(
        "UPDATE jobs SET state = 'running', attempts = max_attempts,
                         locked_until = now() - interval '1 second'",
    )
    .execute(&db.pool)
    .await
    .unwrap();
    assert_eq!(cyex_tv_worker::sweep_exhausted(&db.pool).await.unwrap(), 1);
    let mut conn = db.conn().await;
    assert!(
        files::get(&mut conn, file_id)
            .await
            .unwrap()
            .unwrap()
            .processing_failed_at
            .is_some()
    );
    assert_eq!(
        notifications::list(&mut conn, owner, false)
            .await
            .unwrap()
            .len(),
        1
    );

    // A file that already has its variants is never marked (its emote is usable).
    let (done_id, _) = uploaded(&db, &store, &png(23, 23), "image/png").await;
    files::set_variants(&mut conn, done_id, &json!([{ "name": "1x.webp" }]))
        .await
        .unwrap();
    assert!(!files::mark_failed(&mut conn, done_id).await.unwrap());
    drop(conn);
    db.close().await;
}

#[tokio::test]
async fn the_job_is_checked_under_the_fixed_emote_rules_whatever_it_carries() {
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
    let run = || async {
        run_one(&ctx, &handlers(), limits())
            .await
            .unwrap()
            .unwrap()
            .1
    };
    // A job from before carries limits (a role's 64 px a side): ignored, a 100 px file is fine.
    let (file, _) = uploaded_under(
        &db,
        &store,
        &png(100, 100),
        "image/png",
        json!({ "limits": { "max_bytes": 8_388_608, "max_side": 64, "max_frames": 2 } }),
    )
    .await;
    assert_eq!(run().await, Outcome::Done);
    assert!(!variants(&db, file).await.is_empty());
    // Asking for more than an emote may be (1000 px) doesn't get it either.
    uploaded_under(
        &db,
        &store,
        &png(1100, 1100),
        "image/png",
        json!({ "limits": { "max_side": 5000 } }),
    )
    .await;
    assert_eq!(run().await, Outcome::Failed);
    // The 4:1 rule is the upload's (the API refuses a wider file first): an original that is
    // already stored, 6:1 say, is made again, fitted in the 4:1 box as before.
    let (six, _) = uploaded_under(&db, &store, &png(600, 100), "image/png", json!({})).await;
    assert_eq!(run().await, Outcome::Done);
    let v = variants(&db, six).await;
    assert!(!v.is_empty());
    assert_eq!(
        (v[0]["width"].as_u64(), v[0]["height"].as_u64()),
        (Some(128), Some(21))
    );
    // 4:1 exactly is made.
    let (wide, _) = uploaded_under(&db, &store, &png(400, 100), "image/png", json!({})).await;
    assert_eq!(run().await, Outcome::Done);
    assert!(!variants(&db, wide).await.is_empty());
    // Over 1000 frames fails (1001 tiny frames).
    uploaded_under(&db, &store, &gif(2, 2, 1001), "image/gif", json!({})).await;
    assert_eq!(run().await, Outcome::Failed);
    db.close().await;
}
