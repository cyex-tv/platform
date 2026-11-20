//! `searchEmotes` against a real Postgres. Runs when `CYEX_TV_TEST_DATABASE_URL` is set (CI sets
//! it), skips otherwise.

mod common;

use std::sync::Arc;

use axum::body::Body;
use axum::http::Request;
use common::TestDb;
use cyex_tv_api::state::AppState;
use cyex_tv_auth::{Auth, AuthConfig};
use cyex_tv_db::accounts::{self, Profile};
use cyex_tv_db::emotes::{self, EmotePatch, NewEmote};
use cyex_tv_db::{channel_counts, channels, ids, search, sets};
use cyex_tv_shared::model::{EmoteState, Platform, SetKind};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use tower::ServiceExt;
use uuid::Uuid;

async fn account(conn: &mut sqlx::PgConnection, twitch_id: &str) -> Uuid {
    let login = format!("user{twitch_id}");
    let id = accounts::sign_in(
        conn,
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
    channels::upsert(conn, id, Platform::Twitch, twitch_id, &login)
        .await
        .unwrap();
    id
}

struct Made {
    owner: Uuid,
    other: Uuid,
}

/// The emotes every test searches.
async fn seed(db: &TestDb) -> Made {
    let mut c = db.conn().await;
    let owner = account(&mut c, "1").await;
    let other = account(&mut c, "2").await;
    let make = |name: &'static str| NewEmote {
        owner_account_id: owner,
        name,
        ..NewEmote::default()
    };
    let mut approved = Vec::new();
    for (name, animated) in [
        ("PogChamp", false),
        ("PogU", true),
        ("KEKW", false),
        ("pog", false),
    ] {
        let e = emotes::create(
            &mut c,
            &NewEmote {
                animated,
                ..make(name)
            },
        )
        .await
        .unwrap();
        emotes::set_state(&mut c, e.id, EmoteState::Approved)
            .await
            .unwrap();
        approved.push(e.id);
    }
    // KEKW is found by its tag.
    emotes::set_tags(&mut c, approved[2], &["poggers".to_owned()])
        .await
        .unwrap();
    // Never found: pending, private, deleted; NSFW only when asked.
    emotes::create(&mut c, &make("PogPending")).await.unwrap();
    let private = emotes::create(&mut c, &make("PogPrivate")).await.unwrap();
    emotes::set_state(&mut c, private.id, EmoteState::Approved)
        .await
        .unwrap();
    emotes::update(
        &mut c,
        private.id,
        &EmotePatch {
            private: Some(true),
            ..EmotePatch::default()
        },
    )
    .await
    .unwrap();
    let deleted = emotes::create(&mut c, &make("PogGone")).await.unwrap();
    emotes::set_state(&mut c, deleted.id, EmoteState::Approved)
        .await
        .unwrap();
    emotes::delete(&mut c, deleted.id).await.unwrap();
    let nsfw = emotes::create(
        &mut c,
        &NewEmote {
            nsfw: true,
            ..make("PogLewd")
        },
    )
    .await
    .unwrap();
    emotes::set_state(&mut c, nsfw.id, EmoteState::Approved)
        .await
        .unwrap();
    // The other account's emote, for the owner filter.
    let theirs = emotes::create(
        &mut c,
        &NewEmote {
            owner_account_id: other,
            ..make("PogTheirs")
        },
    )
    .await
    .unwrap();
    emotes::set_state(&mut c, theirs.id, EmoteState::Approved)
        .await
        .unwrap();

    // PogU is in both channels' active sets; PogChamp in one.
    for (who, platform_id, members) in [
        (owner, "1", vec![approved[1], approved[0]]),
        (other, "2", vec![approved[1]]),
    ] {
        let set = sets::create(&mut c, who, "Main", SetKind::Channel)
            .await
            .unwrap();
        for emote in members {
            sets::add_member(
                &mut c,
                set.id,
                sets::NewMember {
                    emote_id: emote,
                    name: None,
                    zero_width: None,
                    added_by: Some(who),
                },
            )
            .await
            .unwrap();
        }
        let ch = channels::find(&mut c, Platform::Twitch, platform_id)
            .await
            .unwrap()
            .unwrap();
        channels::set_active_set(&mut c, ch.id, Some(set.id))
            .await
            .unwrap();
    }
    assert!(search::refresh_popularity(&mut c).await.unwrap() >= 2);
    // The worker's minutely round: the stored channel counts.
    assert!(channel_counts::refresh_queued(&mut c).await.unwrap() >= 2);
    Made { owner, other }
}

async fn find(app: &axum::Router, args: &str) -> Value {
    let query = format!(
        "{{ searchEmotes({args}) {{ total page perPage items {{ name animated flags {{ nsfw }} }} }} }}"
    );
    let request = Request::post("/v1/gql")
        .header("content-type", "application/json")
        .body(Body::from(json!({ "query": query }).to_string()))
        .unwrap();
    let response = app.clone().oneshot(request).await.unwrap();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let v: Value = serde_json::from_slice(&bytes).unwrap();
    assert!(v["errors"].is_null(), "{v}");
    v["data"]["searchEmotes"].clone()
}

fn names(result: &Value) -> Vec<String> {
    result["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| i["name"].as_str().unwrap().to_owned())
        .collect()
}

#[tokio::test]
async fn search_finds_approved_listed_emotes_in_rank_order() {
    let Some(db) = TestDb::new().await else {
        return;
    };
    let made = seed(&db).await;
    let auth = Arc::new(Auth::new(AuthConfig::from_source(|_| None).unwrap()).unwrap());
    let app = cyex_tv_api::app(AppState::new(db.pool.clone(), auth));

    // The exact name first, then prefixes, then the tag-only match. Never pending, private,
    // deleted or (unasked) NSFW ones.
    let r = find(&app, r#"query: "pog""#).await;
    let got = names(&r);
    assert_eq!(got.first().map(String::as_str), Some("pog"), "{got:?}");
    assert_eq!(got.last().map(String::as_str), Some("KEKW"), "{got:?}");
    for hidden in ["PogPending", "PogPrivate", "PogGone", "PogLewd"] {
        assert!(!got.iter().any(|n| n == hidden), "{hidden} in {got:?}");
    }
    assert_eq!(r["total"], got.len());
    assert!(got.contains(&"PogTheirs".to_owned()));

    // NSFW when asked.
    let r = find(&app, r#"query: "pog", nsfw: true"#).await;
    assert!(names(&r).contains(&"PogLewd".to_owned()));

    // Filters.
    let r = find(&app, r#"query: "pog", animated: true"#).await;
    assert_eq!(names(&r), ["PogU"]);
    let owner = ids::to_text(made.other);
    let r = find(&app, &format!(r#"query: "pog", owner: "{owner}""#)).await;
    assert_eq!(names(&r), ["PogTheirs"]);
    let r = find(&app, r#"query: "pog", owner: "not-an-id""#).await;
    assert_eq!(r["total"], 0);
    let _ = made.owner;

    // No query: the most used first.
    let r = find(&app, r#"query: """#).await;
    assert_eq!(names(&r)[..2], ["PogU".to_owned(), "PogChamp".to_owned()]);

    // Paging.
    let r = find(&app, r#"query: "pog", perPage: 2, page: 2"#).await;
    assert_eq!(
        (r["page"].as_u64(), r["perPage"].as_u64()),
        (Some(2), Some(2))
    );
    assert_eq!(names(&r).len(), 2);
    let r = find(&app, r#"query: "pog", perPage: 500"#).await;
    assert_eq!(r["perPage"], 100);

    // LIKE wildcards are literal.
    let r = find(&app, r#"query: "%""#).await;
    assert_eq!(r["total"], 0);
    db.close().await;
}

#[tokio::test]
async fn popularity_counts_channels_showing_the_emote() {
    let Some(db) = TestDb::new().await else {
        return;
    };
    seed(&db).await;
    let mut c = db.conn().await;
    let rows: Vec<(String, i32)> =
        sqlx::query_as("SELECT name, popularity FROM emotes WHERE popularity > 0 ORDER BY name")
            .fetch_all(&mut *c)
            .await
            .unwrap();
    assert_eq!(rows, [("PogChamp".to_owned(), 1), ("PogU".to_owned(), 2)]);
    // Nothing changes on a second run.
    assert_eq!(search::refresh_popularity(&mut c).await.unwrap(), 0);
    drop(c);
    db.close().await;
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

fn app_for(db: &TestDb) -> axum::Router {
    let auth = Arc::new(Auth::new(AuthConfig::from_source(|_| None).unwrap()).unwrap());
    cyex_tv_api::app(AppState::new(db.pool.clone(), auth))
}

#[tokio::test]
async fn search_sorts_by_relevance_top_and_new() {
    let Some(db) = TestDb::new().await else {
        return;
    };
    seed(&db).await;
    let app = app_for(&db);

    // An empty query lists every approved, listed emote, whatever the sort.
    let r = find(&app, r#"query: """#).await;
    assert_eq!(r["total"], 5);
    let r = find(&app, r#"query: "", sort: RELEVANCE"#).await;
    assert_eq!(names(&r)[..2], ["PogU".to_owned(), "PogChamp".to_owned()]);

    // Most used first, then by name.
    let r = find(&app, r#"query: "", sort: TOP"#).await;
    assert_eq!(names(&r), ["PogU", "PogChamp", "KEKW", "pog", "PogTheirs"]);

    // Newest first.
    let r = find(&app, r#"query: "", sort: NEW"#).await;
    assert_eq!(names(&r), ["PogTheirs", "pog", "KEKW", "PogU", "PogChamp"]);

    // A text query still filters.
    let r = find(&app, r#"query: "kekw", sort: NEW"#).await;
    assert_eq!(names(&r), ["KEKW"]);
    db.close().await;
}

#[tokio::test]
async fn channel_count_is_on_every_emote_of_a_list() {
    let Some(db) = TestDb::new().await else {
        return;
    };
    seed(&db).await;
    let app = app_for(&db);
    let data = gql(
        &app,
        r#"{ searchEmotes(query: "") { items { name channelCount } } }"#,
    )
    .await;
    let counts: Vec<(String, u64)> = data["searchEmotes"]["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| {
            (
                i["name"].as_str().unwrap().to_owned(),
                i["channelCount"].as_u64().unwrap(),
            )
        })
        .collect();
    let count = |name: &str| counts.iter().find(|(n, _)| n == name).unwrap().1;
    assert_eq!(count("PogU"), 2);
    assert_eq!(count("PogChamp"), 1);
    assert_eq!(count("KEKW"), 0);
    db.close().await;
}

#[tokio::test]
async fn account_is_public_and_null_when_unknown_or_blocked() {
    let Some(db) = TestDb::new().await else {
        return;
    };
    let made = seed(&db).await;
    let app = app_for(&db);
    let id = ids::to_text(made.owner);
    let data = gql(
        &app,
        &format!(r#"{{ account(id: "{id}") {{ id displayName avatar }} }}"#),
    )
    .await;
    assert_eq!(data["account"]["id"], id);
    assert_eq!(data["account"]["displayName"], "user1");

    for unknown in ["not-an-id", &ids::to_text(Uuid::from_u128(0x1234))] {
        let data = gql(&app, &format!(r#"{{ account(id: "{unknown}") {{ id }} }}"#)).await;
        assert!(data["account"].is_null(), "{unknown}");
    }

    let mut c = db.conn().await;
    accounts::set_blocked(&mut c, made.owner, true)
        .await
        .unwrap();
    drop(c);
    let data = gql(&app, &format!(r#"{{ account(id: "{id}") {{ id }} }}"#)).await;
    assert!(data["account"].is_null());
    db.close().await;
}

#[tokio::test]
async fn emote_channels_lists_public_channels_and_clamps_the_limit() {
    let Some(db) = TestDb::new().await else {
        return;
    };
    let made = seed(&db).await;
    let app = app_for(&db);
    let mut c = db.conn().await;
    let (pogu,): (Uuid,) = sqlx::query_as("SELECT id FROM emotes WHERE name = 'PogU'")
        .fetch_one(&mut *c)
        .await
        .unwrap();
    let (pending,): (Uuid,) = sqlx::query_as("SELECT id FROM emotes WHERE name = 'PogPending'")
        .fetch_one(&mut *c)
        .await
        .unwrap();
    drop(c);
    let pogu = ids::to_text(pogu);

    let logins = |data: &Value| -> Vec<String> {
        data["emoteChannels"]
            .as_array()
            .unwrap()
            .iter()
            .map(|c| c["login"].as_str().unwrap().to_owned())
            .collect()
    };
    let q = |args: &str| {
        format!(r#"{{ emoteChannels({args}) {{ login platform account {{ displayName }} }} }}"#)
    };

    // Most recently added first.
    let data = gql(&app, &q(&format!(r#"id: "{pogu}""#))).await;
    assert_eq!(logins(&data), ["user2", "user1"]);

    // The limit is clamped to 1..=50.
    let data = gql(&app, &q(&format!(r#"id: "{pogu}", limit: 1"#))).await;
    assert_eq!(logins(&data), ["user2"]);
    let data = gql(&app, &q(&format!(r#"id: "{pogu}", limit: 0"#))).await;
    assert_eq!(logins(&data).len(), 1);
    let data = gql(&app, &q(&format!(r#"id: "{pogu}", limit: 500"#))).await;
    assert_eq!(logins(&data).len(), 2);

    // `offset` pages through the same order; past the end, or past the cap, is empty.
    let data = gql(&app, &q(&format!(r#"id: "{pogu}", limit: 1, offset: 1"#))).await;
    assert_eq!(logins(&data), ["user1"]);
    let data = gql(&app, &q(&format!(r#"id: "{pogu}", offset: 0"#))).await;
    assert_eq!(logins(&data), ["user2", "user1"]);
    let data = gql(&app, &q(&format!(r#"id: "{pogu}", offset: 2"#))).await;
    assert!(logins(&data).is_empty());
    let data = gql(&app, &q(&format!(r#"id: "{pogu}", offset: 10001"#))).await;
    assert!(logins(&data).is_empty());

    // Unknown, malformed and unlisted emotes have no channels.
    let data = gql(&app, &q(r#"id: "not-an-id""#)).await;
    assert!(logins(&data).is_empty());
    let data = gql(&app, &q(&format!(r#"id: "{}""#, ids::to_text(pending)))).await;
    assert!(logins(&data).is_empty());

    // A blocked account's channel drops out of the list and the count.
    let mut c = db.conn().await;
    accounts::set_blocked(&mut c, made.other, true)
        .await
        .unwrap();
    // The stored count follows on the worker's next round.
    channel_counts::refresh_queued(&mut c).await.unwrap();
    drop(c);
    let data = gql(&app, &q(&format!(r#"id: "{pogu}""#))).await;
    assert_eq!(logins(&data), ["user1"]);
    let data = gql(
        &app,
        &format!(r#"{{ emote(id: "{pogu}") {{ channelCount }} }}"#),
    )
    .await;
    assert_eq!(data["emote"]["channelCount"], 1);
    db.close().await;
}
