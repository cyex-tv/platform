//! The clean-up of direct uploads (ADR 0036) against a real Postgres and the in-memory store:
//! an upload never finalized is deleted once it expires, a finalized one is forgotten after a
//! day, and nothing else is touched. Runs when `CYEX_TV_TEST_DATABASE_URL` is set (CI sets it),
//! skips otherwise.

use chrono::{Duration, Utc};
use cyex_tv_db::accounts::{self, Profile};
use cyex_tv_db::direct_uploads::{self, NewUpload};
use cyex_tv_db::ids;
use cyex_tv_db::testing::TestDb;
use cyex_tv_shared::model::{CosmeticKind, Platform};
use cyex_tv_storage::{Put, Store, upload_key};

async fn make(
    conn: &mut sqlx::PgConnection,
    store: &Store,
    account: uuid::Uuid,
    expires: chrono::DateTime<Utc>,
) -> (uuid::Uuid, String) {
    let id = ids::new_id();
    let key = upload_key(&ids::to_text(id));
    store
        .put(
            &key,
            b"bytes".to_vec().into(),
            Put {
                content_type: "image/png",
                cache_control: None,
            },
        )
        .await
        .unwrap();
    direct_uploads::insert(
        conn,
        &NewUpload {
            id,
            account_id: account,
            kind: CosmeticKind::Badge,
            storage_key: &key,
            content_type: "image/png",
            declared_bytes: 5,
            expires_at: expires,
        },
    )
    .await
    .unwrap();
    (id, key)
}

#[tokio::test]
async fn expired_and_old_finalized_uploads_are_removed() {
    let Some(db) = TestDb::new().await else {
        return;
    };
    let store = Store::memory();
    let mut conn = db.pool.acquire().await.unwrap();
    let account = accounts::sign_in(
        &mut conn,
        Profile {
            platform: Platform::Twitch,
            platform_user_id: "1",
            login: "staff",
            display_name: "staff",
            avatar: None,
        },
    )
    .await
    .unwrap()
    .account
    .id;
    let (expired, expired_key) = make(
        &mut conn,
        &store,
        account,
        Utc::now() - Duration::minutes(1),
    )
    .await;
    let (open, open_key) = make(&mut conn, &store, account, Utc::now() + Duration::hours(1)).await;
    let (old, _) = make(&mut conn, &store, account, Utc::now() + Duration::hours(1)).await;
    sqlx::query("UPDATE direct_uploads SET finalized_at = now() - interval '2 days' WHERE id = $1")
        .bind(old)
        .execute(&mut *conn)
        .await
        .unwrap();

    let removed = cyex_tv_worker::uploads::clean_up(&db.pool, &store)
        .await
        .unwrap();
    assert_eq!(removed, 2);
    assert!(
        direct_uploads::get(&mut conn, expired)
            .await
            .unwrap()
            .is_none()
    );
    assert!(direct_uploads::get(&mut conn, old).await.unwrap().is_none());
    assert!(
        direct_uploads::get(&mut conn, open)
            .await
            .unwrap()
            .is_some()
    );
    let keys = store.keys();
    assert!(!keys.contains(&expired_key));
    assert!(keys.contains(&open_key), "an open upload keeps its bytes");
    // Nothing more to do.
    assert_eq!(
        cyex_tv_worker::uploads::clean_up(&db.pool, &store)
            .await
            .unwrap(),
        0
    );
    drop(conn);
    db.close().await;
}
