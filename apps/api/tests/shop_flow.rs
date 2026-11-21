//! The cosmetics shop and redeem codes (ADR 0016) end to end through the real router against a
//! real Postgres, with a fake Paddle API on a local port: item CRUD and permissions, checkout
//! unavailable without Paddle, the webhook's signature (valid, invalid, replayed, stale),
//! idempotent fulfilment, gifts, owning already, stock, refunds that revoke and clear the
//! selection, and redeem codes (happy path, wrong, expired, used up, disabled, lockouts, hashing).
//! Runs when `CYEX_TV_TEST_DATABASE_URL` is set (CI sets it) and skips otherwise. Sandbox shapes
//! only; no real money and no real Paddle.

mod common;

use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use axum::Router;
use axum::body::Body;
use axum::extract::ConnectInfo;
use axum::http::{Request, StatusCode};
use chrono::{Duration, Utc};
use common::TestDb;
use cyex_tv_api::paddle::{Paddle, PaddleConfig, PaddleEnv, sign};
use cyex_tv_api::state::AppState;
use cyex_tv_auth::{Auth, AuthConfig, tokens};
use cyex_tv_db::accounts::{self, Profile};
use cyex_tv_db::cosmetics::{self, NewImage, PaintFields};
use cyex_tv_db::roles::{self, NewRole};
use cyex_tv_db::sessions::{self, SessionKind};
use cyex_tv_db::{ids, outbox};
use cyex_tv_shared::model::{CosmeticKind, Platform};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use tower::ServiceExt;
use uuid::Uuid;

/// The owner's Twitch ID in this test (`CYEX_TV_OWNER_IDS`); a made-up number.
const OWNER: &str = "900";
const SECRET: &str = "pdl_ntfset_test_only_secret";
const PEER: SocketAddr = SocketAddr::new(
    std::net::IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 1)),
    40000,
);

// A fake Paddle API: answers transactions and adjustments, remembers what it was sent.

#[derive(Default)]
struct Fake {
    calls: Mutex<Vec<(String, Value)>>,
    fail: AtomicBool,
    /// The error code a failing answer carries (default `bad_request`).
    fail_code: Mutex<Option<String>>,
    /// How long each answer waits, in milliseconds.
    delay_ms: AtomicU64,
    next: AtomicUsize,
    /// Transactions asked to be cancelled (`PATCH /transactions/{id}`), in order.
    cancels: Mutex<Vec<String>>,
    /// Cancels are refused (the transaction was paid meanwhile).
    cancel_fail: AtomicBool,
    /// Transactions come with a checkout link on a host we don't hand out.
    bad_url: AtomicBool,
}

async fn fake_paddle() -> (String, Arc<Fake>) {
    use axum::extract::State;
    use axum::routing::post;
    let fake = Arc::new(Fake::default());
    async fn answer(fake: &Fake, path: &str, body: Value) -> (StatusCode, axum::Json<Value>) {
        fake.calls.lock().unwrap().push((path.to_owned(), body));
        let delay = fake.delay_ms.load(Ordering::SeqCst);
        if delay > 0 {
            tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
        }
        if fake.fail.load(Ordering::SeqCst) {
            let code = fake
                .fail_code
                .lock()
                .unwrap()
                .clone()
                .unwrap_or_else(|| "bad_request".into());
            return (
                StatusCode::BAD_REQUEST,
                axum::Json(json!({ "error": { "code": code, "type": "request_error" } })),
            );
        }
        let n = fake.next.fetch_add(1, Ordering::SeqCst);
        let data = if path == "/transactions" {
            let url = if fake.bad_url.load(Ordering::SeqCst) {
                format!("https://pay.example.test/checkout?_ptxn=txn_test{n}")
            } else {
                format!("https://sandbox-buy.paddle.com/checkout?_ptxn=txn_test{n}")
            };
            json!({ "id": format!("txn_test{n}"), "status": "ready", "currency_code": "EUR",
                    "checkout": { "url": url } })
        } else {
            json!({ "id": format!("adj_test{n}"), "status": "pending_approval", "action": "refund" })
        };
        (StatusCode::CREATED, axum::Json(json!({ "data": data })))
    }
    let app = Router::new()
        .route(
            "/transactions",
            post(
                |State(f): State<Arc<Fake>>, axum::Json(b): axum::Json<Value>| async move {
                    answer(&f, "/transactions", b).await
                },
            ),
        )
        .route(
            "/transactions/{id}",
            axum::routing::patch(
                |State(f): State<Arc<Fake>>,
                 axum::extract::Path(id): axum::extract::Path<String>,
                 axum::Json(b): axum::Json<Value>| async move {
                    assert_eq!(b, json!({ "status": "canceled" }));
                    f.cancels.lock().unwrap().push(id.clone());
                    if f.cancel_fail.load(Ordering::SeqCst) {
                        return (
                            StatusCode::BAD_REQUEST,
                            axum::Json(json!({ "error": { "code": "transaction_immutable" } })),
                        );
                    }
                    (
                        StatusCode::OK,
                        axum::Json(json!({ "data": { "id": id, "status": "canceled" } })),
                    )
                },
            ),
        )
        .route(
            "/adjustments",
            post(
                |State(f): State<Arc<Fake>>, axum::Json(b): axum::Json<Value>| async move {
                    answer(&f, "/adjustments", b).await
                },
            ),
        )
        .with_state(fake.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    (format!("http://{addr}"), fake)
}

struct User {
    id: Uuid,
    token: String,
}

impl User {
    fn tid(&self) -> String {
        ids::to_text(self.id)
    }
}

struct World {
    db: TestDb,
    app: Router,
    fake: Arc<Fake>,
}

fn code(response: &Value) -> &str {
    response["errors"][0]["extensions"]["code"]
        .as_str()
        .unwrap_or_default()
}

const CREATE_ITEM: &str = "mutation($input: ShopItemInput!) {
    createShopItem(input: $input) { id name priceCents currency paddlePriceId visible available owned stockLimit stockLeft badges { id } paints { id } }
}";
const UPDATE_ITEM: &str = "mutation($id: String!, $input: ShopItemInput!) {
    updateShopItem(id: $id, input: $input) { id name priceCents visible }
}";
const DELETE_ITEM: &str = "mutation($id: String!) { deleteShopItem(id: $id) }";
const ITEMS: &str =
    "query($all: Boolean) { shopItems(all: $all) { id name available owned visible stockLeft } }";
const CHECKOUT: &str = "mutation($itemId: String!, $recipient: RecipientInput) {
    startCheckout(itemId: $itemId, recipient: $recipient) {
        transactionId checkoutUrl clientToken environment
        order { id status gift amountCents currency providerTransactionId buyer { id } recipient { id } }
    }
}";
const ORDER: &str =
    "query($id: String!) { order(id: $id) { id status paidTotalCents paidCurrency refundReason } }";
const MY_ORDERS: &str = "{ myOrders { total items { id status gift } } }";
const ORDERS: &str =
    "query($status: OrderStatus) { orders(status: $status) { total items { id status } } }";
const REFUND: &str = "mutation($id: String!) { refundOrder(id: $id, reason: \"asked by the buyer\") { id status refundReason } }";
const MINE: &str = "{ myCosmetics { badgeId paintId badges { id } paints { id } } }";
const SELECT_PAINT: &str = "mutation($p: String) { selectCosmetics(paintId: $p) { paintId } }";
const CREATE_BATCH: &str = "mutation($input: CodeBatchInput!) {
    createCodeBatch(input: $input) { codes batch { id name prefix codeCount usesPerCode redemptions itemId badgeIds paintIds disabledAt } }
}";
const DISABLE_BATCH: &str =
    "mutation($id: String!) { disableCodeBatch(id: $id) { id disabledAt } }";
const BATCHES: &str = "{ codeBatches { total items { id redemptions } } }";
const REDEEM: &str =
    "mutation($code: String!) { redeemCode(code: $code) { name badges { id } paints { id } } }";

impl World {
    async fn new(with_paddle: bool) -> Option<Self> {
        let db = TestDb::new().await?;
        let cfg = AuthConfig::from_source(|name| {
            (name == "CYEX_TV_OWNER_IDS").then(|| format!("twitch:{OWNER}"))
        })
        .unwrap();
        let (url, fake) = fake_paddle().await;
        let mut state = AppState::new(db.pool.clone(), Arc::new(Auth::new(cfg).unwrap()))
            .with_cdn_url("https://cdn.example.test");
        if with_paddle {
            state = state.with_paddle(
                Paddle::new(PaddleConfig {
                    env: PaddleEnv::Sandbox,
                    api_key: Some("pdl_sdbx_apikey_test_only".into()),
                    webhook_secret: Some(SECRET.into()),
                    client_token: Some("test_client_token".into()),
                    api_url: url,
                })
                .unwrap(),
            );
        }
        Some(Self {
            app: cyex_tv_api::app(state),
            db,
            fake,
        })
    }

    async fn user(&self, twitch_id: &str) -> User {
        let mut conn = self.db.conn().await;
        let login = format!("user{twitch_id}");
        let signed = accounts::sign_in(
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
        .unwrap();
        let token = tokens::random_token();
        sessions::create(
            &mut conn,
            &tokens::hash(&token),
            signed.account.id,
            SessionKind::Site,
            Duration::days(1),
        )
        .await
        .unwrap();
        User {
            id: signed.account.id,
            token,
        }
    }

    async fn staff(&self, twitch_id: &str, perms: &[&str]) -> User {
        let user = self.user(twitch_id).await;
        let mut conn = self.db.conn().await;
        let role = roles::create(
            &mut conn,
            &NewRole {
                name: &format!("role{twitch_id}"),
                description: None,
                position: 10,
                permissions: perms,
                created_by: None,
            },
        )
        .await
        .unwrap();
        roles::assign(&mut conn, user.id, role.id, None)
            .await
            .unwrap();
        user
    }

    /// A GraphQL request on a website session of `user` (the cookie and the site's Origin), for
    /// what only the website may do.
    async fn site_gql(&self, user: &User, query: &str) -> Value {
        let mut conn = self.db.conn().await;
        let token = tokens::random_token();
        sessions::create(
            &mut conn,
            &tokens::hash(&token),
            user.id,
            SessionKind::Site,
            Duration::days(1),
        )
        .await
        .unwrap();
        drop(conn);
        let mut request = Request::post("/v1/gql")
            .header("content-type", "application/json")
            .header("cookie", format!("cyex_tv_session={token}"))
            .header("origin", "https://cyex.tv")
            .body(Body::from(json!({ "query": query }).to_string()))
            .unwrap();
        request.extensions_mut().insert(ConnectInfo(PEER));
        let response = self.app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        serde_json::from_slice(&bytes).unwrap()
    }

    /// A GraphQL request from 127.0.0.1 (every request of a test comes from the same peer).
    async fn gql(&self, user: Option<&User>, query: &str, variables: Value) -> Value {
        self.gql_from(Some(PEER), user, query, variables).await
    }

    /// A GraphQL request from `peer`, or with no peer address at all.
    async fn gql_from(
        &self,
        peer: Option<SocketAddr>,
        user: Option<&User>,
        query: &str,
        variables: Value,
    ) -> Value {
        let mut request = Request::post("/v1/gql").header("content-type", "application/json");
        if let Some(user) = user {
            request = request
                .header(
                    "cookie",
                    format!("{}={}", cyex_tv_auth::cookies::SESSION, user.token),
                )
                .header("origin", "https://cyex.tv");
        }
        let mut request = request
            .body(Body::from(
                json!({ "query": query, "variables": variables }).to_string(),
            ))
            .unwrap();
        if let Some(peer) = peer {
            request.extensions_mut().insert(ConnectInfo(peer));
        }
        let response = self.app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        serde_json::from_slice(&bytes).unwrap()
    }

    /// Posts a webhook body with the given signature header.
    async fn hook_raw(&self, body: &str, signature: Option<String>) -> (StatusCode, Value) {
        let mut request =
            Request::post("/v1/shop/paddle/webhook").header("content-type", "application/json");
        if let Some(sig) = signature {
            request = request.header("paddle-signature", sig);
        }
        let response = self
            .app
            .clone()
            .oneshot(request.body(Body::from(body.to_owned())).unwrap())
            .await
            .unwrap();
        let status = response.status();
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        (
            status,
            serde_json::from_slice(&bytes).unwrap_or(Value::Null),
        )
    }

    /// A correctly signed webhook, as Paddle sends it now.
    async fn hook(&self, event: Value) -> StatusCode {
        let body = event.to_string();
        let sig = sign(SECRET, body.as_bytes(), Utc::now().timestamp());
        self.hook_raw(&body, Some(sig)).await.0
    }

    async fn badge(&self, name: &str) -> String {
        let mut conn = self.db.conn().await;
        let mut sha = [0u8; 32];
        sha[..name.len().min(32)].copy_from_slice(&name.as_bytes()[..name.len().min(32)]);
        let (image, _) = cosmetics::insert_image(
            &mut conn,
            &NewImage {
                kind: CosmeticKind::Badge,
                sha256: &sha,
                mime: "image/png",
                width: 72,
                height: 72,
                frame_count: 1,
                animated: false,
                storage_key: "originals/cosmetics/x",
                created_by: None,
            },
        )
        .await
        .unwrap();
        let badge = cosmetics::create_badge(&mut conn, name, "tip", image.id, None)
            .await
            .unwrap();
        ids::to_text(badge.id)
    }

    async fn paint(&self, name: &str) -> String {
        let mut conn = self.db.conn().await;
        let stops = json!([{ "at": 0.0, "color": "#FF0000" }, { "at": 1.0, "color": "#0000FF" }]);
        let paint = cosmetics::create_paint(
            &mut conn,
            &PaintFields {
                name,
                kind: "linear_gradient",
                angle: 90,
                repeat: false,
                shape: "circle",
                color: None,
                stops: &stops,
                shadows: &json!([]),
                image_id: None,
            },
            None,
        )
        .await
        .unwrap();
        ids::to_text(paint.id)
    }

    async fn item(&self, by: &User, input: Value) -> Value {
        let r = self
            .gql(Some(by), CREATE_ITEM, json!({ "input": input }))
            .await;
        assert!(r["errors"].is_null(), "{r}");
        r["data"]["createShopItem"].clone()
    }

    async fn audit_count(&self, action: &str) -> i64 {
        let mut conn = self.db.conn().await;
        sqlx::query_scalar("SELECT count(*) FROM audit_log WHERE action = $1")
            .bind(action)
            .fetch_one(&mut *conn)
            .await
            .unwrap()
    }

    async fn sources(&self, account: Uuid) -> Vec<(String, Option<Uuid>)> {
        let mut conn = self.db.conn().await;
        sqlx::query_as(
            "SELECT source, order_id FROM badge_entitlements WHERE account_id = $1
             UNION ALL SELECT source, order_id FROM paint_entitlements WHERE account_id = $1",
        )
        .bind(account)
        .fetch_all(&mut *conn)
        .await
        .unwrap()
    }
}

fn bundle_input(name: &str, badges: &[&str], paints: &[&str]) -> Value {
    json!({
        "name": name,
        "description": "Sample bundle",
        "priceCents": 499,
        "paddlePriceId": "pri_01testprice",
        "badgeIds": badges,
        "paintIds": paints,
        "visible": true,
    })
}

fn completed(event_id: &str, txn: &str, order: &str) -> Value {
    json!({
        "event_id": event_id,
        "event_type": "transaction.completed",
        "occurred_at": "2026-11-17T12:00:00Z",
        "notification_id": format!("ntf_{event_id}"),
        "data": {
            "id": txn,
            "status": "completed",
            "currency_code": "EUR",
            "custom_data": { "cyex_tv_order_id": order },
            "items": [{ "price": { "id": "pri_01testprice" }, "quantity": 1 }],
            "details": { "totals": { "grand_total": "594" } }
        }
    })
}

fn adjustment(event_id: &str, txn: &str, action: &str, status: &str) -> Value {
    json!({
        "event_id": event_id,
        "event_type": "adjustment.updated",
        "occurred_at": "2026-11-17T12:00:00Z",
        "data": { "id": format!("adj_{event_id}"), "action": action, "type": "full", "status": status, "transaction_id": txn }
    })
}

#[tokio::test]
async fn items_are_made_by_staff_and_listed_when_on_sale() {
    let Some(w) = World::new(true).await else {
        return;
    };
    let manager = w.staff("11", &["shop.manage"]).await;
    let viewer = w.user("12").await;
    let badge = w.badge("Early").await;
    let paint = w.paint("Sunset").await;

    // Not staff, or signed out: no.
    let r = w
        .gql(
            Some(&viewer),
            CREATE_ITEM,
            json!({ "input": bundle_input("x", &[&badge], &[]) }),
        )
        .await;
    assert_eq!(code(&r), "forbidden");
    let r = w
        .gql(
            None,
            CREATE_ITEM,
            json!({ "input": bundle_input("x", &[&badge], &[]) }),
        )
        .await;
    assert_eq!(code(&r), "signed_out");

    let item = w
        .item(&manager, bundle_input("Starter", &[&badge], &[&paint]))
        .await;
    assert_eq!(item["currency"], "EUR");
    assert_eq!(item["available"], true);
    assert_eq!(item["badges"][0]["id"], badge.as_str());
    let id = item["id"].as_str().unwrap().to_owned();
    let mut hidden = bundle_input("Hidden", &[&badge], &[]);
    hidden["visible"] = json!(false);
    w.item(&manager, hidden).await;
    let mut later = bundle_input("Later", &[&badge], &[]);
    later["activeFrom"] = json!((Utc::now() + Duration::days(1)).to_rfc3339());
    w.item(&manager, later).await;

    // The public list has only what is on sale now.
    let public = w.gql(None, ITEMS, json!({})).await;
    let names: Vec<&str> = public["data"]["shopItems"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| i["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, ["Starter"]);
    assert_eq!(
        code(&w.gql(Some(&viewer), ITEMS, json!({ "all": true })).await),
        "forbidden"
    );
    let all = w.gql(Some(&manager), ITEMS, json!({ "all": true })).await;
    assert_eq!(all["data"]["shopItems"].as_array().unwrap().len(), 3);

    // Checks.
    let r = w
        .gql(
            Some(&manager),
            CREATE_ITEM,
            json!({ "input": bundle_input("Bad", &[&ids::to_text(ids::new_id())], &[]) }),
        )
        .await;
    assert_eq!(code(&r), "bad_cosmetics");
    let r = w
        .gql(
            Some(&manager),
            CREATE_ITEM,
            json!({ "input": bundle_input("Bad", &[], &[]) }),
        )
        .await;
    assert_eq!(code(&r), "bad_cosmetics");
    let mut bad_price = bundle_input("Bad", &[&badge], &[]);
    bad_price["paddlePriceId"] = json!("price_123");
    assert_eq!(
        code(
            &w.gql(Some(&manager), CREATE_ITEM, json!({ "input": bad_price }))
                .await
        ),
        "bad_price_id"
    );

    let mut changed = bundle_input("Starter pack", &[&badge], &[&paint]);
    changed["priceCents"] = json!(599);
    let r = w
        .gql(
            Some(&manager),
            UPDATE_ITEM,
            json!({ "id": id, "input": changed }),
        )
        .await;
    assert_eq!(r["data"]["updateShopItem"]["priceCents"], 599);

    let r = w
        .gql(Some(&manager), DELETE_ITEM, json!({ "id": id }))
        .await;
    assert_eq!(r["data"]["deleteShopItem"], true);
    let r = w
        .gql(Some(&manager), DELETE_ITEM, json!({ "id": id }))
        .await;
    assert_eq!(r["data"]["deleteShopItem"], false);
    assert!(
        w.gql(None, ITEMS, json!({})).await["data"]["shopItems"]
            .as_array()
            .unwrap()
            .is_empty()
    );

    assert_eq!(w.audit_count("shop_item.created").await, 3);
    assert_eq!(w.audit_count("shop_item.updated").await, 1);
    assert_eq!(w.audit_count("shop_item.deleted").await, 1);
    w.db.close().await;
}

#[tokio::test]
async fn checkout_is_unavailable_without_paddle() {
    let Some(w) = World::new(false).await else {
        return;
    };
    let owner = w.user(OWNER).await;
    let buyer = w.user("21").await;
    let badge = w.badge("Early").await;
    let item = w
        .item(&owner, bundle_input("Starter", &[&badge], &[]))
        .await;
    // Still listed.
    assert_eq!(
        w.gql(None, ITEMS, json!({})).await["data"]["shopItems"][0]["available"],
        true
    );
    let r = w
        .gql(Some(&buyer), CHECKOUT, json!({ "itemId": item["id"] }))
        .await;
    assert_eq!(code(&r), "checkout_unavailable");
    // And the webhook isn't open.
    let (status, body) = w.hook_raw("{}", Some("ts=1;h1=00".into())).await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(body["error"], "unavailable");
    assert_eq!(
        w.gql(None, CHECKOUT, json!({ "itemId": item["id"] })).await["errors"][0]["extensions"]["code"],
        "signed_out"
    );
    // An item without a Paddle price can't be bought either, even with Paddle set up.
    w.db.close().await;
}

#[tokio::test]
async fn the_webhook_fulfils_once_and_only_when_signed() {
    let Some(w) = World::new(true).await else {
        return;
    };
    let owner = w.user(OWNER).await;
    let buyer = w.user("31").await;
    let badge = w.badge("Early").await;
    let paint = w.paint("Sunset").await;
    let item = w
        .item(&owner, bundle_input("Starter", &[&badge], &[&paint]))
        .await;
    let item_id = item["id"].as_str().unwrap().to_owned();

    let r = w
        .gql(Some(&buyer), CHECKOUT, json!({ "itemId": item_id }))
        .await;
    let checkout = &r["data"]["startCheckout"];
    assert_eq!(checkout["environment"], "sandbox", "{r}");
    assert_eq!(checkout["clientToken"], "test_client_token");
    let txn = checkout["transactionId"].as_str().unwrap().to_owned();
    assert!(checkout["checkoutUrl"].as_str().unwrap().contains(&txn));
    assert_eq!(checkout["order"]["status"], "PENDING");
    assert_eq!(checkout["order"]["gift"], false);
    assert_eq!(checkout["order"]["providerTransactionId"], txn.as_str());
    let order = checkout["order"]["id"].as_str().unwrap().to_owned();
    // Paddle got one of our price, tagged with the order.
    {
        let calls = w.fake.calls.lock().unwrap();
        let (path, body) = &calls[0];
        assert_eq!(path, "/transactions");
        assert_eq!(body["items"][0]["price_id"], "pri_01testprice");
        assert_eq!(body["custom_data"]["cyex_tv_order_id"], order.as_str());
    }

    let event = completed("evt_1", &txn, &order).to_string();
    let now = Utc::now().timestamp();
    // Unsigned, wrongly signed, signed for another body, and stale: refused, nothing changes.
    assert_eq!(w.hook_raw(&event, None).await.0, StatusCode::UNAUTHORIZED);
    let (status, body) = w
        .hook_raw(
            &event,
            Some(sign("pdl_ntfset_wrong", event.as_bytes(), now)),
        )
        .await;
    assert_eq!(
        (status, body["error"].as_str()),
        (StatusCode::UNAUTHORIZED, Some("bad_signature"))
    );
    assert_eq!(
        w.hook_raw(&event, Some(sign(SECRET, b"{}", now))).await.0,
        StatusCode::UNAUTHORIZED
    );
    let (status, body) = w
        .hook_raw(&event, Some(sign(SECRET, event.as_bytes(), now - 600)))
        .await;
    assert_eq!(
        (status, body["error"].as_str()),
        (StatusCode::UNAUTHORIZED, Some("stale_signature"))
    );
    let r = w.gql(Some(&buyer), ORDER, json!({ "id": order })).await;
    assert_eq!(r["data"]["order"]["status"], "PENDING");
    assert!(w.sources(buyer.id).await.is_empty());

    // Signed: paid, and the buyer owns both with source purchase and the order.
    let signature = sign(SECRET, event.as_bytes(), now);
    assert_eq!(
        w.hook_raw(&event, Some(signature.clone())).await.0,
        StatusCode::OK
    );
    let r = w.gql(Some(&buyer), ORDER, json!({ "id": order })).await;
    assert_eq!(r["data"]["order"]["status"], "PAID");
    assert_eq!(r["data"]["order"]["paidTotalCents"], 594);
    assert_eq!(r["data"]["order"]["paidCurrency"], "EUR");
    let sources = w.sources(buyer.id).await;
    assert_eq!(sources.len(), 2);
    let order_uuid = ids::from_text(&order).unwrap();
    assert!(
        sources
            .iter()
            .all(|(s, o)| s == "purchase" && *o == Some(order_uuid))
    );

    // Replayed exactly, and the same completion under a new event: nothing more happens.
    assert_eq!(w.hook_raw(&event, Some(signature)).await.0, StatusCode::OK);
    assert_eq!(
        w.hook(completed("evt_2", &txn, &order)).await,
        StatusCode::OK
    );
    assert_eq!(w.audit_count("shop.order_paid").await, 1);
    assert_eq!(w.sources(buyer.id).await.len(), 2);
    // Events we don't use, and transactions that aren't ours, are acknowledged.
    assert_eq!(
        w.hook(json!({ "event_id": "evt_3", "event_type": "customer.created", "data": {} }))
            .await,
        StatusCode::OK
    );
    assert_eq!(
        w.hook(completed(
            "evt_4",
            "txn_unknown",
            &ids::to_text(ids::new_id())
        ))
        .await,
        StatusCode::OK
    );
    // Not a notification.
    let junk = "not json";
    assert_eq!(
        w.hook_raw(junk, Some(sign(SECRET, junk.as_bytes(), now)))
            .await
            .0,
        StatusCode::BAD_REQUEST
    );

    // Owning everything: can't buy again, only gift.
    let item_view = w.gql(Some(&buyer), ITEMS, json!({})).await;
    assert_eq!(item_view["data"]["shopItems"][0]["owned"], true);
    let r = w
        .gql(Some(&buyer), CHECKOUT, json!({ "itemId": item_id }))
        .await;
    assert_eq!(code(&r), "already_owned");
    let friend = w.user("32").await;
    let r = w
        .gql(
            Some(&buyer),
            CHECKOUT,
            json!({ "itemId": item_id, "recipient": { "platform": "TWITCH", "login": "USER32" } }),
        )
        .await;
    let gift = &r["data"]["startCheckout"]["order"];
    assert_eq!(gift["gift"], true, "{r}");
    assert_eq!(
        gift["recipient"]["id"],
        w.db.public(friend.id).await.as_str()
    );
    assert_eq!(gift["buyer"]["id"], w.db.public(buyer.id).await.as_str());
    let gift_txn = r["data"]["startCheckout"]["transactionId"]
        .as_str()
        .unwrap()
        .to_owned();
    let gift_order = gift["id"].as_str().unwrap().to_owned();
    assert_eq!(
        w.hook(completed("evt_5", &gift_txn, &gift_order)).await,
        StatusCode::OK
    );
    assert_eq!(w.sources(friend.id).await.len(), 2);
    // The friend sees the gift in their orders; the buyer sees both.
    let mine = w.gql(Some(&friend), MY_ORDERS, json!({})).await;
    assert_eq!(mine["data"]["myOrders"]["total"], 1);
    assert_eq!(mine["data"]["myOrders"]["items"][0]["gift"], true);
    assert_eq!(
        w.gql(Some(&buyer), MY_ORDERS, json!({})).await["data"]["myOrders"]["total"],
        2
    );
    // Others can't read an order.
    let stranger = w.user("33").await;
    assert!(w.gql(Some(&stranger), ORDER, json!({ "id": order })).await["data"]["order"].is_null());
    // No such recipient.
    let r = w
        .gql(
            Some(&buyer),
            CHECKOUT,
            json!({ "itemId": item_id, "recipient": { "platform": "KICK", "login": "user32" } }),
        )
        .await;
    assert_eq!(code(&r), "recipient_not_found");
    // Paddle failing: the order is failed and the error named.
    w.fake.fail.store(true, Ordering::SeqCst);
    let r = w
        .gql(Some(&stranger), CHECKOUT, json!({ "itemId": item_id }))
        .await;
    assert_eq!(code(&r), "payment_provider_error");
    w.fake.fail.store(false, Ordering::SeqCst);
    let staff = w.staff("34", &["shop.orders"]).await;
    let failed = w
        .gql(Some(&staff), ORDERS, json!({ "status": "FAILED" }))
        .await;
    assert_eq!(failed["data"]["orders"]["total"], 1);
    assert_eq!(
        code(&w.gql(Some(&buyer), ORDERS, json!({})).await),
        "forbidden"
    );
    w.db.close().await;
}

#[tokio::test]
async fn stock_runs_out() {
    let Some(w) = World::new(true).await else {
        return;
    };
    let owner = w.user(OWNER).await;
    let badge = w.badge("Rare").await;
    let mut input = bundle_input("Limited", &[&badge], &[]);
    input["stockLimit"] = json!(1);
    let item = w.item(&owner, input).await;
    assert_eq!(item["stockLeft"], 1);
    let first = w.user("41").await;
    let second = w.user("42").await;
    let r = w
        .gql(Some(&first), CHECKOUT, json!({ "itemId": item["id"] }))
        .await;
    assert!(r["errors"].is_null(), "{r}");
    // The pending checkout holds the copy.
    let r = w
        .gql(Some(&second), CHECKOUT, json!({ "itemId": item["id"] }))
        .await;
    assert_eq!(code(&r), "sold_out");
    let list = w.gql(None, ITEMS, json!({})).await;
    assert_eq!(list["data"]["shopItems"][0]["stockLeft"], 0);
    assert_eq!(list["data"]["shopItems"][0]["available"], false);
    w.db.close().await;
}

#[tokio::test]
async fn refunds_take_back_what_the_order_gave() {
    let Some(w) = World::new(true).await else {
        return;
    };
    let owner = w.user(OWNER).await;
    let buyer = w.user("51").await;
    let refunder = w.staff("52", &["shop.orders"]).await;
    let badge = w.badge("Early").await;
    let paint = w.paint("Sunset").await;
    let item = w
        .item(&owner, bundle_input("Starter", &[&badge], &[&paint]))
        .await;
    let buy = |n: &'static str| {
        let w = &w;
        let buyer = &buyer;
        let item = item["id"].clone();
        async move {
            let r = w
                .gql(Some(buyer), CHECKOUT, json!({ "itemId": item }))
                .await;
            let c = &r["data"]["startCheckout"];
            let txn = c["transactionId"].as_str().unwrap().to_owned();
            let order = c["order"]["id"].as_str().unwrap().to_owned();
            assert_eq!(w.hook(completed(n, &txn, &order)).await, StatusCode::OK);
            (txn, order)
        }
    };
    let (txn, order) = buy("evt_a").await;
    // The buyer shows the paint they bought.
    let r = w
        .gql(Some(&buyer), SELECT_PAINT, json!({ "p": paint }))
        .await;
    assert_eq!(r["data"]["selectCosmetics"]["paintId"], paint.as_str());

    // Not staff: no. Staff: Paddle is asked, the cosmetics are gone, the selection cleared.
    assert_eq!(
        code(&w.gql(Some(&buyer), REFUND, json!({ "id": order })).await),
        "forbidden"
    );
    let r = w.gql(Some(&refunder), REFUND, json!({ "id": order })).await;
    assert_eq!(r["data"]["refundOrder"]["status"], "REFUNDED", "{r}");
    assert_eq!(
        r["data"]["refundOrder"]["refundReason"],
        "asked by the buyer"
    );
    // The reason is a staff note: the buyer doesn't see it.
    let seen = w.gql(Some(&buyer), ORDER, json!({ "id": order })).await;
    assert_eq!(seen["data"]["order"]["status"], "REFUNDED");
    assert!(seen["data"]["order"]["refundReason"].is_null());
    {
        let calls = w.fake.calls.lock().unwrap();
        let (path, body) = calls.last().unwrap();
        assert_eq!(path, "/adjustments");
        assert_eq!(body["transaction_id"], txn.as_str());
        assert_eq!(body["action"], "refund");
        assert_eq!(body["type"], "full");
    }
    let mine = w.gql(Some(&buyer), MINE, json!({})).await;
    assert!(mine["data"]["myCosmetics"]["paintId"].is_null());
    assert!(
        mine["data"]["myCosmetics"]["paints"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    let mut conn = w.db.conn().await;
    let events = outbox::after(&mut conn, 0, 100).await.unwrap();
    drop(conn);
    let last = events.last().unwrap();
    assert_eq!(last.event_type, "user.cosmetics_updated");
    assert!(last.payload["paint_id"].is_null());
    // Again: not refundable. Paddle's later approval changes nothing.
    assert_eq!(
        code(&w.gql(Some(&refunder), REFUND, json!({ "id": order })).await),
        "not_refundable"
    );
    assert_eq!(
        w.hook(adjustment("evt_b", &txn, "refund", "approved"))
            .await,
        StatusCode::OK
    );
    assert_eq!(w.audit_count("shop.order_refunded").await, 1);

    // A chargeback from Paddle refunds the second purchase; a pending or partial refund doesn't.
    let (txn2, order2) = buy("evt_c").await;
    let mut partial = adjustment("evt_p", &txn2, "refund", "approved");
    partial["data"]["type"] = json!("partial");
    assert_eq!(w.hook(partial).await, StatusCode::OK);
    assert_eq!(
        w.gql(Some(&buyer), ORDER, json!({ "id": order2 })).await["data"]["order"]["status"],
        "PAID"
    );
    assert_eq!(w.audit_count("shop.order_partial_refund").await, 1);
    assert_eq!(w.sources(buyer.id).await.len(), 2);
    assert_eq!(
        w.hook(adjustment("evt_d", &txn2, "refund", "pending_approval"))
            .await,
        StatusCode::OK
    );
    let r = w.gql(Some(&buyer), ORDER, json!({ "id": order2 })).await;
    assert_eq!(r["data"]["order"]["status"], "PAID");
    assert_eq!(
        w.hook(adjustment("evt_e", &txn2, "chargeback", "approved"))
            .await,
        StatusCode::OK
    );
    let r = w.gql(Some(&buyer), ORDER, json!({ "id": order2 })).await;
    assert_eq!(r["data"]["order"]["status"], "REFUNDED");
    assert!(w.sources(buyer.id).await.is_empty());

    // A cosmetic granted by staff isn't touched by a refund of an order that found it owned.
    let r = w
        .gql(
            Some(&owner),
            "mutation($a: String!, $c: String!) { grantCosmetic(input: { accountId: $a, kind: BADGE, cosmeticId: $c, reason: \"gift\" }) { source } }",
            json!({ "a": buyer.tid(), "c": badge }),
        )
        .await;
    assert_eq!(r["data"]["grantCosmetic"]["source"], "GRANT");
    let (_, order3) = buy("evt_f").await;
    let r = w
        .gql(Some(&refunder), REFUND, json!({ "id": order3 }))
        .await;
    assert_eq!(r["data"]["refundOrder"]["status"], "REFUNDED");
    let left = w.sources(buyer.id).await;
    assert_eq!(left, vec![("grant".to_owned(), None)]);
    w.db.close().await;
}

#[tokio::test]
async fn codes_are_redeemed_once_and_stored_hashed() {
    let Some(w) = World::new(false).await else {
        return;
    };
    let manager = w.staff("61", &["codes.manage"]).await;
    let alice = w.user("62").await;
    let bob = w.user("63").await;
    let badge = w.badge("Early").await;
    let paint = w.paint("Sunset").await;
    let input = json!({ "name": "Launch", "prefix": "launch", "count": 3, "badgeIds": [badge], "paintIds": [paint] });

    assert_eq!(
        code(
            &w.gql(Some(&alice), CREATE_BATCH, json!({ "input": input }))
                .await
        ),
        "forbidden"
    );
    let r = w
        .gql(Some(&manager), CREATE_BATCH, json!({ "input": input }))
        .await;
    let made = &r["data"]["createCodeBatch"];
    let codes: Vec<String> = serde_json::from_value(made["codes"].clone()).unwrap();
    assert_eq!(codes.len(), 3, "{r}");
    assert!(
        codes
            .iter()
            .all(|c| c.starts_with("LAUNCH-") && c.len() == 26)
    );
    assert_eq!(made["batch"]["prefix"], "LAUNCH");
    let batch = made["batch"]["id"].as_str().unwrap().to_owned();

    // No plain code anywhere in the database.
    let mut conn = w.db.conn().await;
    let dump: String = sqlx::query_scalar(
        "SELECT coalesce(string_agg(t::text, ' '), '') FROM (
             SELECT row_to_json(r)::text AS t FROM redeem_codes r
             UNION ALL SELECT row_to_json(b)::text FROM code_batches b
             UNION ALL SELECT row_to_json(a)::text FROM audit_log a) x",
    )
    .fetch_one(&mut *conn)
    .await
    .unwrap();
    drop(conn);
    for c in &codes {
        let body = c.trim_start_matches("LAUNCH-").replace('-', "");
        assert!(
            !dump.contains(&body) && !dump.contains(c.as_str()),
            "{c} stored in plain text"
        );
    }

    // Happy path, typed sloppily.
    let sloppy = codes[0].to_lowercase().replace('-', " ");
    let r = w.gql(Some(&alice), REDEEM, json!({ "code": sloppy })).await;
    assert_eq!(r["data"]["redeemCode"]["name"], "Launch", "{r}");
    assert_eq!(r["data"]["redeemCode"]["badges"][0]["id"], badge.as_str());
    assert_eq!(
        w.sources(alice.id)
            .await
            .iter()
            .filter(|(s, _)| s == "code")
            .count(),
        2
    );
    let r = w
        .gql(Some(&alice), "{ myCosmetics { badges { id } } }", json!({}))
        .await;
    assert_eq!(r["data"]["myCosmetics"]["badges"][0]["id"], badge.as_str());
    // The same code again, and another code giving nothing new.
    assert_eq!(
        code(
            &w.gql(Some(&alice), REDEEM, json!({ "code": codes[0] }))
                .await
        ),
        "already_redeemed"
    );
    assert_eq!(
        code(
            &w.gql(Some(&alice), REDEEM, json!({ "code": codes[1] }))
                .await
        ),
        "already_owned"
    );
    // Used up: one use per code by default.
    assert_eq!(
        code(&w.gql(Some(&bob), REDEEM, json!({ "code": codes[0] })).await),
        "code_used_up"
    );
    // Expired.
    let mut conn = w.db.conn().await;
    sqlx::query("UPDATE code_batches SET expires_at = now() - interval '1 minute'")
        .execute(&mut *conn)
        .await
        .unwrap();
    drop(conn);
    assert_eq!(
        code(&w.gql(Some(&bob), REDEEM, json!({ "code": codes[1] })).await),
        "code_expired"
    );
    let mut conn = w.db.conn().await;
    sqlx::query("UPDATE code_batches SET expires_at = NULL")
        .execute(&mut *conn)
        .await
        .unwrap();
    drop(conn);
    // Disabled.
    let r = w
        .gql(Some(&manager), DISABLE_BATCH, json!({ "id": batch }))
        .await;
    assert!(
        r["data"]["disableCodeBatch"]["disabledAt"].is_string(),
        "{r}"
    );
    assert_eq!(
        code(&w.gql(Some(&bob), REDEEM, json!({ "code": codes[1] })).await),
        "code_disabled"
    );
    let list = w.gql(Some(&manager), BATCHES, json!({})).await;
    assert_eq!(list["data"]["codeBatches"]["items"][0]["redemptions"], 1);

    // Checks on a batch.
    for (bad, expect) in [
        (
            json!({ "name": "x", "prefix": "A-B", "count": 1, "badgeIds": [badge] }),
            "bad_prefix",
        ),
        (
            json!({ "name": "x", "prefix": "AB", "count": 1001, "badgeIds": [badge] }),
            "bad_count",
        ),
        (
            json!({ "name": "x", "prefix": "AB", "count": 1, "usesPerCode": 0, "badgeIds": [badge] }),
            "bad_uses",
        ),
        (
            json!({ "name": "x", "prefix": "AB", "count": 1 }),
            "bad_cosmetics",
        ),
        (
            json!({ "name": "x", "prefix": "AB", "count": 1, "badgeIds": [badge], "expiresAt": "2020-01-01T00:00:00Z" }),
            "bad_time",
        ),
    ] {
        assert_eq!(
            code(
                &w.gql(Some(&manager), CREATE_BATCH, json!({ "input": bad }))
                    .await
            ),
            expect
        );
    }
    assert_eq!(w.audit_count("code_batch.created").await, 1);
    assert_eq!(w.audit_count("code_batch.disabled").await, 1);
    assert_eq!(w.audit_count("code.redeemed").await, 1);
    w.db.close().await;
}

#[tokio::test]
async fn codes_of_an_item_with_many_uses() {
    let Some(w) = World::new(false).await else {
        return;
    };
    let owner = w.user(OWNER).await;
    let badge = w.badge("Early").await;
    let item = w
        .item(&owner, bundle_input("Starter", &[&badge], &[]))
        .await;
    let r = w
        .gql(
            Some(&owner),
            CREATE_BATCH,
            json!({ "input": { "name": "Stream", "prefix": "LIVE", "count": 1, "usesPerCode": 2, "itemId": item["id"] } }),
        )
        .await;
    let the_code = r["data"]["createCodeBatch"]["codes"][0]
        .as_str()
        .unwrap()
        .to_owned();
    assert_eq!(r["data"]["createCodeBatch"]["batch"]["itemId"], item["id"]);
    let (a, b, c) = (w.user("71").await, w.user("72").await, w.user("73").await);
    for u in [&a, &b] {
        let r = w.gql(Some(u), REDEEM, json!({ "code": the_code })).await;
        assert_eq!(
            r["data"]["redeemCode"]["badges"][0]["id"],
            badge.as_str(),
            "{r}"
        );
    }
    assert_eq!(
        code(&w.gql(Some(&c), REDEEM, json!({ "code": the_code })).await),
        "code_used_up"
    );
    w.db.close().await;
}

#[tokio::test]
async fn wrong_codes_lock_out_the_account_and_the_ip() {
    let Some(w) = World::new(false).await else {
        return;
    };
    let owner = w.user(OWNER).await;
    let badge = w.badge("Early").await;
    let r = w
        .gql(
            Some(&owner),
            CREATE_BATCH,
            json!({ "input": { "name": "x", "prefix": "X", "count": 1, "usesPerCode": 100, "badgeIds": [badge] } }),
        )
        .await;
    let good = r["data"]["createCodeBatch"]["codes"][0]
        .as_str()
        .unwrap()
        .to_owned();

    // Five wrong codes lock the account, even for the right one.
    let guesser = w.user("81").await;
    for _ in 0..5 {
        let r = w
            .gql(
                Some(&guesser),
                REDEEM,
                json!({ "code": "X-AAAA-BBBB-CCCC-DDDD" }),
            )
            .await;
        assert_eq!(code(&r), "bad_code");
    }
    assert_eq!(
        code(&w.gql(Some(&guesser), REDEEM, json!({ "code": good })).await),
        "locked_out"
    );
    // Twenty from one IP (here every test request comes from 127.0.0.1) lock it for everyone
    // there. 5 + 4 + 4 + 4 + 3 = 20.
    for (n, tries) in [("82", 4), ("83", 4), ("84", 4), ("85", 3)] {
        let u = w.user(n).await;
        for _ in 0..tries {
            assert_eq!(
                code(&w.gql(Some(&u), REDEEM, json!({ "code": "nope" })).await),
                "bad_code"
            );
        }
    }
    let fresh = w.user("86").await;
    assert_eq!(
        code(&w.gql(Some(&fresh), REDEEM, json!({ "code": good })).await),
        "locked_out"
    );
    w.db.close().await;
}

#[tokio::test]
async fn completions_that_dont_match_their_order_are_not_fulfilled() {
    let Some(w) = World::new(true).await else {
        return;
    };
    let owner = w.user(OWNER).await;
    let buyer = w.user("91").await;
    let badge = w.badge("Rare").await;
    let item = w
        .item(&owner, bundle_input("Expensive", &[&badge], &[]))
        .await;
    let r = w
        .gql(Some(&buyer), CHECKOUT, json!({ "itemId": item["id"] }))
        .await;
    let txn = r["data"]["startCheckout"]["transactionId"]
        .as_str()
        .unwrap()
        .to_owned();
    let order = r["data"]["startCheckout"]["order"]["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let pending = || async {
        w.gql(Some(&buyer), ORDER, json!({ "id": order })).await["data"]["order"]["status"].clone()
    };

    // Another (cheaper) price, two of it, another currency, or an extra item: not ours to fulfil.
    let mut cheap = completed("evt_m1", &txn, &order);
    cheap["data"]["items"][0]["price"]["id"] = json!("pri_01cheap");
    let mut many = completed("evt_m2", &txn, &order);
    many["data"]["items"][0]["quantity"] = json!(2);
    let mut usd = completed("evt_m3", &txn, &order);
    usd["data"]["currency_code"] = json!("USD");
    let mut extra = completed("evt_m4", &txn, &order);
    extra["data"]["items"] = json!([{ "price": { "id": "pri_01testprice" }, "quantity": 1 }, { "price": { "id": "pri_01other" }, "quantity": 1 }]);
    for event in [cheap, many, usd, extra] {
        assert_eq!(w.hook(event).await, StatusCode::OK);
        assert_eq!(pending().await, "PENDING");
    }
    assert_eq!(w.audit_count("shop.order_mismatch").await, 4);
    assert!(w.sources(buyer.id).await.is_empty());

    // A transaction we never made, tagged with our order (anyone can make one with the client
    // token): ignored, even for a failed order.
    let mut conn = w.db.conn().await;
    sqlx::query("UPDATE shop_orders SET status = 'failed'")
        .execute(&mut *conn)
        .await
        .unwrap();
    drop(conn);
    assert_eq!(
        w.hook(completed("evt_m5", "txn_someone_elses", &order))
            .await,
        StatusCode::OK
    );
    // A failed order isn't fulfilled even by its own transaction.
    assert_eq!(
        w.hook(completed("evt_m6", &txn, &order)).await,
        StatusCode::OK
    );
    assert_eq!(pending().await, "FAILED");
    assert!(w.sources(buyer.id).await.is_empty());

    // An adjustment that arrives before its order is paid is answered 503, so Paddle retries
    // it; once the order is paid, the same event goes through.
    let mut conn = w.db.conn().await;
    sqlx::query("UPDATE shop_orders SET status = 'pending'")
        .execute(&mut *conn)
        .await
        .unwrap();
    drop(conn);
    let early = adjustment("evt_m7", &txn, "chargeback", "approved");
    assert_eq!(w.hook(early.clone()).await, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(
        w.hook(completed("evt_m8", &txn, &order)).await,
        StatusCode::OK
    );
    assert_eq!(pending().await, "PAID");
    assert_eq!(w.sources(buyer.id).await.len(), 1);
    assert_eq!(w.hook(early).await, StatusCode::OK);
    assert_eq!(pending().await, "REFUNDED");
    assert!(w.sources(buyer.id).await.is_empty());
    w.db.close().await;
}

// Hardening (B1-B3, M1-M3, L1): each of these fails without its fix.

impl World {
    async fn sql(&self, query: &str) {
        let mut conn = self.db.conn().await;
        sqlx::query(query).execute(&mut *conn).await.unwrap();
    }

    /// Starts a checkout that has to work; gives the transaction and the order.
    async fn checkout(&self, by: &User, item: &Value, recipient: Option<&str>) -> (String, String) {
        let mut vars = json!({ "itemId": item["id"] });
        if let Some(login) = recipient {
            vars["recipient"] = json!({ "platform": "TWITCH", "login": login });
        }
        let r = self.gql(Some(by), CHECKOUT, vars).await;
        let c = &r["data"]["startCheckout"];
        assert!(r["errors"].is_null(), "{r}");
        (
            c["transactionId"].as_str().unwrap().to_owned(),
            c["order"]["id"].as_str().unwrap().to_owned(),
        )
    }

    async fn status(&self, order: &str) -> String {
        let mut conn = self.db.conn().await;
        sqlx::query_scalar("SELECT status FROM shop_orders WHERE id = $1")
            .bind(ids::from_text(order).unwrap())
            .fetch_one(&mut *conn)
            .await
            .unwrap()
    }

    /// The `after_json` of each audit row with this action, oldest first.
    async fn audits(&self, action: &str) -> Vec<Value> {
        let mut conn = self.db.conn().await;
        sqlx::query_scalar(
            "SELECT coalesce(after_json, 'null'::jsonb) FROM audit_log WHERE action = $1 ORDER BY id",
        )
        .bind(action)
        .fetch_all(&mut *conn)
        .await
        .unwrap()
    }

    async fn notes(&self, account: Uuid) -> Vec<(String, Value)> {
        let mut conn = self.db.conn().await;
        sqlx::query_as("SELECT kind, payload FROM notifications WHERE account_id = $1 ORDER BY id")
            .bind(account)
            .fetch_all(&mut *conn)
            .await
            .unwrap()
    }

    /// Owned cosmetic IDs, sorted.
    async fn owned(&self, account: Uuid) -> Vec<String> {
        let mut conn = self.db.conn().await;
        let mut out: Vec<String> = sqlx::query_scalar::<_, Uuid>(
            "SELECT badge_id FROM badge_entitlements WHERE account_id = $1
             UNION ALL SELECT paint_id FROM paint_entitlements WHERE account_id = $1",
        )
        .bind(account)
        .fetch_all(&mut *conn)
        .await
        .unwrap()
        .into_iter()
        .map(ids::to_text)
        .collect();
        out.sort();
        out
    }

    async fn code_for(&self, by: &User, badges: &[&str], uses: u32) -> String {
        let r = self
            .gql(
                Some(by),
                CREATE_BATCH,
                json!({ "input": { "name": "Giveaway", "prefix": "GIVE", "count": 1, "usesPerCode": uses, "badgeIds": badges } }),
            )
            .await;
        r["data"]["createCodeBatch"]["codes"][0]
            .as_str()
            .unwrap_or_else(|| panic!("{r}"))
            .to_owned()
    }
}

fn sorted(ids: &[&str]) -> Vec<String> {
    let mut v: Vec<String> = ids.iter().map(|s| (*s).to_owned()).collect();
    v.sort();
    v
}

#[tokio::test]
async fn refunding_one_order_keeps_what_other_orders_and_codes_cover() {
    let Some(w) = World::new(true).await else {
        return;
    };
    let owner = w.user(OWNER).await;
    let buyer = w.user("101").await;
    let refunder = w.staff("102", &["shop.orders"]).await;
    let (a, b, c, d) = (
        w.badge("A").await,
        w.badge("B").await,
        w.badge("C").await,
        w.badge("D").await,
    );
    let x = w.item(&owner, bundle_input("X", &[&a, &b], &[])).await;
    let y = w.item(&owner, bundle_input("Y", &[&b, &c], &[])).await;
    let (tx_x, order_x) = w.checkout(&buyer, &x, None).await;
    assert_eq!(
        w.hook(completed("evt_x", &tx_x, &order_x)).await,
        StatusCode::OK
    );
    let (tx_y, order_y) = w.checkout(&buyer, &y, None).await;
    assert_eq!(
        w.hook(completed("evt_y", &tx_y, &order_y)).await,
        StatusCode::OK
    );
    assert_eq!(w.owned(buyer.id).await, sorted(&[&a, &b, &c]));

    // Refunding X takes a back; b stays, because Y covers it, and now names Y.
    let r = w
        .gql(Some(&refunder), REFUND, json!({ "id": order_x }))
        .await;
    assert_eq!(r["data"]["refundOrder"]["status"], "REFUNDED", "{r}");
    assert_eq!(w.owned(buyer.id).await, sorted(&[&b, &c]));
    let y_uuid = ids::from_text(&order_y).unwrap();
    let mut conn = w.db.conn().await;
    let b_order: Option<Uuid> = sqlx::query_scalar(
        "SELECT order_id FROM badge_entitlements WHERE account_id = $1 AND badge_id = $2",
    )
    .bind(buyer.id)
    .bind(ids::from_text(&b).unwrap())
    .fetch_one(&mut *conn)
    .await
    .unwrap();
    drop(conn);
    assert_eq!(b_order, Some(y_uuid));

    // A code that gives c (and d) covers c from now on: refunding Y takes only b.
    let the_code = w.code_for(&owner, &[&c, &d], 1).await;
    let r = w
        .gql(Some(&buyer), REDEEM, json!({ "code": the_code }))
        .await;
    assert_eq!(
        r["data"]["redeemCode"]["badges"][0]["id"],
        d.as_str(),
        "{r}"
    );
    let r = w
        .gql(Some(&refunder), REFUND, json!({ "id": order_y }))
        .await;
    assert_eq!(r["data"]["refundOrder"]["status"], "REFUNDED", "{r}");
    assert_eq!(w.owned(buyer.id).await, sorted(&[&c, &d]));
    let sources: Vec<String> = w
        .sources(buyer.id)
        .await
        .into_iter()
        .map(|(s, _)| s)
        .collect();
    assert!(sources.iter().all(|s| s == "code"), "{sources:?}");

    // Notifications: granted with source purchase (a, b; then c), taken back (a; then b), and
    // the code's d with source code.
    let notes = w.notes(buyer.id).await;
    let of = |kind: &str, source: &str| {
        notes
            .iter()
            .filter(|(k, p)| k == kind && p["source"] == source)
            .count()
    };
    assert_eq!(of("cosmetic.granted", "purchase"), 3, "{notes:?}");
    assert_eq!(of("cosmetic.revoked", "purchase"), 2, "{notes:?}");
    assert_eq!(of("cosmetic.granted", "code"), 1, "{notes:?}");
    let revoked: Vec<&Value> = notes
        .iter()
        .filter(|(k, _)| k == "cosmetic.revoked")
        .map(|(_, p)| p)
        .collect();
    assert_eq!(revoked[0]["cosmetic_id"], a.as_str());
    assert_eq!(revoked[0]["order_id"], order_x.as_str());
    assert_eq!(revoked[0]["name"], "A");
    // The staff reason is a staff note, not in the buyer's notification.
    assert!(
        !notes
            .iter()
            .any(|(_, p)| p.to_string().contains("asked by the buyer"))
    );

    // The export lists both orders and the code (no Paddle price, no staff note).
    let r = w.site_gql(&buyer, "mutation { exportMyData }").await;
    let text = r["data"]["exportMyData"]
        .as_str()
        .unwrap_or_else(|| panic!("{r}"));
    let doc: Value = serde_json::from_str(text).unwrap();
    let orders = doc["orders"].as_array().unwrap();
    assert_eq!(orders.len(), 2);
    assert!(
        orders
            .iter()
            .all(|o| o["status"] == "refunded" && o["as_buyer"] == true)
    );
    assert!(
        orders
            .iter()
            .any(|o| o["provider_transaction_id"] == tx_x.as_str())
    );
    assert_eq!(doc["code_redemptions"].as_array().unwrap().len(), 1);
    assert_eq!(doc["code_redemptions"][0]["batch_name"], "Giveaway");
    assert!(!text.contains("pri_01testprice") && !text.contains("asked by the buyer"));
    assert!(!text.contains(&the_code));
    w.db.close().await;
}

#[tokio::test]
async fn payments_that_give_nothing_are_audited() {
    let Some(w) = World::new(true).await else {
        return;
    };
    let owner = w.user(OWNER).await;
    let buyer = w.user("111").await;
    let friend = w.user("112").await;
    let other = w.user("113").await;
    let badge = w.badge("Early").await;
    let item = w
        .item(&owner, bundle_input("Starter", &[&badge], &[]))
        .await;

    // (a) Cancelled at Paddle, then paid anyway: nothing given, audited with the transaction.
    let (txn, order) = w.checkout(&buyer, &item, None).await;
    let mut canceled = completed("evt_c1", &txn, &order);
    canceled["event_type"] = json!("transaction.canceled");
    assert_eq!(w.hook(canceled).await, StatusCode::OK);
    assert_eq!(
        w.hook(completed("evt_c2", &txn, &order)).await,
        StatusCode::OK
    );
    assert_eq!(w.status(&order).await, "failed");
    assert!(w.owned(buyer.id).await.is_empty());
    let rows = w.audits("shop.order_paid_unfulfilled").await;
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["provider_transaction_id"], txn.as_str());
    assert_eq!(rows[0]["status"], "failed");
    assert_eq!(rows[0]["reason"], "order_failed_canceled");

    // An order we failed ourselves (expired) still takes a late payment.
    let (txn, order) = w.checkout(&buyer, &item, None).await;
    w.sql(&format!(
        "UPDATE shop_orders SET status = 'failed', failed_reason = 'expired' WHERE id = '{}'",
        ids::from_text(&order).unwrap()
    ))
    .await;
    assert_eq!(
        w.hook(completed("evt_c3", &txn, &order)).await,
        StatusCode::OK
    );
    assert_eq!(w.status(&order).await, "paid");
    assert_eq!(w.owned(buyer.id).await, std::slice::from_ref(&badge));

    // (b) A gift whose recipient deleted their account before paying: paid, nothing granted to
    // the deleted account, audited.
    let (txn, order) = w.checkout(&buyer, &item, Some("user112")).await;
    w.sql(&format!(
        "UPDATE accounts SET deleted_at = now() WHERE id = '{}'",
        friend.id
    ))
    .await;
    assert_eq!(
        w.hook(completed("evt_c4", &txn, &order)).await,
        StatusCode::OK
    );
    assert_eq!(w.status(&order).await, "paid");
    assert!(w.owned(friend.id).await.is_empty());
    let rows = w.audits("shop.order_paid_unfulfilled").await;
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[1]["reason"], "recipient_deleted");
    assert_eq!(rows[1]["provider_transaction_id"], txn.as_str());

    // A recipient that is gone entirely (NULL).
    let (txn, order) = w.checkout(&other, &item, None).await;
    w.sql(&format!(
        "UPDATE shop_orders SET recipient_id = NULL WHERE id = '{}'",
        ids::from_text(&order).unwrap()
    ))
    .await;
    assert_eq!(
        w.hook(completed("evt_c5", &txn, &order)).await,
        StatusCode::OK
    );
    let rows = w.audits("shop.order_paid_unfulfilled").await;
    assert_eq!(rows.len(), 3);
    assert_eq!(rows[2]["reason"], "recipient_gone");

    // Two checkouts for the same recipient paid one after the other (a gift and their own):
    // the second gives nothing new, audited.
    let late = w.user("114").await;
    let (t1, o1) = w.checkout(&late, &item, None).await;
    let (t2, o2) = w.checkout(&other, &item, Some("user114")).await;
    assert_eq!(w.hook(completed("evt_c6", &t1, &o1)).await, StatusCode::OK);
    assert_eq!(w.hook(completed("evt_c7", &t2, &o2)).await, StatusCode::OK);
    assert_eq!(w.status(&o2).await, "paid");
    let rows = w.audits("shop.order_paid_no_grant").await;
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["provider_transaction_id"], t2.as_str());
    w.db.close().await;
}

#[tokio::test]
async fn a_transaction_that_cant_be_saved_is_retried_then_audited_with_its_id() {
    let Some(w) = World::new(true).await else {
        return;
    };
    let owner = w.user(OWNER).await;
    let buyer = w.user("121").await;
    let badge = w.badge("Early").await;
    let item = w
        .item(&owner, bundle_input("Starter", &[&badge], &[]))
        .await;
    // Saving a transaction ID fails while the sequence is at or under the budget.
    for q in [
        "CREATE SEQUENCE test_txn_fail",
        "CREATE TABLE test_txn_budget (n bigint NOT NULL)",
        "INSERT INTO test_txn_budget VALUES (1)",
        "CREATE FUNCTION test_txn_fail() RETURNS trigger LANGUAGE plpgsql AS $$
         BEGIN
           IF NEW.provider_transaction_id IS DISTINCT FROM OLD.provider_transaction_id
              AND nextval('test_txn_fail') <= (SELECT n FROM test_txn_budget) THEN
             RAISE EXCEPTION 'test: saving the transaction fails';
           END IF;
           RETURN NEW;
         END $$",
        "CREATE TRIGGER test_txn_fail BEFORE UPDATE ON shop_orders
         FOR EACH ROW EXECUTE FUNCTION test_txn_fail()",
    ] {
        w.sql(q).await;
    }

    // One failure: the retry saves it.
    let (txn, order) = w.checkout(&buyer, &item, None).await;
    assert_eq!(w.status(&order).await, "pending");
    assert_eq!(
        w.hook(completed("evt_s1", &txn, &order)).await,
        StatusCode::OK
    );
    assert_eq!(w.status(&order).await, "paid");

    // Always failing: the checkout fails, and the audit row has Paddle's transaction ID.
    w.sql("UPDATE test_txn_budget SET n = 1000000").await;
    let gift_to = w.user("122").await;
    let r = w
        .gql(
            Some(&buyer),
            CHECKOUT,
            json!({ "itemId": item["id"], "recipient": { "platform": "TWITCH", "login": "user122" } }),
        )
        .await;
    assert_eq!(code(&r), "checkout_failed", "{r}");
    let rows = w.audits("shop.checkout_failed").await;
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["reason"], "transaction_not_saved");
    let lost = rows[0]["provider_transaction_id"]
        .as_str()
        .unwrap()
        .to_owned();
    assert!(lost.starts_with("txn_test"), "{lost}");
    let mut conn = w.db.conn().await;
    let (lost_order, status): (Uuid, String) =
        sqlx::query_as("SELECT id, status FROM shop_orders WHERE recipient_id = $1")
            .bind(gift_to.id)
            .fetch_one(&mut *conn)
            .await
            .unwrap();
    drop(conn);
    assert_eq!(status, "failed");
    // If it is paid anyway, it isn't fulfilled from custom_data; staff see it.
    assert_eq!(
        w.hook(completed("evt_s2", &lost, &ids::to_text(lost_order)))
            .await,
        StatusCode::OK
    );
    assert!(w.owned(gift_to.id).await.is_empty());
    let rows = w.audits("shop.order_paid_unfulfilled").await;
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["reason"], "transaction_not_saved");
    assert_eq!(rows[0]["provider_transaction_id"], lost.as_str());
    w.db.close().await;
}

#[tokio::test]
async fn checkout_reads_the_item_again_under_its_lock() {
    let Some(w) = World::new(true).await else {
        return;
    };
    let owner = w.user(OWNER).await;
    let buyer = w.user("131").await;
    let badge = w.badge("Early").await;
    let item = w
        .item(&owner, bundle_input("Starter", &[&badge], &[]))
        .await;
    let item_id = ids::from_text(item["id"].as_str().unwrap()).unwrap();

    // Staff hold the item's row and change it while the checkout waits for the lock.
    let change = |set: &'static str| {
        let pool = w.db.pool.clone();
        async move {
            let mut tx = pool.begin().await.unwrap();
            sqlx::query("SELECT 1 FROM shop_items WHERE id = $1 FOR UPDATE")
                .bind(item_id)
                .execute(&mut *tx)
                .await
                .unwrap();
            (tx, set)
        }
    };
    let finish = |tx: sqlx::Transaction<'static, sqlx::Postgres>, set: &'static str| async move {
        let mut tx = tx;
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        sqlx::query(&format!("UPDATE shop_items SET {set} WHERE id = $1"))
            .bind(item_id)
            .execute(&mut *tx)
            .await
            .unwrap();
        tx.commit().await.unwrap();
    };

    // A new Paddle price: Paddle gets the locked row's price, and the order stores it.
    let (tx, set) = change("paddle_price_id = 'pri_01newprice'").await;
    let (r, ()) = tokio::join!(
        w.gql(Some(&buyer), CHECKOUT, json!({ "itemId": item["id"] })),
        finish(tx, set)
    );
    assert!(r["errors"].is_null(), "{r}");
    let price_sent = w.fake.calls.lock().unwrap().last().unwrap().1["items"][0]["price_id"].clone();
    assert_eq!(price_sent, "pri_01newprice");
    let order = r["data"]["startCheckout"]["order"]["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let mut conn = w.db.conn().await;
    let stored: Option<String> =
        sqlx::query_scalar("SELECT paddle_price_id FROM shop_orders WHERE id = $1")
            .bind(ids::from_text(&order).unwrap())
            .fetch_one(&mut *conn)
            .await
            .unwrap();
    drop(conn);
    assert_eq!(stored.as_deref(), Some("pri_01newprice"));

    // Hidden while the checkout waited: it can't be bought.
    let other = w.user("132").await;
    let calls = w.fake.calls.lock().unwrap().len();
    let (tx, set) = change("visible = false").await;
    let (r, ()) = tokio::join!(
        w.gql(Some(&other), CHECKOUT, json!({ "itemId": item["id"] })),
        finish(tx, set)
    );
    assert_eq!(code(&r), "item_unavailable", "{r}");
    assert_eq!(w.fake.calls.lock().unwrap().len(), calls);
    let mut conn = w.db.conn().await;
    let theirs: i64 = sqlx::query_scalar("SELECT count(*) FROM shop_orders WHERE account_id = $1")
        .bind(other.id)
        .fetch_one(&mut *conn)
        .await
        .unwrap();
    drop(conn);
    assert_eq!(theirs, 0);
    w.db.close().await;
}

#[tokio::test]
async fn one_pending_order_per_buyer_item_and_recipient() {
    let Some(w) = World::new(true).await else {
        return;
    };
    let owner = w.user(OWNER).await;
    let buyer = w.user("141").await;
    let badge = w.badge("Early").await;
    let mut input = bundle_input("Limited", &[&badge], &[]);
    input["stockLimit"] = json!(2);
    let item = w.item(&owner, input).await;

    // Asking again hands out the same order and transaction; Paddle is asked once.
    let (txn, order) = w.checkout(&buyer, &item, None).await;
    let (txn2, order2) = w.checkout(&buyer, &item, None).await;
    assert_eq!((&txn2, &order2), (&txn, &order));
    assert_eq!(w.fake.calls.lock().unwrap().len(), 1);
    // So the buyer holds one copy, not two.
    let list = w.gql(None, ITEMS, json!({})).await;
    assert_eq!(list["data"]["shopItems"][0]["stockLeft"], 1);

    // While another request is still making the transaction: busy.
    let order_uuid = ids::from_text(&order).unwrap();
    w.sql(&format!(
        "UPDATE shop_orders SET provider_transaction_id = NULL WHERE id = '{order_uuid}'"
    ))
    .await;
    let r = w
        .gql(Some(&buyer), CHECKOUT, json!({ "itemId": item["id"] }))
        .await;
    assert_eq!(code(&r), "checkout_busy", "{r}");
    w.sql(&format!(
        "UPDATE shop_orders SET provider_transaction_id = '{txn}' WHERE id = '{order_uuid}'"
    ))
    .await;

    // Once its hold ran out, a new checkout replaces it. Paddle refuses to cancel the old
    // transaction here (it is being paid), so a late payment of it is still taken.
    w.sql(&format!(
        "UPDATE shop_orders SET created_at = now() - interval '2 hours' WHERE id = '{order_uuid}'"
    ))
    .await;
    w.fake.cancel_fail.store(true, Ordering::SeqCst);
    let (txn3, order3) = w.checkout(&buyer, &item, None).await;
    w.fake.cancel_fail.store(false, Ordering::SeqCst);
    assert_eq!(*w.fake.cancels.lock().unwrap(), std::slice::from_ref(&txn));
    assert_eq!(w.audit_count("shop.transaction_cancel_failed").await, 1);
    assert_ne!(order3, order);
    assert_eq!(w.status(&order).await, "failed");
    assert_eq!(w.audit_count("shop.order_replaced").await, 1);
    let mut conn = w.db.conn().await;
    let pending: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM shop_orders WHERE account_id = $1 AND status = 'pending'",
    )
    .bind(buyer.id)
    .fetch_one(&mut *conn)
    .await
    .unwrap();
    drop(conn);
    assert_eq!(pending, 1);

    // The old tab is paid after all: taken (late payment); the new one then gives nothing new.
    assert_eq!(
        w.hook(completed("evt_p1", &txn, &order)).await,
        StatusCode::OK
    );
    assert_eq!(w.status(&order).await, "paid");
    assert_eq!(w.owned(buyer.id).await, std::slice::from_ref(&badge));
    assert_eq!(
        w.hook(completed("evt_p2", &txn3, &order3)).await,
        StatusCode::OK
    );
    assert_eq!(w.audit_count("shop.order_paid_no_grant").await, 1);
    // Two sold of two: nothing oversold yet.
    assert_eq!(w.audit_count("shop.order_oversold").await, 0);

    // A pending order whose hold ran out lets another buyer take the copy; when it is paid
    // late, it is still taken, and the oversell is audited.
    let mut input = bundle_input("One", &[&badge], &[]);
    input["stockLimit"] = json!(1);
    let one = w.item(&owner, input).await;
    let slow = w.user("142").await;
    let quick = w.user("143").await;
    let (slow_txn, slow_order) = w.checkout(&slow, &one, None).await;
    w.sql(&format!(
        "UPDATE shop_orders SET created_at = now() - interval '2 hours' WHERE id = '{}'",
        ids::from_text(&slow_order).unwrap()
    ))
    .await;
    let (quick_txn, quick_order) = w.checkout(&quick, &one, None).await;
    assert_eq!(
        w.hook(completed("evt_p3", &quick_txn, &quick_order)).await,
        StatusCode::OK
    );
    assert_eq!(
        w.hook(completed("evt_p4", &slow_txn, &slow_order)).await,
        StatusCode::OK
    );
    assert_eq!(w.status(&slow_order).await, "paid");
    assert_eq!(w.owned(slow.id).await, std::slice::from_ref(&badge));
    let rows = w.audits("shop.order_oversold").await;
    assert_eq!(rows.len(), 1);
    assert_eq!(
        (rows[0]["stock_limit"].as_i64(), rows[0]["sold"].as_i64()),
        (Some(1), Some(2))
    );
    w.db.close().await;
}

#[tokio::test]
async fn refunds_ask_paddle_outside_the_lock_and_rejections_are_flagged() {
    let Some(w) = World::new(true).await else {
        return;
    };
    let owner = w.user(OWNER).await;
    let buyer = w.user("151").await;
    let refunder = w.staff("152", &["shop.orders"]).await;
    let badge = w.badge("Early").await;
    let paint = w.paint("Sunset").await;
    let item = w
        .item(&owner, bundle_input("Starter", &[&badge], &[&paint]))
        .await;
    let (txn, order) = w.checkout(&buyer, &item, None).await;
    assert_eq!(
        w.hook(completed("evt_r1", &txn, &order)).await,
        StatusCode::OK
    );
    let order_uuid = ids::from_text(&order).unwrap();

    // Paddle says it was refunded already: a clear message, and nothing changes.
    w.fake.fail.store(true, Ordering::SeqCst);
    *w.fake.fail_code.lock().unwrap() = Some("adjustment_amount_above_remaining_allowed".into());
    let r = w.gql(Some(&refunder), REFUND, json!({ "id": order })).await;
    assert_eq!(code(&r), "already_refunded", "{r}");
    assert_eq!(w.status(&order).await, "paid");
    assert_eq!(w.owned(buyer.id).await.len(), 2);
    assert_eq!(w.audit_count("shop.refund_failed").await, 1);
    w.fake.fail.store(false, Ordering::SeqCst);

    // While Paddle is asked, the order isn't locked (it shows the refund as requested).
    w.fake.delay_ms.store(700, Ordering::SeqCst);
    let probe = async {
        tokio::time::sleep(std::time::Duration::from_millis(300)).await;
        let mut tx = w.db.pool.begin().await.unwrap();
        let status: Result<String, sqlx::Error> =
            sqlx::query_scalar("SELECT status FROM shop_orders WHERE id = $1 FOR UPDATE NOWAIT")
                .bind(order_uuid)
                .fetch_one(&mut *tx)
                .await;
        tx.rollback().await.unwrap();
        status
    };
    let (r, during) = tokio::join!(
        w.gql(Some(&refunder), REFUND, json!({ "id": order })),
        probe
    );
    w.fake.delay_ms.store(0, Ordering::SeqCst);
    assert_eq!(
        during.expect("the order is free while Paddle is asked"),
        "refund_requested"
    );
    assert_eq!(r["data"]["refundOrder"]["status"], "REFUNDED", "{r}");
    assert!(w.owned(buyer.id).await.is_empty());
    assert_eq!(w.audit_count("shop.refund_requested").await, 2);

    // Paddle rejects the refund later: audited, the order is flagged for staff, and the
    // cosmetics stay taken back.
    const STAFF_ORDER: &str = "query($id: String!) { order(id: $id) { status refundRejectedAt } }";
    let staff_view = w
        .gql(Some(&refunder), STAFF_ORDER, json!({ "id": order }))
        .await;
    assert!(staff_view["data"]["order"]["refundRejectedAt"].is_null());
    assert_eq!(
        w.hook(adjustment("evt_r2", &txn, "refund", "rejected"))
            .await,
        StatusCode::OK
    );
    let rows = w.audits("shop.refund_rejected").await;
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["flagged"], true);
    let staff_view = w
        .gql(Some(&refunder), STAFF_ORDER, json!({ "id": order }))
        .await;
    assert!(
        staff_view["data"]["order"]["refundRejectedAt"].is_string(),
        "{staff_view}"
    );
    assert_eq!(staff_view["data"]["order"]["status"], "REFUNDED");
    let buyer_view = w
        .gql(Some(&buyer), STAFF_ORDER, json!({ "id": order }))
        .await;
    assert!(buyer_view["data"]["order"]["refundRejectedAt"].is_null());
    assert!(w.owned(buyer.id).await.is_empty());
    w.db.close().await;
}

#[tokio::test]
async fn price_ids_redeem_ips_and_webhook_size() {
    let Some(w) = World::new(true).await else {
        return;
    };
    let manager = w.staff("161", &["shop.manage", "codes.manage"]).await;
    let viewer = w.user("162").await;
    let badge = w.badge("Early").await;
    w.item(&manager, bundle_input("Starter", &[&badge], &[]))
        .await;

    // Paddle price IDs only for shop.manage.
    const PRICES: &str = "{ shopItems { paddlePriceId } }";
    for who in [None, Some(&viewer)] {
        let r = w.gql(who, PRICES, json!({})).await;
        assert!(r["data"]["shopItems"][0]["paddlePriceId"].is_null(), "{r}");
    }
    let r = w.gql(Some(&manager), PRICES, json!({})).await;
    assert_eq!(
        r["data"]["shopItems"][0]["paddlePriceId"],
        "pri_01testprice"
    );

    // A redeem whose IP isn't known is refused, not counted under a shared "unknown".
    let the_code = w.code_for(&manager, &[&badge], 5).await;
    let r = w
        .gql_from(None, Some(&viewer), REDEEM, json!({ "code": the_code }))
        .await;
    assert_eq!(code(&r), "unknown_client", "{r}");
    let r = w
        .gql(Some(&viewer), REDEEM, json!({ "code": the_code }))
        .await;
    assert_eq!(
        r["data"]["redeemCode"]["badges"][0]["id"],
        badge.as_str(),
        "{r}"
    );

    // The webhook refuses bodies over 64 KB before reading them.
    let big = json!({ "event_id": "evt_big", "event_type": "x", "data": { "pad": "x".repeat(70 * 1024) } })
        .to_string();
    let sig = sign(SECRET, big.as_bytes(), Utc::now().timestamp());
    let (status, _) = w.hook_raw(&big, Some(sig)).await;
    assert_eq!(status, StatusCode::PAYLOAD_TOO_LARGE);
    w.db.close().await;
}

// Concurrency

#[tokio::test]
async fn two_checkouts_of_the_last_copy() {
    let Some(w) = World::new(true).await else {
        return;
    };
    let owner = w.user(OWNER).await;
    let badge = w.badge("Rare").await;
    let mut input = bundle_input("Last", &[&badge], &[]);
    input["stockLimit"] = json!(1);
    let item = w.item(&owner, input).await;
    let (a, b) = (w.user("171").await, w.user("172").await);
    w.fake.delay_ms.store(100, Ordering::SeqCst);
    let (ra, rb) = tokio::join!(
        w.gql(Some(&a), CHECKOUT, json!({ "itemId": item["id"] })),
        w.gql(Some(&b), CHECKOUT, json!({ "itemId": item["id"] }))
    );
    let mut outcomes = [
        if ra["errors"].is_null() {
            "ok"
        } else {
            code(&ra)
        },
        if rb["errors"].is_null() {
            "ok"
        } else {
            code(&rb)
        },
    ];
    outcomes.sort_unstable();
    assert_eq!(outcomes, ["ok", "sold_out"], "{ra} {rb}");
    let mut conn = w.db.conn().await;
    let pending: i64 =
        sqlx::query_scalar("SELECT count(*) FROM shop_orders WHERE status = 'pending'")
            .fetch_one(&mut *conn)
            .await
            .unwrap();
    drop(conn);
    assert_eq!(pending, 1);
    w.db.close().await;
}

#[tokio::test]
async fn a_completion_and_a_staff_refund_at_once_stay_consistent() {
    let Some(w) = World::new(true).await else {
        return;
    };
    let owner = w.user(OWNER).await;
    let refunder = w.staff("181", &["shop.orders"]).await;
    let badge = w.badge("Early").await;
    let item = w
        .item(&owner, bundle_input("Starter", &[&badge], &[]))
        .await;
    for n in 0..6 {
        let buyer = w.user(&format!("18{}", n + 2)).await;
        let (txn, order) = w.checkout(&buyer, &item, None).await;
        let (status, r) = tokio::join!(
            w.hook(completed(&format!("evt_race{n}"), &txn, &order)),
            w.gql(Some(&refunder), REFUND, json!({ "id": order }))
        );
        assert_eq!(status, StatusCode::OK);
        let owned = w.owned(buyer.id).await;
        if r["errors"].is_null() {
            // The refund went after the payment: refunded, nothing left.
            assert_eq!(w.status(&order).await, "refunded", "{r}");
            assert!(owned.is_empty(), "{owned:?}");
        } else {
            // The refund came first: the order wasn't paid yet, and then the payment gave it.
            assert_eq!(code(&r), "not_refundable", "{r}");
            assert_eq!(w.status(&order).await, "paid");
            assert_eq!(owned, std::slice::from_ref(&badge));
        }
    }
    w.db.close().await;
}

#[tokio::test]
async fn two_redemptions_of_a_single_use_code() {
    let Some(w) = World::new(false).await else {
        return;
    };
    let owner = w.user(OWNER).await;
    let badge = w.badge("Early").await;
    let paint = w.paint("Sunset").await;
    let r = w
        .gql(
            Some(&owner),
            CREATE_BATCH,
            json!({ "input": { "name": "One", "prefix": "ONE", "count": 1, "badgeIds": [badge], "paintIds": [paint] } }),
        )
        .await;
    let the_code = r["data"]["createCodeBatch"]["codes"][0]
        .as_str()
        .unwrap()
        .to_owned();
    let (a, b) = (w.user("191").await, w.user("192").await);
    let (ra, rb) = tokio::join!(
        w.gql(Some(&a), REDEEM, json!({ "code": the_code })),
        w.gql(Some(&b), REDEEM, json!({ "code": the_code }))
    );
    let mut outcomes = [
        if ra["errors"].is_null() {
            "ok"
        } else {
            code(&ra)
        },
        if rb["errors"].is_null() {
            "ok"
        } else {
            code(&rb)
        },
    ];
    outcomes.sort_unstable();
    assert_eq!(outcomes, ["code_used_up", "ok"], "{ra} {rb}");
    let owners = w.owned(a.id).await.len() + w.owned(b.id).await.len();
    assert_eq!(owners, 2);
    w.db.close().await;
}

// Sign-off fixes

#[tokio::test]
async fn replaced_checkouts_are_cancelled_and_take_no_late_payment() {
    let Some(w) = World::new(true).await else {
        return;
    };
    let owner = w.user(OWNER).await;
    let buyer = w.user("301").await;
    let badge = w.badge("Early").await;
    let item = w
        .item(&owner, bundle_input("Starter", &[&badge], &[]))
        .await;
    let (txn, order) = w.checkout(&buyer, &item, None).await;
    w.sql(&format!(
        "UPDATE shop_orders SET created_at = now() - interval '2 hours' WHERE id = '{}'",
        ids::from_text(&order).unwrap()
    ))
    .await;
    let (txn2, order2) = w.checkout(&buyer, &item, None).await;
    assert_ne!(order2, order);
    // Paddle was asked to cancel the old transaction, and did.
    assert_eq!(*w.fake.cancels.lock().unwrap(), std::slice::from_ref(&txn));
    let mut conn = w.db.conn().await;
    let canceled: bool = sqlx::query_scalar(
        "SELECT provider_canceled_at IS NOT NULL FROM shop_orders WHERE id = $1",
    )
    .bind(ids::from_text(&order).unwrap())
    .fetch_one(&mut *conn)
    .await
    .unwrap();
    drop(conn);
    assert!(canceled);
    assert_eq!(w.audit_count("shop.transaction_cancel_failed").await, 0);
    // A completion for the cancelled transaction gives nothing and is audited for staff.
    assert_eq!(
        w.hook(completed("evt_rc1", &txn, &order)).await,
        StatusCode::OK
    );
    assert_eq!(w.status(&order).await, "failed");
    assert!(w.owned(buyer.id).await.is_empty());
    let rows = w.audits("shop.order_paid_unfulfilled").await;
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["reason"], "order_failed_replaced");
    // An adjustment for it is not held back either (it can't be paid): answered 200.
    assert_eq!(
        w.hook(adjustment("evt_rc2", &txn, "refund", "approved"))
            .await,
        StatusCode::OK
    );
    // The new checkout is paid normally.
    assert_eq!(
        w.hook(completed("evt_rc3", &txn2, &order2)).await,
        StatusCode::OK
    );
    assert_eq!(w.owned(buyer.id).await, std::slice::from_ref(&badge));
    w.db.close().await;
}

#[tokio::test]
async fn adjustments_wait_while_the_order_can_still_be_paid() {
    let Some(w) = World::new(true).await else {
        return;
    };
    let owner = w.user(OWNER).await;
    let buyer = w.user("311").await;
    let badge = w.badge("Early").await;
    let item = w
        .item(&owner, bundle_input("Starter", &[&badge], &[]))
        .await;
    let (txn, order) = w.checkout(&buyer, &item, None).await;
    // Expired by us, its cancel failed: a late payment is still taken, so a refund arriving
    // first is answered 503 and Paddle sends it again.
    w.sql(&format!(
        "UPDATE shop_orders SET status = 'failed', failed_reason = 'expired', cancel_failed_at = now()
          WHERE id = '{}'",
        ids::from_text(&order).unwrap()
    ))
    .await;
    assert_eq!(
        w.hook(adjustment("evt_aw1", &txn, "refund", "approved"))
            .await,
        StatusCode::SERVICE_UNAVAILABLE
    );
    // The payment lands, then the refund again: taken back.
    assert_eq!(
        w.hook(completed("evt_aw2", &txn, &order)).await,
        StatusCode::OK
    );
    assert_eq!(w.owned(buyer.id).await, std::slice::from_ref(&badge));
    assert_eq!(
        w.hook(adjustment("evt_aw3", &txn, "refund", "approved"))
            .await,
        StatusCode::OK
    );
    assert_eq!(w.status(&order).await, "refunded");
    assert!(w.owned(buyer.id).await.is_empty());
    w.db.close().await;
}

#[tokio::test]
async fn checkout_links_outside_the_allowed_hosts_are_dropped() {
    let Some(w) = World::new(true).await else {
        return;
    };
    let owner = w.user(OWNER).await;
    let buyer = w.user("321").await;
    let badge = w.badge("Early").await;
    let item = w
        .item(&owner, bundle_input("Starter", &[&badge], &[]))
        .await;
    w.fake.bad_url.store(true, Ordering::SeqCst);
    let r = w
        .gql(Some(&buyer), CHECKOUT, json!({ "itemId": item["id"] }))
        .await;
    assert!(r["errors"].is_null(), "{r}");
    let checkout = &r["data"]["startCheckout"];
    assert!(checkout["checkoutUrl"].is_null(), "{r}");
    // Paddle.js still opens it by transaction ID.
    assert!(
        checkout["transactionId"]
            .as_str()
            .unwrap()
            .starts_with("txn_")
    );
    w.db.close().await;
}

#[tokio::test]
async fn fulfilment_grants_what_was_sold_at_checkout() {
    let Some(w) = World::new(true).await else {
        return;
    };
    let owner = w.user(OWNER).await;
    let buyer = w.user("331").await;
    let early = w.badge("Early").await;
    let later = w.badge("Later").await;
    let item = w
        .item(&owner, bundle_input("Starter", &[&early], &[]))
        .await;
    let (txn, order) = w.checkout(&buyer, &item, None).await;
    // Staff swap the item's badge (same price) before the payment lands.
    let r = w
        .gql(
            Some(&owner),
            UPDATE_ITEM,
            json!({ "id": item["id"], "input": bundle_input("Starter", &[&later], &[]) }),
        )
        .await;
    assert!(r["errors"].is_null(), "{r}");
    assert_eq!(
        w.hook(completed("evt_sn1", &txn, &order)).await,
        StatusCode::OK
    );
    // The buyer gets what they paid for, not what the item holds now.
    assert_eq!(w.owned(buyer.id).await, std::slice::from_ref(&early));

    // A pending checkout of the old bundle isn't handed out again: it is replaced.
    let other = w.user("332").await;
    let old = w.item(&owner, bundle_input("Swap", &[&early], &[])).await;
    let (_, first) = w.checkout(&other, &old, None).await;
    let r = w
        .gql(
            Some(&owner),
            UPDATE_ITEM,
            json!({ "id": old["id"], "input": bundle_input("Swap", &[&later], &[]) }),
        )
        .await;
    assert!(r["errors"].is_null(), "{r}");
    let (_, second) = w.checkout(&other, &old, None).await;
    assert_ne!(second, first);
    assert_eq!(w.status(&first).await, "failed");

    // Every cosmetic it sold was deleted since: paid, nothing granted, audited `item_empty`.
    let third = w.user("333").await;
    let gone = w.badge("Gone").await;
    let doomed = w.item(&owner, bundle_input("Doomed", &[&gone], &[])).await;
    let (t3, o3) = w.checkout(&third, &doomed, None).await;
    w.sql(&format!(
        "DELETE FROM badges WHERE id = '{}'",
        ids::from_text(&gone).unwrap()
    ))
    .await;
    assert_eq!(w.hook(completed("evt_sn2", &t3, &o3)).await, StatusCode::OK);
    assert_eq!(w.status(&o3).await, "paid");
    assert!(w.owned(third.id).await.is_empty());
    let rows = w.audits("shop.order_paid_unfulfilled").await;
    assert_eq!(rows.len(), 1, "{rows:?}");
    assert_eq!(rows[0]["reason"], "item_empty");
    w.db.close().await;
}

#[tokio::test]
async fn stuck_refunds_can_be_tried_again_after_ten_minutes() {
    let Some(w) = World::new(true).await else {
        return;
    };
    let owner = w.user(OWNER).await;
    let staff = w.staff("341", &["shop.orders"]).await;
    let buyer = w.user("342").await;
    let badge = w.badge("Early").await;
    let item = w
        .item(&owner, bundle_input("Starter", &[&badge], &[]))
        .await;
    let (txn, order) = w.checkout(&buyer, &item, None).await;
    assert_eq!(
        w.hook(completed("evt_st1", &txn, &order)).await,
        StatusCode::OK
    );
    let order_uuid = ids::from_text(&order).unwrap();
    // The api stopped after recording the request: five minutes on, still in progress.
    w.sql(&format!(
        "UPDATE shop_orders SET status = 'refund_requested', refund_reason = 'asked',
                refund_requested_at = now() - interval '5 minutes' WHERE id = '{order_uuid}'"
    ))
    .await;
    let r = w.gql(Some(&staff), REFUND, json!({ "id": order })).await;
    assert_eq!(code(&r), "refund_in_progress", "{r}");
    // Eleven minutes on, staff may try again: Paddle is asked, the cosmetics go.
    w.sql(&format!(
        "UPDATE shop_orders SET refund_requested_at = now() - interval '11 minutes'
          WHERE id = '{order_uuid}'"
    ))
    .await;
    let adjustments = |w: &World| {
        w.fake
            .calls
            .lock()
            .unwrap()
            .iter()
            .filter(|(p, _)| p == "/adjustments")
            .count()
    };
    let r = w.gql(Some(&staff), REFUND, json!({ "id": order })).await;
    assert!(r["errors"].is_null(), "{r}");
    assert_eq!(r["data"]["refundOrder"]["status"], "REFUNDED");
    assert_eq!(adjustments(&w), 1);
    assert!(w.owned(buyer.id).await.is_empty());

    // Paddle answers a retry of a refund it took as a duplicate: already_refunded.
    let buyer2 = w.user("343").await;
    let (txn, order) = w.checkout(&buyer2, &item, None).await;
    assert_eq!(
        w.hook(completed("evt_st2", &txn, &order)).await,
        StatusCode::OK
    );
    w.sql(&format!(
        "UPDATE shop_orders SET status = 'refund_requested', refund_reason = 'asked',
                refund_requested_at = now() - interval '11 minutes' WHERE id = '{}'",
        ids::from_text(&order).unwrap()
    ))
    .await;
    w.fake.fail.store(true, Ordering::SeqCst);
    *w.fake.fail_code.lock().unwrap() = Some("transaction_already_refunded".into());
    let r = w.gql(Some(&staff), REFUND, json!({ "id": order })).await;
    assert_eq!(code(&r), "already_refunded", "{r}");
    w.fake.fail.store(false, Ordering::SeqCst);
    w.db.close().await;
}

#[tokio::test]
async fn staff_refunds_never_revoke_without_paddle() {
    let Some(w) = World::new(false).await else {
        return;
    };
    let staff = w.staff("351", &["shop.orders"]).await;
    let owner = w.user(OWNER).await;
    let buyer = w.user("352").await;
    let badge = w.badge("Early").await;
    let item = w
        .item(&owner, bundle_input("Starter", &[&badge], &[]))
        .await;
    let item_id = ids::from_text(item["id"].as_str().unwrap()).unwrap();
    let badge_id = ids::from_text(&badge).unwrap();
    // A paid order with a Paddle transaction, made while Paddle was set up.
    let order = ids::new_id();
    w.sql(&format!(
        "INSERT INTO shop_orders (id, account_id, recipient_id, item_id, item_name, status,
                                  amount_cents, currency, provider_transaction_id, paid_at)
         VALUES ('{order}', '{b}', '{b}', '{item_id}', 'Starter', 'paid', 499, 'EUR',
                 'txn_nopaddle', now())",
        b = buyer.id
    ))
    .await;
    w.sql(&format!(
        "INSERT INTO badge_entitlements (account_id, badge_id, source, reason, order_id)
         VALUES ('{b}', '{badge_id}', 'purchase', 'Order', '{order}')",
        b = buyer.id
    ))
    .await;
    w.sql(&format!(
        "INSERT INTO shop_order_entitlements (order_id, account_id, kind, cosmetic_id)
         VALUES ('{order}', '{b}', 'badge', '{badge_id}')",
        b = buyer.id
    ))
    .await;
    let r = w
        .gql(Some(&staff), REFUND, json!({ "id": ids::to_text(order) }))
        .await;
    assert_eq!(code(&r), "refund_unavailable", "{r}");
    assert_eq!(w.status(&ids::to_text(order)).await, "paid");
    assert_eq!(w.owned(buyer.id).await, std::slice::from_ref(&badge));
    w.db.close().await;
}

#[tokio::test]
async fn gift_recipients_see_paid_gifts_without_the_money() {
    let Some(w) = World::new(true).await else {
        return;
    };
    let owner = w.user(OWNER).await;
    let buyer = w.user("361").await;
    let friend = w.user("362").await;
    let staff = w.staff("363", &["shop.orders"]).await;
    let badge = w.badge("Early").await;
    let item = w
        .item(&owner, bundle_input("Starter", &[&badge], &[]))
        .await;
    const FULL: &str = "query($id: String!) { order(id: $id) { id gift amountCents paidTotalCents paidCurrency providerTransactionId } }";
    const LIST: &str =
        "{ myOrders { total items { id gift amountCents paidTotalCents providerTransactionId } } }";
    let (txn, order) = w.checkout(&buyer, &item, Some("user362")).await;
    // Unpaid: the recipient sees nothing of it.
    let r = w.gql(Some(&friend), FULL, json!({ "id": order })).await;
    assert!(r["data"]["order"].is_null(), "{r}");
    let r = w.gql(Some(&friend), LIST, json!({})).await;
    assert_eq!(r["data"]["myOrders"]["total"], 0, "{r}");
    assert_eq!(r["data"]["myOrders"]["items"], json!([]));

    assert_eq!(
        w.hook(completed("evt_gf1", &txn, &order)).await,
        StatusCode::OK
    );
    // Paid: the recipient sees it, without what was paid or the transaction.
    let r = w.gql(Some(&friend), FULL, json!({ "id": order })).await;
    let seen = &r["data"]["order"];
    assert_eq!(seen["gift"], true, "{r}");
    assert_eq!(seen["amountCents"], 0);
    assert!(seen["paidTotalCents"].is_null());
    assert!(seen["paidCurrency"].is_null());
    assert!(seen["providerTransactionId"].is_null());
    let r = w.gql(Some(&friend), LIST, json!({})).await;
    assert_eq!(r["data"]["myOrders"]["total"], 1);
    assert_eq!(r["data"]["myOrders"]["items"][0]["amountCents"], 0);
    assert!(r["data"]["myOrders"]["items"][0]["providerTransactionId"].is_null());
    // The buyer sees the money.
    let r = w.gql(Some(&buyer), FULL, json!({ "id": order })).await;
    assert_eq!(r["data"]["order"]["amountCents"], 499);
    assert_eq!(r["data"]["order"]["paidTotalCents"], 594);
    assert_eq!(r["data"]["order"]["providerTransactionId"], txn.as_str());
    // The gift flag was stored at checkout: it stays when the buyer's account is gone.
    w.sql(&format!(
        "UPDATE shop_orders SET account_id = NULL WHERE id = '{}'",
        ids::from_text(&order).unwrap()
    ))
    .await;
    let r = w.gql(Some(&staff), FULL, json!({ "id": order })).await;
    assert_eq!(r["data"]["order"]["gift"], true, "{r}");
    assert_eq!(r["data"]["order"]["amountCents"], 499);
    w.db.close().await;
}

// Idempotency keys: a retry with the same key answers what the first request did.

const CHECKOUT_KEYED: &str =
    "mutation($itemId: String!, $recipient: RecipientInput, $key: String) {
    startCheckout(itemId: $itemId, recipient: $recipient, idempotencyKey: $key) {
        transactionId order { id status }
    }
}";
const REDEEM_KEYED: &str = "mutation($code: String!, $key: String) {
    redeemCode(code: $code, idempotencyKey: $key) { name badges { id } paints { id } }
}";

impl World {
    async fn count(&self, sql: &str) -> i64 {
        let mut conn = self.db.conn().await;
        sqlx::query_scalar(sql).fetch_one(&mut *conn).await.unwrap()
    }

    async fn sql_text(&self, sql: &str) -> String {
        let mut conn = self.db.conn().await;
        sqlx::query_scalar(sql).fetch_one(&mut *conn).await.unwrap()
    }
}

#[tokio::test]
async fn a_checkout_retried_with_its_key_starts_one_order() {
    let Some(w) = World::new(true).await else {
        return;
    };
    let owner = w.user(OWNER).await;
    let buyer = w.user("971").await;
    let other = w.user("972").await;
    let badge = w.badge("Early").await;
    let paint = w.paint("Sunset").await;
    let item = w
        .item(&owner, bundle_input("Starter", &[&badge], &[]))
        .await;
    let second = w.item(&owner, bundle_input("Second", &[], &[&paint])).await;
    let key = ids::new_id().to_string();
    let vars = json!({ "itemId": item["id"], "key": key });

    // Two at once with the same key (Paddle answers slowly): one starts the order, the other is
    // told it's in progress.
    w.fake.delay_ms.store(300, Ordering::SeqCst);
    let (a, b) = tokio::join!(
        w.gql(Some(&buyer), CHECKOUT_KEYED, vars.clone()),
        w.gql(Some(&buyer), CHECKOUT_KEYED, vars.clone())
    );
    w.fake.delay_ms.store(0, Ordering::SeqCst);
    let (first, busy) = if a["errors"].is_null() {
        (a, b)
    } else {
        (b, a)
    };
    assert!(first["errors"].is_null(), "{first}");
    assert_eq!(code(&busy), "idempotency_in_progress", "{busy}");
    let order = first["data"]["startCheckout"]["order"]["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let txn = first["data"]["startCheckout"]["transactionId"]
        .as_str()
        .unwrap()
        .to_owned();
    assert_eq!(w.count("SELECT count(*) FROM shop_orders").await, 1);

    // Paid meanwhile; the retry still answers the first checkout, and nothing new starts.
    assert_eq!(
        w.hook(completed("evt_key1", &txn, &order)).await,
        StatusCode::OK
    );
    let calls = w.fake.calls.lock().unwrap().len();
    let again = w.gql(Some(&buyer), CHECKOUT_KEYED, vars.clone()).await;
    assert!(again["errors"].is_null(), "{again}");
    assert_eq!(
        again["data"]["startCheckout"]["order"]["id"],
        order.as_str()
    );
    assert_eq!(
        again["data"]["startCheckout"]["transactionId"],
        txn.as_str()
    );
    assert_eq!(w.fake.calls.lock().unwrap().len(), calls);
    assert_eq!(w.count("SELECT count(*) FROM shop_orders").await, 1);
    // Without the key the same purchase is refused: they own it now.
    let r = w
        .gql(Some(&buyer), CHECKOUT, json!({ "itemId": item["id"] }))
        .await;
    assert_eq!(code(&r), "already_owned", "{r}");

    // The same key for another item: refused, nothing started.
    let r = w
        .gql(
            Some(&buyer),
            CHECKOUT_KEYED,
            json!({ "itemId": second["id"], "key": key }),
        )
        .await;
    assert_eq!(code(&r), "idempotency_key_reused", "{r}");
    assert_eq!(w.count("SELECT count(*) FROM shop_orders").await, 1);
    // Keys are per account: someone else's request with it is their own.
    let r = w.gql(Some(&other), CHECKOUT_KEYED, vars.clone()).await;
    assert!(r["errors"].is_null(), "{r}");
    assert_ne!(r["data"]["startCheckout"]["order"]["id"], order.as_str());
    // Not a UUID.
    let r = w
        .gql(
            Some(&buyer),
            CHECKOUT_KEYED,
            json!({ "itemId": item["id"], "key": "twice" }),
        )
        .await;
    assert_eq!(code(&r), "bad_idempotency_key", "{r}");

    // A request that fails gives its key back: the retry acts.
    let failing = ids::new_id().to_string();
    w.fake.fail.store(true, Ordering::SeqCst);
    let r = w
        .gql(
            Some(&buyer),
            CHECKOUT_KEYED,
            json!({ "itemId": second["id"], "key": failing }),
        )
        .await;
    assert!(!r["errors"].is_null(), "{r}");
    w.fake.fail.store(false, Ordering::SeqCst);
    let r = w
        .gql(
            Some(&buyer),
            CHECKOUT_KEYED,
            json!({ "itemId": second["id"], "key": failing }),
        )
        .await;
    assert!(r["errors"].is_null(), "{r}");
    assert_eq!(r["data"]["startCheckout"]["order"]["status"], "PENDING");

    // Kept 24 hours: an older key is new again (the worker removes it).
    w.sql(&format!(
        "UPDATE idempotency_keys SET created_at = now() - interval '25 hours' WHERE key = '{key}'"
    ))
    .await;
    let r = w.gql(Some(&buyer), CHECKOUT_KEYED, vars).await;
    assert_eq!(code(&r), "already_owned", "{r}");
    w.db.close().await;
}

#[tokio::test]
async fn a_redeem_retried_with_its_key_redeems_once() {
    let Some(w) = World::new(false).await else {
        return;
    };
    let manager = w.staff("981", &["codes.manage"]).await;
    let alice = w.user("982").await;
    let badge = w.badge("Early").await;
    let input = json!({ "name": "Launch", "prefix": "launch", "count": 2, "usesPerCode": 5, "badgeIds": [badge] });
    let r = w
        .gql(Some(&manager), CREATE_BATCH, json!({ "input": input }))
        .await;
    let codes: Vec<String> =
        serde_json::from_value(r["data"]["createCodeBatch"]["codes"].clone()).unwrap();
    assert_eq!(codes.len(), 2, "{r}");
    let key = ids::new_id().to_string();

    let first = w
        .gql(
            Some(&alice),
            REDEEM_KEYED,
            json!({ "code": codes[0], "key": key }),
        )
        .await;
    assert!(first["errors"].is_null(), "{first}");
    assert_eq!(
        first["data"]["redeemCode"]["badges"][0]["id"],
        badge.as_str()
    );
    // The retry (typed differently, the same code) answers the same, and redeems nothing more.
    let again = w
        .gql(
            Some(&alice),
            REDEEM_KEYED,
            json!({ "code": codes[0].to_lowercase(), "key": key }),
        )
        .await;
    assert_eq!(again["data"], first["data"], "{again}");
    assert_eq!(w.count("SELECT count(*) FROM code_redemptions").await, 1);
    assert_eq!(
        w.count("SELECT sum(uses)::bigint FROM redeem_codes").await,
        1
    );
    assert_eq!(w.audit_count("code.redeemed").await, 1);
    // Without the key it's a second try, and says so.
    let r = w
        .gql(Some(&alice), REDEEM, json!({ "code": codes[0] }))
        .await;
    assert_eq!(code(&r), "already_redeemed", "{r}");
    // Another code with the same key: refused, not redeemed.
    let r = w
        .gql(
            Some(&alice),
            REDEEM_KEYED,
            json!({ "code": codes[1], "key": key }),
        )
        .await;
    assert_eq!(code(&r), "idempotency_key_reused", "{r}");
    assert_eq!(w.count("SELECT count(*) FROM code_redemptions").await, 1);
    // Only a hash of the request is kept, never the code.
    let dump: String = w
        .sql_text(
            "SELECT coalesce(string_agg(row_to_json(k)::text, ' '), '') FROM idempotency_keys k",
        )
        .await;
    for c in &codes {
        let body = c.trim_start_matches("LAUNCH-").replace('-', "");
        assert!(!dump.contains(&body), "{c} stored with its key");
    }
    w.db.close().await;
}
