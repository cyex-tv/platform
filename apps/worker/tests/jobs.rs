//! The job queue and the worker loop against a real Postgres. Runs when
//! `CYEX_TV_TEST_DATABASE_URL` is set (CI sets it), skips otherwise.

use std::collections::HashSet;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use cyex_tv_db::jobs;
use cyex_tv_db::testing::TestDb;
use cyex_tv_worker::{Ctx, Handlers, JobError, Limits, Outcome, run_one};
use serde_json::json;
use sqlx::postgres::PgListener;

fn limits() -> Limits {
    Limits {
        lease: Duration::from_secs(60),
        timeout: Duration::from_millis(500),
        poll: Duration::from_millis(50),
    }
}

async fn enqueue(db: &TestDb, kind: &str, max_attempts: i32) -> i64 {
    let mut conn = db.conn().await;
    jobs::enqueue(&mut conn, kind, &json!({ "n": 1 }), max_attempts)
        .await
        .unwrap()
}

async fn job(db: &TestDb, id: i64) -> jobs::Job {
    let mut conn = db.conn().await;
    jobs::get(&mut conn, id).await.unwrap().unwrap()
}

/// Makes a retried job due now, instead of after its backoff.
async fn make_due(db: &TestDb, id: i64) {
    let mut conn = db.conn().await;
    sqlx::query("UPDATE jobs SET run_after = now() WHERE id = $1")
        .bind(id)
        .execute(&mut *conn)
        .await
        .unwrap();
}

#[tokio::test]
async fn jobs_run_retry_and_fail() {
    let Some(db) = TestDb::new().await else {
        return;
    };
    let ctx = Ctx {
        db: db.pool.clone(),
        store: None,
        purge: None,
    };
    let calls = Arc::new(AtomicU32::new(0));
    let seen = calls.clone();
    let handlers = Handlers::default()
        .on("ok", |_, job| async move {
            assert_eq!(job.payload["n"], 1);
            Ok(())
        })
        .on("flaky", move |_, _| {
            let calls = seen.clone();
            async move {
                // Fails twice, then works.
                if calls.fetch_add(1, Ordering::SeqCst) < 2 {
                    Err(JobError::Retry("busy".into()))
                } else {
                    Ok(())
                }
            }
        })
        .on("broken", |_, _| async {
            Err(JobError::Permanent("corrupt file".into()))
        })
        .on("slow", |_, _| async {
            tokio::time::sleep(Duration::from_secs(5)).await;
            Ok(())
        });

    assert_eq!(run_one(&ctx, &handlers, limits()).await.unwrap(), None);

    let ok = enqueue(&db, "ok", 5).await;
    assert_eq!(
        run_one(&ctx, &handlers, limits()).await.unwrap(),
        Some((ok, Outcome::Done))
    );
    assert_eq!(job(&db, ok).await.state, "done");

    let flaky = enqueue(&db, "flaky", 5).await;
    assert_eq!(
        run_one(&ctx, &handlers, limits()).await.unwrap(),
        Some((flaky, Outcome::Retrying))
    );
    let j = job(&db, flaky).await;
    assert_eq!((j.state.as_str(), j.attempts), ("queued", 1));
    assert_eq!(j.last_error.as_deref(), Some("busy"));
    // Not due yet: the backoff holds it back.
    assert!(j.run_after > chrono::Utc::now());
    assert_eq!(run_one(&ctx, &handlers, limits()).await.unwrap(), None);
    make_due(&db, flaky).await;
    assert_eq!(
        run_one(&ctx, &handlers, limits()).await.unwrap(),
        Some((flaky, Outcome::Retrying))
    );
    make_due(&db, flaky).await;
    assert_eq!(
        run_one(&ctx, &handlers, limits()).await.unwrap(),
        Some((flaky, Outcome::Done))
    );
    assert_eq!(job(&db, flaky).await.attempts, 3);

    let broken = enqueue(&db, "broken", 5).await;
    assert_eq!(
        run_one(&ctx, &handlers, limits()).await.unwrap(),
        Some((broken, Outcome::Failed))
    );
    let j = job(&db, broken).await;
    assert_eq!((j.state.as_str(), j.attempts), ("failed", 1));

    // Too slow: the time limit ends the attempt, and the last attempt fails the job.
    let slow = enqueue(&db, "slow", 1).await;
    assert_eq!(
        run_one(&ctx, &handlers, limits()).await.unwrap(),
        Some((slow, Outcome::Failed))
    );
    assert!(
        job(&db, slow)
            .await
            .last_error
            .unwrap()
            .contains("timed out")
    );

    // Kinds nobody handles stay queued.
    let other = enqueue(&db, "someone.else", 5).await;
    assert_eq!(run_one(&ctx, &handlers, limits()).await.unwrap(), None);
    assert_eq!(job(&db, other).await.state, "queued");
    db.close().await;
}

#[tokio::test]
async fn a_dead_workers_job_runs_again_after_its_lease() {
    let Some(db) = TestDb::new().await else {
        return;
    };
    let id = enqueue(&db, "ok", 5).await;
    let mut conn = db.conn().await;
    // A worker claims it and dies.
    let claimed = jobs::claim(&mut conn, &["ok".to_owned()], chrono::Duration::seconds(60))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(claimed.id, id);
    assert!(
        jobs::claim(&mut conn, &["ok".to_owned()], chrono::Duration::seconds(60))
            .await
            .unwrap()
            .is_none()
    );
    sqlx::query("UPDATE jobs SET locked_until = now() - interval '1 second' WHERE id = $1")
        .bind(id)
        .execute(&mut *conn)
        .await
        .unwrap();
    let again = jobs::claim(&mut conn, &["ok".to_owned()], chrono::Duration::seconds(60))
        .await
        .unwrap()
        .unwrap();
    assert_eq!((again.id, again.attempts), (id, 2));
    drop(conn);
    db.close().await;
}

#[tokio::test]
async fn parallel_workers_never_share_a_job() {
    let Some(db) = TestDb::new().await else {
        return;
    };
    for _ in 0..40 {
        enqueue(&db, "count", 5).await;
    }
    let ran = Arc::new(Mutex::new(Vec::new()));
    let seen = ran.clone();
    let handlers = Handlers::default().on("count", move |_, job| {
        let ran = seen.clone();
        async move {
            ran.lock().unwrap().push(job.id);
            Ok(())
        }
    });
    let ctx = Ctx {
        db: db.pool.clone(),
        store: None,
        purge: None,
    };
    let mut workers = Vec::new();
    for _ in 0..6 {
        let ctx = ctx.clone();
        let handlers = handlers.clone();
        workers.push(tokio::spawn(async move {
            while run_one(&ctx, &handlers, limits()).await.unwrap().is_some() {}
        }));
    }
    for w in workers {
        w.await.unwrap();
    }
    let ran = ran.lock().unwrap().clone();
    assert_eq!(ran.len(), 40);
    assert_eq!(ran.iter().collect::<HashSet<_>>().len(), 40);
    db.close().await;
}

#[tokio::test]
async fn new_jobs_notify_and_the_loop_runs_them() {
    let Some(db) = TestDb::new().await else {
        return;
    };
    let mut listener = PgListener::connect_with(&db.pool).await.unwrap();
    listener.listen(jobs::NOTIFY_CHANNEL).await.unwrap();
    enqueue(&db, "ok", 5).await;
    let note = tokio::time::timeout(Duration::from_secs(5), listener.recv())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(note.payload(), "ok");
    drop(listener);

    // The whole loop: it picks the job up and stops on shutdown.
    let (tx, rx) = tokio::sync::oneshot::channel::<()>();
    let done = Arc::new(AtomicU32::new(0));
    let seen = done.clone();
    let handlers = Handlers::default().on("ok", move |_, _| {
        let done = seen.clone();
        async move {
            done.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
    });
    let ctx = Ctx {
        db: db.pool.clone(),
        store: None,
        purge: None,
    };
    let worker = tokio::spawn(cyex_tv_worker::run(
        ctx,
        handlers,
        limits(),
        Duration::from_secs(3600),
        async {
            rx.await.ok();
        },
    ));
    for _ in 0..100 {
        if done.load(Ordering::SeqCst) == 1 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    assert_eq!(done.load(Ordering::SeqCst), 1);
    enqueue(&db, "ok", 5).await;
    for _ in 0..100 {
        if done.load(Ordering::SeqCst) == 2 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    assert_eq!(done.load(Ordering::SeqCst), 2);
    tx.send(()).unwrap();
    worker.await.unwrap().unwrap();
    db.close().await;
}

#[tokio::test]
async fn clean_up_removes_what_expired() {
    let Some(db) = TestDb::new().await else {
        return;
    };
    let mut conn = db.conn().await;
    let account = cyex_tv_db::accounts::sign_in(
        &mut conn,
        cyex_tv_db::accounts::Profile {
            platform: cyex_tv_shared::model::Platform::Twitch,
            platform_user_id: "1",
            login: "a",
            display_name: "a",
            avatar: None,
        },
    )
    .await
    .unwrap()
    .account;
    for (hash, ttl) in [
        ([1u8; 32], chrono::Duration::days(1)),
        ([2u8; 32], chrono::Duration::days(1)),
    ] {
        cyex_tv_db::sessions::create(
            &mut conn,
            &hash,
            account.id,
            cyex_tv_db::sessions::SessionKind::Site,
            ttl,
        )
        .await
        .unwrap();
    }
    sqlx::query("UPDATE sessions SET created_at = now() - interval '2 days', last_seen_at = now() - interval '2 days', expires_at = now() - interval '1 second' WHERE id_hash = $1")
        .bind(&[1u8; 32][..])
        .execute(&mut *conn)
        .await
        .unwrap();
    let done = jobs::enqueue(&mut conn, "ok", &json!({}), 1).await.unwrap();
    jobs::complete(&mut conn, done).await.unwrap();
    sqlx::query("UPDATE jobs SET updated_at = now() - interval '8 days' WHERE id = $1")
        .bind(done)
        .execute(&mut *conn)
        .await
        .unwrap();
    let recent = jobs::enqueue(&mut conn, "ok", &json!({}), 1).await.unwrap();
    jobs::complete(&mut conn, recent).await.unwrap();
    drop(conn);

    cyex_tv_worker::clean_up(&db.pool).await.unwrap();
    let mut conn = db.conn().await;
    let sessions: i64 = sqlx::query_scalar("SELECT count(*) FROM sessions")
        .fetch_one(&mut *conn)
        .await
        .unwrap();
    assert_eq!(sessions, 1);
    assert!(jobs::get(&mut conn, done).await.unwrap().is_none());
    assert!(jobs::get(&mut conn, recent).await.unwrap().is_some());
    drop(conn);
    db.close().await;
}
