//! Set writes against a real Postgres: they never deadlock each other, and an emote change bumps
//! each set holding it once per statement (ADR 0035, the set-version locks).

mod common;

use std::time::Duration;

use common::TestDb;
use cyex_tv_db::accounts::{self, Profile};
use cyex_tv_db::emotes::{self, NewEmote};
use cyex_tv_db::sets::{self, NewMember};
use cyex_tv_db::{DbError, is_conflict};
use cyex_tv_shared::model::{EmoteState, Platform, SetKind};
use sqlx::{Connection, PgConnection};
use uuid::Uuid;

macro_rules! db {
    () => {
        match TestDb::new().await {
            Some(db) => db,
            None => return,
        }
    };
}

async fn account(conn: &mut PgConnection, twitch_id: &str) -> Uuid {
    let login = format!("user{twitch_id}");
    accounts::sign_in(
        conn,
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

async fn approved_emote(conn: &mut PgConnection, owner: Uuid, name: &str) -> Uuid {
    let emote = emotes::create(
        conn,
        &NewEmote {
            owner_account_id: owner,
            name,
            ..NewEmote::default()
        },
    )
    .await
    .unwrap();
    emotes::set_state(conn, emote.id, EmoteState::Approved)
        .await
        .unwrap();
    emote.id
}

fn member(emote_id: Uuid) -> NewMember<'static> {
    NewMember {
        emote_id,
        name: None,
        zero_width: None,
        added_by: None,
    }
}

/// A wait that never ends (locks held by a connection nobody uses again) fails the test here
/// instead of hanging the run.
const PATIENCE: Duration = Duration::from_secs(30);

async fn within<T>(what: &str, f: impl std::future::Future<Output = T>) -> T {
    tokio::time::timeout(PATIENCE, f)
        .await
        .unwrap_or_else(|_| panic!("{what} still waiting after {PATIENCE:?}"))
}

fn is_deadlock<T>(result: &Result<T, DbError>) -> bool {
    result.as_ref().err().is_some_and(is_conflict)
}

async fn version(conn: &mut PgConnection, set: Uuid) -> i64 {
    sqlx::query_scalar("SELECT version FROM emote_sets WHERE id = $1")
        .bind(set)
        .fetch_one(conn)
        .await
        .unwrap()
}

/// The exact interleaving that deadlocked: an `add_member` holds the set while a removal of the
/// same emote runs. The removal used to delete the member row first and then wait for the set
/// (its version trigger), while the insert waited for that deleted row: a cycle Postgres broke
/// with `40P01`. Now the removal waits for the set before it touches any member row.
#[tokio::test]
async fn add_member_and_remove_from_all_sets_never_deadlock() {
    let db = db!();
    let mut setup = db.conn().await;
    let owner = account(&mut setup, "7101").await;
    let set = sets::create(&mut setup, owner, "main", SetKind::Channel)
        .await
        .unwrap();
    let emote = approved_emote(&mut setup, owner, "wave").await;
    sets::add_member(&mut setup, set.id, member(emote))
        .await
        .unwrap();

    let mut a = db.conn().await;
    let mut tx = a.begin().await.unwrap();
    // What `add_member` does first: the set, locked.
    sqlx::query("SELECT 1 FROM emote_sets WHERE id = $1 FOR NO KEY UPDATE")
        .bind(set.id)
        .execute(&mut *tx)
        .await
        .unwrap();

    let pool = db.pool.clone();
    let removal = tokio::spawn(async move {
        let mut b = pool.acquire().await.unwrap();
        emotes::remove_from_all_sets(&mut b, emote).await
    });
    // Let the removal get as far as it can while the set is held.
    tokio::time::sleep(Duration::from_millis(300)).await;
    let added = within(
        "add_member",
        sets::add_member(&mut tx, set.id, member(emote)),
    )
    .await;
    assert!(!is_deadlock(&added), "add_member deadlocked: {added:?}");
    // The member was still there when the insert ran, so it is a duplicate.
    assert!(matches!(added, Err(DbError::AlreadyInSet)), "{added:?}");
    within("commit", tx.commit()).await.unwrap();

    let removed = within("the removal", removal).await.unwrap();
    assert!(!is_deadlock(&removed), "removal deadlocked: {removed:?}");
    assert_eq!(removed.unwrap(), vec![(set.id, "wave".to_owned())]);
    db.close().await;
}

/// The same two writes raced many times over two connections, the member alternately there and
/// gone: none of them ever fails with a deadlock.
#[tokio::test]
async fn racing_add_and_remove_many_times_never_deadlocks() {
    let db = db!();
    let mut setup = db.conn().await;
    let owner = account(&mut setup, "7102").await;
    let set = sets::create(&mut setup, owner, "main", SetKind::Channel)
        .await
        .unwrap();
    let emote = approved_emote(&mut setup, owner, "race").await;

    let mut a = db.conn().await;
    let mut b = db.conn().await;
    for round in 0..200 {
        if round % 2 == 0 {
            let _ = within("setup", sets::add_member(&mut setup, set.id, member(emote))).await;
        }
        let (added, removed) = within(&format!("round {round}"), async {
            tokio::join!(
                sets::add_member(&mut a, set.id, member(emote)),
                emotes::remove_from_all_sets(&mut b, emote),
            )
        })
        .await;
        assert!(!is_deadlock(&added), "round {round}: {added:?}");
        assert!(!is_deadlock(&removed), "round {round}: {removed:?}");
        assert!(
            matches!(added, Ok(_) | Err(DbError::AlreadyInSet)),
            "round {round}: {added:?}"
        );
        removed.unwrap();
    }
    db.close().await;
}

/// One `UPDATE` of several emotes in the same set bumps the set once, logging each emote at that
/// one version. Row by row it bumped once per emote.
#[tokio::test]
async fn a_multi_row_emote_update_bumps_each_set_once() {
    let db = db!();
    let mut c = db.conn().await;
    let owner = account(&mut c, "7103").await;
    let one = sets::create(&mut c, owner, "one", SetKind::Channel)
        .await
        .unwrap();
    let two = sets::create(&mut c, owner, "two", SetKind::Channel)
        .await
        .unwrap();
    let a = approved_emote(&mut c, owner, "alpha").await;
    let b = approved_emote(&mut c, owner, "beta").await;
    let lone = approved_emote(&mut c, owner, "gamma").await;
    for emote in [a, b, lone] {
        sets::add_member(&mut c, one.id, member(emote))
            .await
            .unwrap();
    }
    sets::add_member(&mut c, two.id, member(a)).await.unwrap();
    let (one_before, two_before) = (version(&mut c, one.id).await, version(&mut c, two.id).await);

    sqlx::query("UPDATE emotes SET nsfw = true WHERE id = ANY($1)")
        .bind(vec![a, b])
        .execute(&mut *c)
        .await
        .unwrap();
    assert_eq!(version(&mut c, one.id).await, one_before + 1);
    assert_eq!(version(&mut c, two.id).await, two_before + 1);
    let mut logged: Vec<Uuid> = sqlx::query_scalar(
        "SELECT emote_id FROM emote_set_changes WHERE set_id = $1 AND version = $2 AND kind = 'emote'",
    )
    .bind(one.id)
    .bind(one_before + 1)
    .fetch_all(&mut *c)
    .await
    .unwrap();
    logged.sort();
    let mut want = vec![a, b];
    want.sort();
    assert_eq!(logged, want);

    // A statement that changes nothing a set shows bumps nothing.
    sqlx::query("UPDATE emotes SET popularity = popularity + 1 WHERE id = ANY($1)")
        .bind(vec![a, b, lone])
        .execute(&mut *c)
        .await
        .unwrap();
    assert_eq!(version(&mut c, one.id).await, one_before + 1);
    db.close().await;
}
