//! Owner feedback 2026/11/19, section D: removing connections (never the last), the display
//! name choice, signing out everywhere, and the activity feed's filters (group, who, when) with
//! its editor lines. Through the real router against a real Postgres; runs when
//! `CYEX_TV_TEST_DATABASE_URL` is set (CI sets it) and skips otherwise.

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
use cyex_tv_db::app_auth;
use cyex_tv_db::emotes::{self, NewEmote};
use cyex_tv_db::sessions::{self, SessionKind};
use cyex_tv_db::{channels, ids};
use cyex_tv_shared::model::{EmoteState, Platform};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use tower::ServiceExt;
use uuid::Uuid;

const SITE: &str = "http://localhost:5173";

const ME: &str = "{ me { id displayPlatform identities { platform login displayName } } }";
const UNLINK: &str = "mutation($p: Platform!) { unlinkIdentity(platform: $p) { id } }";
const DISPLAY: &str = "mutation($p: Platform!) {
    setDisplayPlatform(platform: $p) { displayPlatform identities { platform } }
}";
const PROFILE: &str =
    "query($a: String!) { accountProfile(accountId: $a) { account { displayName } } }";
const SEARCH: &str =
    "query($q: String!) { searchAccounts(query: $q) { items { account { id displayName } } } }";
const FEED: &str = "query($a: String!, $f: ActivityFilter) {
    accountActivityPage(accountId: $a, filter: $f) {
        entries { kind actor { id } editor { id displayName } createdAt }
        next
    }
}";
const ADD_EDITOR: &str = "mutation($a: String!) {
    addAccountEditor(input: { accountId: $a, permissions: [EMOTES] }) { permissions }
}";
const REMOVE_EDITOR: &str = "mutation($a: String!) { removeAccountEditor(accountId: $a) }";
const CREATE_SET: &str =
    "mutation($n: String!) { createSet(input: { name: $n, kind: CHANNEL }) { id } }";
const ADD_EMOTE: &str =
    "mutation($s: String!, $e: String!) { addSetEmote(setId: $s, emoteId: $e) { name } }";

struct User {
    id: Uuid,
    token: String,
    cookie: String,
}

impl User {
    fn tid(&self) -> String {
        ids::to_text(self.id)
    }
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
        let cfg = AuthConfig::from_source(|key| match key {
            "CYEX_TV_SITE_URL" => Some(SITE.to_owned()),
            _ => None,
        })
        .unwrap();
        let auth = Arc::new(Auth::new(cfg).unwrap());
        let state = AppState::new(db.pool.clone(), auth);
        Some(Self {
            app: cyex_tv_api::app(state),
            db,
        })
    }

    async fn session(&self, account: Uuid, kind: SessionKind) -> String {
        let mut conn = self.db.conn().await;
        let token = tokens::random_token();
        match kind {
            SessionKind::Site => {
                sessions::create(
                    &mut conn,
                    &tokens::hash(&token),
                    account,
                    kind,
                    Duration::days(1),
                )
                .await
                .unwrap();
            }
            // An app session names its client, as an app sign-in makes it.
            SessionKind::App => {
                app_auth::create_session(
                    &mut conn,
                    &tokens::hash(&token),
                    account,
                    "cyex-deck",
                    Duration::days(1),
                    Duration::days(1),
                    None,
                )
                .await
                .unwrap();
            }
        }
        token
    }

    /// A Twitch user `tw<id>` with a channel, an app session and a site session.
    async fn user(&self, twitch_id: &str) -> User {
        let login = format!("tw{twitch_id}");
        let id = {
            let mut conn = self.db.conn().await;
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
            channels::upsert(
                &mut conn,
                signed.account.id,
                Platform::Twitch,
                twitch_id,
                &login,
            )
            .await
            .unwrap();
            signed.account.id
        };
        User {
            id,
            token: self.session(id, SessionKind::App).await,
            cookie: self.session(id, SessionKind::Site).await,
        }
    }

    async fn link_kick(&self, user: &User, kick_id: &str, login: &str) {
        let mut conn = self.db.conn().await;
        accounts::link(
            &mut conn,
            user.id,
            Profile {
                platform: Platform::Kick,
                platform_user_id: kick_id,
                login,
                display_name: login,
                avatar: None,
            },
        )
        .await
        .unwrap();
    }

    async fn post(
        &self,
        auth: Option<(&str, bool)>,
        query: &str,
        vars: Value,
    ) -> (Value, Option<String>) {
        let mut request = Request::post("/v1/gql").header("content-type", "application/json");
        match auth {
            Some((token, false)) => {
                request = request.header("authorization", format!("Bearer {token}"));
            }
            Some((token, true)) => {
                request = request
                    .header("cookie", format!("cyex_tv_session={token}"))
                    .header("origin", SITE);
            }
            None => {}
        }
        let request = request
            .body(Body::from(
                json!({ "query": query, "variables": vars }).to_string(),
            ))
            .unwrap();
        let response = self.app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let cookie = response
            .headers()
            .get("set-cookie")
            .and_then(|v| v.to_str().ok())
            .map(str::to_owned);
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        (serde_json::from_slice(&bytes).unwrap(), cookie)
    }

    /// As the website (the cookie session): full rights.
    async fn gql(&self, user: &User, query: &str, vars: Value) -> Value {
        self.post(Some((&user.cookie, true)), query, vars).await.0
    }

    async fn public(&self, query: &str, vars: Value) -> Value {
        self.post(None, query, vars).await.0
    }

    async fn feed(&self, account: &User, filter: Value) -> Vec<Value> {
        let r = self
            .public(FEED, json!({ "a": account.tid(), "f": filter }))
            .await;
        r["data"]["accountActivityPage"]["entries"]
            .as_array()
            .unwrap_or_else(|| panic!("{r}"))
            .clone()
    }

    async fn kinds(&self, account: &User, filter: Value) -> Vec<String> {
        self.feed(account, filter)
            .await
            .iter()
            .map(|e| e["kind"].as_str().unwrap().to_owned())
            .collect()
    }

    async fn approved_emote(&self, owner: &User, name: &str) -> String {
        let mut conn = self.db.conn().await;
        let emote = emotes::create(
            &mut conn,
            &NewEmote {
                owner_account_id: owner.id,
                name,
                ..NewEmote::default()
            },
        )
        .await
        .unwrap();
        emotes::set_state(&mut conn, emote.id, EmoteState::Approved)
            .await
            .unwrap();
        self.db.public(emote.id).await
    }
}

#[tokio::test]
async fn a_connection_can_go_but_never_the_last() {
    let Some(w) = World::new().await else { return };
    let alice = w.user("7001").await;

    // One connection: it can't go.
    let r = w.gql(&alice, UNLINK, json!({ "p": "TWITCH" })).await;
    assert_eq!(code(&r), "last_identity", "{r}");

    // Two: either can go, then the other is the last again.
    w.link_kick(&alice, "8001", "alice_kick").await;
    let r = w.gql(&alice, UNLINK, json!({ "p": "TWITCH" })).await;
    assert!(r["errors"].is_null(), "{r}");
    let r = w.gql(&alice, ME, json!({})).await;
    assert_eq!(
        r["data"]["me"]["identities"].as_array().unwrap().len(),
        1,
        "{r}"
    );
    assert_eq!(r["data"]["me"]["identities"][0]["platform"], "KICK");
    let r = w.gql(&alice, UNLINK, json!({ "p": "KICK" })).await;
    assert_eq!(code(&r), "last_identity", "{r}");
    // A platform that isn't linked.
    let r = w.gql(&alice, UNLINK, json!({ "p": "TWITCH" })).await;
    assert_eq!(code(&r), "not_found", "{r}");

    // A restricted account keeps both (its identity hold would be lost otherwise).
    let bob = w.user("7002").await;
    w.link_kick(&bob, "8002", "bob_kick").await;
    {
        let mut conn = w.db.conn().await;
        accounts::set_restricted(&mut conn, bob.id, true)
            .await
            .unwrap();
    }
    let r = w.gql(&bob, UNLINK, json!({ "p": "KICK" })).await;
    assert_eq!(code(&r), "restricted", "{r}");
    w.db.close().await;
}

#[tokio::test]
async fn the_display_name_is_the_chosen_connection_and_both_logins_find_you() {
    let Some(w) = World::new().await else { return };
    let alice = w.user("7101").await;

    // One connection: the default, and choosing an unlinked platform is refused.
    let r = w.gql(&alice, ME, json!({})).await;
    assert!(r["data"]["me"]["displayPlatform"].is_null(), "{r}");
    let r = w.gql(&alice, DISPLAY, json!({ "p": "KICK" })).await;
    assert_eq!(code(&r), "not_found", "{r}");

    w.link_kick(&alice, "8101", "alicekick").await;
    // Twitch first by default.
    let r = w.public(PROFILE, json!({ "a": alice.tid() })).await;
    assert_eq!(
        r["data"]["accountProfile"]["account"]["displayName"],
        "tw7101"
    );

    let r = w.gql(&alice, DISPLAY, json!({ "p": "KICK" })).await;
    assert!(r["errors"].is_null(), "{r}");
    assert_eq!(r["data"]["setDisplayPlatform"]["displayPlatform"], "KICK");
    assert_eq!(
        r["data"]["setDisplayPlatform"]["identities"][0]["platform"],
        "KICK"
    );
    let r = w.public(PROFILE, json!({ "a": alice.tid() })).await;
    assert_eq!(
        r["data"]["accountProfile"]["account"]["displayName"],
        "alicekick"
    );

    // Either login still finds her, shown by the chosen name.
    for q in ["tw7101", "alicekick"] {
        let r = w.public(SEARCH, json!({ "q": q })).await;
        let items = r["data"]["searchAccounts"]["items"]
            .as_array()
            .unwrap_or_else(|| panic!("{r}"));
        let alice_public = w.db.public(alice.id).await;
        let hit = items
            .iter()
            .find(|i| i["account"]["id"] == alice_public.as_str())
            .unwrap_or_else(|| panic!("{q}: {r}"));
        assert_eq!(hit["account"]["displayName"], "alicekick");
    }

    // Unlinking the chosen platform goes back to the default.
    let r = w.gql(&alice, UNLINK, json!({ "p": "KICK" })).await;
    assert!(r["errors"].is_null(), "{r}");
    let r = w.gql(&alice, ME, json!({})).await;
    assert!(r["data"]["me"]["displayPlatform"].is_null(), "{r}");
    let r = w.public(PROFILE, json!({ "a": alice.tid() })).await;
    assert_eq!(
        r["data"]["accountProfile"]["account"]["displayName"],
        "tw7101"
    );

    // Signed out: no.
    let r = w.public(DISPLAY, json!({ "p": "TWITCH" })).await;
    assert_eq!(code(&r), "signed_out", "{r}");
    w.db.close().await;
}

#[tokio::test]
async fn sign_out_everywhere_ends_every_session_and_clears_the_cookie() {
    let Some(w) = World::new().await else { return };
    let alice = w.user("7201").await;
    let bob = w.user("7202").await;

    let (r, cookie) = w
        .post(
            Some((&alice.cookie, true)),
            "mutation { signOutEverywhere }",
            json!({}),
        )
        .await;
    assert_eq!(r["data"]["signOutEverywhere"], 2, "{r}");
    let cookie = cookie.expect("a Set-Cookie");
    assert!(
        cookie.starts_with("cyex_tv_session=;") || cookie.contains("Max-Age=0"),
        "{cookie}"
    );

    // Both of Alice's sessions are gone; Bob's aren't.
    for (token, site) in [(&alice.token, false), (&alice.cookie, true)] {
        let (r, _) = w.post(Some((token, site)), ME, json!({})).await;
        assert_eq!(code(&r), "signed_out", "{r}");
    }
    let r = w.gql(&bob, ME, json!({})).await;
    assert!(r["errors"].is_null(), "{r}");

    // An app token can't sign anyone out everywhere (security review 2026/11/20): a leaked token
    // can't sign the person out of the website. Both of Bob's sessions stay.
    let (r, cookie) = w
        .post(
            Some((&bob.token, false)),
            "mutation { signOutEverywhere }",
            json!({}),
        )
        .await;
    assert_eq!(code(&r), "insufficient_scope", "{r}");
    assert!(cookie.is_none());
    let (r, _) = w.post(Some((&bob.token, false)), ME, json!({})).await;
    assert!(r["errors"].is_null(), "{r}");
    let r = w.gql(&bob, ME, json!({})).await;
    assert!(r["errors"].is_null(), "{r}");
    w.db.close().await;
}

#[tokio::test]
async fn activity_filters_by_group_actor_and_time_with_editor_lines() {
    let Some(w) = World::new().await else { return };
    let alice = w.user("7301").await;
    let bob = w.user("7302").await;

    let r = w.gql(&alice, CREATE_SET, json!({ "n": "Main" })).await;
    let set = r["data"]["createSet"]["id"]
        .as_str()
        .unwrap_or_else(|| panic!("{r}"))
        .to_owned();
    let r = w.gql(&alice, ADD_EDITOR, json!({ "a": bob.tid() })).await;
    assert!(r["errors"].is_null(), "{r}");
    let ok = w.approved_emote(&alice, "ok").await;
    let r = w.gql(&bob, ADD_EMOTE, json!({ "s": set, "e": ok })).await;
    assert!(r["errors"].is_null(), "{r}");
    // Bob leaves on his own: that line is his.
    let r = w
        .gql(&bob, "mutation($o: String!, $a: String!) { removeAccountEditor(ownerId: $o, accountId: $a) }",
            json!({ "o": alice.tid(), "a": bob.tid() }))
        .await;
    assert!(r["errors"].is_null(), "{r}");

    // No filter: the feed as before, without editor lines.
    assert_eq!(
        w.kinds(&alice, Value::Null).await,
        ["SET_EMOTE_ADDED", "SET_CREATED"]
    );
    // Every group: the editor lines too, names only.
    let all = w
        .feed(
            &alice,
            json!({ "groups": ["EMOTES", "SETS", "COSMETICS", "EDITORS"] }),
        )
        .await;
    let kinds: Vec<&str> = all.iter().map(|e| e["kind"].as_str().unwrap()).collect();
    assert_eq!(
        kinds,
        [
            "EDITOR_REMOVED",
            "SET_EMOTE_ADDED",
            "EDITOR_ADDED",
            "SET_CREATED"
        ]
    );
    assert_eq!(all[0]["actor"]["id"], w.db.public(bob.id).await.as_str());
    assert_eq!(all[0]["editor"]["id"], w.db.public(bob.id).await.as_str());
    assert_eq!(all[2]["actor"]["id"], w.db.public(alice.id).await.as_str());
    assert_eq!(all[2]["editor"]["displayName"], "tw7302");
    assert!(all[1]["editor"].is_null());
    assert!(
        !json!(all).to_string().contains("EMOTES\""),
        "no permissions in the feed"
    );

    // One group at a time.
    assert_eq!(
        w.kinds(&alice, json!({ "groups": ["EMOTES"] })).await,
        ["SET_EMOTE_ADDED"]
    );
    assert_eq!(
        w.kinds(&alice, json!({ "groups": ["SETS"] })).await,
        ["SET_CREATED"]
    );
    assert_eq!(
        w.kinds(&alice, json!({ "groups": ["EDITORS"] })).await,
        ["EDITOR_REMOVED", "EDITOR_ADDED"]
    );
    assert!(
        w.kinds(&alice, json!({ "groups": ["COSMETICS"] }))
            .await
            .is_empty()
    );

    // By who: Bob's lines, then Alice's own.
    assert_eq!(
        w.kinds(
            &alice,
            json!({ "actorId": bob.tid(), "groups": ["EMOTES", "SETS", "EDITORS"] })
        )
        .await,
        ["EDITOR_REMOVED", "SET_EMOTE_ADDED"]
    );
    assert_eq!(
        w.kinds(
            &alice,
            json!({ "actorId": alice.tid(), "groups": ["EMOTES", "SETS", "EDITORS"] })
        )
        .await,
        ["EDITOR_ADDED", "SET_CREATED"]
    );

    // When: the window around now holds everything, a past or future one nothing.
    let now = chrono::Utc::now();
    let iso = |d: chrono::DateTime<chrono::Utc>| d.to_rfc3339();
    assert_eq!(
        w.kinds(&alice, json!({ "since": iso(now - Duration::hours(1)), "until": iso(now + Duration::hours(1)) })).await.len(),
        2
    );
    assert!(
        w.kinds(&alice, json!({ "since": iso(now + Duration::hours(1)) }))
            .await
            .is_empty()
    );
    assert!(
        w.kinds(&alice, json!({ "until": iso(now - Duration::hours(1)) }))
            .await
            .is_empty()
    );

    // Bad input is named.
    let r = w
        .public(
            FEED,
            json!({ "a": alice.tid(), "f": { "since": "2026/11/19" } }),
        )
        .await;
    assert_eq!(code(&r), "bad_time", "{r}");
    let r = w
        .public(
            FEED,
            json!({ "a": alice.tid(), "f": { "actorId": "nope" } }),
        )
        .await;
    assert!(!r["errors"].is_null(), "{r}");

    // Alice removing an editor is her line.
    let r = w.gql(&alice, ADD_EDITOR, json!({ "a": bob.tid() })).await;
    assert!(r["errors"].is_null(), "{r}");
    let r = w
        .gql(&alice, REMOVE_EDITOR, json!({ "a": bob.tid() }))
        .await;
    assert!(r["errors"].is_null(), "{r}");
    let lines = w.feed(&alice, json!({ "groups": ["EDITORS"] })).await;
    assert_eq!(lines[0]["kind"], "EDITOR_REMOVED");
    assert_eq!(
        lines[0]["actor"]["id"],
        w.db.public(alice.id).await.as_str()
    );
    // Nothing of this lands on Bob's own feed.
    assert!(
        w.kinds(&bob, json!({ "groups": ["EDITORS"] }))
            .await
            .is_empty()
    );
    w.db.close().await;
}
