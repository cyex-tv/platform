//! Owner's answers of 2026/11/19: unlisted emotes (ADR 0020) and your own hidden roles. Through
//! the real router against a real Postgres; runs when `CYEX_TV_TEST_DATABASE_URL` is set (CI sets
//! it) and skips otherwise.

mod common;

use std::sync::Arc;

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use chrono::Duration;
use common::TestDb;
use cyex_tv_api::state::AppState;
use cyex_tv_auth::{Auth, AuthConfig, tokens};
use cyex_tv_db::accounts::{self, Profile};
use cyex_tv_db::emotes::{self, NewEmote};
use cyex_tv_db::sessions::{self, SessionKind};
use cyex_tv_db::{channels, ids, roles};
use cyex_tv_shared::model::{EmoteState, Platform};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use tower::ServiceExt;
use uuid::Uuid;

struct User {
    id: Uuid,
    token: String,
}

struct World {
    db: TestDb,
    app: Router,
}

fn code(r: &Value) -> &str {
    r["errors"][0]["extensions"]["code"]
        .as_str()
        .unwrap_or_default()
}

impl World {
    async fn new() -> Option<Self> {
        let db = TestDb::new().await?;
        let cfg = AuthConfig::from_source(|_| None).unwrap();
        let auth = Arc::new(Auth::new(cfg).unwrap());
        let state = AppState::new(db.pool.clone(), auth);
        Some(Self {
            app: cyex_tv_api::app(state),
            db,
        })
    }

    async fn user(&self, twitch_id: &str) -> User {
        let login = format!("user{twitch_id}");
        let mut conn = self.db.conn().await;
        let id = accounts::sign_in(
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
        .unwrap()
        .account
        .id;
        channels::upsert(&mut conn, id, Platform::Twitch, twitch_id, &login)
            .await
            .unwrap();
        let token = tokens::random_token();
        sessions::create_with(
            &mut conn,
            &tokens::hash(&token),
            id,
            SessionKind::Site,
            Duration::days(1),
            None,
        )
        .await
        .unwrap();
        User { id, token }
    }

    async fn gql(&self, who: Option<&User>, query: &str, variables: Value) -> Value {
        let mut request = Request::post("/v1/gql").header("content-type", "application/json");
        if let Some(u) = who {
            request = request
                .header(
                    "cookie",
                    format!("{}={}", cyex_tv_auth::cookies::SESSION, u.token),
                )
                .header("origin", "https://cyex.tv");
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

    async fn approved(&self, owner: &User, name: &str) -> String {
        let mut conn = self.db.conn().await;
        let e = emotes::create(
            &mut conn,
            &NewEmote {
                owner_account_id: owner.id,
                name,
                ..NewEmote::default()
            },
        )
        .await
        .unwrap();
        emotes::set_state(&mut conn, e.id, EmoteState::Approved)
            .await
            .unwrap();
        ids::to_text(e.id)
    }

    async fn set(&self, owner: &User, name: &str) -> String {
        let r = self
            .gql(
                Some(owner),
                "mutation($n: String!) { createSet(input: { name: $n, kind: CHANNEL }) { id } }",
                json!({ "n": name }),
            )
            .await;
        r["data"]["createSet"]["id"]
            .as_str()
            .unwrap_or_else(|| panic!("{r}"))
            .to_owned()
    }

    async fn add(&self, by: &User, set: &str, emote: &str) -> Value {
        self.gql(
            Some(by),
            "mutation($s: String!, $e: String!) { addSetEmote(setId: $s, emoteId: $e) { name } }",
            json!({ "s": set, "e": emote }),
        )
        .await
    }
}

const UPDATE: &str = "mutation($id: String!, $i: UpdateEmoteInput!) {
    updateEmote(id: $id, input: $i) { visibility flags { private } }
}";

#[tokio::test]
async fn unlisted_emotes_open_by_link_but_stay_out_of_listings() {
    let Some(w) = World::new().await else { return };
    let ann = w.user("6101").await;
    let ben = w.user("6102").await;
    let shown = w.approved(&ann, "AnnShown").await;
    let quiet = w.approved(&ann, "AnnQuiet").await;

    // New emotes are public.
    let r = w
        .gql(
            None,
            "query($e: String!) { emote(id: $e) { visibility flags { private } } }",
            json!({ "e": quiet }),
        )
        .await;
    assert_eq!(r["data"]["emote"]["visibility"], "PUBLIC", "{r}");

    // Unlisted through `visibility`; `private` stays false.
    let r = w
        .gql(
            Some(&ann),
            UPDATE,
            json!({ "id": quiet, "i": { "visibility": "UNLISTED" } }),
        )
        .await;
    assert_eq!(r["data"]["updateEmote"]["visibility"], "UNLISTED", "{r}");
    assert_eq!(r["data"]["updateEmote"]["flags"]["private"], false);
    // An old client's `private: false` leaves it unlisted; the two can't disagree.
    let r = w
        .gql(
            Some(&ann),
            UPDATE,
            json!({ "id": quiet, "i": { "private": false } }),
        )
        .await;
    assert_eq!(r["data"]["updateEmote"]["visibility"], "UNLISTED", "{r}");
    let r = w
        .gql(
            Some(&ann),
            UPDATE,
            json!({ "id": quiet, "i": { "private": true, "visibility": "PUBLIC" } }),
        )
        .await;
    assert_eq!(code(&r), "bad_visibility", "{r}");

    // Reachable by link, for anyone.
    let r = w
        .gql(
            None,
            "query($e: String!) { emote(id: $e) { name visibility } }",
            json!({ "e": quiet }),
        )
        .await;
    assert_eq!(r["data"]["emote"]["name"], "AnnQuiet", "{r}");
    assert_eq!(r["data"]["emote"]["visibility"], "UNLISTED");

    // Not in search or browse, nor on the creator's page (search by owner), for anyone.
    for (q, owner) in [("Ann", Value::Null), ("", json!(ids::to_text(ann.id)))] {
        for who in [None, Some(&ann), Some(&ben)] {
            let r = w
                .gql(
                    who,
                    "query($q: String!, $o: String) { searchEmotes(query: $q, owner: $o) { items { id } } }",
                    json!({ "q": q, "o": owner }),
                )
                .await;
            let found: Vec<&str> = r["data"]["searchEmotes"]["items"]
                .as_array()
                .unwrap_or_else(|| panic!("{r}"))
                .iter()
                .filter_map(|e| e["id"].as_str())
                .collect();
            assert!(found.contains(&shown.as_str()), "{r}");
            assert!(!found.contains(&quiet.as_str()), "{r}");
        }
    }
    // Its owner still has it in their own list.
    let r = w
        .gql(
            Some(&ann),
            "{ myEmotes { items { id visibility } } }",
            json!({}),
        )
        .await;
    assert!(r.to_string().contains(&quiet), "{r}");

    // Anyone with the link can add it to a set.
    let bens = w.set(&ben, "Ben's").await;
    let r = w.add(&ben, &bens, &quiet).await;
    assert!(r["errors"].is_null(), "{r}");

    // Ann's activity feed shows the public emote she added, never the unlisted one.
    let anns = w.set(&ann, "Ann's").await;
    for e in [&shown, &quiet] {
        let r = w.add(&ann, &anns, e).await;
        assert!(r["errors"].is_null(), "{r}");
    }
    let r = w
        .gql(
            None,
            "query($a: String!) { accountActivity(accountId: $a) { kind emote { id } } }",
            json!({ "a": ids::to_text(ann.id) }),
        )
        .await;
    let text = r["data"]["accountActivity"].to_string();
    assert!(text.contains(&shown), "{r}");
    assert!(!text.contains(&quiet), "{r}");

    // Private still means only its owner can add it.
    let r = w
        .gql(
            Some(&ann),
            UPDATE,
            json!({ "id": quiet, "i": { "visibility": "PRIVATE" } }),
        )
        .await;
    assert_eq!(r["data"]["updateEmote"]["visibility"], "PRIVATE", "{r}");
    assert_eq!(r["data"]["updateEmote"]["flags"]["private"], true);
    let other = w.set(&ben, "Ben's other").await;
    let r = w.add(&ben, &other, &quiet).await;
    assert_eq!(code(&r), "emote_private", "{r}");
    // And an old client's `private: false` makes a private emote public again.
    let r = w
        .gql(
            Some(&ann),
            UPDATE,
            json!({ "id": quiet, "i": { "private": false } }),
        )
        .await;
    assert_eq!(r["data"]["updateEmote"]["visibility"], "PUBLIC", "{r}");
    w.db.close().await;
}

#[tokio::test]
async fn your_profile_shows_your_hidden_roles_to_you_only() {
    let Some(w) = World::new().await else { return };
    let ann = w.user("6201").await;
    let ben = w.user("6202").await;
    {
        let mut conn = w.db.conn().await;
        for (name, public) in [("Shown", true), ("Quiet", false)] {
            let role = roles::create(
                &mut conn,
                &roles::NewRole {
                    name,
                    description: None,
                    position: if public { 20 } else { 10 },
                    permissions: &[],
                    created_by: None,
                },
            )
            .await
            .unwrap();
            roles::set_public(&mut conn, role.id, public).await.unwrap();
            roles::assign(&mut conn, ann.id, role.id, None)
                .await
                .unwrap();
        }
    }
    let r = w
        .gql(Some(&ann), "{ me { roles { name public } } }", json!({}))
        .await;
    assert_eq!(
        r["data"]["me"]["roles"],
        json!([{ "name": "Shown", "public": true }, { "name": "Quiet", "public": false }]),
        "{r}"
    );
    // Others, signed in or not, see the public one only.
    for who in [None, Some(&ben)] {
        let r = w
            .gql(
                who,
                "query($a: String!) { accountProfile(accountId: $a) { roles { name } } }",
                json!({ "a": ids::to_text(ann.id) }),
            )
            .await;
        assert_eq!(
            r["data"]["accountProfile"]["roles"],
            json!([{ "name": "Shown" }]),
            "{r}"
        );
    }
    // Ben's own `me` lists none of Ann's.
    let r = w
        .gql(Some(&ben), "{ me { roles { name } } }", json!({}))
        .await;
    assert_eq!(r["data"]["me"]["roles"], json!([]), "{r}");
    w.db.close().await;
}
