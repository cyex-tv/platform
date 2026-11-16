//! The feeder against a real Postgres, including a commit out of ID order and a rolled-back ID.
//! Runs when `CYEX_TV_TEST_DATABASE_URL` is set (CI sets it), skips otherwise.

use std::time::Duration;

use cyex_tv_db::outbox::{self, NewEvent};
use cyex_tv_db::testing::TestDb;
use cyex_tv_hub::feeder::Feeder;
use cyex_tv_hub::{Hub, Message, Subscription};
use cyex_tv_shared::model::EventType;
use serde_json::json;
use sqlx::{Acquire, PgConnection};

fn new_event(topic: &str) -> NewEvent {
    NewEvent {
        event_type: EventType::SetEmoteAdded,
        topics: vec![topic.to_owned()],
        set_id: None,
        payload: json!({ "name": "x" }),
    }
}

async fn publish(conn: &mut PgConnection, topic: &str) -> i64 {
    outbox::publish(conn, &new_event(topic)).await.unwrap()
}

async fn ids(sub: &mut Subscription) -> Vec<i64> {
    let mut out = Vec::new();
    while let Ok(Some(Message::Event(d))) =
        tokio::time::timeout(Duration::from_millis(30), sub.recv()).await
    {
        out.push(d.event.id);
    }
    out
}

#[tokio::test]
async fn the_feeder_delivers_late_commits_and_passes_rolled_back_ids() {
    let Some(db) = TestDb::new().await else {
        return;
    };
    let hub = Hub::new();
    let mut sub = hub.subscriber();
    sub.subscribe(["set:a"]);
    let mut feeder = Feeder::new(0).with_gap_wait(Duration::from_millis(200));

    let mut c = db.conn().await;
    let first = publish(&mut c, "set:a").await;
    publish(&mut c, "set:other").await;
    feeder.tick(&db.pool, &hub).await.unwrap();
    assert_eq!(ids(&mut sub).await, [first]);
    assert_eq!(feeder.watermark(), first + 1);

    // A takes the next ID but commits after B, which takes the one after.
    let mut a_conn = db.pool.acquire().await.unwrap();
    let mut a = a_conn.begin().await.unwrap();
    let late = publish(&mut a, "set:a").await;
    let early = publish(&mut c, "set:a").await;
    assert!(early > late);
    feeder.tick(&db.pool, &hub).await.unwrap();
    assert_eq!(ids(&mut sub).await, [early]);
    // The watermark waits below the uncommitted ID.
    assert_eq!(feeder.watermark(), late - 1);
    a.commit().await.unwrap();
    feeder.tick(&db.pool, &hub).await.unwrap();
    assert_eq!(ids(&mut sub).await, [late]);
    assert_eq!(feeder.watermark(), early);

    // A rolled-back ID leaves a hole that is passed after the wait.
    let mut r_conn = db.pool.acquire().await.unwrap();
    let mut r = r_conn.begin().await.unwrap();
    let rolled = publish(&mut r, "set:a").await;
    r.rollback().await.unwrap();
    let after = publish(&mut c, "set:a").await;
    feeder.tick(&db.pool, &hub).await.unwrap();
    assert_eq!(ids(&mut sub).await, [after]);
    assert_eq!(feeder.watermark(), rolled - 1);
    tokio::time::sleep(Duration::from_millis(250)).await;
    feeder.tick(&db.pool, &hub).await.unwrap();
    assert_eq!(feeder.watermark(), after);
    // Nothing is delivered twice.
    feeder.tick(&db.pool, &hub).await.unwrap();
    assert!(ids(&mut sub).await.is_empty());
    drop(c);
    drop(a_conn);
    drop(r_conn);
    db.close().await;
}
