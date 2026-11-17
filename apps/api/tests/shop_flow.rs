//! The cosmetics shop and redeem codes (ADR 0016) end to end through the real router against a
//! real Postgres, with a fake Paddle API on a local port: item CRUD and permissions, checkout
//! unavailable without Paddle, the webhook's signature (valid, invalid, replayed, stale),
//! idempotent fulfilment, gifts, owning already, stock, refunds that revoke and clear the
//! selection, and redeem codes (happy path, wrong, expired, used up, disabled, lockouts, hashing).
//! Runs when `CYEX_TV_TEST_DATABASE_URL` is set (CI sets it) and skips otherwise. Sandbox shapes
//! only; no real money and no real Paddle.

mod common;

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use axum::Router;
use axum::body::Body;
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

// A fake Paddle API: answers transactions and adjustments, remembers what it was sent.

#[derive(Default)]
struct Fake {
    calls: Mutex<Vec<(String, Value)>>,
    fail: AtomicBool,
    next: AtomicUsize,
}

async fn fake_paddle() -> (String, Arc<Fake>) {
    use axum::extract::State;
    use axum::routing::post;
    let fake = Arc::new(Fake::default());
    async fn answer(fake: &Fake, path: &str, body: Value) -> (StatusCode, axum::Json<Value>) {
        fake.calls.lock().unwrap().push((path.to_owned(), body));
        if fake.fail.load(Ordering::SeqCst) {
            return (
                StatusCode::BAD_REQUEST,
                axum::Json(json!({ "error": { "code": "bad_request", "type": "request_error" } })),
            );
        }
        let n = fake.next.fetch_add(1, Ordering::SeqCst);
        let data = if path == "/transactions" {
            json!({ "id": format!("txn_test{n}"), "status": "ready",
                    "checkout": { "url": format!("https://pay.example.test/checkout?_ptxn=txn_test{n}") } })
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
            SessionKind::App,
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

    async fn gql(&self, user: Option<&User>, query: &str, variables: Value) -> Value {
        let mut request = Request::post("/v1/gql").header("content-type", "application/json");
        if let Some(user) = user {
            request = request.header("authorization", format!("Bearer {}", user.token));
        }
        let request = request
            .body(Body::from(
                json!({ "query": query, "variables": variables }).to_string(),
            ))
            .unwrap();
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
        "data": { "id": format!("adj_{event_id}"), "action": action, "status": status, "transaction_id": txn }
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
    assert_eq!(gift["recipient"]["id"], friend.tid().as_str());
    assert_eq!(gift["buyer"]["id"], buyer.tid().as_str());
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

    // A chargeback from Paddle refunds the second purchase; a pending refund doesn't.
    let (txn2, order2) = buy("evt_c").await;
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
    // Twenty from one IP (here every test request comes from the same "unknown" peer) lock it for
    // everyone there. 5 + 4 + 4 + 4 + 3 = 20.
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
