//! `webhook.deliver` (ADR 0034) against a real Postgres and a local receiver: the outbox trigger
//! makes deliveries for the events a webhook asks for, the worker signs and sends them, retries
//! failures, and turns a webhook off after 5 deliveries in a row fail for good. Runs when
//! `CYEX_TV_TEST_DATABASE_URL` is set (CI sets it), skips otherwise.

use std::sync::{Arc, Mutex};

use axum::Router;
use axum::body::Bytes;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::routing::post;
use cyex_tv_db::accounts::{self, Profile};
use cyex_tv_db::outbox::{self, NewEvent};
use cyex_tv_db::testing::TestDb;
use cyex_tv_db::{channels, jobs, notifications, webhooks};
use cyex_tv_shared::model::{EventType, Platform};
use cyex_tv_shared::webhooks::verify;
use cyex_tv_worker::webhooks::{Policy, deliver_with};
use cyex_tv_worker::{Ctx, JobError};
use serde_json::{Value, json};
use uuid::Uuid;

const SECRET: &str = "whsec_0123456789abcdef0123456789abcdef";

#[derive(Default)]
struct Seen {
    calls: Vec<(HeaderMap, Bytes)>,
    status: u16,
}

type Shared = Arc<Mutex<Seen>>;

async fn receiver(seen: Shared) -> u16 {
    let app = Router::new()
        .route(
            "/hook",
            post(
                |State(s): State<Shared>, headers: HeaderMap, body: Bytes| async move {
                    let mut s = s.lock().unwrap();
                    s.calls.push((headers, body));
                    StatusCode::from_u16(s.status).unwrap()
                },
            ),
        )
        .route(
            "/moved",
            post(|| async { (StatusCode::FOUND, [("location", "http://169.254.169.254/")]) }),
        )
        .with_state(seen);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    port
}

struct World {
    db: TestDb,
    ctx: Ctx,
    owner: Uuid,
    hook: Uuid,
    topic: String,
}

async fn world(port: u16, path: &str) -> Option<World> {
    let db = TestDb::new().await?;
    let _ = rustls::crypto::ring::default_provider().install_default();
    let mut conn = db.conn().await;
    let owner = accounts::sign_in(
        &mut conn,
        Profile {
            platform: Platform::Twitch,
            platform_user_id: "4242",
            login: "streamer",
            display_name: "Streamer",
            avatar: None,
        },
    )
    .await
    .unwrap()
    .account
    .id;
    let ch = channels::upsert(&mut conn, owner, Platform::Twitch, "4242", "streamer")
        .await
        .unwrap();
    let topic = outbox::channel_topic(Platform::Twitch, "4242");
    let hook = webhooks::create(
        &mut conn,
        ch.id,
        &topic,
        &format!("https://127.0.0.1:{port}{path}"),
        &["set.emote_added".to_owned()],
        SECRET,
        owner,
        5,
    )
    .await
    .unwrap()
    .unwrap();
    drop(conn);
    let ctx = Ctx {
        db: db.pool.clone(),
        store: None,
        purge: None,
        paddle: None,
    };
    Some(World {
        db,
        ctx,
        owner,
        hook: hook.id,
        topic,
    })
}

impl World {
    async fn publish(&self, event_type: EventType, topic: &str) -> i64 {
        let mut conn = self.db.conn().await;
        outbox::publish(
            &mut conn,
            &NewEvent {
                event_type,
                topics: vec![topic.to_owned()],
                set_id: None,
                payload: json!({ "emote_id": "1", "name": "KEKW" }),
            },
        )
        .await
        .unwrap()
    }

    /// The queued `webhook.deliver` jobs, oldest first.
    async fn jobs(&self) -> Vec<jobs::Job> {
        let mut conn = self.db.conn().await;
        let ids: Vec<i64> = sqlx::query_scalar(
            "SELECT id FROM jobs WHERE kind = 'webhook.deliver' AND state = 'queued' ORDER BY id",
        )
        .fetch_all(&mut *conn)
        .await
        .unwrap();
        let mut out = Vec::new();
        for id in ids {
            out.push(jobs::get(&mut conn, id).await.unwrap().unwrap());
        }
        out
    }

    async fn deliveries(&self) -> Vec<webhooks::Delivery> {
        let mut conn = self.db.conn().await;
        webhooks::deliveries(&mut conn, self.hook, 100)
            .await
            .unwrap()
    }

    async fn webhook(&self) -> webhooks::Webhook {
        let mut conn = self.db.conn().await;
        webhooks::get(&mut conn, self.hook).await.unwrap().unwrap()
    }
}

/// A job as the queue hands it over on its `attempt`th try.
fn attempt(mut job: jobs::Job, attempt: i32) -> jobs::Job {
    job.attempts = attempt;
    job
}

#[tokio::test]
async fn events_are_delivered_signed() {
    let seen = Shared::default();
    seen.lock().unwrap().status = 204;
    let port = receiver(seen.clone()).await;
    let Some(w) = world(port, "/hook").await else {
        return;
    };
    let event = w.publish(EventType::SetEmoteAdded, &w.topic).await;
    // Not asked for, or another channel's: no delivery.
    w.publish(EventType::SetEmoteRemoved, &w.topic).await;
    w.publish(EventType::SetEmoteAdded, "channel:twitch:9999")
        .await;
    let queued = w.jobs().await;
    assert_eq!(queued.len(), 1, "one delivery for one matching event");
    assert_eq!(queued[0].max_attempts, webhooks::DELIVER_ATTEMPTS);

    deliver_with(
        w.ctx.clone(),
        attempt(queued[0].clone(), 1),
        Policy::loopback_for_tests(),
    )
    .await
    .unwrap();
    let calls = std::mem::take(&mut seen.lock().unwrap().calls);
    assert_eq!(calls.len(), 1);
    let (headers, body) = &calls[0];
    let sig = headers["x-cyex-signature"].to_str().unwrap();
    let now = chrono::Utc::now().timestamp();
    assert!(
        verify(SECRET, sig, body, now, 300),
        "the signature checks out"
    );
    assert!(!verify(
        "whsec_wrong_wrong_wrong_wrong_wrong_wrong",
        sig,
        body,
        now,
        300
    ));
    assert_eq!(headers["x-cyex-event"], "set.emote_added");
    assert_eq!(headers["content-type"], "application/json");
    let body: Value = serde_json::from_slice(body).unwrap();
    assert_eq!(body["event"]["type"], "set.emote_added");
    assert_eq!(body["event"]["id"], event.to_string());
    assert_eq!(body["event"]["data"]["name"], "KEKW");
    assert_eq!(
        body["channel"],
        json!({ "platform": "twitch", "id": "4242", "login": "streamer" })
    );
    assert_eq!(
        headers["x-cyex-delivery"].to_str().unwrap(),
        body["delivery_id"].as_str().unwrap()
    );

    let d = &w.deliveries().await[0];
    assert_eq!(d.state, "delivered");
    assert_eq!(d.response_status, Some(204));
    assert_eq!(d.attempts, 1);
    assert!(d.finished_at.is_some());

    // A delivery already done isn't sent again (a job run twice).
    deliver_with(
        w.ctx.clone(),
        attempt(queued[0].clone(), 2),
        Policy::loopback_for_tests(),
    )
    .await
    .unwrap();
    assert!(seen.lock().unwrap().calls.is_empty());

    // Turned off: nothing new is queued.
    let mut conn = w.db.conn().await;
    sqlx::query("UPDATE jobs SET state = 'done' WHERE kind = 'webhook.deliver'")
        .execute(&mut *conn)
        .await
        .unwrap();
    webhooks::update(&mut conn, w.hook, None, None, Some(false))
        .await
        .unwrap();
    drop(conn);
    w.publish(EventType::SetEmoteAdded, &w.topic).await;
    assert!(w.jobs().await.is_empty());
    w.db.close().await;
}

#[tokio::test]
async fn private_addresses_and_redirects_are_not_followed() {
    let seen = Shared::default();
    seen.lock().unwrap().status = 204;
    let port = receiver(seen.clone()).await;
    let Some(w) = world(port, "/moved").await else {
        return;
    };
    w.publish(EventType::SetEmoteAdded, &w.topic).await;
    let job = w.jobs().await.remove(0);
    // A redirect is a failure, never followed.
    let err = deliver_with(
        w.ctx.clone(),
        attempt(job.clone(), 1),
        Policy::loopback_for_tests(),
    )
    .await
    .unwrap_err();
    assert!(
        matches!(&err, JobError::Retry(e) if e.contains("redirects are not followed")),
        "{err}"
    );
    // The real policy refuses loopback before connecting.
    let err = deliver_with(w.ctx.clone(), attempt(job, 2), Policy::default())
        .await
        .unwrap_err();
    assert!(
        matches!(&err, JobError::Retry(e) if e.contains("isn't allowed")),
        "{err}"
    );
    let d = &w.deliveries().await[0];
    assert_eq!(d.state, "pending");
    assert_eq!(d.attempts, 2);
    assert!(d.error.as_deref().unwrap().contains("isn't allowed"));
    w.db.close().await;
}

#[tokio::test]
async fn failures_retry_then_turn_the_webhook_off() {
    let seen = Shared::default();
    seen.lock().unwrap().status = 500;
    let port = receiver(seen.clone()).await;
    let Some(w) = world(port, "/hook").await else {
        return;
    };
    let policy = Policy::loopback_for_tests();
    for n in 1..=webhooks::DISABLE_AFTER {
        w.publish(EventType::SetEmoteAdded, &w.topic).await;
        let job = w.jobs().await.pop().unwrap();
        // Not the last attempt: tried again later.
        let err = deliver_with(w.ctx.clone(), attempt(job.clone(), 1), policy)
            .await
            .unwrap_err();
        assert!(
            matches!(err, JobError::Retry(ref e) if e == "answered 500"),
            "{err}"
        );
        assert_eq!(w.deliveries().await[0].state, "pending");
        // The last one: failed for good, counted.
        deliver_with(
            w.ctx.clone(),
            attempt(job, webhooks::DELIVER_ATTEMPTS),
            policy,
        )
        .await
        .unwrap();
        let mut conn = w.db.conn().await;
        sqlx::query("UPDATE jobs SET state = 'done' WHERE kind = 'webhook.deliver'")
            .execute(&mut *conn)
            .await
            .unwrap();
        drop(conn);
        let d = &w.deliveries().await[0];
        assert_eq!((d.state.as_str(), d.response_status), ("failed", Some(500)));
        let hook = w.webhook().await;
        assert_eq!(hook.failed_deliveries, n);
        assert_eq!(hook.enabled, n < webhooks::DISABLE_AFTER, "after {n}");
    }
    let hook = w.webhook().await;
    assert_eq!(hook.disabled_reason.as_deref(), Some("failures"));
    let mut conn = w.db.conn().await;
    let told = notifications::all_for(&mut conn, w.owner).await.unwrap();
    assert_eq!(
        told.iter().filter(|n| n.kind == "webhook.disabled").count(),
        1,
        "the owner is told once"
    );

    // A test delivery is sent once even while it is off, and never counts.
    seen.lock().unwrap().status = 200;
    webhooks::queue_test(&mut conn, w.hook, &json!({ "type": "webhook.test" }))
        .await
        .unwrap();
    drop(conn);
    let job = w.jobs().await.pop().unwrap();
    assert_eq!(job.max_attempts, 1);
    deliver_with(w.ctx.clone(), attempt(job, 1), policy)
        .await
        .unwrap();
    let d = &w.deliveries().await[0];
    assert_eq!(
        (d.event_type.as_str(), d.state.as_str()),
        ("webhook.test", "delivered")
    );
    assert_eq!(w.webhook().await.failed_deliveries, webhooks::DISABLE_AFTER);

    // Turned on again: the count starts over.
    let mut conn = w.db.conn().await;
    let on = webhooks::update(&mut conn, w.hook, None, None, Some(true))
        .await
        .unwrap()
        .unwrap();
    assert!(on.enabled && on.failed_deliveries == 0 && on.disabled_reason.is_none());
    drop(conn);
    w.db.close().await;
}

#[tokio::test]
async fn the_log_keeps_the_latest_fifty() {
    let seen = Shared::default();
    seen.lock().unwrap().status = 200;
    let port = receiver(seen.clone()).await;
    let Some(w) = world(port, "/hook").await else {
        return;
    };
    for _ in 0..55 {
        w.publish(EventType::SetEmoteAdded, &w.topic).await;
    }
    for job in w.jobs().await {
        deliver_with(w.ctx.clone(), attempt(job, 1), Policy::loopback_for_tests())
            .await
            .unwrap();
    }
    let kept = w.deliveries().await;
    assert_eq!(kept.len(), 50);
    assert!(kept.iter().all(|d| d.state == "delivered"));
    w.db.close().await;
}
