//! Snowflake public IDs (ADR 0027): the generator in Postgres, the backfill of existing rows, and
//! the lookups the API's edges use. Runs when `CYEX_TV_TEST_DATABASE_URL` is set (CI sets it) and
//! skips otherwise.

mod common;

use std::collections::{HashMap, HashSet};

use chrono::{DateTime, Duration, Utc};
use common::TestDb;
use cyex_tv_db::{ids, outbox, public_ids};
use cyex_tv_shared::model::EventType;
use sqlx::PgConnection;
use uuid::Uuid;

/// The migration that adds `public_id`.
const SNOWFLAKE_MIGRATION: i64 = 20_261_212_000_001;

async fn account(c: &mut PgConnection) -> (Uuid, i64) {
    sqlx::query_as("INSERT INTO accounts (id) VALUES ($1) RETURNING id, public_id")
        .bind(ids::new_id())
        .fetch_one(&mut *c)
        .await
        .unwrap()
}

async fn set(c: &mut PgConnection, owner: Uuid) -> (Uuid, i64) {
    sqlx::query_as(
        "INSERT INTO emote_sets (id, owner_account_id, name, kind, capacity)
         VALUES ($1, $2, 'mine', 'channel', 600) RETURNING id, public_id",
    )
    .bind(ids::new_id())
    .bind(owner)
    .fetch_one(&mut *c)
    .await
    .unwrap()
}

async fn emote(c: &mut PgConnection, owner: Uuid) -> (Uuid, i64) {
    sqlx::query_as(
        "INSERT INTO emotes (id, owner_account_id, name, state)
         VALUES ($1, $2, 'wave', 'approved') RETURNING id, public_id",
    )
    .bind(ids::new_id())
    .bind(owner)
    .fetch_one(&mut *c)
    .await
    .unwrap()
}

#[tokio::test]
async fn ids_increase_and_carry_their_time() {
    let Some(db) = TestDb::new().await else {
        return;
    };
    let mut c = db.conn().await;
    let before = Utc::now();
    let mut made = Vec::new();
    // Far more than 4096: several milliseconds are filled to the last sequence number.
    for _ in 0..6000 {
        made.push(public_ids::generate(&mut c).await.unwrap());
    }
    let after = Utc::now();
    assert!(made.windows(2).all(|w| w[0] < w[1]), "strictly increasing");
    for id in [made[0], made[made.len() - 1]] {
        let parts = public_ids::decode(id);
        assert!(parts.time >= before - Duration::seconds(2), "{parts:?}");
        assert!(parts.time <= after + Duration::seconds(2), "{parts:?}");
        assert_eq!(parts.instance, 0);
    }
    // The database reads its own IDs the same way.
    let time: DateTime<Utc> = sqlx::query_scalar("SELECT snowflake_time($1)")
        .bind(made[0])
        .fetch_one(&mut *c)
        .await
        .unwrap();
    assert_eq!(
        time.timestamp_millis(),
        public_ids::decode(made[0]).time.timestamp_millis()
    );
    drop(c);
    db.close().await;
}

#[tokio::test]
async fn ids_are_unique_under_concurrency_and_carry_the_instance() {
    let Some(db) = TestDb::new().await else {
        return;
    };
    let mut tasks = Vec::new();
    for _ in 0..8 {
        let pool = db.pool.clone();
        tasks.push(tokio::spawn(async move {
            let mut c = pool.acquire().await.unwrap();
            let mut mine = Vec::new();
            for _ in 0..500 {
                mine.push(public_ids::generate(&mut c).await.unwrap());
            }
            mine
        }));
    }
    let mut all = HashSet::new();
    for task in tasks {
        let mine = task.await.unwrap();
        assert!(mine.windows(2).all(|w| w[0] < w[1]));
        for id in mine {
            assert!(all.insert(id), "{id} handed out twice");
        }
    }
    assert_eq!(all.len(), 4000);

    // A second database (or a deliberately split one) sets its instance.
    let mut c = db.conn().await;
    sqlx::query("SET cyex_tv.snowflake_instance = '7'")
        .execute(&mut *c)
        .await
        .unwrap();
    let id = public_ids::generate(&mut c).await.unwrap();
    assert_eq!(public_ids::decode(id).instance, 7);
    assert!(id > *all.iter().max().unwrap());
    sqlx::query("SET cyex_tv.snowflake_instance = '1024'")
        .execute(&mut *c)
        .await
        .unwrap();
    assert!(public_ids::generate(&mut c).await.is_err());
    // An error inside the generator gives its lock back.
    sqlx::query("RESET cyex_tv.snowflake_instance")
        .execute(&mut *c)
        .await
        .unwrap();
    drop(c);
    let mut other = db.conn().await;
    assert!(public_ids::generate(&mut other).await.is_ok());
    drop(other);
    db.close().await;
}

#[tokio::test]
async fn new_rows_get_ids_unique_across_tables_and_the_lookups_find_them() {
    let Some(db) = TestDb::new().await else {
        return;
    };
    let mut c = db.conn().await;
    let (a, a_pub) = account(&mut c).await;
    let (s, s_pub) = set(&mut c, a).await;
    let (e, e_pub) = emote(&mut c, a).await;
    assert!(a_pub < s_pub && s_pub < e_pub, "in creation order");

    let publics = public_ids::publics(&mut c, &[a, s, e, ids::new_id()])
        .await
        .unwrap();
    assert_eq!(publics, HashMap::from([(a, a_pub), (s, s_pub), (e, e_pub)]));
    let rows = public_ids::resolve(&mut c, &[a_pub, s_pub, e_pub, 1])
        .await
        .unwrap();
    assert_eq!(rows, HashMap::from([(a_pub, a), (s_pub, s), (e_pub, e)]));
    assert_eq!(
        public_ids::canonical(&mut c, &e_pub.to_string())
            .await
            .unwrap(),
        ids::to_text(e)
    );
    assert_eq!(public_ids::canonical(&mut c, "1").await.unwrap(), "1");

    // `from_text` reads a resolved Snowflake inside the request only.
    let text = s_pub.to_string();
    assert_eq!(ids::from_text(&text), None);
    let seen = public_ids::resolving(rows, async { ids::from_text(&text) }).await;
    assert_eq!(seen, Some(s));

    // Answers: account, emote and set IDs become Snowflakes; others stay.
    let role = ids::new_id();
    let mut answer = serde_json::json!({
        "id": ids::to_text(e),
        "owner": { "id": ids::to_text(a) },
        "set_ids": [ids::to_text(s)],
        "roleId": ids::to_text(role),
    });
    public_ids::publish_json(&mut c, &mut answer).await.unwrap();
    assert_eq!(
        answer,
        serde_json::json!({
            "id": e_pub.to_string(),
            "owner": { "id": a_pub.to_string() },
            "set_ids": [s_pub.to_string()],
            "roleId": ids::to_text(role),
        })
    );
    drop(c);
    db.close().await;
}

#[tokio::test]
async fn events_are_stored_by_snowflake_and_old_set_topics_map() {
    let Some(db) = TestDb::new().await else {
        return;
    };
    let mut c = db.conn().await;
    let (a, _) = account(&mut c).await;
    let (s, s_pub) = set(&mut c, a).await;
    let (e, e_pub) = emote(&mut c, a).await;
    let id = outbox::publish(
        &mut c,
        &outbox::NewEvent {
            event_type: EventType::SetEmoteAdded,
            topics: vec![
                outbox::set_topic(s),
                outbox::channel_topic(cyex_tv_shared::model::Platform::Twitch, "1"),
            ],
            set_id: Some(s),
            payload: serde_json::json!({ "emote_id": ids::to_text(e), "name": "wave" }),
        },
    )
    .await
    .unwrap();
    let stored = outbox::get(&mut c, id).await.unwrap().unwrap();
    assert_eq!(
        stored.topics,
        vec![format!("set:{s_pub}"), "channel:twitch:1".to_owned()]
    );
    assert_eq!(stored.payload["emote_id"], e_pub.to_string());
    assert_eq!(stored.payload["set_id"], s_pub.to_string());
    assert_eq!(
        stored.set_id,
        Some(s),
        "the column keeps the row for the caches"
    );

    // Subscribing by either form hears the same topic.
    let legacy = outbox::set_topic(s);
    assert_eq!(
        outbox::public_topic(&mut c, &legacy).await.unwrap(),
        Some(format!("set:{s_pub}"))
    );
    let new = format!("set:{s_pub}");
    assert_eq!(
        outbox::public_topic(&mut c, &new).await.unwrap(),
        Some(new.clone())
    );
    assert_eq!(
        outbox::public_topic(&mut c, &outbox::set_topic(ids::new_id()))
            .await
            .unwrap(),
        None
    );
    let replay = outbox::since(&mut c, 0, &[new], 10).await.unwrap();
    assert_eq!(replay.len(), 1);
    drop(c);
    db.close().await;
}

#[tokio::test]
async fn the_backfill_follows_created_at_and_stays_unique() {
    let Some(db) = TestDb::before(SNOWFLAKE_MIGRATION).await else {
        return;
    };
    let mut c = db.conn().await;
    // Seeded as they were before: one account from before the epoch, accounts, sets and emotes
    // over a few days, and 5000 emotes made in the same millisecond (more than one millisecond
    // of sequence numbers).
    let base: DateTime<Utc> = "2026-11-01T10:00:00.123Z".parse().unwrap();
    let mut expected: Vec<(Uuid, DateTime<Utc>)> = Vec::new();
    let old = ids::new_id();
    let old_at: DateTime<Utc> = "2025-12-24T00:00:00Z".parse().unwrap();
    sqlx::query("INSERT INTO accounts (id, created_at) VALUES ($1, $2)")
        .bind(old)
        .bind(old_at)
        .execute(&mut *c)
        .await
        .unwrap();
    expected.push((old, old_at));
    let owner = ids::new_id();
    sqlx::query("INSERT INTO accounts (id, created_at) VALUES ($1, $2)")
        .bind(owner)
        .bind(base)
        .execute(&mut *c)
        .await
        .unwrap();
    expected.push((owner, base));
    for day in 1..=3 {
        let at = base + Duration::days(day);
        let set = ids::new_id();
        sqlx::query(
            "INSERT INTO emote_sets (id, owner_account_id, name, kind, capacity, created_at)
             VALUES ($1, $2, 'mine', 'channel', 600, $3)",
        )
        .bind(set)
        .bind(owner)
        .bind(at)
        .execute(&mut *c)
        .await
        .unwrap();
        expected.push((set, at));
    }
    let burst = base + Duration::days(4);
    let burst_ids: Vec<Uuid> = (0..5000).map(|_| ids::new_id()).collect();
    sqlx::query(
        "INSERT INTO emotes (id, owner_account_id, name, state, created_at)
         SELECT id, $2, 'wave', 'approved', $3 FROM unnest($1::uuid[]) AS t(id)",
    )
    .bind(&burst_ids)
    .bind(owner)
    .bind(burst)
    .execute(&mut *c)
    .await
    .unwrap();
    let later = base + Duration::days(5);
    let last = ids::new_id();
    sqlx::query(
        "INSERT INTO emotes (id, owner_account_id, name, state, created_at)
         VALUES ($1, $2, 'late', 'approved', $3)",
    )
    .bind(last)
    .bind(owner)
    .bind(later)
    .execute(&mut *c)
    .await
    .unwrap();
    drop(c);

    db.migrate().await;

    let mut c = db.conn().await;
    let rows: Vec<(Uuid, i64, DateTime<Utc>)> = sqlx::query_as(
        "SELECT id, public_id, created_at FROM accounts
         UNION ALL SELECT id, public_id, created_at FROM emote_sets
         UNION ALL SELECT id, public_id, created_at FROM emotes",
    )
    .fetch_all(&mut *c)
    .await
    .unwrap();
    assert_eq!(rows.len(), 2 + 3 + 5000 + 1);
    let unique: HashSet<i64> = rows.iter().map(|r| r.1).collect();
    assert_eq!(unique.len(), rows.len(), "unique across all three tables");
    let by_id: HashMap<Uuid, i64> = rows.iter().map(|r| (r.0, r.1)).collect();

    // Ordered as created.
    let mut sorted = rows.clone();
    sorted.sort_by_key(|r| (r.2, r.1));
    assert!(sorted.windows(2).all(|w| w[0].1 < w[1].1));

    // The time part is the row's created_at (to the millisecond), except before the epoch.
    assert_eq!(
        public_ids::decode(by_id[&old]).time.timestamp_millis(),
        public_ids::EPOCH_MS + 1
    );
    for (id, at) in &expected[1..] {
        assert_eq!(
            public_ids::decode(by_id[id]).time.timestamp_millis(),
            at.timestamp_millis(),
            "{id}"
        );
    }
    // The burst fills its millisecond, then the next one.
    let first_burst = burst_ids.iter().map(|id| by_id[id]).min().unwrap();
    assert_eq!(
        public_ids::decode(first_burst).time.timestamp_millis(),
        burst.timestamp_millis()
    );
    assert_eq!(public_ids::decode(first_burst).sequence, 0);
    assert_eq!(
        public_ids::decode(by_id[&last]).time.timestamp_millis(),
        later.timestamp_millis()
    );

    // New rows come after every backfilled one.
    let max = rows.iter().map(|r| r.1).max().unwrap();
    let (_, fresh) = account(&mut c).await;
    assert!(fresh > max);
    drop(c);
    db.close().await;
}
