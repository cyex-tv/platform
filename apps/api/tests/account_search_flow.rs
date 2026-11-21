//! User search (`searchAccounts`): matches on Twitch and Kick logins and display names, ranking,
//! public fields only, blocked and deleted accounts never listed, paging and the rate limit.
//! Through the real router against a real Postgres; runs when `CYEX_TV_TEST_DATABASE_URL` is set
//! (CI sets it) and skips otherwise.

mod common;

use std::sync::Arc;

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use common::TestDb;
use cyex_tv_api::limits::{Limits, Rates};
use cyex_tv_api::state::AppState;
use cyex_tv_auth::{Auth, AuthConfig, tokens};
use cyex_tv_db::accounts::{self, Profile};
use cyex_tv_db::sessions::{self, SessionKind};
use cyex_tv_shared::model::Platform;
use cyex_tv_shared::ratelimit::Rate;
use http_body_util::BodyExt;
use serde_json::{Value, json};
use tower::ServiceExt;
use uuid::Uuid;

struct World {
    db: TestDb,
    app: Router,
}

const SEARCH: &str = "query($q: String!, $p: Int, $n: Int) {
    searchAccounts(query: $q, page: $p, perPage: $n) {
        total page perPage
        items { account { id displayName avatar } identities { platform login displayName } }
    }
}";

fn code(response: &Value) -> &str {
    response["errors"][0]["extensions"]["code"]
        .as_str()
        .unwrap_or_default()
}

impl World {
    async fn new(rates: Rates) -> Option<Self> {
        let db = TestDb::new().await?;
        let auth = Arc::new(Auth::new(AuthConfig::from_source(|_| None).unwrap()).unwrap());
        let state = AppState::new(db.pool.clone(), auth).with_limits(Limits::new(rates, false));
        Some(Self {
            app: cyex_tv_api::app(state),
            db,
        })
    }

    async fn account(&self, platform: Platform, id: &str, login: &str, display: &str) -> Uuid {
        let mut conn = self.db.conn().await;
        accounts::sign_in(
            &mut conn,
            Profile {
                platform,
                platform_user_id: id,
                login,
                display_name: display,
                avatar: None,
            },
        )
        .await
        .unwrap()
        .account
        .id
    }

    async fn token(&self, account: Uuid) -> String {
        let mut conn = self.db.conn().await;
        let token = tokens::random_token();
        sessions::create(
            &mut conn,
            &tokens::hash(&token),
            account,
            SessionKind::Site,
            chrono::Duration::days(1),
        )
        .await
        .unwrap();
        token
    }

    async fn search(&self, token: Option<&str>, variables: Value) -> Value {
        let mut request = Request::post("/v1/gql").header("content-type", "application/json");
        if let Some(token) = token {
            request = request
                .header(
                    "cookie",
                    format!("{}={token}", cyex_tv_auth::cookies::SESSION),
                )
                .header("origin", "https://cyex.tv");
        }
        let request = request
            .body(Body::from(
                json!({ "query": SEARCH, "variables": variables }).to_string(),
            ))
            .unwrap();
        let response = self.app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        serde_json::from_slice(&bytes).unwrap()
    }
}

fn ids_of(r: &Value) -> Vec<String> {
    r["data"]["searchAccounts"]["items"]
        .as_array()
        .unwrap_or_else(|| panic!("{r}"))
        .iter()
        .map(|i| i["account"]["id"].as_str().unwrap().to_owned())
        .collect()
}

#[tokio::test]
async fn finds_accounts_by_login_and_name() {
    let Some(w) = World::new(Rates::default()).await else {
        return;
    };
    let fox = w
        .account(Platform::Twitch, "3001", "lumen_fox", "Lumen_Fox")
        .await;
    // The same person on Kick: one hit, both logins.
    {
        let mut conn = w.db.conn().await;
        accounts::link(
            &mut conn,
            fox,
            Profile {
                platform: Platform::Kick,
                platform_user_id: "9001",
                login: "lumenkick",
                display_name: "LumenKick",
                avatar: None,
            },
        )
        .await
        .unwrap();
    }
    let foxy = w
        .account(Platform::Twitch, "3002", "lumen_foxy_fan", "FoxyFan")
        .await;
    let other = w
        .account(Platform::Kick, "3003", "quietoak", "Quiet Lumen")
        .await;
    let _unrelated = w.account(Platform::Twitch, "3004", "zzz", "zzz").await;

    // Exact login first, then the login prefix, then a display-name match.
    let r = w.search(None, json!({ "q": "lumen_fox" })).await;
    let found = ids_of(&r);
    assert_eq!(found[0], w.db.public(fox).await, "{r}");
    assert!(found.contains(&w.db.public(foxy).await));
    let r = w.search(None, json!({ "q": "@LUMEN" })).await;
    let found = ids_of(&r);
    assert_eq!(found.len(), 3, "{r}");
    assert!(
        found.contains(&w.db.public(other).await),
        "display names match too"
    );

    // Public fields only: platform, login and name; no platform IDs.
    let r = w.search(None, json!({ "q": "lumenkick" })).await;
    let hit = &r["data"]["searchAccounts"]["items"][0];
    assert_eq!(hit["account"]["id"], w.db.public(fox).await.as_str(), "{r}");
    assert_eq!(
        hit["identities"],
        json!([
            { "platform": "TWITCH", "login": "lumen_fox", "displayName": "Lumen_Fox" },
            { "platform": "KICK", "login": "lumenkick", "displayName": "LumenKick" }
        ])
    );
    let text = r.to_string();
    assert!(!text.contains("3001") && !text.contains("9001"), "{text}");

    // Paging.
    let r = w
        .search(None, json!({ "q": "lumen", "p": 2, "n": 2 }))
        .await;
    let page = &r["data"]["searchAccounts"];
    assert_eq!(page["total"], 3, "{r}");
    assert_eq!(page["page"], 2);
    assert_eq!(page["perPage"], 2);
    assert_eq!(page["items"].as_array().unwrap().len(), 1);

    // Nothing for nothing, and `%` and `_` mean themselves.
    let r = w.search(None, json!({ "q": "   " })).await;
    assert_eq!(r["data"]["searchAccounts"]["total"], 0, "{r}");
    let r = w.search(None, json!({ "q": "%" })).await;
    assert_eq!(r["data"]["searchAccounts"]["total"], 0, "{r}");
    w.db.close().await;
}

#[tokio::test]
async fn blocked_and_deleted_accounts_are_never_listed() {
    let Some(w) = World::new(Rates::default()).await else {
        return;
    };
    let blocked = w
        .account(Platform::Twitch, "3101", "hidden_one", "hidden_one")
        .await;
    let deleted = w
        .account(Platform::Twitch, "3102", "hidden_two", "hidden_two")
        .await;
    let live = w
        .account(Platform::Twitch, "3103", "hidden_three", "hidden_three")
        .await;
    {
        let mut conn = w.db.conn().await;
        accounts::set_blocked(&mut conn, blocked, true)
            .await
            .unwrap();
        sqlx::query("UPDATE accounts SET deleted_at = now() WHERE id = $1")
            .bind(deleted)
            .execute(&mut *conn)
            .await
            .unwrap();
    }
    let token = w.token(live).await;
    for t in [None, Some(token.as_str())] {
        let r = w.search(t, json!({ "q": "hidden" })).await;
        assert_eq!(ids_of(&r), vec![w.db.public(live).await], "{r}");
        assert_eq!(r["data"]["searchAccounts"]["total"], 1);
        // Even the exact login finds only similar live names, never the blocked account.
        let r = w.search(t, json!({ "q": "hidden_one" })).await;
        let found = ids_of(&r);
        assert!(!found.contains(&w.db.public(blocked).await), "{r}");
        assert!(!found.contains(&w.db.public(deleted).await), "{r}");
    }
    w.db.close().await;
}

#[tokio::test]
async fn searches_are_rate_limited() {
    let Some(w) = World::new(Rates {
        account_search: Rate::per_minute(3),
        ..Rates::default()
    })
    .await
    else {
        return;
    };
    let a = w.account(Platform::Twitch, "3201", "alpha", "alpha").await;
    let b = w.account(Platform::Twitch, "3202", "bravo", "bravo").await;
    let ta = w.token(a).await;
    let tb = w.token(b).await;
    for _ in 0..3 {
        let r = w.search(Some(&ta), json!({ "q": "alpha" })).await;
        assert!(r["errors"].is_null(), "{r}");
    }
    let r = w.search(Some(&ta), json!({ "q": "alpha" })).await;
    assert_eq!(code(&r), "rate_limited", "{r}");
    // Per account: someone else still searches.
    let r = w.search(Some(&tb), json!({ "q": "alpha" })).await;
    assert!(r["errors"].is_null(), "{r}");
    // Signed out, per IP.
    for _ in 0..3 {
        let r = w.search(None, json!({ "q": "alpha" })).await;
        assert!(r["errors"].is_null(), "{r}");
    }
    let r = w.search(None, json!({ "q": "alpha" })).await;
    assert_eq!(code(&r), "rate_limited", "{r}");
    w.db.close().await;
}

#[tokio::test]
async fn search_past_the_last_page_is_empty() {
    let Some(w) = World::new(Rates::default()).await else {
        return;
    };
    for i in 0..3 {
        w.account(
            Platform::Twitch,
            &format!("340{i}"),
            &format!("pagefan{i}"),
            &format!("PageFan{i}"),
        )
        .await;
    }
    // The last page holds one; the one after it holds none, and the total stays right.
    let r = w
        .search(None, json!({ "q": "pagefan", "p": 2, "n": 2 }))
        .await;
    assert_eq!(ids_of(&r).len(), 1, "{r}");
    let r = w
        .search(None, json!({ "q": "pagefan", "p": 3, "n": 2 }))
        .await;
    let page = &r["data"]["searchAccounts"];
    assert_eq!(page["items"].as_array().unwrap().len(), 0, "{r}");
    assert_eq!(page["total"], 3, "{r}");
    assert_eq!(page["page"], 3, "{r}");
    // Far past the counted window too: nothing, not the last page again.
    let r = w
        .search(None, json!({ "q": "pagefan", "p": 100000, "n": 50 }))
        .await;
    assert_eq!(ids_of(&r).len(), 0, "{r}");
    assert_eq!(r["data"]["searchAccounts"]["total"], 3, "{r}");
    w.db.close().await;
}
