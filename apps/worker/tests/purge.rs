//! The `files.delete` job against a real Postgres, the in-memory store and a fake Cloudflare
//! API. Runs when `CYEX_TV_TEST_DATABASE_URL` is set (CI sets it), skips otherwise.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::routing::post;
use axum::{Json, Router};
use cyex_tv_db::files::{self, NewFile};
use cyex_tv_db::testing::TestDb;
use cyex_tv_db::{ids, jobs};
use cyex_tv_storage::{Put, Store};
use cyex_tv_worker::purge::{self, Purge};
use cyex_tv_worker::{Ctx, Handlers, Limits, Outcome, run_one};
use serde_json::{Value, json};
use url::Url;

#[derive(Default)]
struct Seen {
    calls: Vec<(String, Option<String>, Value)>,
    fail: bool,
}

type Shared = Arc<Mutex<Seen>>;

async fn fake_cloudflare(seen: Shared) -> Url {
    let app = Router::new()
        .route(
            "/client/v4/zones/{zone}/purge_cache",
            post(
                |State(s): State<Shared>,
                 Path(zone): Path<String>,
                 headers: HeaderMap,
                 Json(body): Json<Value>| async move {
                    let auth = headers
                        .get("authorization")
                        .map(|v| v.to_str().unwrap().to_owned());
                    let mut s = s.lock().unwrap();
                    s.calls.push((zone, auth, body));
                    if s.fail {
                        (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            Json(json!({ "success": false })),
                        )
                    } else {
                        (
                            StatusCode::OK,
                            Json(json!({ "success": true, "errors": [] })),
                        )
                    }
                },
            ),
        )
        .with_state(seen);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    Url::parse(&format!("http://{addr}/client/v4/")).unwrap()
}

fn limits() -> Limits {
    Limits {
        lease: Duration::from_secs(60),
        timeout: Duration::from_secs(30),
        poll: Duration::from_millis(50),
    }
}

/// A transcoded file: its original and two variants in the store.
async fn transcoded(db: &TestDb, store: &Store) -> (uuid::Uuid, String) {
    let sha = [7u8; 32];
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
    files::set_variants(
        &mut conn,
        file.id,
        &json!([{ "name": "1x.webp" }, { "name": "1x.avif" }]),
    )
    .await
    .unwrap();
    for key in [
        format!("originals/{hex}"),
        format!("emotes/{hex}/1x.webp"),
        format!("emotes/{hex}/1x.avif"),
    ] {
        store
            .put(
                &key,
                b"x".to_vec().into(),
                Put {
                    content_type: "image/webp",
                    cache_control: None,
                },
            )
            .await
            .unwrap();
    }
    jobs::enqueue(
        &mut conn,
        purge::KIND,
        &json!({ "file_id": ids::to_text(file.id) }),
        3,
    )
    .await
    .unwrap();
    (file.id, hex)
}

#[tokio::test]
async fn files_are_deleted_and_purged() {
    let Some(db) = TestDb::new().await else {
        return;
    };
    let seen = Shared::default();
    let api = fake_cloudflare(seen.clone()).await;
    let store = Store::memory();
    let (file, hex) = transcoded(&db, &store).await;
    let ctx = Ctx {
        db: db.pool.clone(),
        store: Some(store.clone()),
        purge: Some(Purge {
            http: reqwest::Client::new(),
            api,
            zone_id: "zone123".to_owned(),
            token: "purge-token".to_owned(),
            cdn_url: "https://cdn.example.test".to_owned(),
        }),
    };
    let handlers = Handlers::default().on(purge::KIND, purge::run);

    // Cloudflare refuses once: the job retries, and nothing is forgotten.
    seen.lock().unwrap().fail = true;
    assert_eq!(
        run_one(&ctx, &handlers, limits()).await.unwrap().unwrap().1,
        Outcome::Retrying
    );
    let mut conn = db.conn().await;
    assert_eq!(
        files::get(&mut conn, file)
            .await
            .unwrap()
            .unwrap()
            .variants
            .as_array()
            .unwrap()
            .len(),
        2
    );
    sqlx::query("UPDATE jobs SET run_after = now()")
        .execute(&mut *conn)
        .await
        .unwrap();
    drop(conn);

    seen.lock().unwrap().fail = false;
    assert_eq!(
        run_one(&ctx, &handlers, limits()).await.unwrap().unwrap().1,
        Outcome::Done
    );
    assert!(store.keys().is_empty());
    let mut conn = db.conn().await;
    assert_eq!(
        files::get(&mut conn, file).await.unwrap().unwrap().variants,
        json!([])
    );
    drop(conn);

    let (zone, auth, body) = seen.lock().unwrap().calls.last().cloned().unwrap();
    assert_eq!(zone.as_str(), "zone123");
    assert_eq!(auth.as_deref(), Some("Bearer purge-token"));
    assert_eq!(
        body["files"],
        json!([
            format!("https://cdn.example.test/emotes/{hex}/1x.webp"),
            format!("https://cdn.example.test/emotes/{hex}/1x.avif"),
        ])
    );
    db.close().await;
}

#[tokio::test]
async fn without_a_token_files_are_still_deleted() {
    let Some(db) = TestDb::new().await else {
        return;
    };
    let store = Store::memory();
    transcoded(&db, &store).await;
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
    assert!(store.keys().is_empty());
    assert!(Purge::from_env(|_| None).unwrap().is_none());
    db.close().await;
}
