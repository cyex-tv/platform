//! The account hub: sessions (only yours, never tokens, revoke one or the others), notifications
//! (unread count, marking some read, made for reviews, editor grants and cosmetics), the data
//! export (only yours, rate limited, audited) and deleting an account (confirm, website only,
//! owner protected, what stays and what goes, invisible afterwards, audited). Through the real
//! router against a real Postgres; runs when `CYEX_TV_TEST_DATABASE_URL` is set (CI sets it) and
//! skips otherwise.

mod common;

use std::sync::Arc;

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use chrono::Duration;
use common::TestDb;
use cyex_tv_api::limits::{Limits, Rates};
use cyex_tv_api::state::AppState;
use cyex_tv_auth::{Auth, AuthConfig, tokens};
use cyex_tv_db::accounts::{self, Profile};
use cyex_tv_db::emotes::{self, NewEmote};
use cyex_tv_db::sessions::{self, SessionKind};
use cyex_tv_db::{audit, channels, ids, notifications, roles};
use cyex_tv_shared::model::{EmoteState, Platform};
use cyex_tv_shared::ratelimit::Rate;
use http_body_util::BodyExt;
use serde_json::{Value, json};
use tower::ServiceExt;
use uuid::Uuid;

const SITE: &str = "http://localhost:5173";

struct User {
    id: Uuid,
    /// An app session (Bearer).
    token: String,
    /// A site session (the cookie).
    cookie: String,
    twitch_id: String,
    login: String,
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

fn code(response: &Value) -> &str {
    response["errors"][0]["extensions"]["code"]
        .as_str()
        .unwrap_or_default()
}

const SESSIONS: &str = "{ mySessions { id kind device createdAt lastSeenAt expiresAt current } }";

impl World {
    async fn new() -> Option<Self> {
        Self::with_rates(Rates::default(), None).await
    }

    async fn with_rates(rates: Rates, owner: Option<&str>) -> Option<Self> {
        let db = TestDb::new().await?;
        let owner = owner.map(str::to_owned);
        let cfg = AuthConfig::from_source(|key| match key {
            "CYEX_TV_SITE_URL" => Some(SITE.to_owned()),
            "CYEX_TV_OWNER_IDS" => owner.clone(),
            _ => None,
        })
        .unwrap();
        let auth = Arc::new(Auth::new(cfg).unwrap());
        let state = AppState::new(db.pool.clone(), auth).with_limits(Limits::new(rates, false));
        Some(Self {
            app: cyex_tv_api::app(state),
            db,
        })
    }

    async fn session(&self, account: Uuid, kind: SessionKind, ua: Option<&str>) -> String {
        let mut conn = self.db.conn().await;
        let token = tokens::random_token();
        sessions::create_with(
            &mut conn,
            &tokens::hash(&token),
            account,
            kind,
            Duration::days(1),
            ua,
        )
        .await
        .unwrap();
        token
    }

    /// A Twitch user `user<id>` with a channel, an app session and a site session.
    async fn user(&self, twitch_id: &str) -> User {
        let login = format!("user{twitch_id}");
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
        let token = self.session(id, SessionKind::App, None).await;
        let cookie = self
            .session(
                id,
                SessionKind::Site,
                Some("Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:133.0) Gecko/20100101 Firefox/133.0"),
            )
            .await;
        User {
            id,
            token,
            cookie,
            twitch_id: twitch_id.to_owned(),
            login,
        }
    }

    async fn post(
        &self,
        auth: Option<(&str, bool)>,
        query: &str,
        variables: Value,
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
                json!({ "query": query, "variables": variables }).to_string(),
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

    async fn gql(&self, user: &User, query: &str, variables: Value) -> Value {
        self.post(Some((&user.token, false)), query, variables)
            .await
            .0
    }

    async fn site(&self, user: &User, query: &str, variables: Value) -> (Value, Option<String>) {
        self.post(Some((&user.cookie, true)), query, variables)
            .await
    }

    async fn emote(&self, owner: &User, name: &str, state: EmoteState, private: bool) -> String {
        let mut conn = self.db.conn().await;
        let emote = emotes::create(
            &mut conn,
            &NewEmote {
                owner_account_id: owner.id,
                name,
                private,
                ..NewEmote::default()
            },
        )
        .await
        .unwrap();
        if state != EmoteState::Pending {
            emotes::set_state(&mut conn, emote.id, state).await.unwrap();
        }
        ids::to_text(emote.id)
    }

    async fn set(&self, owner: &User, name: &str) -> String {
        let r = self
            .gql(
                owner,
                "mutation($n: String!) { createSet(input: { name: $n, kind: CHANNEL }) { id } }",
                json!({ "n": name }),
            )
            .await;
        r["data"]["createSet"]["id"]
            .as_str()
            .unwrap_or_else(|| panic!("{r}"))
            .to_owned()
    }

    async fn add(&self, by: &User, set: &str, emote: &str) {
        let r = self
            .gql(
                by,
                "mutation($s: String!, $e: String!) { addSetEmote(setId: $s, emoteId: $e) { name } }",
                json!({ "s": set, "e": emote }),
            )
            .await;
        assert!(r["errors"].is_null(), "{r}");
    }

    async fn grant_role(&self, user: &User, perms: &[&str]) {
        let mut conn = self.db.conn().await;
        let role = roles::create(
            &mut conn,
            &roles::NewRole {
                name: &format!("role{}", user.twitch_id),
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
    }

    async fn notes(&self, user: &User) -> Vec<(String, Value)> {
        let mut conn = self.db.conn().await;
        notifications::all_for(&mut conn, user.id)
            .await
            .unwrap()
            .into_iter()
            .map(|n| (n.kind, n.payload))
            .collect()
    }

    async fn audit_actions(&self) -> Vec<audit::Entry> {
        let mut conn = self.db.conn().await;
        audit::list(&mut conn, &audit::Filter::default(), 100)
            .await
            .unwrap()
    }
}

#[tokio::test]
async fn sessions_are_yours_only_and_never_tokens() {
    let Some(w) = World::new().await else { return };
    let alice = w.user("4001").await;
    let bob = w.user("4002").await;

    let r = w.gql(&alice, SESSIONS, json!({})).await;
    let list = r["data"]["mySessions"]
        .as_array()
        .unwrap_or_else(|| panic!("{r}"));
    assert_eq!(list.len(), 2, "{r}");
    let current: Vec<&Value> = list.iter().filter(|s| s["current"] == true).collect();
    assert_eq!(current.len(), 1);
    assert_eq!(current[0]["kind"], "APP");
    assert_eq!(current[0]["device"], "cyex.tv app");
    let site = list.iter().find(|s| s["kind"] == "SITE").unwrap();
    assert_eq!(site["device"], "Firefox on Windows");
    // No token, no hash, no raw user agent anywhere.
    let text = r.to_string();
    assert!(!text.contains(&alice.token) && !text.contains(&alice.cookie));
    assert!(!text.contains("Gecko"), "{text}");
    for s in list {
        assert_eq!(s["id"].as_str().unwrap().len(), 26);
    }
    let site_id = site["id"].as_str().unwrap().to_owned();

    // Bob can't sign Alice out (IDOR): her ID is nothing to him.
    let r = w
        .gql(
            &bob,
            "mutation($id: String!) { revokeSession(id: $id) }",
            json!({ "id": site_id }),
        )
        .await;
    assert_eq!(r["data"]["revokeSession"], false, "{r}");
    let (r, _) = w.site(&alice, "{ me { id } }", json!({})).await;
    assert_eq!(r["data"]["me"]["id"], alice.tid().as_str(), "{r}");

    // Alice signs her browser out from the app; the cookie stops working.
    let r = w
        .gql(
            &alice,
            "mutation($id: String!) { revokeSession(id: $id) }",
            json!({ "id": site_id }),
        )
        .await;
    assert_eq!(r["data"]["revokeSession"], true, "{r}");
    let (r, _) = w.site(&alice, "{ me { id } }", json!({})).await;
    assert_eq!(code(&r), "signed_out", "{r}");
    // Signing out the session you're using on the website also clears its cookie.
    let fresh = w.session(alice.id, SessionKind::Site, None).await;
    let (r, _) = w.post(Some((&fresh, true)), SESSIONS, json!({})).await;
    let fresh_id = r["data"]["mySessions"]
        .as_array()
        .unwrap_or_else(|| panic!("{r}"))
        .iter()
        .find(|s| s["current"] == true)
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let (r, cookie) = w
        .post(
            Some((&fresh, true)),
            "mutation($id: String!) { revokeSession(id: $id) }",
            json!({ "id": fresh_id }),
        )
        .await;
    assert_eq!(r["data"]["revokeSession"], true, "{r}");
    assert!(
        cookie.is_some_and(|c| c.contains("Max-Age=0")),
        "the cookie is cleared"
    );
    let r = w
        .gql(
            &alice,
            "mutation($id: String!) { revokeSession(id: $id) }",
            json!({ "id": "nope" }),
        )
        .await;
    assert_eq!(r["data"]["revokeSession"], false, "{r}");

    // Sign out everywhere else keeps the current session only.
    let _ = w
        .session(alice.id, SessionKind::App, Some("cyex-deck/1.0 (Windows)"))
        .await;
    let _ = w.session(alice.id, SessionKind::Site, None).await;
    let r = w
        .gql(&alice, "mutation { revokeOtherSessions }", json!({}))
        .await;
    assert_eq!(r["data"]["revokeOtherSessions"], 2, "{r}");
    let r = w.gql(&alice, SESSIONS, json!({})).await;
    assert_eq!(r["data"]["mySessions"].as_array().unwrap().len(), 1, "{r}");
    // Bob's sessions are untouched.
    let r = w.gql(&bob, SESSIONS, json!({})).await;
    assert_eq!(r["data"]["mySessions"].as_array().unwrap().len(), 2, "{r}");
    // Signed out: nothing.
    let (r, _) = w.post(None, SESSIONS, json!({})).await;
    assert_eq!(code(&r), "signed_out", "{r}");
    w.db.close().await;
}

#[tokio::test]
async fn notifications_are_made_counted_and_marked() {
    let Some(w) = World::new().await else { return };
    let alice = w.user("4101").await;
    let bob = w.user("4102").await;
    let staff = w.user("4103").await;
    w.grant_role(
        &staff,
        &["emotes.review", "cosmetics.manage", "cosmetics.grant"],
    )
    .await;

    // Reviews tell the owner.
    let ok = w.emote(&alice, "aliceOk", EmoteState::Pending, false).await;
    let no = w.emote(&alice, "aliceNo", EmoteState::Pending, false).await;
    let r = w
        .gql(
            &staff,
            "mutation($id: String!) { approveEmote(id: $id) { state } }",
            json!({ "id": ok }),
        )
        .await;
    assert!(r["errors"].is_null(), "{r}");
    let r = w
        .gql(
            &staff,
            "mutation($id: String!) { rejectEmote(id: $id, reason: \"blurry\") { state } }",
            json!({ "id": no }),
        )
        .await;
    assert!(r["errors"].is_null(), "{r}");

    // Editor grants tell the editor (and the owner when someone else did it).
    let r = w
        .gql(
            &alice,
            "mutation($a: String!) { addAccountEditor(input: { accountId: $a, permissions: [EMOTES] }) { permissions } }",
            json!({ "a": bob.tid() }),
        )
        .await;
    assert!(r["errors"].is_null(), "{r}");
    let r = w
        .gql(
            &alice,
            "mutation($a: String!) { updateAccountEditor(accountId: $a, permissions: [EMOTES, SETS]) { permissions } }",
            json!({ "a": bob.tid() }),
        )
        .await;
    assert!(r["errors"].is_null(), "{r}");
    // Bob leaves: Alice hears it, Bob doesn't (he did it).
    let r = w
        .gql(
            &bob,
            "mutation($o: String!, $a: String!) { removeAccountEditor(ownerId: $o, accountId: $a) }",
            json!({ "o": alice.tid(), "a": bob.tid() }),
        )
        .await;
    assert_eq!(r["data"]["removeAccountEditor"], true, "{r}");

    let mine: Vec<String> = w.notes(&alice).await.into_iter().map(|(k, _)| k).collect();
    assert_eq!(
        mine,
        ["editor.removed", "emote.rejected", "emote.approved"],
        "{mine:?}"
    );
    let bobs = w.notes(&bob).await;
    let kinds: Vec<&str> = bobs.iter().map(|(k, _)| k.as_str()).collect();
    assert_eq!(kinds, ["editor.changed", "editor.added"]);
    let (_, added) = &bobs[1];
    assert_eq!(added["role"], "editor");
    assert_eq!(added["owner_id"], alice.tid().as_str());
    assert_eq!(added["owner_name"], alice.login.as_str());
    assert_eq!(added["permissions"], json!(["emotes"]));
    let rejected = &w.notes(&alice).await[1].1;
    assert_eq!(rejected["reason"], "blurry");
    assert_eq!(rejected["name"], "aliceNo");

    // Cosmetics granted and revoked tell the account.
    let r = w
        .gql(
            &staff,
            "mutation($p: PaintInput!) { createPaint(input: $p) { id } }",
            json!({ "p": {
                "name": "Sunrise",
                "kind": "LINEAR_GRADIENT",
                "angle": 90,
                "repeat": false,
                "color": "#ff0000",
                "stops": [{ "at": 0.0, "color": "#ff0000" }, { "at": 1.0, "color": "#0000ff" }],
                "shadows": []
            } }),
        )
        .await;
    if let Some(paint) = r["data"]["createPaint"]["id"].as_str() {
        let r = w
            .gql(
                &staff,
                "mutation($a: String!, $c: String!) { grantCosmetic(input: { accountId: $a, kind: PAINT, cosmeticId: $c, reason: \"thanks\" }) { cosmeticId } }",
                json!({ "a": alice.tid(), "c": paint }),
            )
            .await;
        assert!(r["errors"].is_null(), "{r}");
        let r = w
            .gql(
                &staff,
                "mutation($a: String!, $c: String!) { revokeCosmetic(accountId: $a, kind: PAINT, cosmeticId: $c) }",
                json!({ "a": alice.tid(), "c": paint }),
            )
            .await;
        assert_eq!(r["data"]["revokeCosmetic"], true, "{r}");
        let notes = w.notes(&alice).await;
        assert_eq!(notes[0].0, "cosmetic.revoked");
        assert_eq!(notes[1].0, "cosmetic.granted");
        assert_eq!(notes[1].1["name"], "Sunrise");
        assert_eq!(notes[1].1["reason"], "thanks");
    } else {
        panic!("paint input changed: {r}");
    }

    // Counting and marking: only your own.
    let r = w
        .gql(&alice, "{ unreadNotificationCount }", json!({}))
        .await;
    assert_eq!(r["data"]["unreadNotificationCount"], 5, "{r}");
    let r = w
        .gql(&alice, "{ myNotifications { id kind read } }", json!({}))
        .await;
    let first = r["data"]["myNotifications"][0]["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let bobs_first = w
        .gql(&bob, "{ myNotifications { id } }", json!({}))
        .await["data"]["myNotifications"][0]["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let r = w
        .gql(
            &alice,
            "mutation($ids: [String!]) { markNotificationsRead(ids: $ids) }",
            json!({ "ids": [first, bobs_first, "nope"] }),
        )
        .await;
    assert_eq!(r["data"]["markNotificationsRead"], 1, "{r}");
    let r = w.gql(&bob, "{ unreadNotificationCount }", json!({})).await;
    assert_eq!(
        r["data"]["unreadNotificationCount"], 2,
        "Bob's stay unread: {r}"
    );
    // Without ids: all of them, as before.
    let r = w
        .gql(&alice, "mutation { markNotificationsRead }", json!({}))
        .await;
    assert_eq!(r["data"]["markNotificationsRead"], 4, "{r}");
    let r = w
        .gql(&alice, "{ unreadNotificationCount }", json!({}))
        .await;
    assert_eq!(r["data"]["unreadNotificationCount"], 0, "{r}");
    let many: Vec<String> = (0..101).map(|i| format!("x{i}")).collect();
    let r = w
        .gql(
            &alice,
            "mutation($ids: [String!]) { markNotificationsRead(ids: $ids) }",
            json!({ "ids": many }),
        )
        .await;
    assert_eq!(code(&r), "too_many", "{r}");
    w.db.close().await;
}

#[tokio::test]
async fn the_export_is_yours_complete_limited_and_audited() {
    let Some(w) = World::with_rates(
        Rates {
            data_export: Rate::per_hour(2),
            ..Rates::default()
        },
        None,
    )
    .await
    else {
        return;
    };
    let alice = w.user("4201").await;
    let bob = w.user("4202").await;
    let set = w.set(&alice, "Main").await;
    let e = w
        .emote(&alice, "aliceOk", EmoteState::Approved, false)
        .await;
    let _pending = w
        .emote(&alice, "alicePending", EmoteState::Pending, false)
        .await;
    w.add(&alice, &set, &e).await;
    let _bobs = w
        .emote(&bob, "bobSecret", EmoteState::Approved, false)
        .await;

    let r = w.gql(&alice, "mutation { exportMyData }", json!({})).await;
    let text = r["data"]["exportMyData"]
        .as_str()
        .unwrap_or_else(|| panic!("{r}"));
    let doc: Value = serde_json::from_str(text).unwrap();
    assert_eq!(doc["format"], "cyex.tv account export");
    assert_eq!(doc["version"], 1);
    assert_eq!(doc["account"]["id"], alice.tid().as_str());
    assert_eq!(doc["identities"][0]["login"], alice.login.as_str());
    assert_eq!(
        doc["identities"][0]["platform_user_id"],
        alice.twitch_id.as_str()
    );
    assert_eq!(doc["emotes"].as_array().unwrap().len(), 2);
    assert_eq!(doc["sets"][0]["id"], set.as_str());
    assert_eq!(doc["sets"][0]["emotes"][0]["name"], "aliceOk");
    assert_eq!(doc["channels"][0]["login"], alice.login.as_str());
    assert_eq!(doc["sessions"].as_array().unwrap().len(), 2);
    for key in [
        "cosmetics",
        "account_editors",
        "editor_of",
        "roles",
        "reports_filed",
        "notifications",
        "orders",
    ] {
        assert!(!doc[key].is_null(), "{key} missing");
    }
    // Only Alice's: nothing of Bob's, and no tokens.
    assert!(!text.contains("bobSecret") && !text.contains(&bob.login));
    assert!(!text.contains(&alice.token) && !text.contains(&alice.cookie));

    let rows = w.audit_actions().await;
    let row = rows
        .iter()
        .find(|e| e.action == "account.exported")
        .expect("audited");
    assert_eq!(row.actor_account_id, Some(alice.id));
    assert_eq!(row.target_id, alice.tid());

    let r = w.gql(&alice, "mutation { exportMyData }", json!({})).await;
    assert!(r["errors"].is_null(), "{r}");
    let r = w.gql(&alice, "mutation { exportMyData }", json!({})).await;
    assert_eq!(code(&r), "rate_limited", "{r}");
    let (r, _) = w.post(None, "mutation { exportMyData }", json!({})).await;
    assert_eq!(code(&r), "signed_out", "{r}");
    w.db.close().await;
}

#[tokio::test]
async fn deleting_an_account_removes_it_and_keeps_what_others_use() {
    let Some(w) = World::new().await else { return };
    let alice = w.user("4301").await;
    let bob = w.user("4302").await;
    let own_set = w.set(&alice, "Mine").await;
    let bobs_set = w.set(&bob, "Bob's").await;
    let shared = w
        .emote(&alice, "aliceShared", EmoteState::Approved, false)
        .await;
    let lonely = w
        .emote(&alice, "aliceLonely", EmoteState::Approved, false)
        .await;
    let pending = w
        .emote(&alice, "alicePending", EmoteState::Pending, false)
        .await;
    w.add(&alice, &own_set, &shared).await;
    w.add(&alice, &own_set, &lonely).await;
    w.add(&alice, &own_set, &pending).await;
    w.add(&bob, &bobs_set, &shared).await;
    // Alice edits Bob's account and Bob edits hers.
    for (by, who) in [(&alice, &bob), (&bob, &alice)] {
        let r = w
            .gql(
                by,
                "mutation($a: String!) { addAccountEditor(input: { accountId: $a, permissions: [EMOTES] }) { permissions } }",
                json!({ "a": who.tid() }),
            )
            .await;
        assert!(r["errors"].is_null(), "{r}");
    }
    let r = w
        .gql(
            &alice,
            "mutation($c: String!, $s: String) { updateChannelSettings(platform: TWITCH, id: $c, input: { activeSetId: $s }) { activeSetId } }",
            json!({ "c": alice.twitch_id, "s": own_set }),
        )
        .await;
    assert!(r["errors"].is_null(), "{r}");

    const DELETE: &str = "mutation($c: String!) { deleteAccount(confirm: $c) }";
    // An app token can't; a wrong login can't.
    let r = w.gql(&alice, DELETE, json!({ "c": alice.login })).await;
    assert_eq!(code(&r), "website_only", "{r}");
    let (r, _) = w.site(&alice, DELETE, json!({ "c": bob.login })).await;
    assert_eq!(code(&r), "confirm_mismatch", "{r}");
    let (r, _) = w.post(None, DELETE, json!({ "c": alice.login })).await;
    assert_eq!(code(&r), "signed_out", "{r}");
    // While restricted, no: signing in again would make a fresh, unrestricted account.
    {
        let mut conn = w.db.conn().await;
        accounts::set_restricted(&mut conn, alice.id, true)
            .await
            .unwrap();
    }
    let (r, _) = w.site(&alice, DELETE, json!({ "c": alice.login })).await;
    assert_eq!(code(&r), "restricted", "{r}");
    {
        let mut conn = w.db.conn().await;
        assert!(
            !accounts::identities(&mut conn, alice.id)
                .await
                .unwrap()
                .is_empty()
        );
        accounts::set_restricted(&mut conn, alice.id, false)
            .await
            .unwrap();
    }

    let (r, cookie) = w
        .site(
            &alice,
            DELETE,
            json!({ "c": format!("@{}", alice.login.to_uppercase()) }),
        )
        .await;
    assert_eq!(r["data"]["deleteAccount"], true, "{r}");
    let cookie = cookie.expect("the cookie is cleared");
    assert!(
        cookie.contains("cyex_tv_session=") && cookie.contains("Max-Age=0"),
        "{cookie}"
    );

    // Signed out everywhere.
    let r = w.gql(&alice, "{ me { id } }", json!({})).await;
    assert_eq!(code(&r), "signed_out", "{r}");
    // Invisible: no account, no sets, no search hit, no channel.
    let (r, _) = w
        .post(
            None,
            "query($a: String!, $s: String!, $l: String!) {
                account(id: $a) { id }
                accountSets(accountId: $a) { id }
                set(id: $s) { id }
                channelByLogin(platform: TWITCH, login: $l) { login }
                searchAccounts(query: $l) { total }
            }",
            json!({ "a": alice.tid(), "s": own_set, "l": alice.login }),
        )
        .await;
    assert!(r["data"]["account"].is_null(), "{r}");
    assert_eq!(r["data"]["accountSets"], json!([]));
    assert!(r["data"]["set"].is_null());
    assert!(r["data"]["channelByLogin"].is_null());
    assert!(
        !r.to_string()
            .contains(&format!("\"id\":\"{}\"", alice.tid())),
        "not found by search: {r}"
    );

    // The emote in Bob's set stays there, by nobody; the others are gone.
    let (r, _) = w
        .post(
            None,
            "query($s: String!, $a: String!, $b: String!) {
                set(id: $s) { emotes { name emote { id owner { id } } } }
                a: emote(id: $a) { id }
                b: emote(id: $b) { id }
            }",
            json!({ "s": bobs_set, "a": lonely, "b": pending }),
        )
        .await;
    let members = r["data"]["set"]["emotes"].as_array().unwrap();
    assert_eq!(members.len(), 1, "{r}");
    assert_eq!(members[0]["emote"]["id"], shared.as_str());
    assert!(members[0]["emote"]["owner"].is_null());
    assert!(r["data"]["a"].is_null() && r["data"]["b"].is_null(), "{r}");

    // Bob's editor rows for and by Alice are gone.
    let r = w
        .gql(
            &bob,
            "{ accountEditors { account { id } } editorOf { account { id } } }",
            json!({}),
        )
        .await;
    assert_eq!(r["data"]["accountEditors"], json!([]), "{r}");
    assert_eq!(r["data"]["editorOf"], json!([]), "{r}");

    // Rows that named her: identities and sessions gone, the account row kept as deleted.
    {
        let mut conn = w.db.conn().await;
        assert!(
            accounts::identities(&mut conn, alice.id)
                .await
                .unwrap()
                .is_empty()
        );
        assert!(
            sessions::list_for_account(&mut conn, alice.id)
                .await
                .unwrap()
                .is_empty()
        );
        assert!(
            accounts::get(&mut conn, alice.id)
                .await
                .unwrap()
                .unwrap()
                .deleted_at
                .is_some()
        );
    }
    let rows = w.audit_actions().await;
    let row = rows
        .iter()
        .find(|e| e.action == "account.deleted")
        .expect("audited");
    assert_eq!(row.actor_account_id, Some(alice.id));
    let after = row.after_json.as_ref().unwrap();
    assert_eq!(after["emotes_kept"], 1);
    assert_eq!(after["emotes_deleted"], 2);
    assert_eq!(after["sets_deleted"], 1);
    assert!(
        !after.to_string().contains(&alice.login),
        "no names in the audit row"
    );

    // Signing in with the same Twitch account later makes a new account.
    let mut conn = w.db.conn().await;
    let again = accounts::sign_in(
        &mut conn,
        Profile {
            platform: Platform::Twitch,
            platform_user_id: &alice.twitch_id,
            login: &alice.login,
            display_name: &alice.login,
            avatar: None,
        },
    )
    .await
    .unwrap();
    assert!(again.created);
    assert_ne!(again.account.id, alice.id);
    drop(conn);
    w.db.close().await;
}

#[tokio::test]
async fn the_owner_cannot_delete_their_account() {
    let Some(w) = World::with_rates(Rates::default(), Some("twitch:4401")).await else {
        return;
    };
    let owner = w.user("4401").await;
    let (r, _) = w
        .site(
            &owner,
            "mutation($c: String!) { deleteAccount(confirm: $c) }",
            json!({ "c": owner.login }),
        )
        .await;
    assert_eq!(code(&r), "owner_protected", "{r}");
    let r = w.gql(&owner, "{ me { id } }", json!({})).await;
    assert_eq!(r["data"]["me"]["id"], owner.tid().as_str(), "{r}");
    w.db.close().await;
}
