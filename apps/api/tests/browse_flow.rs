//! `searchEmotes` flags, tags, exact names and the trending sort, and `emoteTrending`, against a
//! real Postgres. Runs when `CYEX_TV_TEST_DATABASE_URL` is set (CI sets it), skips otherwise.

mod common;

use std::sync::Arc;

use axum::body::Body;
use axum::http::Request;
use common::TestDb;
use cyex_tv_api::state::AppState;
use cyex_tv_auth::{Auth, AuthConfig};
use cyex_tv_db::accounts::{self, Profile};
use cyex_tv_db::emotes::{self, NewEmote};
use cyex_tv_db::{files, ids};
use cyex_tv_shared::model::{EmoteState, Platform};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use tower::ServiceExt;
use uuid::Uuid;

struct Emotes {
    alpha: Uuid,
    bravo: Uuid,
    charlie: Uuid,
    delta: Uuid,
    echo: Uuid,
}

/// An approved emote with a `1x.webp` of the given size (none: no file).
async fn make(
    c: &mut sqlx::PgConnection,
    owner: Uuid,
    name: &str,
    size: Option<(i32, i32)>,
    zero_width: bool,
    nsfw: bool,
    tags: &[&str],
) -> Uuid {
    let mut file_id = None;
    if let Some((width, height)) = size {
        let mut sha = [0u8; 32];
        let digest = name.as_bytes();
        sha[..digest.len()].copy_from_slice(digest);
        let key = format!("emotes/{name}");
        let (file, _) = files::insert_or_get(
            c,
            &files::NewFile {
                sha256: &sha,
                dhash: None,
                mime: "image/webp",
                width,
                height,
                frame_count: 1,
                storage_key: &key,
            },
        )
        .await
        .unwrap();
        files::set_variants(
            c,
            file.id,
            &json!([{
                "name": "1x.webp", "format": "webp", "scale": 1,
                "width": width, "height": height, "size": 100, "is_static": false
            }]),
        )
        .await
        .unwrap();
        file_id = Some(file.id);
    }
    let e = emotes::create(
        c,
        &NewEmote {
            owner_account_id: owner,
            file_id,
            name,
            zero_width,
            nsfw,
            ..NewEmote::default()
        },
    )
    .await
    .unwrap();
    emotes::set_state(c, e.id, EmoteState::Approved)
        .await
        .unwrap();
    let tags: Vec<String> = tags.iter().map(|t| (*t).to_owned()).collect();
    emotes::set_tags(c, e.id, &tags).await.unwrap();
    e.id
}

async fn seed(db: &TestDb) -> Emotes {
    let mut c = db.conn().await;
    let owner = accounts::sign_in(
        &mut c,
        Profile {
            platform: Platform::Twitch,
            platform_user_id: "1",
            login: "user1",
            display_name: "user1",
            avatar: None,
        },
    )
    .await
    .unwrap()
    .account
    .id;
    let c = &mut *c;
    Emotes {
        alpha: make(
            c,
            owner,
            "alpha",
            Some((64, 32)),
            true,
            false,
            &["cool", "new"],
        )
        .await,
        bravo: make(c, owner, "bravo", Some((32, 32)), true, false, &["cool"]).await,
        // 7 to 4 is the limit: 56 by 32 is wide, 55 by 32 is not.
        charlie: make(
            c,
            owner,
            "charlie",
            Some((56, 32)),
            false,
            false,
            &["cool", "new"],
        )
        .await,
        delta: make(c, owner, "delta", None, false, false, &[]).await,
        echo: make(c, owner, "echo", Some((100, 20)), false, true, &["new"]).await,
    }
}

fn app_for(db: &TestDb) -> axum::Router {
    let auth = Arc::new(Auth::new(AuthConfig::from_source(|_| None).unwrap()).unwrap());
    cyex_tv_api::app(AppState::new(db.pool.clone(), auth))
}

async fn gql(app: &axum::Router, query: &str) -> Value {
    let request = Request::post("/v1/gql")
        .header("content-type", "application/json")
        .body(Body::from(json!({ "query": query }).to_string()))
        .unwrap();
    let response = app.clone().oneshot(request).await.unwrap();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let v: Value = serde_json::from_slice(&bytes).unwrap();
    assert!(v["errors"].is_null(), "{v}");
    v["data"].clone()
}

async fn find(app: &axum::Router, args: &str) -> Value {
    let data = gql(
        app,
        &format!(
            "{{ searchEmotes({args}) {{ total items {{ name flags {{ zeroWidth nsfw wide }} }} }} }}"
        ),
    )
    .await;
    data["searchEmotes"].clone()
}

fn names(result: &Value) -> Vec<String> {
    result["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| i["name"].as_str().unwrap().to_owned())
        .collect()
}

async fn trending(app: &axum::Router, id: Uuid, window: &str) -> Value {
    let data = gql(
        app,
        &format!(
            r#"{{ emoteTrending(id: "{}", window: {window}) {{ rank channels window }} }}"#,
            ids::to_text(id)
        ),
    )
    .await;
    data["emoteTrending"].clone()
}

#[tokio::test]
async fn flags_tags_and_exact_narrow_the_search() {
    let Some(db) = TestDb::new().await else {
        return;
    };
    seed(&db).await;
    let app = app_for(&db);

    // Defaults are as before: NSFW hidden, every other emote listed.
    let r = find(&app, r#"query: "", sort: TOP"#).await;
    assert_eq!(names(&r), ["alpha", "bravo", "charlie", "delta"]);
    assert_eq!(r["total"], 4);

    // The flags are on the emotes: wide comes from the 1x file.
    let wide: Vec<(String, bool)> = r["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| {
            (
                i["name"].as_str().unwrap().to_owned(),
                i["flags"]["wide"].as_bool().unwrap(),
            )
        })
        .collect();
    assert_eq!(
        wide,
        [
            ("alpha".to_owned(), true),
            ("bravo".to_owned(), false),
            ("charlie".to_owned(), true),
            ("delta".to_owned(), false),
        ]
    );

    let r = find(&app, r#"query: "", sort: TOP, flags: [ZERO_WIDTH]"#).await;
    assert_eq!(names(&r), ["alpha", "bravo"]);
    assert!(
        r["items"]
            .as_array()
            .unwrap()
            .iter()
            .all(|i| i["flags"]["zeroWidth"] == true)
    );

    let r = find(&app, r#"query: "", sort: TOP, flags: [WIDE]"#).await;
    assert_eq!(names(&r), ["alpha", "charlie"]);
    assert_eq!(r["total"], 2);
    let r = find(
        &app,
        r#"query: "", sort: TOP, flags: [WIDE, WIDE], nsfw: true"#,
    )
    .await;
    assert_eq!(names(&r), ["alpha", "charlie", "echo"]);

    let r = find(&app, r#"query: "", sort: TOP, flags: [ZERO_WIDTH, WIDE]"#).await;
    assert_eq!(names(&r), ["alpha"]);

    // NSFW lists only NSFW emotes, asked for or not.
    let r = find(&app, r#"query: "", flags: [NSFW]"#).await;
    assert_eq!(names(&r), ["echo"]);
    assert_eq!(r["items"][0]["flags"]["nsfw"], true);
    assert_eq!(r["items"][0]["flags"]["wide"], true);
    let r = find(&app, r#"query: "", flags: [NSFW, ZERO_WIDTH]"#).await;
    assert_eq!(r["total"], 0);

    // Every tag must match; case, spaces and a leading # don't matter.
    let r = find(&app, r#"query: "", sort: TOP, tags: ["cool", "new"]"#).await;
    assert_eq!(names(&r), ["alpha", "charlie"]);
    let r = find(
        &app,
        r##"query: "", sort: TOP, tags: ["#COOL", " new ", ""]"##,
    )
    .await;
    assert_eq!(names(&r), ["alpha", "charlie"]);
    let r = find(&app, r#"query: "", sort: TOP, tags: [""]"#).await;
    assert_eq!(r["total"], 4);
    let r = find(&app, r#"query: "", tags: ["cool", "nothere"]"#).await;
    assert_eq!(r["total"], 0);
    // A tag that can never exist matches nothing, without an error.
    let r = find(&app, r#"query: "", tags: ["not a tag"]"#).await;
    assert_eq!(r["total"], 0);
    // Exact: the whole name, any case.
    let r = find(&app, r#"query: "ALPHA", exact: true"#).await;
    assert_eq!(names(&r), ["alpha"]);
    let r = find(&app, r#"query: "alph", exact: true"#).await;
    assert_eq!(r["total"], 0);
    let r = find(&app, r#"query: "alph""#).await;
    assert_eq!(names(&r), ["alpha"]);
    let r = find(&app, r#"query: "", exact: true, sort: TOP"#).await;
    assert_eq!(r["total"], 4);
    db.close().await;
}

/// Net channel adds `ago` hours back (bucketed by hour).
async fn adds(c: &mut sqlx::PgConnection, emote: Uuid, ago: &str, adds: i32, removes: i32) {
    sqlx::query(
        "INSERT INTO emote_channel_adds (emote_id, hour, adds, removes)
         VALUES ($1, date_trunc('hour', now() - $2::interval), $3, $4)",
    )
    .bind(emote)
    .bind(ago)
    .bind(adds)
    .bind(removes)
    .execute(c)
    .await
    .unwrap();
}

#[tokio::test]
async fn trending_orders_by_net_channel_adds_in_the_window() {
    let Some(db) = TestDb::new().await else {
        return;
    };
    let e = seed(&db).await;
    {
        let mut c = db.conn().await;
        let c = &mut *c;
        // Net per window: alpha 2 / 3 / 4, bravo 1 / 6 / 11, charlie 0 / 0 / 10, echo 9 / 9 / 9.
        adds(c, e.alpha, "2 hours", 2, 0).await;
        adds(c, e.alpha, "3 days", 1, 0).await;
        adds(c, e.alpha, "20 days", 1, 0).await;
        adds(c, e.alpha, "40 days", 50, 0).await;
        adds(c, e.bravo, "2 hours", 1, 0).await;
        adds(c, e.bravo, "3 days", 5, 0).await;
        adds(c, e.bravo, "20 days", 5, 0).await;
        adds(c, e.charlie, "3 days", 2, 2).await;
        adds(c, e.charlie, "20 days", 10, 0).await;
        // Removed more than added: never listed.
        adds(c, e.delta, "2 hours", 1, 3).await;
        adds(c, e.echo, "2 hours", 9, 0).await;
    }
    let app = app_for(&db);

    let r = find(&app, r#"query: "", sort: TRENDING, window: DAY"#).await;
    assert_eq!(names(&r), ["alpha", "bravo"]);
    assert_eq!(r["total"], 2);
    // WEEK is the default.
    let r = find(&app, r#"query: "", sort: TRENDING, window: WEEK"#).await;
    assert_eq!(names(&r), ["bravo", "alpha"]);
    let r = find(&app, r#"query: "", sort: TRENDING"#).await;
    assert_eq!(names(&r), ["bravo", "alpha"]);
    let r = find(&app, r#"query: "", sort: TRENDING, window: MONTH"#).await;
    assert_eq!(names(&r), ["bravo", "charlie", "alpha"]);
    assert_eq!(r["total"], 3);

    // Filters still apply.
    let r = find(
        &app,
        r#"query: "", sort: TRENDING, window: DAY, nsfw: true"#,
    )
    .await;
    assert_eq!(names(&r), ["echo", "alpha", "bravo"]);
    let r = find(
        &app,
        r#"query: "", sort: TRENDING, window: DAY, flags: [NSFW]"#,
    )
    .await;
    assert_eq!(names(&r), ["echo"]);
    let r = find(
        &app,
        r#"query: "", sort: TRENDING, window: MONTH, flags: [WIDE]"#,
    )
    .await;
    assert_eq!(names(&r), ["charlie", "alpha"]);
    let r = find(
        &app,
        r#"query: "", sort: TRENDING, window: MONTH, flags: [ZERO_WIDTH]"#,
    )
    .await;
    assert_eq!(names(&r), ["bravo", "alpha"]);
    let r = find(
        &app,
        r#"query: "", sort: TRENDING, window: MONTH, tags: ["new"]"#,
    )
    .await;
    assert_eq!(names(&r), ["charlie", "alpha"]);
    let r = find(&app, r#"query: "alp", sort: TRENDING, window: MONTH"#).await;
    assert_eq!(names(&r), ["alpha"]);
    let r = find(
        &app,
        r#"query: "alpha", exact: true, sort: TRENDING, window: DAY"#,
    )
    .await;
    assert_eq!(names(&r), ["alpha"]);
    let r = find(
        &app,
        r#"query: "", sort: TRENDING, window: MONTH, perPage: 1, page: 2"#,
    )
    .await;
    assert_eq!(names(&r), ["charlie"]);
    assert_eq!(r["total"], 3);

    // Where an emote stands.
    let t = trending(&app, e.alpha, "DAY").await;
    assert_eq!(t, json!({ "rank": 1, "channels": 2, "window": "DAY" }));
    let t = trending(&app, e.alpha, "WEEK").await;
    assert_eq!(t, json!({ "rank": 2, "channels": 3, "window": "WEEK" }));
    let t = trending(&app, e.alpha, "MONTH").await;
    assert_eq!(t, json!({ "rank": 3, "channels": 4, "window": "MONTH" }));
    let t = trending(&app, e.bravo, "MONTH").await;
    assert_eq!(t, json!({ "rank": 1, "channels": 11, "window": "MONTH" }));
    // Net zero or below, or no adds in the window: null.
    assert!(trending(&app, e.charlie, "WEEK").await.is_null());
    assert!(trending(&app, e.delta, "DAY").await.is_null());
    assert!(trending(&app, e.delta, "MONTH").await.is_null());
    // An NSFW emote ranks among everything; the others don't see it.
    let t = trending(&app, e.echo, "DAY").await;
    assert_eq!(t, json!({ "rank": 1, "channels": 9, "window": "DAY" }));
    // Without a window, WEEK.
    let data = gql(
        &app,
        &format!(
            r#"{{ emoteTrending(id: "{}") {{ rank window }} }}"#,
            ids::to_text(e.bravo)
        ),
    )
    .await;
    assert_eq!(
        data["emoteTrending"],
        json!({ "rank": 1, "window": "WEEK" })
    );
    let data = gql(&app, r#"{ emoteTrending(id: "nope") { rank } }"#).await;
    assert!(data["emoteTrending"].is_null());
    db.close().await;
}
