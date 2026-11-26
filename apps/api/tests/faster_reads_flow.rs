//! Faster client reads (ADR 0035): bulk reads (`emotes`, `sets`, `channels` and their REST
//! twins) with caps, the single reads' visibility and a query count that doesn't grow with the
//! IDs; set versions bumped by every write path and delta sync (`setChanges`,
//! `GET /v1/sets/{id}?since=`) including a `since` too old; persisted queries over `GET` (hash
//! miss and hit, mutations refused, cache headers public or private). Through the real router
//! against a real Postgres; runs when `CYEX_TV_TEST_DATABASE_URL` is set (CI sets it), skips
//! otherwise.

mod common;

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use axum::Router;
use axum::body::Body;
use axum::http::{HeaderMap, Request, StatusCode};
use chrono::Duration;
use common::TestDb;
use cyex_tv_api::state::AppState;
use cyex_tv_auth::{Auth, AuthConfig, tokens};
use cyex_tv_db::accounts::{self, Profile};
use cyex_tv_db::emotes::{self, EmotePatch, NewEmote};
use cyex_tv_db::sessions::{self, SessionKind};
use cyex_tv_db::{channels, files, sets};
use cyex_tv_shared::model::{EmoteState, Platform, SetKind};
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

struct Answer {
    status: StatusCode,
    headers: HeaderMap,
    body: Value,
}

fn code(r: &Value) -> &str {
    r["errors"][0]["extensions"]["code"]
        .as_str()
        .unwrap_or_default()
}

fn data<'a>(r: &'a Value, field: &str) -> &'a Value {
    assert!(r["errors"].is_null(), "{r}");
    &r["data"][field]
}

fn ids(list: &Value) -> Vec<String> {
    list.as_array()
        .unwrap()
        .iter()
        .map(|i| i["id"].as_str().unwrap().to_owned())
        .collect()
}

fn member_ids(set: &Value) -> Vec<String> {
    set["emotes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|m| m["emote"]["id"].as_str().unwrap().to_owned())
        .collect()
}

fn header(a: &Answer, name: &str) -> String {
    a.headers
        .get(name)
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default()
        .to_owned()
}

impl World {
    async fn new() -> Option<Self> {
        let db = TestDb::new().await?;
        let auth = Arc::new(Auth::new(AuthConfig::from_source(|_| None).unwrap()).unwrap());
        Some(Self {
            app: cyex_tv_api::app(AppState::new(db.pool.clone(), auth)),
            db,
        })
    }

    async fn user(&self, twitch_id: &str, login: &str, kind: SessionKind) -> User {
        let mut conn = self.db.conn().await;
        let signed = accounts::sign_in(
            &mut conn,
            Profile {
                platform: Platform::Twitch,
                platform_user_id: twitch_id,
                login,
                display_name: login,
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
            kind,
            Duration::days(1),
        )
        .await
        .unwrap();
        User {
            id: signed.account.id,
            token,
        }
    }

    /// An emote of `owner`, approved unless `pending`; (row, ID as the API shows it).
    async fn emote(&self, owner: &User, name: &str, pending: bool) -> (Uuid, String) {
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
        if !pending {
            emotes::set_state(&mut conn, e.id, EmoteState::Approved)
                .await
                .unwrap();
        }
        (e.id, self.db.public(e.id).await)
    }

    /// A channel set of `owner` holding `members`; (row, ID as the API shows it).
    async fn set(&self, owner: &User, name: &str, members: &[Uuid]) -> (Uuid, String) {
        let mut conn = self.db.conn().await;
        let set = sets::create(&mut conn, owner.id, name, SetKind::Channel)
            .await
            .unwrap();
        for m in members {
            sets::add_member(
                &mut conn,
                set.id,
                sets::NewMember {
                    emote_id: *m,
                    name: None,
                    zero_width: None,
                    added_by: Some(owner.id),
                },
            )
            .await
            .unwrap();
        }
        (set.id, self.db.public(set.id).await)
    }

    async fn version(&self, set: Uuid) -> i64 {
        let mut conn = self.db.conn().await;
        sets::get(&mut conn, set).await.unwrap().unwrap().version
    }

    async fn kinds_after(&self, set: Uuid, since: i64) -> Vec<String> {
        let mut conn = self.db.conn().await;
        sets::changes_since(&mut conn, set, since, i64::MAX, 1000)
            .await
            .unwrap()
            .into_iter()
            .map(|c| c.kind)
            .collect()
    }

    async fn send(&self, request: Request<Body>) -> Answer {
        let response = self.app.clone().oneshot(request).await.unwrap();
        let status = response.status();
        let headers = response.headers().clone();
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        let body = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
        Answer {
            status,
            headers,
            body,
        }
    }

    async fn gql(&self, user: Option<&User>, query: &str, variables: Value) -> Value {
        self.post_gql(user, json!({ "query": query, "variables": variables }))
            .await
            .body
    }

    async fn post_gql(&self, user: Option<&User>, body: Value) -> Answer {
        let mut request = Request::post("/v1/gql").header("content-type", "application/json");
        if let Some(user) = user {
            request = request
                .header(
                    "cookie",
                    format!("{}={}", cyex_tv_auth::cookies::SESSION, user.token),
                )
                .header("origin", "https://cyex.tv");
        }
        self.send(request.body(Body::from(body.to_string())).unwrap())
            .await
    }

    async fn post(&self, path: &str, body: Value) -> Answer {
        self.send(
            Request::post(path)
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
    }

    /// `GET /v1/gql` with these query-string fields, and extra headers.
    async fn get_gql(&self, fields: &[(&str, String)], headers: &[(&str, String)]) -> Answer {
        let query: String = fields
            .iter()
            .map(|(k, v)| {
                format!(
                    "{k}={}",
                    url::form_urlencoded::byte_serialize(v.as_bytes()).collect::<String>()
                )
            })
            .collect::<Vec<_>>()
            .join("&");
        let mut request = Request::get(format!("/v1/gql?{query}"));
        for (k, v) in headers {
            request = request.header(*k, v.as_str());
        }
        self.send(request.body(Body::empty()).unwrap()).await
    }

    async fn get(&self, path: &str, headers: &[(&str, String)]) -> Answer {
        let mut request = Request::get(path);
        for (k, v) in headers {
            request = request.header(*k, v.as_str());
        }
        self.send(request.body(Body::empty()).unwrap()).await
    }
}

// Counting queries: sqlx reports every statement as a `sqlx::query` tracing event. The tests run
// on a current-thread runtime, so a thread-local subscriber sees this test's statements only.

struct Counter(Arc<AtomicUsize>);

impl tracing::Subscriber for Counter {
    fn enabled(&self, metadata: &tracing::Metadata<'_>) -> bool {
        metadata.target() == "sqlx::query"
    }
    fn new_span(&self, _: &tracing::span::Attributes<'_>) -> tracing::span::Id {
        tracing::span::Id::from_u64(1)
    }
    fn record(&self, _: &tracing::span::Id, _: &tracing::span::Record<'_>) {}
    fn record_follows_from(&self, _: &tracing::span::Id, _: &tracing::span::Id) {}
    fn event(&self, event: &tracing::Event<'_>) {
        if event.metadata().target() == "sqlx::query" {
            self.0.fetch_add(1, Ordering::SeqCst);
        }
    }
    fn enter(&self, _: &tracing::span::Id) {}
    fn exit(&self, _: &tracing::span::Id) {}
}

/// How many statements `run` made.
async fn statements<F: std::future::Future<Output = Value>>(run: F) -> (usize, Value) {
    let count = Arc::new(AtomicUsize::new(0));
    let guard = tracing::subscriber::set_default(Counter(count.clone()));
    let out = run.await;
    drop(guard);
    (count.load(Ordering::SeqCst), out)
}

const SETS: &str =
    "query($ids: [ID!]!) { sets(ids: $ids) { id version emotes { name emote { id } } } }";
const EMOTES: &str = "query($ids: [ID!]!) { emotes(ids: $ids) { id name } }";
const CHANNELS: &str =
    "query($ids: [String!]!) { channels(platform: TWITCH, ids: $ids) { platformChannelId login } }";
const CHANGES: &str = "query($id: String!, $since: Int!) {
    setChanges(id: $id, since: $since) {
        id version since full name set { id emotes { emote { id } } }
        added { name emote { id } } renamed { name emote { id } } updated { emote { id } } removedIds
    }
}";

#[tokio::test]
async fn bulk_reads_keep_order_hide_like_single_reads_and_cap() {
    let Some(w) = World::new().await else {
        return;
    };
    let owner = w.user("1", "owner", SessionKind::Site).await;
    let other = w.user("2", "other", SessionKind::Site).await;
    let (a, a_id) = w.emote(&owner, "Alpha", false).await;
    let (b, b_id) = w.emote(&owner, "Beta", false).await;
    let (p, p_id) = w.emote(&owner, "Pend", true).await;
    let (_, s1) = w.set(&owner, "One", &[a, b, p]).await;
    let (_, s2) = w.set(&other, "Two", &[]).await;
    let (_, s3) = w.set(&owner, "Hidden", &[a]).await;
    w.db.hide_set(&s3, "PRIVATE").await;

    // Order asked, each once, unknown and hidden ones left out; pending only for the owner.
    let ask = json!({ "ids": [s2, s1, s3, "999999", s1] });
    let anon = w.gql(None, SETS, ask.clone()).await;
    let got = data(&anon, "sets");
    assert_eq!(ids(got), vec![s2.clone(), s1.clone()]);
    assert_eq!(member_ids(&got[1]), vec![a_id.clone(), b_id.clone()]);
    let mine = w.gql(Some(&owner), SETS, ask).await;
    let got = data(&mine, "sets");
    assert_eq!(ids(got), vec![s2.clone(), s1.clone(), s3.clone()]);
    assert_eq!(
        member_ids(&got[1]),
        vec![a_id.clone(), b_id.clone(), p_id.clone()]
    );
    // The same as the single read.
    let single = w
        .gql(
            None,
            "query($id: String!) { set(id: $id) { id version emotes { name emote { id } } } }",
            json!({ "id": s1 }),
        )
        .await;
    assert_eq!(data(&single, "set"), &data(&anon, "sets")[1]);

    let anon = w
        .gql(None, EMOTES, json!({ "ids": [p_id, b_id, a_id, b_id] }))
        .await;
    assert_eq!(ids(data(&anon, "emotes")), vec![b_id.clone(), a_id.clone()]);
    let mine = w
        .gql(Some(&owner), EMOTES, json!({ "ids": [p_id, b_id, a_id] }))
        .await;
    assert_eq!(
        ids(data(&mine, "emotes")),
        vec![p_id.clone(), b_id.clone(), a_id.clone()]
    );
    let others = w
        .gql(Some(&other), EMOTES, json!({ "ids": [p_id, a_id] }))
        .await;
    assert_eq!(ids(data(&others, "emotes")), vec![a_id.clone()]);

    {
        let mut conn = w.db.conn().await;
        channels::upsert(&mut conn, owner.id, Platform::Twitch, "1", "owner")
            .await
            .unwrap();
        channels::upsert(&mut conn, other.id, Platform::Twitch, "2", "other")
            .await
            .unwrap();
    }
    let r = w
        .gql(None, CHANNELS, json!({ "ids": ["2", "404", "1", "2"] }))
        .await;
    let logins: Vec<&str> = data(&r, "channels")
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["login"].as_str().unwrap())
        .collect();
    assert_eq!(logins, vec!["other", "owner"]);

    // REST twins: the public view.
    let r = w
        .post("/v1/sets/lookup", json!({ "ids": [s3, s1, s2] }))
        .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    assert_eq!(ids(&r.body["sets"]), vec![s1.clone(), s2.clone()]);
    assert_eq!(
        member_ids(&r.body["sets"][0]),
        vec![a_id.clone(), b_id.clone()]
    );
    assert!(r.body["sets"][0]["version"].is_i64());
    assert!(!header(&r, "ratelimit-limit").is_empty());
    let r = w
        .post(
            "/v1/channels/lookup",
            json!({ "platform": "twitch", "ids": ["1", "2", "3"] }),
        )
        .await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    assert_eq!(r.body["channels"].as_array().unwrap().len(), 2);
    assert_eq!(r.body["channels"][0]["login"], "owner");

    // Caps.
    let many = |n: usize| (0..n).map(|i| (i + 1).to_string()).collect::<Vec<_>>();
    let r = w.gql(None, SETS, json!({ "ids": many(26) })).await;
    assert_eq!(code(&r), "too_many", "{r}");
    assert_eq!(r["errors"][0]["extensions"]["limit"], 25, "{r}");
    assert_eq!(r["errors"][0]["extensions"]["current"], 26, "{r}");
    assert!(data(&w.gql(None, SETS, json!({ "ids": many(25) })).await, "sets").is_array());
    let r = w.gql(None, EMOTES, json!({ "ids": many(101) })).await;
    assert_eq!(code(&r), "too_many", "{r}");
    let r = w.gql(None, CHANNELS, json!({ "ids": many(101) })).await;
    assert_eq!(code(&r), "too_many", "{r}");
    let r = w.post("/v1/sets/lookup", json!({ "ids": many(26) })).await;
    assert_eq!(r.status, StatusCode::BAD_REQUEST);
    assert_eq!(r.body["error"], "too_many");
    assert_eq!(r.body["details"], json!({ "limit": 25, "current": 26 }));
    let r = w
        .post(
            "/v1/channels/lookup",
            json!({ "platform": "twitch", "ids": many(101) }),
        )
        .await;
    assert_eq!(r.status, StatusCode::BAD_REQUEST);
    assert_eq!(r.body["error"], "too_many");
    w.db.close().await;
}

#[tokio::test(flavor = "current_thread")]
async fn bulk_reads_cost_the_same_queries_for_2_ids_as_for_20() {
    let Some(w) = World::new().await else {
        return;
    };
    let owner = w.user("1", "owner", SessionKind::Site).await;
    let mut emote_rows = Vec::new();
    let mut emote_ids = Vec::new();
    for i in 0..20 {
        let (row, id) = w.emote(&owner, &format!("E{i:02}"), false).await;
        emote_rows.push(row);
        emote_ids.push(id);
    }
    let mut set_ids = Vec::new();
    for i in 0..20 {
        let (_, id) = w.set(&owner, &format!("S{i}"), &emote_rows[..3]).await;
        set_ids.push(id);
    }
    {
        let mut conn = w.db.conn().await;
        for i in 0..20 {
            let account = w
                .user(&format!("10{i}"), &format!("ch{i}"), SessionKind::Site)
                .await;
            channels::upsert(
                &mut conn,
                account.id,
                Platform::Twitch,
                &format!("10{i}"),
                &format!("ch{i}"),
            )
            .await
            .unwrap();
        }
    }
    let channel_ids: Vec<String> = (0..20).map(|i| format!("10{i}")).collect();
    for who in [None, Some(&owner)] {
        // Warm the per-account limits cache, so it isn't counted in the first round only.
        w.gql(who, "{ __typename }", json!({})).await;
        for (query, list) in [
            (SETS, &set_ids),
            (EMOTES, &emote_ids),
            (CHANNELS, &channel_ids),
        ] {
            let (few, a) = statements(w.gql(who, query, json!({ "ids": list[..2] }))).await;
            let (all, b) = statements(w.gql(who, query, json!({ "ids": list }))).await;
            assert!(a["errors"].is_null() && b["errors"].is_null(), "{a} {b}");
            assert!(few > 0, "statements are counted");
            assert_eq!(
                few, all,
                "{query}: {few} statements for 2 IDs, {all} for 20"
            );
        }
    }
    w.db.close().await;
}

#[tokio::test]
async fn every_write_path_bumps_the_set_version_and_logs_the_change() {
    let Some(w) = World::new().await else {
        return;
    };
    let owner = w.user("1", "owner", SessionKind::Site).await;
    let (a, a_id) = w.emote(&owner, "Alpha", false).await;
    let (b, b_id) = w.emote(&owner, "Beta", false).await;
    let (p, _) = w.emote(&owner, "Pend", true).await;
    let (set, set_id) = w.set(&owner, "Main", &[]).await;
    let (other_set, _) = w.set(&owner, "Other", &[a]).await;
    assert_eq!(w.version(set).await, 0, "a new set starts at 0");

    let mut v = 0;
    let r = w
        .gql(
            Some(&owner),
            "mutation($s: String!, $e: String!) { addSetEmote(setId: $s, emoteId: $e) { name } }",
            json!({ "s": set_id, "e": a_id }),
        )
        .await;
    assert!(r["errors"].is_null(), "{r}");
    v = step(&w, set, v, "addSetEmote", "added").await;
    w.gql(
        Some(&owner),
        "mutation($s: String!, $e: String!) { addSetEmote(setId: $s, emoteId: $e) { name } }",
        json!({ "s": set_id, "e": b_id }),
    )
    .await;
    v = step(&w, set, v, "addSetEmote again", "added").await;
    let r = w
        .gql(Some(&owner), "mutation($s: String!, $e: String!) { updateSetEmote(setId: $s, emoteId: $e, input: { name: \"Bee\" }) { name } }", json!({ "s": set_id, "e": b_id }))
        .await;
    assert!(r["errors"].is_null(), "{r}");
    v = step(&w, set, v, "updateSetEmote", "renamed").await;
    let r = w
        .gql(
            Some(&owner),
            "mutation($s: String!) { updateSet(id: $s, input: { name: \"Renamed\" }) { name } }",
            json!({ "s": set_id }),
        )
        .await;
    assert!(r["errors"].is_null(), "{r}");
    v = step(&w, set, v, "updateSet name", "set").await;
    let r = w
        .gql(
            Some(&owner),
            "mutation($s: String!) { updateSet(id: $s, input: { capacity: 500 }) { capacity } }",
            json!({ "s": set_id }),
        )
        .await;
    assert!(r["errors"].is_null(), "{r}");
    assert_eq!(
        w.version(set).await,
        v,
        "capacity isn't something a set shows in chat"
    );
    {
        let mut conn = w.db.conn().await;
        sets::add_member(
            &mut conn,
            set,
            sets::NewMember {
                emote_id: p,
                name: None,
                zero_width: None,
                added_by: None,
            },
        )
        .await
        .unwrap();
    }
    v = step(&w, set, v, "pending member", "added").await;
    {
        let mut conn = w.db.conn().await;
        emotes::set_state(&mut conn, p, EmoteState::Approved)
            .await
            .unwrap();
    }
    v = step(&w, set, v, "emote approved", "emote").await;
    {
        let mut conn = w.db.conn().await;
        emotes::update(
            &mut conn,
            a,
            &EmotePatch {
                name: Some("AlphaNew"),
                ..EmotePatch::default()
            },
        )
        .await
        .unwrap();
    }
    v = step(&w, set, v, "emote renamed", "emote").await;
    {
        let mut conn = w.db.conn().await;
        let file = files::insert_or_get(
            &mut conn,
            &files::NewFile {
                sha256: &[7u8; 32],
                dhash: None,
                mime: "image/png",
                width: 32,
                height: 32,
                frame_count: 1,
                storage_key: "uploads/test-file",
            },
        )
        .await
        .unwrap()
        .0;
        sqlx::query("UPDATE emotes SET file_id = $2 WHERE id = $1")
            .bind(b)
            .bind(file.id)
            .execute(&mut *conn)
            .await
            .unwrap();
        files::set_variants(&mut conn, file.id, &json!([{ "name": "1x.webp" }]))
            .await
            .unwrap();
    }
    v = step(&w, set, v, "file finished", "emote").await;
    let r = w
        .gql(
            Some(&owner),
            "mutation($s: String!, $e: String!) { removeSetEmote(setId: $s, emoteId: $e) }",
            json!({ "s": set_id, "e": a_id }),
        )
        .await;
    assert!(r["errors"].is_null(), "{r}");
    v = step(&w, set, v, "removeSetEmote", "removed").await;
    {
        let mut conn = w.db.conn().await;
        emotes::delete(&mut conn, b).await.unwrap();
    }
    step(&w, set, v, "emote deleted", "emote").await;
    // The other set holding `a` saw a's rename too; nothing else.
    assert!(
        w.kinds_after(other_set, 1)
            .await
            .iter()
            .all(|k| k == "emote")
    );

    // A copy: its members arrive as changes of the new set.
    let r = w
        .gql(
            Some(&owner),
            "mutation($s: String!) { cloneSet(id: $s, name: \"Copy\") { set { id version } } }",
            json!({ "s": set_id }),
        )
        .await;
    assert!(r["errors"].is_null(), "{r}");
    let copy =
        w.db.row(r["data"]["cloneSet"]["set"]["id"].as_str().unwrap())
            .await;
    assert!(w.version(copy).await > 0);
    assert!(w.kinds_after(copy, 0).await.iter().all(|k| k == "added"));
    // Deleting a set takes its log with it, without tripping over its own members' removal.
    let r = w
        .gql(
            Some(&owner),
            "mutation($s: String!) { deleteSet(id: $s) }",
            json!({ "s": set_id }),
        )
        .await;
    assert!(r["errors"].is_null(), "{r}");
    assert!(w.kinds_after(set, 0).await.is_empty());
    w.db.close().await;
}

#[tokio::test]
async fn deltas_bring_a_copy_up_to_date_and_old_versions_get_a_full_resync() {
    let Some(w) = World::new().await else {
        return;
    };
    let owner = w.user("1", "owner", SessionKind::Site).await;
    let (a, a_id) = w.emote(&owner, "Alpha", false).await;
    let (b, b_id) = w.emote(&owner, "Beta", false).await;
    let (c, c_id) = w.emote(&owner, "Gamma", false).await;
    let (p, p_id) = w.emote(&owner, "Pend", true).await;
    let (set, set_id) = w.set(&owner, "Main", &[a]).await;
    let start = w.version(set).await;
    let full = w.get(&format!("/v1/sets/{set_id}"), &[]).await;
    assert_eq!(full.body["version"], start);

    // Nothing changed: an empty delta.
    let r = w
        .gql(None, CHANGES, json!({ "id": set_id, "since": start }))
        .await;
    let d = data(&r, "setChanges");
    assert_eq!(d["full"], false);
    assert_eq!(d["version"], start);
    assert!(
        d["added"].as_array().unwrap().is_empty() && d["removedIds"].as_array().unwrap().is_empty()
    );

    {
        let mut conn = w.db.conn().await;
        for e in [b, c, p] {
            sets::add_member(
                &mut conn,
                set,
                sets::NewMember {
                    emote_id: e,
                    name: None,
                    zero_width: None,
                    added_by: None,
                },
            )
            .await
            .unwrap();
        }
        sets::update_member(&mut conn, set, c, Some("Gam"), None)
            .await
            .unwrap();
        sets::update_member(&mut conn, set, a, Some("Alf"), None)
            .await
            .unwrap();
        sets::remove_member(&mut conn, set, b).await.unwrap();
        sets::rename(&mut conn, set, "Main2").await.unwrap();
    }
    let now = w.version(set).await;
    let r = w
        .gql(None, CHANGES, json!({ "id": set_id, "since": start }))
        .await;
    let d = data(&r, "setChanges");
    assert_eq!(d["full"], false, "{d}");
    assert_eq!(d["version"], now);
    assert_eq!(d["name"], "Main2");
    // c was added and renamed since: one upsert, under added, with its new name.
    let added: Vec<(String, String)> = d["added"]
        .as_array()
        .unwrap()
        .iter()
        .map(|m| {
            (
                m["emote"]["id"].as_str().unwrap().to_owned(),
                m["name"].as_str().unwrap().to_owned(),
            )
        })
        .collect();
    assert_eq!(added, vec![(c_id.clone(), "Gam".to_owned())]);
    assert_eq!(d["renamed"][0]["emote"]["id"], a_id);
    assert_eq!(d["renamed"][0]["name"], "Alf");
    // b went; the pending one isn't shown to the public, so it reads as not there.
    let mut removed = ids_text(&d["removedIds"]);
    removed.sort();
    let mut want = vec![b_id.clone(), p_id.clone()];
    want.sort();
    assert_eq!(removed, want);
    // The owner sees the pending one as added.
    let r = w
        .gql(
            Some(&owner),
            CHANGES,
            json!({ "id": set_id, "since": start }),
        )
        .await;
    let d = data(&r, "setChanges");
    assert!(
        d["added"]
            .as_array()
            .unwrap()
            .iter()
            .any(|m| m["emote"]["id"] == p_id)
    );
    // Applying the delta to the old copy gives what a full read gives now.
    let mut copy: Vec<String> = member_ids(&full.body);
    let r = w
        .gql(None, CHANGES, json!({ "id": set_id, "since": start }))
        .await;
    let d = data(&r, "setChanges");
    for list in ["added", "renamed", "updated"] {
        for m in d[list].as_array().unwrap() {
            let id = m["emote"]["id"].as_str().unwrap().to_owned();
            if !copy.contains(&id) {
                copy.push(id);
            }
        }
    }
    copy.retain(|id| !ids_text(&d["removedIds"]).contains(id));
    let now_full = w.get(&format!("/v1/sets/{set_id}"), &[]).await;
    let mut want = member_ids(&now_full.body);
    want.sort();
    copy.sort();
    assert_eq!(copy, want);

    // Approval reaches the public as an upsert.
    {
        let mut conn = w.db.conn().await;
        emotes::set_state(&mut conn, p, EmoteState::Approved)
            .await
            .unwrap();
    }
    let r = w
        .gql(None, CHANGES, json!({ "id": set_id, "since": now }))
        .await;
    let d = data(&r, "setChanges");
    assert_eq!(d["updated"][0]["emote"]["id"], p_id, "{d}");

    // REST: snake_case, an ETag and 304.
    let r = w.get(&format!("/v1/sets/{set_id}?since={now}"), &[]).await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.body);
    assert_eq!(r.body["full"], false);
    assert!(r.body["removed_ids"].is_array());
    let etag = header(&r, "etag");
    assert!(!etag.is_empty());
    let again = w
        .get(
            &format!("/v1/sets/{set_id}?since={now}"),
            &[("if-none-match", etag)],
        )
        .await;
    assert_eq!(again.status, StatusCode::NOT_MODIFIED);

    // A since from the future, or one the pruned log no longer reaches: a full resync.
    let r = w
        .gql(None, CHANGES, json!({ "id": set_id, "since": now + 1000 }))
        .await;
    let d = data(&r, "setChanges");
    assert_eq!(d["full"], true);
    assert_eq!(
        member_ids(&d["set"]).len(),
        member_ids(&now_full.body).len() + 1
    );
    {
        let mut conn = w.db.conn().await;
        let gone = sets::prune_changes(&mut conn, chrono::Utc::now() + Duration::days(1), 1000)
            .await
            .unwrap();
        assert!(gone > 0);
    }
    let r = w
        .gql(None, CHANGES, json!({ "id": set_id, "since": start }))
        .await;
    assert_eq!(data(&r, "setChanges")["full"], true);
    let latest = w.version(set).await;
    let r = w
        .gql(None, CHANGES, json!({ "id": set_id, "since": latest }))
        .await;
    assert_eq!(
        data(&r, "setChanges")["full"],
        false,
        "the newest version stays answerable"
    );
    let r = w.get(&format!("/v1/sets/{set_id}?since=0"), &[]).await;
    assert_eq!(r.body["full"], true);
    assert_eq!(r.body["set"]["id"], set_id);

    // Only the newest 1.000 versions keep their rows.
    {
        let mut conn = w.db.conn().await;
        for i in 0..5 {
            sets::rename(&mut conn, set, &format!("Name{i}"))
                .await
                .unwrap();
        }
        sets::prune_changes(&mut conn, chrono::Utc::now() - Duration::days(7), 2)
            .await
            .unwrap();
        let s = sets::get(&mut conn, set).await.unwrap().unwrap();
        assert_eq!(s.change_floor, s.version - 2);
    }
    // Hidden sets stay hidden.
    w.db.hide_set(&set_id, "PRIVATE").await;
    let r = w
        .gql(None, CHANGES, json!({ "id": set_id, "since": 0 }))
        .await;
    assert!(data(&r, "setChanges").is_null());
    w.db.close().await;
}

/// The version after the step `what`, which must have gone up and logged a `kind` change.
async fn step(w: &World, set: Uuid, v: i64, what: &str, kind: &str) -> i64 {
    let now = w.version(set).await;
    assert!(now > v, "{what}: version {now} did not go past {v}");
    let kinds = w.kinds_after(set, v).await;
    assert!(
        kinds.iter().any(|k| k == kind),
        "{what}: {kinds:?} has no {kind}"
    );
    now
}

fn ids_text(list: &Value) -> Vec<String> {
    list.as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_owned())
        .collect()
}

#[tokio::test]
async fn persisted_queries_over_get_miss_hit_refuse_mutations_and_cache_publicly() {
    let Some(w) = World::new().await else {
        return;
    };
    let site = w.user("1", "owner", SessionKind::Site).await;
    let console = w.user("2", "dev", SessionKind::Console).await;
    let query = "{ globalSet { id name } }";
    let hash = cyex_tv_api::gql::hash_of(query);
    let ext = |h: &str| json!({ "persistedQuery": { "version": 1, "sha256Hash": h } }).to_string();

    // Miss: the hash alone is unknown.
    let r = w.get_gql(&[("extensions", ext(&hash))], &[]).await;
    assert_eq!(code(&r.body), "persisted_query_not_found", "{}", r.body);
    assert_eq!(r.body["errors"][0]["message"], "PersistedQueryNotFound");
    assert_eq!(header(&r, "cache-control"), "no-store");
    // A wrong hash with the text is refused, and stores nothing.
    let r = w
        .get_gql(
            &[
                ("query", query.to_owned()),
                ("extensions", ext(&"0".repeat(64))),
            ],
            &[],
        )
        .await;
    assert_eq!(code(&r.body), "persisted_query_mismatch");
    let r = w.get_gql(&[("extensions", ext("nothex"))], &[]).await;
    assert_eq!(code(&r.body), "bad_persisted_query");
    // Register with the text, then hit by the hash alone.
    let r = w
        .get_gql(
            &[("query", query.to_owned()), ("extensions", ext(&hash))],
            &[],
        )
        .await;
    assert_eq!(r.status, StatusCode::OK);
    assert!(r.body["errors"].is_null(), "{}", r.body);
    let hit = w.get_gql(&[("extensions", ext(&hash))], &[]).await;
    assert_eq!(hit.status, StatusCode::OK);
    assert_eq!(hit.body["data"], r.body["data"]);
    // Public: cacheable, with an ETag, a 304, and Vary on Authorization.
    let cache = header(&hit, "cache-control");
    assert!(cache.starts_with("public, max-age=30"), "{cache}");
    assert!(
        header(&hit, "vary")
            .to_lowercase()
            .contains("authorization")
    );
    let etag = header(&hit, "etag");
    assert!(!etag.is_empty());
    let again = w
        .get_gql(&[("extensions", ext(&hash))], &[("if-none-match", etag)])
        .await;
    assert_eq!(again.status, StatusCode::NOT_MODIFIED);
    // POST takes persisted queries too, and still answers as before.
    let r = w
        .post_gql(
            None,
            json!({ "extensions": { "persistedQuery": { "version": 1, "sha256Hash": hash } } }),
        )
        .await;
    assert!(r.body["errors"].is_null(), "{}", r.body);
    assert_eq!(r.body["data"], hit.body["data"]);

    // Mutations never run over GET: by text, or persisted (registered over POST) by hash.
    let mutation = "mutation { revokeOtherSessions }";
    let r = w.get_gql(&[("query", mutation.to_owned())], &[]).await;
    assert_eq!(r.status, StatusCode::METHOD_NOT_ALLOWED);
    assert_eq!(code(&r.body), "post_required");
    assert_eq!(header(&r, "allow"), "POST");
    let m_hash = cyex_tv_api::gql::hash_of(mutation);
    w.post_gql(Some(&site), json!({ "query": mutation, "extensions": { "persistedQuery": { "version": 1, "sha256Hash": m_hash } } }))
        .await;
    let r = w
        .get_gql(
            &[("extensions", ext(&m_hash))],
            &[("authorization", format!("Bearer {}", console.token))],
        )
        .await;
    assert_eq!(r.status, StatusCode::METHOD_NOT_ALLOWED);
    assert_eq!(code(&r.body), "post_required");
    let sub = "subscription { events(topics: [\"set:1\"]) { id } }";
    let r = w.get_gql(&[("query", sub.to_owned())], &[]).await;
    assert_eq!(code(&r.body), "post_required");
    // A document mixing a query with a mutation is refused whole.
    let r = w
        .get_gql(
            &[
                (
                    "query",
                    "query A { globalSet { id } } mutation B { revokeOtherSessions }".to_owned(),
                ),
                ("operationName", "A".to_owned()),
            ],
            &[],
        )
        .await;
    assert_eq!(code(&r.body), "post_required");

    // The cookie is never read over GET: a signed-in browser runs signed out there.
    let me = "{ me { id } }";
    let r = w
        .get_gql(
            &[("query", me.to_owned())],
            &[
                (
                    "cookie",
                    format!("{}={}", cyex_tv_auth::cookies::SESSION, site.token),
                ),
                ("origin", "https://cyex.tv".to_owned()),
            ],
        )
        .await;
    assert_eq!(code(&r.body), "signed_out", "{}", r.body);
    // A Bearer token is: its answers are private.
    let r = w
        .get_gql(
            &[("query", me.to_owned())],
            &[("authorization", format!("Bearer {}", console.token))],
        )
        .await;
    assert!(r.body["errors"].is_null(), "{}", r.body);
    assert!(!r.body["data"]["me"].is_null());
    assert_eq!(header(&r, "cache-control"), "private, no-cache");
    // GET without a query still opens GraphiQL.
    let r = w
        .send(Request::get("/v1/gql").body(Body::empty()).unwrap())
        .await;
    assert_eq!(r.status, StatusCode::OK);
    assert!(header(&r, "content-type").starts_with("text/html"));
    w.db.close().await;
}
