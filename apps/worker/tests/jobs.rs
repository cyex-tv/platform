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
        paddle: None,
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
        paddle: None,
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
        paddle: None,
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

#[tokio::test]
async fn outcomes_and_the_queue_are_measured() {
    let Some(db) = TestDb::new().await else {
        return;
    };
    let ctx = Ctx {
        db: db.pool.clone(),
        store: None,
        purge: None,
        paddle: None,
    };
    let registry = cyex_tv_shared::metrics::Registry::new();
    let handlers = Handlers::default()
        .on("ok", |_, _| async { Ok(()) })
        .on("broken", |_, _| async {
            Err(JobError::Permanent("corrupt file".into()))
        })
        .with_metrics(registry.clone());

    enqueue(&db, "ok", 5).await;
    enqueue(&db, "broken", 5).await;
    enqueue(&db, "ok", 5).await;
    // Due, but no handler here: it stays queued and shows as due.
    enqueue(&db, "elsewhere", 5).await;
    let before = cyex_tv_worker::queue_metrics(&db.pool).await;
    assert!(before.contains("cyex_tv_metrics_db_up 1\n"), "{before}");
    assert!(
        before.contains(r#"cyex_tv_jobs{kind="ok",state="due"} 2"#),
        "{before}"
    );

    while run_one(&ctx, &handlers, limits()).await.unwrap().is_some() {}

    let text = registry.render();
    assert!(
        text.contains(r#"cyex_tv_jobs_finished_total{kind="ok",outcome="done"} 2"#),
        "{text}"
    );
    assert!(
        text.contains(r#"cyex_tv_jobs_finished_total{kind="broken",outcome="failed"} 1"#),
        "{text}"
    );
    assert!(
        text.contains(r#"cyex_tv_job_duration_seconds_count{kind="ok",outcome="done"} 2"#),
        "{text}"
    );

    let after = cyex_tv_worker::queue_metrics(&db.pool).await;
    assert!(
        !after.contains(r#"kind="ok""#),
        "done jobs aren't counted: {after}"
    );
    assert!(
        after.contains(r#"cyex_tv_jobs{kind="broken",state="failed"} 1"#),
        "{after}"
    );
    assert!(
        after.contains(r#"cyex_tv_jobs{kind="elsewhere",state="due"} 1"#),
        "{after}"
    );
    db.close().await;
}

#[tokio::test]
async fn clean_up_expires_old_pending_orders_and_old_paddle_events() {
    use cyex_tv_db::shop;
    let Some(db) = TestDb::new().await else {
        return;
    };
    let mut conn = db.conn().await;
    let buyer = cyex_tv_db::accounts::sign_in(
        &mut conn,
        cyex_tv_db::accounts::Profile {
            platform: cyex_tv_shared::model::Platform::Twitch,
            platform_user_id: "7",
            login: "buyer",
            display_name: "buyer",
            avatar: None,
        },
    )
    .await
    .unwrap()
    .account;
    let fields = shop::ItemFields {
        name: "Pack",
        description: "",
        price_cents: 100,
        currency: "EUR",
        paddle_price_id: Some("pri_01x"),
        active_from: None,
        active_to: None,
        stock_limit: None,
        visible: true,
        sort_order: 0,
    };
    let item = shop::create_item(&mut conn, &fields, &[], None)
        .await
        .unwrap();
    let mut other = fields.clone();
    other.name = "Other";
    let item2 = shop::create_item(&mut conn, &other, &[], None)
        .await
        .unwrap();
    let old = shop::create_order(&mut conn, buyer.id, buyer.id, &item, &[])
        .await
        .unwrap();
    let fresh = shop::create_order(&mut conn, buyer.id, buyer.id, &item2, &[])
        .await
        .unwrap();
    sqlx::query("UPDATE shop_orders SET created_at = now() - interval '25 hours' WHERE id = $1")
        .bind(old.id)
        .execute(&mut *conn)
        .await
        .unwrap();
    for (id, age) in [("evt_old", "91 days"), ("evt_new", "1 day")] {
        sqlx::query(&format!(
            "INSERT INTO paddle_events (event_id, event_type, received_at)
             VALUES ($1, 'transaction.completed', now() - interval '{age}')"
        ))
        .bind(id)
        .execute(&mut *conn)
        .await
        .unwrap();
    }
    drop(conn);

    cyex_tv_worker::clean_up(&db.pool).await.unwrap();
    let mut conn = db.conn().await;
    let old = shop::get_order(&mut conn, old.id).await.unwrap().unwrap();
    assert_eq!(
        (old.status.as_str(), old.failed_reason.as_deref()),
        ("failed", Some("expired"))
    );
    // An expired order still takes a late payment.
    assert!(old.takes_payment());
    let fresh = shop::get_order(&mut conn, fresh.id).await.unwrap().unwrap();
    assert_eq!(fresh.status, "pending");
    let audited: i64 =
        sqlx::query_scalar("SELECT count(*) FROM audit_log WHERE action = 'shop.order_expired'")
            .fetch_one(&mut *conn)
            .await
            .unwrap();
    assert_eq!(audited, 1);
    let events: Vec<String> =
        sqlx::query_scalar("SELECT event_id FROM paddle_events ORDER BY event_id")
            .fetch_all(&mut *conn)
            .await
            .unwrap();
    assert_eq!(events, ["evt_new"]);
    drop(conn);
    db.close().await;
}

/// A buyer and a priced item, for the shop tests below.
async fn shop_buyer_and_item(
    conn: &mut sqlx::PgConnection,
    twitch_id: &str,
) -> (uuid::Uuid, cyex_tv_db::shop::ItemRow) {
    use cyex_tv_db::shop;
    let login = format!("buyer{twitch_id}");
    let buyer = cyex_tv_db::accounts::sign_in(
        conn,
        cyex_tv_db::accounts::Profile {
            platform: cyex_tv_shared::model::Platform::Twitch,
            platform_user_id: twitch_id,
            login: &login,
            display_name: &login,
            avatar: None,
        },
    )
    .await
    .unwrap()
    .account;
    let fields = shop::ItemFields {
        name: "Pack",
        description: "",
        price_cents: 100,
        currency: "EUR",
        paddle_price_id: Some("pri_01x"),
        active_from: None,
        active_to: None,
        stock_limit: None,
        visible: true,
        sort_order: 0,
    };
    let item = shop::create_item(conn, &fields, &[], None).await.unwrap();
    (buyer.id, item)
}

#[tokio::test]
async fn clean_up_puts_stuck_refund_requests_back_to_paid() {
    use cyex_tv_db::shop;
    let Some(db) = TestDb::new().await else {
        return;
    };
    let mut conn = db.conn().await;
    let (buyer, item) = shop_buyer_and_item(&mut conn, "21").await;
    let stuck = shop::create_order(&mut conn, buyer, buyer, &item, &[])
        .await
        .unwrap();
    sqlx::query("UPDATE shop_orders SET status = 'failed' WHERE id = $1")
        .bind(stuck.id)
        .execute(&mut *conn)
        .await
        .unwrap();
    let recent = shop::create_order(&mut conn, buyer, buyer, &item, &[])
        .await
        .unwrap();
    for (id, age) in [(stuck.id, "61 minutes"), (recent.id, "5 minutes")] {
        sqlx::query(&format!(
            "UPDATE shop_orders SET status = 'refund_requested', paid_at = now(),
                    refund_reason = 'asked', refund_requested_at = now() - interval '{age}'
              WHERE id = $1"
        ))
        .bind(id)
        .execute(&mut *conn)
        .await
        .unwrap();
    }
    drop(conn);

    cyex_tv_worker::clean_up(&db.pool).await.unwrap();
    let mut conn = db.conn().await;
    let stuck = shop::get_order(&mut conn, stuck.id).await.unwrap().unwrap();
    assert_eq!(stuck.status, "paid");
    assert_eq!(stuck.refund_requested_at, None);
    let recent = shop::get_order(&mut conn, recent.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(recent.status, "refund_requested");
    let audited: Vec<String> = sqlx::query_scalar(
        "SELECT target_id FROM audit_log WHERE action = 'shop.refund_request_reverted'",
    )
    .fetch_all(&mut *conn)
    .await
    .unwrap();
    assert_eq!(audited, [cyex_tv_db::ids::to_text(stuck.id)]);
    drop(conn);
    db.close().await;
}

#[tokio::test]
async fn stale_checkouts_are_cancelled_at_paddle_and_failures_audited() {
    use axum::extract::{Path, State};
    use axum::routing::patch;
    use cyex_tv_db::shop;
    use cyex_tv_paddle::{Paddle, PaddleConfig, PaddleEnv};
    let Some(db) = TestDb::new().await else {
        return;
    };
    // A fake Paddle: cancels txn_open, refuses txn_paid (it was paid meanwhile).
    let calls: Arc<Mutex<Vec<String>>> = Arc::default();
    let app = axum::Router::new()
        .route(
            "/transactions/{id}",
            patch(
                |State(calls): State<Arc<Mutex<Vec<String>>>>,
                 Path(id): Path<String>,
                 axum::Json(body): axum::Json<serde_json::Value>| async move {
                    calls.lock().unwrap().push(id.clone());
                    assert_eq!(body, json!({ "status": "canceled" }));
                    if id == "txn_paid" {
                        (
                            axum::http::StatusCode::BAD_REQUEST,
                            axum::Json(json!({ "error": { "code": "transaction_immutable" } })),
                        )
                    } else {
                        (
                            axum::http::StatusCode::OK,
                            axum::Json(json!({ "data": { "id": id, "status": "canceled" } })),
                        )
                    }
                },
            ),
        )
        .with_state(calls.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    let paddle = Paddle::new(PaddleConfig {
        env: PaddleEnv::Sandbox,
        api_key: Some("pdl_sdbx_apikey_test_only".into()),
        webhook_secret: None,
        client_token: None,
        api_url: format!("http://{addr}"),
    })
    .unwrap();

    let mut conn = db.conn().await;
    let (buyer, item) = shop_buyer_and_item(&mut conn, "22").await;
    let mut orders = Vec::new();
    for (txn, reason) in [
        ("txn_open", "expired"),
        ("txn_paid", "replaced"),
        ("txn_gone", "canceled"),
    ] {
        let o = shop::create_order(&mut conn, buyer, buyer, &item, &[])
            .await
            .unwrap();
        sqlx::query(
            "UPDATE shop_orders SET status = 'failed', failed_reason = $2,
                    provider_transaction_id = $3 WHERE id = $1",
        )
        .bind(o.id)
        .bind(reason)
        .bind(txn)
        .execute(&mut *conn)
        .await
        .unwrap();
        orders.push(o.id);
    }
    drop(conn);

    let n = cyex_tv_worker::cancel_stale_checkouts(&db.pool, &paddle)
        .await
        .unwrap();
    assert_eq!(n, 1);
    let mut seen = calls.lock().unwrap().clone();
    seen.sort();
    // Cancelled at Paddle already ("canceled") isn't asked.
    assert_eq!(seen, ["txn_open", "txn_paid"]);
    let mut conn = db.conn().await;
    let open = shop::get_order(&mut conn, orders[0])
        .await
        .unwrap()
        .unwrap();
    assert!(open.provider_canceled_at.is_some());
    assert!(
        !open.takes_payment(),
        "a cancelled checkout takes no payment"
    );
    let paid = shop::get_order(&mut conn, orders[1])
        .await
        .unwrap()
        .unwrap();
    assert!(paid.cancel_failed_at.is_some());
    assert!(
        paid.takes_payment(),
        "a cancel that failed still takes the payment"
    );
    let audited: Vec<String> = sqlx::query_scalar(
        "SELECT target_id FROM audit_log WHERE action = 'shop.transaction_cancel_failed'",
    )
    .fetch_all(&mut *conn)
    .await
    .unwrap();
    assert_eq!(audited, [cyex_tv_db::ids::to_text(paid.id)]);
    drop(conn);

    // Nothing is tried twice.
    let n = cyex_tv_worker::cancel_stale_checkouts(&db.pool, &paddle)
        .await
        .unwrap();
    assert_eq!(n, 0);
    assert_eq!(calls.lock().unwrap().len(), 2);
    db.close().await;
}
