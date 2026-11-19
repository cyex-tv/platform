//! Erasing accounts after their 30-day undo window (ADR 0019): the hourly clean-up erases the ones
//! that are due, and only those. Against a real Postgres; runs when `CYEX_TV_TEST_DATABASE_URL` is
//! set (CI sets it), skips otherwise.

use chrono::{Duration, Utc};
use cyex_tv_db::accounts::{self, Profile};
use cyex_tv_db::emotes::{self, NewEmote};
use cyex_tv_db::testing::TestDb;
use cyex_tv_db::{privacy, sets};
use cyex_tv_shared::model::{EmoteState, Platform, SetKind};
use uuid::Uuid;

async fn account(db: &TestDb, twitch_id: &str) -> Uuid {
    let mut conn = db.conn().await;
    let login = format!("user{twitch_id}");
    accounts::sign_in(
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
    .unwrap()
    .account
    .id
}

async fn schedule(db: &TestDb, id: Uuid, erase_after: chrono::DateTime<Utc>) {
    let mut conn = db.conn().await;
    assert!(
        privacy::schedule_erase(&mut conn, id, erase_after)
            .await
            .unwrap()
    );
}

#[tokio::test]
async fn the_clean_up_erases_accounts_past_their_window_only() {
    let Some(db) = TestDb::new().await else {
        return;
    };
    let due = account(&db, "8801").await;
    let waiting = account(&db, "8802").await;
    let live = account(&db, "8803").await;
    {
        let mut conn = db.conn().await;
        for owner in [due, waiting] {
            let emote = emotes::create(
                &mut conn,
                &NewEmote {
                    owner_account_id: owner,
                    name: "mine",
                    ..NewEmote::default()
                },
            )
            .await
            .unwrap();
            emotes::set_state(&mut conn, emote.id, EmoteState::Approved)
                .await
                .unwrap();
            sets::create_with(&mut conn, owner, "Set", SetKind::Channel, None)
                .await
                .unwrap();
        }
    }
    schedule(&db, due, Utc::now() - Duration::minutes(1)).await;
    schedule(&db, waiting, Utc::now() + Duration::days(29)).await;

    cyex_tv_worker::clean_up(&db.pool).await.unwrap();

    let mut conn = db.conn().await;
    // The due one: identities, sets and emotes gone, the row kept as deleted, audited.
    assert!(
        accounts::identities(&mut conn, due)
            .await
            .unwrap()
            .is_empty()
    );
    assert!(
        sets::list_for_owner(&mut conn, due)
            .await
            .unwrap()
            .is_empty()
    );
    let row = accounts::get(&mut conn, due).await.unwrap().unwrap();
    assert!(row.deleted_at.is_some() && row.erase_after.is_none());
    let erased: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM audit_log WHERE action = 'account.deleted' AND target_id = $1",
    )
    .bind(cyex_tv_db::ids::to_text(due))
    .fetch_one(&mut *conn)
    .await
    .unwrap();
    assert_eq!(erased, 1);
    let left: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM emotes WHERE owner_account_id = $1 AND deleted_at IS NULL",
    )
    .bind(due)
    .fetch_one(&mut *conn)
    .await
    .unwrap();
    assert_eq!(left, 0);

    // The one still inside its window keeps everything, and can be restored.
    assert_eq!(
        accounts::identities(&mut conn, waiting)
            .await
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        sets::list_for_owner(&mut conn, waiting)
            .await
            .unwrap()
            .len(),
        1
    );
    assert!(privacy::restore(&mut conn, waiting).await.unwrap());
    let row = accounts::get(&mut conn, waiting).await.unwrap().unwrap();
    assert!(row.deleted_at.is_none() && row.erase_after.is_none());
    // A live account is never touched, and an erased one can't be restored.
    assert!(
        accounts::get(&mut conn, live)
            .await
            .unwrap()
            .unwrap()
            .deleted_at
            .is_none()
    );
    assert!(!privacy::restore(&mut conn, due).await.unwrap());
    assert!(
        privacy::due_for_erase(&mut conn, 10)
            .await
            .unwrap()
            .is_empty()
    );
    drop(conn);

    // A second clean-up has nothing to do.
    cyex_tv_worker::clean_up(&db.pool).await.unwrap();
    let mut conn = db.conn().await;
    let erased: i64 =
        sqlx::query_scalar("SELECT count(*) FROM audit_log WHERE action = 'account.deleted'")
            .fetch_one(&mut *conn)
            .await
            .unwrap();
    assert_eq!(erased, 1);
    drop(conn);
    db.close().await;
}
