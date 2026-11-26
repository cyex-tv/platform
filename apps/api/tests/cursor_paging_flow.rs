//! Cursor (keyset) paging (ADR 0030): `after` and `next` on `searchEmotes`, `searchAccounts`,
//! `setEmotes`, `myEmotes`, `accountEmotes`, `reviewQueue` and `GET /v1/sets/{id}/emotes`. A walk
//! through a list with rows added and removed between pages shows every row that was there once
//! and never twice; a bad cursor is `bad_cursor`; the page size cap holds. Through the real router
//! against a real Postgres; runs when `CYEX_TV_TEST_DATABASE_URL` is set (CI sets it), skips
//! otherwise.

mod common;

use std::collections::HashSet;
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
use cyex_tv_db::{roles, sets};
use cyex_tv_shared::model::{EmoteState, Platform, SetKind};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use tower::ServiceExt;
use uuid::Uuid;

const SEARCH: &str = "query($q: String!, $sort: EmoteSort, $per: Int, $page: Int, $after: String) {
    searchEmotes(query: $q, sort: $sort, perPage: $per, page: $page, after: $after) {
        total perPage next items { id name }
    }
}";
const ACCOUNTS: &str = "query($q: String!, $per: Int, $after: String) {
    searchAccounts(query: $q, perPage: $per, after: $after) {
        total next items { account { id } }
    }
}";
const SET_EMOTES: &str = "query($id: String!, $per: Int, $after: String) {
    setEmotes(id: $id, perPage: $per, after: $after) { total next items { id name } }
}";
const MY_EMOTES: &str = "query($per: Int, $after: String) {
    myEmotes(perPage: $per, after: $after) { total next items { id name } }
}";
const ACCOUNT_EMOTES: &str = "query($id: String!, $per: Int, $after: String) {
    accountEmotes(accountId: $id, perPage: $per, after: $after) { total next items { id name } }
}";
const QUEUE: &str = "query($per: Int, $after: String, $filter: ReviewQueueFilter) {
    reviewQueue(perPage: $per, after: $after, filter: $filter) { total next items { id name } }
}";

struct User {
    id: Uuid,
    token: String,
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

/// The field's answer; panics on an error.
fn data<'a>(r: &'a Value, field: &str) -> &'a Value {
    assert!(r["errors"].is_null(), "{r}");
    &r["data"][field]
}

fn ids_of(page: &Value) -> Vec<String> {
    page["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| {
            i["id"]
                .as_str()
                .or_else(|| i["account"]["id"].as_str())
                .unwrap()
                .to_owned()
        })
        .collect()
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

    async fn user(&self, twitch_id: &str, login: &str, permission: Option<&str>) -> User {
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
        if let Some(permission) = permission {
            let role = roles::create(
                &mut conn,
                &roles::NewRole {
                    name: &format!("{permission} {twitch_id}"),
                    description: None,
                    position: 1,
                    permissions: &[permission],
                    created_by: None,
                },
            )
            .await
            .unwrap();
            roles::assign(&mut conn, signed.account.id, role.id, None)
                .await
                .unwrap();
        }
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

    /// An emote of `owner`, approved unless `pending`; its ID as the API shows it.
    async fn emote(&self, owner: &User, name: &str, pending: bool) -> String {
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
        self.db.public(e.id).await
    }

    async fn delete(&self, public: &str) {
        let id = self.db.row(public).await;
        let mut conn = self.db.conn().await;
        emotes::delete(&mut conn, id).await.unwrap();
    }

    async fn gql(&self, user: Option<&User>, query: &str, variables: Value) -> Value {
        let mut request = Request::post("/v1/gql").header("content-type", "application/json");
        if let Some(user) = user {
            request = request
                .header(
                    "cookie",
                    format!("{}={}", cyex_tv_auth::cookies::SESSION, user.token),
                )
                .header("origin", "https://cyex.tv");
        }
        let request = request
            .body(Body::from(
                json!({ "query": query, "variables": variables }).to_string(),
            ))
            .unwrap();
        let response = self.app.clone().oneshot(request).await.unwrap();
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        serde_json::from_slice(&bytes).unwrap()
    }
}

/// Every ID of `want` shows exactly once in `seen`, in `want`'s order, and nothing shows twice.
fn assert_walk(what: &str, seen: &[String], want: &[String]) {
    let mut once = HashSet::new();
    for id in seen {
        assert!(once.insert(id), "{what}: {id} shown twice in {seen:?}");
    }
    let kept: Vec<&String> = seen.iter().filter(|id| want.contains(id)).collect();
    let want: Vec<&String> = want.iter().collect();
    assert_eq!(kept, want, "{what}: skipped or out of order");
}

#[tokio::test]
async fn search_walks_by_cursor_without_duplicates_or_skips() {
    let Some(w) = World::new().await else {
        return;
    };
    let owner = w.user("1", "owner", None).await;
    for i in 0..23 {
        w.emote(&owner, &format!("Walk{i:02}"), false).await;
    }
    for (sort, text) in [
        ("NEW", ""),
        ("TOP", ""),
        ("RELEVANCE", ""),
        ("RELEVANCE", "walk"),
        ("TOP", "walk"),
    ] {
        let vars = |per: u32, after: Option<&str>| json!({ "q": text, "sort": sort, "per": per, "after": after });
        // The whole list as numbered pages read it, before anything changes.
        let all = w.gql(None, SEARCH, vars(100, None)).await;
        let mut want = ids_of(data(&all, "searchEmotes"));
        assert!(want.len() >= 23, "{sort} {text:?}: {want:?}");
        assert!(
            data(&all, "searchEmotes")["next"].is_null(),
            "one page, no next"
        );

        let mut seen: Vec<String> = Vec::new();
        let mut after: Option<String> = None;
        let mut round = 0;
        loop {
            let r = w.gql(None, SEARCH, vars(5, after.as_deref())).await;
            let page = data(&r, "searchEmotes");
            let got = ids_of(page);
            assert!(got.len() <= 5);
            seen.extend(got);
            // The first page's cursor matches a numbered first page's.
            if round == 0 {
                let numbered = w
                    .gql(
                        None,
                        SEARCH,
                        json!({ "q": text, "sort": sort, "per": 5, "page": 1 }),
                    )
                    .await;
                assert_eq!(data(&numbered, "searchEmotes")["next"], page["next"]);
                // An emote already shown goes away: with offsets the next page would skip one.
                let gone = seen[0].clone();
                w.delete(&gone).await;
                want.retain(|id| *id != gone);
                seen.retain(|id| *id != gone);
            }
            // Emotes are added between pages; they may show or not, but never move the rest.
            w.emote(&owner, &format!("Walk{sort}{round}x"), false).await;
            round += 1;
            match page["next"].as_str() {
                Some(next) => after = Some(next.to_owned()),
                None => break,
            }
            assert!(round < 50, "the walk never ends");
        }
        assert_walk(&format!("{sort} {text:?}"), &seen, &want);
    }
    w.db.close().await;
}

#[tokio::test]
async fn bad_cursors_are_bad_cursor_and_the_page_size_cap_holds() {
    let Some(w) = World::new().await else {
        return;
    };
    let owner = w.user("1", "owner", None).await;
    for i in 0..105 {
        w.emote(&owner, &format!("Cap{i:03}"), false).await;
    }
    let first = w
        .gql(None, SEARCH, json!({ "q": "", "sort": "NEW", "per": 3 }))
        .await;
    let next = data(&first, "searchEmotes")["next"]
        .as_str()
        .unwrap()
        .to_owned();

    // Made up, cut, edited, or from another list (other order, other text).
    let mut edited: Vec<char> = next.chars().collect();
    let mid = edited.len() / 2;
    edited[mid] = if edited[mid] == 'A' { 'B' } else { 'A' };
    let edited: String = edited.into_iter().collect();
    for (bad, sort, text) in [
        ("junk".to_owned(), "NEW", ""),
        (next[..next.len() - 4].to_owned(), "NEW", ""),
        (edited, "NEW", ""),
        (next.clone(), "TOP", ""),
        (next.clone(), "NEW", "cap"),
    ] {
        let r = w
            .gql(
                None,
                SEARCH,
                json!({ "q": text, "sort": sort, "per": 3, "after": bad }),
            )
            .await;
        assert_eq!(code(&r), "bad_cursor", "{bad} {sort} {text:?}: {r}");
    }
    for (query, vars) in [(ACCOUNTS, json!({ "q": "owner", "after": next }))] {
        let r = w.gql(None, query, vars).await;
        assert_eq!(code(&r), "bad_cursor", "{r}");
    }
    let r = w
        .gql(Some(&owner), MY_EMOTES, json!({ "after": next }))
        .await;
    assert_eq!(
        code(&r),
        "bad_cursor",
        "a search cursor is no myEmotes cursor: {r}"
    );

    // The cap: at most 100 a page, with a cursor as without.
    let r = w
        .gql(
            None,
            SEARCH,
            json!({ "q": "", "sort": "NEW", "per": 500, "after": next }),
        )
        .await;
    let page = data(&r, "searchEmotes");
    assert_eq!(page["perPage"], 100);
    assert_eq!(ids_of(page).len(), 100);
    assert_eq!(page["total"], 105);
    assert!(page["next"].is_string(), "2 are left after 3 + 100");
    let r = w.gql(Some(&owner), MY_EMOTES, json!({ "per": 500 })).await;
    assert_eq!(ids_of(data(&r, "myEmotes")).len(), 100);
    w.db.close().await;
}

#[tokio::test]
async fn set_and_owner_lists_walk_by_cursor() {
    let Some(w) = World::new().await else {
        return;
    };
    let owner = w.user("1", "owner", None).await;
    let staff = w.user("2", "staff", Some("users.view")).await;
    let set = {
        let mut conn = w.db.conn().await;
        sets::create(&mut conn, owner.id, "Big", SetKind::Channel)
            .await
            .unwrap()
    };
    let set_id = w.db.public(set.id).await;
    let add = |emote: String| {
        let w = &w;
        let set = set.id;
        let owner = owner.id;
        async move {
            let id = w.db.row(&emote).await;
            let mut conn = w.db.conn().await;
            sets::add_member(
                &mut conn,
                set,
                sets::NewMember {
                    emote_id: id,
                    name: None,
                    zero_width: None,
                    added_by: Some(owner),
                },
            )
            .await
            .unwrap();
        }
    };
    for i in 0..17 {
        // A pending one in every five: the owner sees it in their own lists and set.
        let e = w.emote(&owner, &format!("Mem{i:02}"), i % 5 == 4).await;
        add(e).await;
    }

    // The set, through GraphQL: members added between pages come at the end (oldest first),
    // and none shown is shown again.
    let all = w
        .gql(
            Some(&owner),
            SET_EMOTES,
            json!({ "id": set_id, "per": 200 }),
        )
        .await;
    let want = ids_of(data(&all, "setEmotes"));
    assert_eq!(want.len(), 17);
    let mut seen = Vec::new();
    let mut after: Option<String> = None;
    let mut round = 0;
    loop {
        let r = w
            .gql(
                Some(&owner),
                SET_EMOTES,
                json!({ "id": set_id, "per": 4, "after": after }),
            )
            .await;
        let page = data(&r, "setEmotes");
        seen.extend(ids_of(page));
        let e = w.emote(&owner, &format!("Late{round}"), false).await;
        add(e).await;
        round += 1;
        match page["next"].as_str() {
            Some(next) => after = Some(next.to_owned()),
            None => break,
        }
        assert!(round < 50);
    }
    assert_walk("setEmotes", &seen, &want);
    assert!(seen.len() > want.len(), "late members show at the end");

    // The REST read: `next` and the `Link` header, followed to the end.
    let mut url = format!("/v1/sets/{set_id}/emotes?per_page=6");
    let mut rest_seen: Vec<String> = Vec::new();
    for _ in 0..20 {
        let response = w
            .app
            .clone()
            .oneshot(Request::get(&url).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let link = response
            .headers()
            .get("link")
            .map(|v| v.to_str().unwrap().to_owned());
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        let page: Value = serde_json::from_slice(&bytes).unwrap();
        rest_seen.extend(ids_of(&page));
        match (page["next"].as_str(), link) {
            (Some(next), Some(link)) => {
                assert!(link.ends_with("; rel=\"next\""), "{link}");
                assert!(link.contains(next), "{link} names {next}");
                url = link
                    .trim_start_matches('<')
                    .split('>')
                    .next()
                    .unwrap()
                    .to_owned();
            }
            (None, None) => break,
            other => panic!("next and Link disagree: {other:?}"),
        }
    }
    let unique: HashSet<&String> = rest_seen.iter().collect();
    assert_eq!(
        unique.len(),
        rest_seen.len(),
        "the REST walk repeats a member"
    );
    // Signed out, the pending members don't show; every approved one does.
    let approved = w
        .gql(None, SET_EMOTES, json!({ "id": set_id, "per": 200 }))
        .await;
    assert_eq!(rest_seen, ids_of(data(&approved, "setEmotes")));
    let response = w
        .app
        .clone()
        .oneshot(
            Request::get(format!("/v1/sets/{set_id}/emotes?after=junk"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let response = w
        .app
        .clone()
        .oneshot(
            Request::get("/v1/sets/99999999999999/emotes")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    // The owner's own emotes, newest first: new uploads go before the position, so a walk
    // never shows them and never repeats one; staff read the same list by account.
    let owner_id = w.db.public(owner.id).await;
    for (who, query, field, vars) in [
        (&owner, MY_EMOTES, "myEmotes", json!({})),
        (
            &staff,
            ACCOUNT_EMOTES,
            "accountEmotes",
            json!({ "id": owner_id }),
        ),
    ] {
        let mut all_vars = vars.clone();
        all_vars["per"] = json!(100);
        let all = w.gql(Some(who), query, all_vars).await;
        let want = ids_of(data(&all, field));
        let mut seen = Vec::new();
        let mut after: Option<String> = None;
        for round in 0..50 {
            let mut v = vars.clone();
            v["per"] = json!(4);
            v["after"] = json!(after);
            let r = w.gql(Some(who), query, v).await;
            let page = data(&r, field);
            seen.extend(ids_of(page));
            w.emote(&owner, &format!("New{field}{round}"), true).await;
            match page["next"].as_str() {
                Some(next) => after = Some(next.to_owned()),
                None => break,
            }
        }
        assert_eq!(seen, want, "{field}: exactly the list as it was");
    }
    w.db.close().await;
}

#[tokio::test]
async fn account_search_and_the_review_queue_walk_by_cursor() {
    let Some(w) = World::new().await else {
        return;
    };
    let mut want_accounts = Vec::new();
    for i in 0..12 {
        let u = w
            .user(&format!("{}", 100 + i), &format!("pager{i:02}"), None)
            .await;
        want_accounts.push(w.db.public(u.id).await);
    }
    let all = w
        .gql(None, ACCOUNTS, json!({ "q": "pager", "per": 50 }))
        .await;
    let want = ids_of(data(&all, "searchAccounts"));
    assert_eq!(want.len(), 12);
    let mut seen = Vec::new();
    let mut after: Option<String> = None;
    for _ in 0..20 {
        let r = w
            .gql(
                None,
                ACCOUNTS,
                json!({ "q": "pager", "per": 5, "after": after }),
            )
            .await;
        let page = data(&r, "searchAccounts");
        seen.extend(ids_of(page));
        assert_eq!(page["total"], 12);
        match page["next"].as_str() {
            Some(next) => after = Some(next.to_owned()),
            None => break,
        }
    }
    assert_eq!(seen, want);

    // The review queue, oldest upload first: uploads between pages come at the end.
    let reviewer = w.user("900", "reviewer", Some("emotes.review")).await;
    let uploader = w.user("901", "uploader", None).await;
    for i in 0..9 {
        w.emote(&uploader, &format!("Wait{i}"), true).await;
    }
    for filter in [json!(null), json!({ "sort": "NAME", "descending": true })] {
        let all = w
            .gql(
                Some(&reviewer),
                QUEUE,
                json!({ "per": 100, "filter": filter }),
            )
            .await;
        let want = ids_of(data(&all, "reviewQueue"));
        let mut seen = Vec::new();
        let mut after: Option<String> = None;
        for round in 0..50 {
            let r = w
                .gql(
                    Some(&reviewer),
                    QUEUE,
                    json!({ "per": 2, "after": after, "filter": filter }),
                )
                .await;
            let page = data(&r, "reviewQueue");
            seen.extend(ids_of(page));
            w.emote(&uploader, &format!("Wait{round}z"), true).await;
            match page["next"].as_str() {
                Some(next) => after = Some(next.to_owned()),
                None => break,
            }
        }
        assert_walk(&format!("reviewQueue {filter}"), &seen, &want);
        // The cursor is bound to the filter.
        let other = w
            .gql(
                Some(&reviewer),
                QUEUE,
                json!({ "per": 2, "filter": { "sort": "UPLOADER" } }),
            )
            .await;
        let cursor = data(&other, "reviewQueue")["next"].clone();
        let r = w
            .gql(
                Some(&reviewer),
                QUEUE,
                json!({ "per": 2, "after": cursor, "filter": filter }),
            )
            .await;
        assert_eq!(code(&r), "bad_cursor", "{r}");
    }
    w.db.close().await;
}
