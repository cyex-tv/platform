//! The public reads that fan out (`accountActivity`, `emoteActivity`, `accountSets`,
//! `searchAccounts`) and the limits around them: paging with a cursor across rows the feed skips,
//! the SQL filter for sets that aren't listed, the preview-only read, the one-letter search, the
//! heavy-reads limit and the delete-account limit. Through the real router against a real
//! Postgres; runs when `CYEX_TV_TEST_DATABASE_URL` is set (CI sets it) and skips otherwise.

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
use cyex_tv_db::sets::{self, NewMember};
use cyex_tv_db::{audit, channels, ids};
use cyex_tv_shared::model::{EmoteState, Platform, SetKind, SetVisibility};
use cyex_tv_shared::ratelimit::Rate;
use http_body_util::BodyExt;
use serde_json::{Value, json};
use tower::ServiceExt;
use uuid::Uuid;

const SITE: &str = "http://localhost:5173";

struct User {
    id: Uuid,
    login: String,
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

fn code(response: &Value) -> &str {
    response["errors"][0]["extensions"]["code"]
        .as_str()
        .unwrap_or_default()
}

const PAGE: &str = "query($a: String!, $b: String, $l: Int) {
    accountActivityPage(accountId: $a, before: $b, limit: $l) { entries { id kind } next }
}";
const EMOTE_PAGE: &str = "query($e: String!, $b: String, $l: Int) {
    emoteActivityPage(emoteId: $e, before: $b, limit: $l) { entries { id kind } next }
}";
const LIST: &str = "query($a: String!) { accountActivity(accountId: $a) { id } }";
const SETS: &str = "query($a: String!, $p: Int) {
    accountSets(accountId: $a, preview: $p) { id slotsUsed preview { name } }
}";
const SEARCH: &str = "query($q: String!, $p: Int, $n: Int) {
    searchAccounts(query: $q, page: $p, perPage: $n) {
        total totalCapped page perPage
        items { account { id } identities { platform login } }
    }
}";
const DELETE: &str = "mutation($c: String!) { deleteAccount(confirm: $c) }";

impl World {
    async fn new(rates: Rates) -> Option<Self> {
        let db = TestDb::new().await?;
        let cfg = AuthConfig::from_source(|key| match key {
            "CYEX_TV_SITE_URL" => Some(SITE.to_owned()),
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

    async fn session(&self, account: Uuid, kind: SessionKind) -> String {
        let mut conn = self.db.conn().await;
        let token = tokens::random_token();
        sessions::create(
            &mut conn,
            &tokens::hash(&token),
            account,
            kind,
            Duration::days(1),
        )
        .await
        .unwrap();
        token
    }

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
        User {
            id,
            login,
            cookie: self.session(id, SessionKind::Site).await,
        }
    }

    /// `auth`: a token and whether it's the site cookie.
    async fn post(&self, auth: Option<(&str, bool)>, query: &str, variables: Value) -> Value {
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
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        serde_json::from_slice(&bytes).unwrap()
    }

    async fn emote(&self, owner: &User, name: &str, state: EmoteState) -> Uuid {
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
        if state != EmoteState::Pending {
            emotes::set_state(&mut conn, emote.id, state).await.unwrap();
        }
        emote.id
    }

    async fn set(&self, owner: &User, name: &str, visibility: SetVisibility) -> Uuid {
        let mut conn = self.db.conn().await;
        let set = sets::create(&mut conn, owner.id, name, SetKind::Channel)
            .await
            .unwrap();
        if visibility != SetVisibility::Listed {
            sets::set_visibility(&mut conn, set.id, visibility)
                .await
                .unwrap();
        }
        set.id
    }

    async fn add(&self, set: Uuid, emote: Uuid, by: &User) {
        let mut conn = self.db.conn().await;
        sets::add_member(
            &mut conn,
            set,
            NewMember {
                emote_id: emote,
                name: None,
                zero_width: None,
                added_by: Some(by.id),
            },
        )
        .await
        .unwrap();
    }

    /// An audit row the feed reads: `set.emote_added` by the set's owner about `emote`.
    async fn added_row(&self, user: &User, set: Uuid, emote: Uuid) {
        let mut conn = self.db.conn().await;
        audit::write_for(
            &mut conn,
            &audit::NewEntry {
                actor: Some(user.id),
                action: "set.emote_added",
                target_kind: "set",
                target_id: &ids::to_text(set),
                before: None,
                after: Some(json!({ "emote_id": ids::to_text(emote), "name": "x" })),
            },
            Some(user.id),
        )
        .await
        .unwrap();
    }
}

fn entries(r: &Value, field: &str) -> usize {
    r["data"][field]["entries"]
        .as_array()
        .unwrap_or_else(|| panic!("{r}"))
        .len()
}

#[tokio::test]
async fn activity_pages_across_rows_the_feed_skips() {
    let Some(w) = World::new(Rates::default()).await else {
        return;
    };
    let fox = w.user("7001").await;
    let public = w.emote(&fox, "Visible", EmoteState::Approved).await;
    let hidden = w.emote(&fox, "StillPending", EmoteState::Pending).await;
    let listed = w.set(&fox, "Main", SetVisibility::Listed).await;
    let unlisted = w.set(&fox, "Secret", SetVisibility::Unlisted).await;

    // Three lines anyone sees, the oldest; then 450 newer rows about a pending emote (SQL can't
    // tell; the feed drops them) and 450 newer still about an unlisted set (SQL drops them).
    for _ in 0..3 {
        w.added_row(&fox, listed, public).await;
    }
    for _ in 0..450 {
        w.added_row(&fox, listed, hidden).await;
    }
    for _ in 0..450 {
        w.added_row(&fox, unlisted, public).await;
    }

    // The unlisted rows never use up the read rounds; the pending ones do: four rounds of 100
    // rows come back empty, and `next` says to carry on.
    let r = w.post(None, PAGE, json!({ "a": fox.tid(), "l": 5 })).await;
    assert!(r["errors"].is_null(), "{r}");
    assert_eq!(entries(&r, "accountActivityPage"), 0, "{r}");
    let next = r["data"]["accountActivityPage"]["next"]
        .as_str()
        .unwrap_or_else(|| panic!("a short page that isn't the end has a cursor: {r}"))
        .to_owned();

    // The old field answers the same short page, without a cursor.
    let r = w.post(None, LIST, json!({ "a": fox.tid() })).await;
    assert_eq!(r["data"]["accountActivity"], json!([]), "{r}");

    // Following the cursor reaches the three lines, then the end.
    let r = w
        .post(None, PAGE, json!({ "a": fox.tid(), "b": next, "l": 5 }))
        .await;
    assert_eq!(entries(&r, "accountActivityPage"), 3, "{r}");
    assert!(r["data"]["accountActivityPage"]["next"].is_null(), "{r}");

    // A full page carries a cursor too, and the next one is the rest.
    let r = w
        .post(None, PAGE, json!({ "a": fox.tid(), "b": next, "l": 2 }))
        .await;
    assert_eq!(entries(&r, "accountActivityPage"), 2, "{r}");
    let again = r["data"]["accountActivityPage"]["next"]
        .as_str()
        .unwrap_or_else(|| panic!("{r}"))
        .to_owned();
    let r = w
        .post(None, PAGE, json!({ "a": fox.tid(), "b": again, "l": 2 }))
        .await;
    assert_eq!(entries(&r, "accountActivityPage"), 1, "{r}");
    assert!(r["data"]["accountActivityPage"]["next"].is_null(), "{r}");
    w.db.close().await;
}

#[tokio::test]
async fn activity_sql_drops_rows_about_sets_that_are_not_listed() {
    let Some(w) = World::new(Rates::default()).await else {
        return;
    };
    let fox = w.user("7101").await;
    let public = w.emote(&fox, "Visible", EmoteState::Approved).await;
    let listed = w.set(&fox, "Main", SetVisibility::Listed).await;
    let unlisted = w.set(&fox, "Secret", SetVisibility::Unlisted).await;
    let private = w.set(&fox, "Mine", SetVisibility::Private).await;
    w.added_row(&fox, listed, public).await;
    w.added_row(&fox, unlisted, public).await;
    w.added_row(&fox, private, public).await;
    // Newest: 450 rows about the unlisted set. One round would do for the one visible line.
    for _ in 0..450 {
        w.added_row(&fox, unlisted, public).await;
    }
    let r = w.post(None, PAGE, json!({ "a": fox.tid(), "l": 30 })).await;
    assert_eq!(entries(&r, "accountActivityPage"), 1, "{r}");
    assert!(r["data"]["accountActivityPage"]["next"].is_null(), "{r}");

    // The same rows through the emote's feed.
    let r = w
        .post(
            None,
            EMOTE_PAGE,
            json!({ "e": ids::to_text(public), "l": 30 }),
        )
        .await;
    assert_eq!(entries(&r, "emoteActivityPage"), 1, "{r}");
    assert!(r["data"]["emoteActivityPage"]["next"].is_null(), "{r}");

    // A set that turns unlisted later hides its old lines too.
    {
        let mut conn = w.db.conn().await;
        sets::set_visibility(&mut conn, listed, SetVisibility::Unlisted)
            .await
            .unwrap();
    }
    let r = w.post(None, PAGE, json!({ "a": fox.tid(), "l": 30 })).await;
    assert_eq!(entries(&r, "accountActivityPage"), 0, "{r}");
    w.db.close().await;
}

#[tokio::test]
async fn emote_activity_page_has_a_cursor() {
    let Some(w) = World::new(Rates::default()).await else {
        return;
    };
    let fox = w.user("7201").await;
    let emote = w.emote(&fox, "Wave", EmoteState::Approved).await;
    let main = w.set(&fox, "Main", SetVisibility::Listed).await;
    for _ in 0..3 {
        w.added_row(&fox, main, emote).await;
    }
    let e = ids::to_text(emote);
    let r = w.post(None, EMOTE_PAGE, json!({ "e": e, "l": 2 })).await;
    assert_eq!(entries(&r, "emoteActivityPage"), 2, "{r}");
    let next = r["data"]["emoteActivityPage"]["next"]
        .as_str()
        .unwrap_or_else(|| panic!("{r}"))
        .to_owned();
    let r = w
        .post(None, EMOTE_PAGE, json!({ "e": e, "b": next, "l": 2 }))
        .await;
    assert_eq!(entries(&r, "emoteActivityPage"), 1, "{r}");
    assert!(r["data"]["emoteActivityPage"]["next"].is_null(), "{r}");
    // An unknown emote has nothing, and no cursor.
    let r = w
        .post(
            None,
            EMOTE_PAGE,
            json!({ "e": ids::to_text(ids::new_id()) }),
        )
        .await;
    assert_eq!(entries(&r, "emoteActivityPage"), 0, "{r}");
    assert!(r["data"]["emoteActivityPage"]["next"].is_null(), "{r}");
    w.db.close().await;
}

#[tokio::test]
async fn account_sets_read_only_the_preview() {
    let Some(w) = World::new(Rates::default()).await else {
        return;
    };
    let fox = w.user("7301").await;
    let other = w.user("7302").await;
    let main = w.set(&fox, "Main", SetVisibility::Listed).await;
    // Eight approved emotes, added in order; then one of the owner's pending ones.
    for n in 0..8 {
        let e = w
            .emote(&fox, &format!("Emote{n}"), EmoteState::Approved)
            .await;
        w.add(main, e, &fox).await;
    }
    let pending = w.emote(&fox, "Fresh", EmoteState::Pending).await;
    w.add(main, pending, &fox).await;

    let names = |r: &Value| -> Vec<String> {
        r["data"]["accountSets"][0]["preview"]
            .as_array()
            .unwrap_or_else(|| panic!("{r}"))
            .iter()
            .map(|e| e["name"].as_str().unwrap().to_owned())
            .collect()
    };
    // Signed out: the newest three approved ones; the pending one isn't shown.
    let r = w.post(None, SETS, json!({ "a": fox.tid(), "p": 3 })).await;
    assert_eq!(names(&r), ["Emote7", "Emote6", "Emote5"], "{r}");
    // The count follows what the viewer may see: eight approved ones, not the pending one.
    assert_eq!(r["data"]["accountSets"][0]["slotsUsed"], 8, "{r}");
    // The owner sees it, newest first.
    let r = w
        .post(
            Some((&fox.cookie, true)),
            SETS,
            json!({ "a": fox.tid(), "p": 3 }),
        )
        .await;
    assert_eq!(names(&r), ["Fresh", "Emote7", "Emote6"], "{r}");
    // Someone else doesn't.
    let r = w
        .post(
            Some((&other.cookie, true)),
            SETS,
            json!({ "a": fox.tid(), "p": 3 }),
        )
        .await;
    assert_eq!(names(&r), ["Emote7", "Emote6", "Emote5"], "{r}");
    // Nothing asked for, nothing read; the facts stay.
    let r = w.post(None, SETS, json!({ "a": fox.tid(), "p": 0 })).await;
    assert_eq!(names(&r), Vec::<String>::new(), "{r}");
    assert_eq!(r["data"]["accountSets"][0]["slotsUsed"], 8, "{r}");
    // The owner's count has the pending one too.
    let r = w
        .post(
            Some((&fox.cookie, true)),
            SETS,
            json!({ "a": fox.tid(), "p": 0 }),
        )
        .await;
    assert_eq!(r["data"]["accountSets"][0]["slotsUsed"], 9, "{r}");
    // The default is 12: all eight approved ones.
    let r = w.post(None, SETS, json!({ "a": fox.tid() })).await;
    assert_eq!(names(&r).len(), 8, "{r}");
    w.db.close().await;
}

#[tokio::test]
async fn short_queries_find_nobody_and_hits_carry_their_identities() {
    let Some(w) = World::new(Rates::default()).await else {
        return;
    };
    let fox = w.user("7401").await;
    {
        let mut conn = w.db.conn().await;
        accounts::link(
            &mut conn,
            fox.id,
            Profile {
                platform: Platform::Kick,
                platform_user_id: "9407401",
                login: "user7401kick",
                display_name: "user7401kick",
                avatar: None,
            },
        )
        .await
        .unwrap();
    }
    // One character, with or without the @, finds nobody, and a short query is not a search.
    for q in ["u", "@u", "  @u  ", "@", " "] {
        let r = w.post(None, SEARCH, json!({ "q": q })).await;
        assert!(r["errors"].is_null(), "{r}");
        assert_eq!(r["data"]["searchAccounts"]["items"], json!([]), "{q}: {r}");
        assert_eq!(r["data"]["searchAccounts"]["total"], 0, "{q}: {r}");
        assert_eq!(
            r["data"]["searchAccounts"]["totalCapped"], false,
            "{q}: {r}"
        );
    }
    // Two characters search, and both logins come with the hit.
    let r = w.post(None, SEARCH, json!({ "q": "@us" })).await;
    let page = &r["data"]["searchAccounts"];
    assert_eq!(page["total"], 1, "{r}");
    assert_eq!(page["totalCapped"], false, "{r}");
    assert_eq!(
        page["items"][0]["identities"],
        json!([
            { "platform": "TWITCH", "login": "user7401" },
            { "platform": "KICK", "login": "user7401kick" }
        ]),
        "{r}"
    );
    // The page size stays capped at 50.
    let r = w.post(None, SEARCH, json!({ "q": "user", "n": 500 })).await;
    assert_eq!(r["data"]["searchAccounts"]["perPage"], 50, "{r}");
    w.db.close().await;
}

#[tokio::test]
async fn search_counts_up_to_a_cap() {
    let Some(w) = World::new(Rates::default()).await else {
        return;
    };
    // 1002 accounts whose names match, made directly.
    {
        let mut conn = w.db.conn().await;
        sqlx::query(
            "WITH made AS (
                 INSERT INTO accounts (id)
                 SELECT gen_random_uuid() FROM generate_series(1, 1002)
                 RETURNING id
             ), numbered AS (
                 SELECT id, row_number() OVER () AS n FROM made
             )
             INSERT INTO identities (platform, platform_user_id, account_id, login, display_name)
             SELECT 'twitch', n::text, id, 'bulkname' || n, 'bulkname' || n FROM numbered",
        )
        .execute(&mut *conn)
        .await
        .unwrap();
    }
    let r = w
        .post(None, SEARCH, json!({ "q": "bulkname", "n": 50, "p": 1 }))
        .await;
    let page = &r["data"]["searchAccounts"];
    assert_eq!(page["total"], 1000, "{r}");
    assert_eq!(page["totalCapped"], true, "{r}");
    assert_eq!(page["items"].as_array().unwrap().len(), 50, "{r}");
    // The last counted page is 20 and full; a page past it is empty (not page 20 again), with
    // the same total.
    let r = w
        .post(None, SEARCH, json!({ "q": "bulkname", "n": 50, "p": 20 }))
        .await;
    assert_eq!(
        r["data"]["searchAccounts"]["items"]
            .as_array()
            .unwrap()
            .len(),
        50,
        "{r}"
    );
    let r = w
        .post(None, SEARCH, json!({ "q": "bulkname", "n": 50, "p": 999 }))
        .await;
    assert_eq!(r["data"]["searchAccounts"]["page"], 999, "{r}");
    assert_eq!(r["data"]["searchAccounts"]["items"], json!([]), "{r}");
    assert_eq!(r["data"]["searchAccounts"]["total"], 1000, "{r}");
    w.db.close().await;
}

#[tokio::test]
async fn heavy_reads_are_limited_per_account_or_ip() {
    let Some(w) = World::new(Rates {
        heavy_reads: Rate::per_minute(3),
        ..Rates::default()
    })
    .await
    else {
        return;
    };
    let fox = w.user("7501").await;
    let other = w.user("7502").await;
    let vars = json!({ "a": fox.tid() });
    for _ in 0..3 {
        let r = w.post(Some((&fox.cookie, true)), LIST, vars.clone()).await;
        assert!(r["errors"].is_null(), "{r}");
    }
    // Any of the heavy reads counts against the same limit.
    let r = w.post(Some((&fox.cookie, true)), SETS, vars.clone()).await;
    assert_eq!(code(&r), "rate_limited", "{r}");
    let r = w.post(Some((&fox.cookie, true)), PAGE, vars.clone()).await;
    assert_eq!(code(&r), "rate_limited", "{r}");
    // Per account: someone else still reads, and so does a signed-out visitor, per IP.
    let r = w
        .post(Some((&other.cookie, true)), LIST, vars.clone())
        .await;
    assert!(r["errors"].is_null(), "{r}");
    for _ in 0..3 {
        let r = w.post(None, LIST, vars.clone()).await;
        assert!(r["errors"].is_null(), "{r}");
    }
    let r = w.post(None, LIST, vars.clone()).await;
    assert_eq!(code(&r), "rate_limited", "{r}");
    // A cheap read isn't touched.
    let r = w
        .post(None, "{ account(id: \"x\") { id } }", json!({}))
        .await;
    assert!(r["errors"].is_null(), "{r}");
    w.db.close().await;
}

#[tokio::test]
async fn one_query_cannot_ask_for_many_heavy_reads() {
    let Some(w) = World::new(Rates::default()).await else {
        return;
    };
    let fox = w.user("7601").await;
    // Each heavy field costs well over a tenth of the 1000 allowed, so a request can't pile up
    // dozens of them.
    let fields: String = (0..12)
        .map(|n| {
            format!(
                "a{n}: accountActivity(accountId: \"{}\") {{ id }} ",
                fox.tid()
            )
        })
        .collect();
    let r = w.post(None, &format!("{{ {fields} }}"), json!({})).await;
    assert!(
        r["errors"][0]["message"]
            .as_str()
            .unwrap_or_default()
            .contains("complex"),
        "{r}"
    );
    w.db.close().await;
}

#[tokio::test]
async fn deleting_an_account_is_limited_to_three_tries_an_hour() {
    let Some(w) = World::new(Rates::default()).await else {
        return;
    };
    let fox = w.user("7701").await;
    let other = w.user("7702").await;
    for _ in 0..3 {
        let r = w
            .post(
                Some((&fox.cookie, true)),
                DELETE,
                json!({ "c": other.login }),
            )
            .await;
        assert_eq!(code(&r), "confirm_mismatch", "{r}");
    }
    // The fourth is refused even with the right login, and nothing was deleted.
    let r = w
        .post(Some((&fox.cookie, true)), DELETE, json!({ "c": fox.login }))
        .await;
    assert_eq!(code(&r), "rate_limited", "{r}");
    let r = w
        .post(Some((&fox.cookie, true)), "{ me { id } }", json!({}))
        .await;
    assert!(r["errors"].is_null(), "still signed in: {r}");
    // Per account: another one can.
    let r = w
        .post(
            Some((&other.cookie, true)),
            DELETE,
            json!({ "c": other.login }),
        )
        .await;
    assert_eq!(r["data"]["deleteAccount"], true, "{r}");
    w.db.close().await;
}

#[tokio::test]
async fn activity_next_is_null_after_a_short_batch() {
    let Some(w) = World::new(Rates::default()).await else {
        return;
    };
    let fox = w.user("7401").await;
    let public = w.emote(&fox, "Visible", EmoteState::Approved).await;
    let main = w.set(&fox, "Main", SetVisibility::Listed).await;
    for _ in 0..3 {
        w.added_row(&fox, main, public).await;
    }
    // The page is exactly full, and so is the feed: no cursor to an empty page.
    let r = w.post(None, PAGE, json!({ "a": fox.tid(), "l": 3 })).await;
    assert_eq!(entries(&r, "accountActivityPage"), 3, "{r}");
    assert!(r["data"]["accountActivityPage"]["next"].is_null(), "{r}");
    let r = w
        .post(
            None,
            EMOTE_PAGE,
            json!({ "e": ids::to_text(public), "l": 3 }),
        )
        .await;
    assert_eq!(entries(&r, "emoteActivityPage"), 3, "{r}");
    assert!(r["data"]["emoteActivityPage"]["next"].is_null(), "{r}");
    // One short of the end still has one.
    let r = w.post(None, PAGE, json!({ "a": fox.tid(), "l": 2 })).await;
    assert!(r["data"]["accountActivityPage"]["next"].is_string(), "{r}");
    w.db.close().await;
}

#[tokio::test]
async fn uploaded_lines_survive_a_deleted_approver() {
    let Some(w) = World::new(Rates::default()).await else {
        return;
    };
    let fox = w.user("7501").await;
    let reviewer = w.user("7502").await;
    let emote = w.emote(&fox, "Wave", EmoteState::Approved).await;
    {
        let mut conn = w.db.conn().await;
        audit::write_for(
            &mut conn,
            &audit::NewEntry {
                actor: Some(reviewer.id),
                action: "emote.approved",
                target_kind: "emote",
                target_id: &ids::to_text(emote),
                before: None,
                after: None,
            },
            Some(fox.id),
        )
        .await
        .unwrap();
        sqlx::query("UPDATE accounts SET deleted_at = now() WHERE id = $1")
            .bind(reviewer.id)
            .execute(&mut *conn)
            .await
            .unwrap();
    }
    // The upload is the owner's; the approver being gone doesn't hide it.
    let r = w.post(None, PAGE, json!({ "a": fox.tid() })).await;
    assert_eq!(entries(&r, "accountActivityPage"), 1, "{r}");
    assert_eq!(
        r["data"]["accountActivityPage"]["entries"][0]["kind"], "EMOTE_UPLOADED",
        "{r}"
    );
    w.db.close().await;
}

const PAGE_IN: &str = "query($a: String!, $b: String, $l: Int, $f: ActivityFilter) {
    accountActivityPage(accountId: $a, before: $b, limit: $l, filter: $f) { entries { id } next }
}";

/// Every line a reader reaches by following `next` from the top, `l` at a time.
async fn reach(w: &World, auth: Option<(&str, bool)>, account: &User, l: u32, f: Value) -> usize {
    let mut seen = 0;
    let mut before: Option<String> = None;
    for _ in 0..50 {
        let r = w
            .post(
                auth,
                PAGE_IN,
                json!({ "a": account.tid(), "b": before, "l": l, "f": f }),
            )
            .await;
        assert!(r["errors"].is_null(), "{r}");
        seen += entries(&r, "accountActivityPage");
        match r["data"]["accountActivityPage"]["next"].as_str() {
            Some(next) => before = Some(next.to_owned()),
            None => return seen,
        }
    }
    panic!("the feed never ended");
}

#[tokio::test]
async fn visitors_reach_the_newest_100_lines_and_the_owner_all_of_them() {
    let Some(w) = World::new(Rates::default()).await else {
        return;
    };
    let fox = w.user("7101").await;
    let visitor = w.user("7102").await;
    let public = w.emote(&fox, "Visible", EmoteState::Approved).await;
    let listed = w.set(&fox, "Main", SetVisibility::Listed).await;
    for _ in 0..130 {
        w.added_row(&fox, listed, public).await;
    }
    let owner = Some((fox.cookie.as_str(), true));
    let someone = Some((visitor.cookie.as_str(), true));

    // Signed out or signed in as someone else: 100 lines, in pages of 30 or in one of 100.
    assert_eq!(reach(&w, None, &fox, 30, Value::Null).await, 100);
    assert_eq!(reach(&w, someone, &fox, 30, Value::Null).await, 100);
    let r = w
        .post(None, PAGE_IN, json!({ "a": fox.tid(), "l": 100 }))
        .await;
    assert_eq!(entries(&r, "accountActivityPage"), 100, "{r}");
    assert!(r["data"]["accountActivityPage"]["next"].is_null(), "{r}");
    // The owner reaches every line.
    assert_eq!(reach(&w, owner, &fox, 30, Value::Null).await, 130);

    // A cursor past the 100th line (the owner's) gives a visitor nothing.
    let r = w
        .post(owner, PAGE_IN, json!({ "a": fox.tid(), "l": 100 }))
        .await;
    let at_100 = r["data"]["accountActivityPage"]["next"]
        .as_str()
        .unwrap_or_else(|| panic!("the owner's feed goes on: {r}"))
        .to_owned();
    let r = w
        .post(
            None,
            PAGE_IN,
            json!({ "a": fox.tid(), "b": at_100, "l": 5 }),
        )
        .await;
    assert_eq!(entries(&r, "accountActivityPage"), 0, "{r}");
    assert!(r["data"]["accountActivityPage"]["next"].is_null(), "{r}");
    let r = w
        .post(
            owner,
            PAGE_IN,
            json!({ "a": fox.tid(), "b": at_100, "l": 5 }),
        )
        .await;
    assert_eq!(entries(&r, "accountActivityPage"), 5, "{r}");
    let at_105 = r["data"]["accountActivityPage"]["next"]
        .as_str()
        .unwrap_or_else(|| panic!("{r}"))
        .to_owned();
    let r = w
        .post(
            None,
            PAGE_IN,
            json!({ "a": fox.tid(), "b": at_105, "l": 5 }),
        )
        .await;
    assert_eq!(entries(&r, "accountActivityPage"), 0, "{r}");

    // A date range is the owner's: theirs narrows the feed, a visitor's is ignored.
    let future = (chrono::Utc::now() + Duration::hours(1)).to_rfc3339();
    let past = (chrono::Utc::now() - Duration::hours(1)).to_rfc3339();
    assert_eq!(
        reach(&w, owner, &fox, 100, json!({ "since": future })).await,
        0
    );
    assert_eq!(
        reach(
            &w,
            owner,
            &fox,
            100,
            json!({ "since": past, "until": future })
        )
        .await,
        130
    );
    assert_eq!(
        reach(&w, None, &fox, 100, json!({ "since": future })).await,
        100
    );
    w.db.close().await;
}
