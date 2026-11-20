//! Stored channel counts: the triggers queue what may have changed, the minutely refresh recounts
//! it, the hourly reconcile recounts everything, and the stored count always ends up equal to the
//! live one. Runs when `CYEX_TV_TEST_DATABASE_URL` is set (CI sets it) and skips otherwise.

mod common;

use std::collections::HashMap;

use common::TestDb;
use cyex_tv_db::{channel_counts, reads};
use sqlx::PgConnection;
use uuid::Uuid;

async fn exec(c: &mut PgConnection, sql: &str) {
    sqlx::query(sql)
        .execute(&mut *c)
        .await
        .unwrap_or_else(|e| panic!("{sql}: {e}"));
}

async fn account(c: &mut PgConnection) -> Uuid {
    sqlx::query_scalar("INSERT INTO accounts (id) VALUES (gen_random_uuid()) RETURNING id")
        .fetch_one(&mut *c)
        .await
        .unwrap()
}

async fn emote(c: &mut PgConnection, owner: Uuid, name: &str) -> Uuid {
    sqlx::query_scalar(
        "INSERT INTO emotes (id, owner_account_id, name, state)
         VALUES (gen_random_uuid(), $1, $2, 'approved') RETURNING id",
    )
    .bind(owner)
    .bind(name)
    .fetch_one(&mut *c)
    .await
    .unwrap()
}

async fn set(c: &mut PgConnection, owner: Uuid, name: &str) -> Uuid {
    sqlx::query_scalar(
        "INSERT INTO emote_sets (id, owner_account_id, name, kind, capacity)
         VALUES (gen_random_uuid(), $1, $2, 'channel', 1000) RETURNING id",
    )
    .bind(owner)
    .bind(name)
    .fetch_one(&mut *c)
    .await
    .unwrap()
}

async fn channel(c: &mut PgConnection, owner: Uuid, n: u32, active: Option<Uuid>) -> Uuid {
    sqlx::query_scalar(
        "INSERT INTO channels (id, account_id, platform, platform_channel_id, login, active_set_id)
         VALUES (gen_random_uuid(), $1, 'twitch', $2, $3, $4) RETURNING id",
    )
    .bind(owner)
    .bind(format!("{}", 7000 + n))
    .bind(format!("chan{n}"))
    .bind(active)
    .fetch_one(&mut *c)
    .await
    .unwrap()
}

async fn add(c: &mut PgConnection, set: Uuid, emotes: &[Uuid]) {
    sqlx::query(
        "INSERT INTO emote_set_members (set_id, emote_id, name)
         SELECT $1, e, 'm' || left(e::text, 8) FROM unnest($2::uuid[]) e",
    )
    .bind(set)
    .bind(emotes)
    .execute(&mut *c)
    .await
    .unwrap();
}

async fn remove(c: &mut PgConnection, set: Uuid, emote: Uuid) {
    sqlx::query("DELETE FROM emote_set_members WHERE set_id = $1 AND emote_id = $2")
        .bind(set)
        .bind(emote)
        .execute(&mut *c)
        .await
        .unwrap();
}

/// The stored counts after the worker's minutely round, checked against the live count.
async fn stored(c: &mut PgConnection, emotes: &[Uuid]) -> HashMap<Uuid, u32> {
    channel_counts::refresh_queued(c).await.unwrap();
    assert_eq!(channel_counts::queued(c).await.unwrap(), 0, "queue left");
    let stored = reads::channel_counts(c, emotes).await.unwrap();
    let live = channel_counts::live(c, emotes).await.unwrap();
    assert_eq!(stored, live, "stored and live counts differ");
    stored
}

fn n(counts: &HashMap<Uuid, u32>, id: Uuid) -> u32 {
    counts.get(&id).copied().unwrap_or(0)
}

#[tokio::test]
async fn stored_counts_follow_adds_removes_switches_and_blocks() {
    let Some(db) = TestDb::new().await else {
        return;
    };
    let mut c = db.conn().await;
    let c = &mut *c;
    let creator = account(c).await;
    let (a, b, z) = (
        emote(c, creator, "Alpha").await,
        emote(c, creator, "Beta").await,
        emote(c, creator, "Zeta").await,
    );
    let all = [a, b, z];
    let owners = [account(c).await, account(c).await, account(c).await];
    let mut sets = Vec::new();
    let mut chans = Vec::new();
    for (i, owner) in owners.iter().enumerate() {
        let s = set(c, *owner, &format!("set{i}")).await;
        chans.push(channel(c, *owner, i as u32, Some(s)).await);
        sets.push(s);
    }
    // Nothing anywhere yet.
    assert!(stored(c, &all).await.is_empty());

    // Adds: `a` in all three channels, `b` in one.
    for s in &sets {
        add(c, *s, &[a]).await;
    }
    add(c, sets[0], &[b]).await;
    assert!(channel_counts::queued(c).await.unwrap() > 0);
    // Reads keep the old value until the worker's round.
    assert!(reads::channel_counts(c, &all).await.unwrap().is_empty());
    let got = stored(c, &all).await;
    assert_eq!((n(&got, a), n(&got, b), n(&got, z)), (3, 1, 0));

    // A set no channel shows: nothing queued, nothing counted.
    let spare = set(c, owners[0], "spare").await;
    add(c, spare, &[z]).await;
    assert_eq!(channel_counts::queued(c).await.unwrap(), 0);
    assert_eq!(n(&stored(c, &all).await, z), 0);

    // A removal.
    remove(c, sets[2], a).await;
    assert_eq!(n(&stored(c, &all).await, a), 2);

    // A channel switching to the spare set: its old set's emotes lose one, the spare's gain one.
    sqlx::query("UPDATE channels SET active_set_id = $1 WHERE id = $2")
        .bind(spare)
        .bind(chans[0])
        .execute(&mut *c)
        .await
        .unwrap();
    let got = stored(c, &all).await;
    assert_eq!((n(&got, a), n(&got, b), n(&got, z)), (1, 0, 1));
    sqlx::query("UPDATE channels SET active_set_id = $1 WHERE id = $2")
        .bind(sets[0])
        .bind(chans[0])
        .execute(&mut *c)
        .await
        .unwrap();
    assert_eq!(n(&stored(c, &all).await, a), 2);

    // An account blocked, then unblocked; one deleted.
    exec(
        c,
        &format!(
            "UPDATE accounts SET blocked_at = now() WHERE id = '{}'",
            owners[1]
        ),
    )
    .await;
    assert_eq!(n(&stored(c, &all).await, a), 1);
    exec(
        c,
        &format!(
            "UPDATE accounts SET blocked_at = NULL WHERE id = '{}'",
            owners[1]
        ),
    )
    .await;
    assert_eq!(n(&stored(c, &all).await, a), 2);
    exec(
        c,
        &format!(
            "UPDATE accounts SET deleted_at = now() WHERE id = '{}'",
            owners[0]
        ),
    )
    .await;
    let got = stored(c, &all).await;
    assert_eq!((n(&got, a), n(&got, b)), (1, 0));

    // A set deleted while a channel shows it (its link is cleared by the foreign key).
    exec(
        c,
        &format!("DELETE FROM emote_sets WHERE id = '{}'", sets[1]),
    )
    .await;
    assert_eq!(n(&stored(c, &all).await, a), 0);

    // A channel removed, and one added that already shows a set.
    let owner = account(c).await;
    let s = set(c, owner, "late").await;
    add(c, s, &[a, b]).await;
    let late = channel(c, owner, 9, Some(s)).await;
    let got = stored(c, &all).await;
    assert_eq!((n(&got, a), n(&got, b)), (1, 1));
    exec(c, &format!("DELETE FROM channels WHERE id = '{late}'")).await;
    assert!(stored(c, &all).await.is_empty());
    db.close().await;
}

#[tokio::test]
async fn the_hourly_reconcile_repairs_what_no_write_announced() {
    let Some(db) = TestDb::new().await else {
        return;
    };
    let mut c = db.conn().await;
    let c = &mut *c;
    let creator = account(c).await;
    let a = emote(c, creator, "Alpha").await;
    let b = emote(c, creator, "Beta").await;
    let owners = [account(c).await, account(c).await];
    for (i, owner) in owners.iter().enumerate() {
        let s = set(c, *owner, &format!("set{i}")).await;
        channel(c, *owner, i as u32, Some(s)).await;
        add(c, s, &[a, b]).await;
    }
    // A timed block that has run out by the time of the next read changes the live count
    // without a write; so does a count edited by hand. Only the reconcile sees them.
    exec(
        c,
        &format!(
            "UPDATE accounts SET blocked_at = now(), blocked_until = now() + interval '1 hour' WHERE id = '{}'",
            owners[0]
        ),
    )
    .await;
    channel_counts::refresh_queued(c).await.unwrap();
    assert_eq!(
        n(&reads::channel_counts(c, &[a]).await.unwrap(), a),
        1,
        "blocked"
    );
    // Turn the clock forward for that block: it ends without touching the row's watched columns.
    exec(c, "ALTER TABLE accounts DISABLE TRIGGER accounts_counts").await;
    exec(
        c,
        &format!(
            "UPDATE accounts SET blocked_until = now() - interval '1 minute' WHERE id = '{}'",
            owners[0]
        ),
    )
    .await;
    exec(c, "ALTER TABLE accounts ENABLE TRIGGER accounts_counts").await;
    exec(
        c,
        &format!("UPDATE emotes SET channel_count = 40 WHERE id = '{b}'"),
    )
    .await;
    channel_counts::refresh_queued(c).await.unwrap();
    assert_eq!(n(&reads::channel_counts(c, &[a]).await.unwrap(), a), 1);
    assert_eq!(n(&reads::channel_counts(c, &[b]).await.unwrap(), b), 40);

    assert_eq!(channel_counts::reconcile_all(c).await.unwrap(), 2);
    let got = reads::channel_counts(c, &[a, b]).await.unwrap();
    assert_eq!(got, channel_counts::live(c, &[a, b]).await.unwrap());
    assert_eq!((n(&got, a), n(&got, b)), (2, 2));
    // Nothing left to change.
    assert_eq!(channel_counts::reconcile_all(c).await.unwrap(), 0);
    db.close().await;
}

#[tokio::test]
async fn a_long_queue_is_worked_through_in_rounds() {
    let Some(db) = TestDb::new().await else {
        return;
    };
    let mut c = db.conn().await;
    let c = &mut *c;
    let creator = account(c).await;
    let a = emote(c, creator, "Alpha").await;
    let owner = account(c).await;
    let s = set(c, owner, "set").await;
    channel(c, owner, 1, Some(s)).await;
    // Many queue rows (more than one round's batch), all for the same few emotes.
    let many: Vec<Uuid> = (0..(channel_counts::QUEUE_BATCH + 10)).map(|_| a).collect();
    sqlx::query("INSERT INTO emote_channel_count_queue (emote_id) SELECT unnest($1::uuid[])")
        .bind(&many)
        .execute(&mut *c)
        .await
        .unwrap();
    add(c, s, &[a]).await;
    // Another add, queued behind them: counted in the second round.
    let o = emote(c, creator, "Omega").await;
    add(c, s, &[o]).await;
    assert_eq!(channel_counts::refresh_queued(c).await.unwrap(), 2);
    assert_eq!(channel_counts::queued(c).await.unwrap(), 0);
    let got = reads::channel_counts(c, &[a, o]).await.unwrap();
    assert_eq!((n(&got, a), n(&got, o)), (1, 1));
    db.close().await;
}
