//! `quickSearch` (the header search: emote tiers, popularity, word and initials matches, typos,
//! visibility, users) and `myEditableChannels` (roles, dedup, order). Through the real router
//! against a real Postgres; runs when `CYEX_TV_TEST_DATABASE_URL` is set (CI sets it) and skips
//! otherwise.

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
use cyex_tv_db::emotes::{self, EmotePatch, NewEmote};
use cyex_tv_db::sessions::{self, SessionKind};
use cyex_tv_db::{channels, editors, sets};
use cyex_tv_shared::model::{EmoteState, Platform, SetKind};
use cyex_tv_shared::ratelimit::Rate;
use http_body_util::BodyExt;
use serde_json::{Value, json};
use tower::ServiceExt;
use uuid::Uuid;

const QUICK: &str = "query($q: String!) {
    quickSearch(query: $q) {
        emotes { name }
        users { account { id displayName } identities { platform login } }
    }
}";

const EDITABLE: &str = "{ myEditableChannels { role channel { platform login account { id } } } }";

struct World {
    db: TestDb,
    app: Router,
}

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

    /// An account with a Twitch channel of the same login.
    async fn account(&self, id: &str, login: &str) -> Uuid {
        let mut conn = self.db.conn().await;
        let account = accounts::sign_in(
            &mut conn,
            Profile {
                platform: Platform::Twitch,
                platform_user_id: id,
                login,
                display_name: login,
                avatar: None,
            },
        )
        .await
        .unwrap()
        .account
        .id;
        channels::upsert(&mut conn, account, Platform::Twitch, id, login)
            .await
            .unwrap();
        account
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

    /// An approved, listed emote.
    async fn emote(&self, owner: Uuid, name: &str) -> Uuid {
        let mut conn = self.db.conn().await;
        let e = emotes::create(
            &mut conn,
            &NewEmote {
                owner_account_id: owner,
                name,
                ..NewEmote::default()
            },
        )
        .await
        .unwrap();
        emotes::set_state(&mut conn, e.id, EmoteState::Approved)
            .await
            .unwrap();
        e.id
    }

    async fn popularity(&self, emote: Uuid, n: i32) {
        sqlx::query("UPDATE emotes SET popularity = $2 WHERE id = $1")
            .bind(emote)
            .bind(n)
            .execute(&self.db.pool)
            .await
            .unwrap();
    }

    async fn gql(&self, token: Option<&str>, query: &str, variables: Value) -> Value {
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
                json!({ "query": query, "variables": variables }).to_string(),
            ))
            .unwrap();
        let response = self.app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        serde_json::from_slice(&bytes).unwrap()
    }

    async fn quick(&self, q: &str) -> Value {
        let r = self.gql(None, QUICK, json!({ "q": q })).await;
        assert!(r["errors"].is_null(), "{r}");
        r["data"]["quickSearch"].clone()
    }

    async fn names(&self, q: &str) -> Vec<String> {
        emote_names(&self.quick(q).await)
    }
}

fn emote_names(result: &Value) -> Vec<String> {
    result["emotes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["name"].as_str().unwrap().to_owned())
        .collect()
}

#[tokio::test]
async fn tiers_rank_exact_then_prefix_then_word_then_substring() {
    let Some(w) = World::new(Rates::default()).await else {
        return;
    };
    let owner = w.account("1", "owner").await;
    // Created worst-first, so only the ranking can put them right.
    for name in ["xpogx", "KEKPog", "PogChamp", "POG", "pog", "Unrelated"] {
        w.emote(owner, name).await;
    }
    assert_eq!(
        w.names("pog").await,
        ["pog", "POG", "PogChamp", "KEKPog", "xpogx"]
    );
    // The query is trimmed and its case doesn't matter, except for the exact tier.
    assert_eq!(w.names("  POG ").await[..2], ["POG", "pog"]);
    w.db.close().await;
}

#[tokio::test]
async fn popularity_breaks_ties_inside_a_tier() {
    let Some(w) = World::new(Rates::default()).await else {
        return;
    };
    let owner = w.account("1", "owner").await;
    let short = w.emote(owner, "PogA").await;
    let longer = w.emote(owner, "PogBb").await;
    let popular = w.emote(owner, "PogCcc").await;
    // Equal use: the shorter name first.
    assert_eq!(w.names("pog").await, ["PogA", "PogBb", "PogCcc"]);
    w.popularity(popular, 5).await;
    w.popularity(longer, 2).await;
    assert_eq!(w.names("pog").await, ["PogCcc", "PogBb", "PogA"]);
    // Use never lifts an emote over a better tier.
    w.popularity(short, 0).await;
    let exact = w.emote(owner, "pog").await;
    assert_eq!(w.names("pog").await[0], "pog");
    w.popularity(exact, 0).await;
    w.db.close().await;
}

#[tokio::test]
async fn words_initials_and_typos_find_camel_case_names() {
    let Some(w) = World::new(Rates::default()).await else {
        return;
    };
    let owner = w.account("1", "owner").await;
    for name in [
        "Appointment",
        "PepePoint",
        "KEKWait",
        "Emote2Fast",
        "snake_case_name",
    ] {
        w.emote(owner, name).await;
    }
    // A word of the name beats a plain substring.
    assert_eq!(w.names("point").await, ["PepePoint", "Appointment"]);
    // Initials.
    assert_eq!(w.names("pp").await[0], "PepePoint");
    // Acronym runs split before the last capital.
    assert_eq!(w.names("wait").await, ["KEKWait"]);
    assert_eq!(w.names("kek").await, ["KEKWait"]);
    // Letters and digits, and `_`, are word boundaries.
    assert_eq!(w.names("fast").await, ["Emote2Fast"]);
    assert_eq!(w.names("2").await, ["Emote2Fast"]);
    assert_eq!(w.names("name").await, ["snake_case_name"]);
    // Typos, from three characters.
    assert_eq!(w.names("PepePiont").await, ["PepePoint"]);
    assert!(w.names("zzz").await.is_empty());
    // `%` and `_` in the query mean themselves.
    assert!(w.names("%").await.is_empty());
    assert_eq!(w.names("_").await, ["snake_case_name"]);
    w.db.close().await;
}

#[tokio::test]
async fn hidden_emotes_never_show_and_the_list_is_capped() {
    let Some(w) = World::new(Rates::default()).await else {
        return;
    };
    let owner = w.account("1", "owner").await;
    {
        let mut c = w.db.conn().await;
        // Pending.
        emotes::create(
            &mut c,
            &NewEmote {
                owner_account_id: owner,
                name: "PepePending",
                ..NewEmote::default()
            },
        )
        .await
        .unwrap();
        // NSFW.
        let lewd = emotes::create(
            &mut c,
            &NewEmote {
                owner_account_id: owner,
                name: "PepeLewd",
                nsfw: true,
                ..NewEmote::default()
            },
        )
        .await
        .unwrap();
        emotes::set_state(&mut c, lewd.id, EmoteState::Approved)
            .await
            .unwrap();
    }
    let private = w.emote(owner, "PepePrivate").await;
    let gone = w.emote(owner, "PepeGone").await;
    {
        let mut c = w.db.conn().await;
        emotes::update(
            &mut c,
            private,
            &EmotePatch {
                private: Some(true),
                ..EmotePatch::default()
            },
        )
        .await
        .unwrap();
        emotes::delete(&mut c, gone).await.unwrap();
    }
    assert!(w.names("pepe").await.is_empty());
    assert!(w.names("").await.is_empty());
    assert!(w.names("   ").await.is_empty());
    // Only the top 5.
    for n in 0..7 {
        w.emote(owner, &format!("Pepe{n}x")).await;
    }
    assert_eq!(w.names("pepe").await.len(), 5);
    w.db.close().await;
}

#[tokio::test]
async fn users_come_with_the_emotes() {
    let Some(w) = World::new(Rates::default()).await else {
        return;
    };
    let fan = w.account("1", "pepefan").await;
    for (i, login) in ["pepeone", "pepetwo", "pepethree", "pepefour"]
        .into_iter()
        .enumerate()
    {
        w.account(&format!("{}", 10 + i), login).await;
    }
    w.emote(fan, "PepeSmile").await;
    let r = w.quick("pepefan").await;
    assert_eq!(r["users"][0]["identities"][0]["login"], "pepefan");
    assert_eq!(r["users"][0]["account"]["id"], w.db.public(fan).await);
    // At most 3, and a leading @ is ignored.
    let r = w.quick("@pepe").await;
    assert_eq!(r["users"].as_array().unwrap().len(), 3, "{r}");
    assert_eq!(emote_names(&r), ["PepeSmile"]);
    // One character finds no users.
    let r = w.quick("p").await;
    assert!(r["users"].as_array().unwrap().is_empty());
    w.db.close().await;
}

#[tokio::test]
async fn quick_search_is_rate_limited_per_person() {
    let Some(w) = World::new(Rates {
        quick_search: Rate::per_minute(2),
        ..Rates::default()
    })
    .await
    else {
        return;
    };
    let a = w.account("1", "alpha").await;
    let b = w.account("2", "bravo").await;
    let ta = w.token(a).await;
    let tb = w.token(b).await;
    for _ in 0..2 {
        let r = w.gql(Some(&ta), QUICK, json!({ "q": "alpha" })).await;
        assert!(r["errors"].is_null(), "{r}");
    }
    let r = w.gql(Some(&ta), QUICK, json!({ "q": "alpha" })).await;
    assert_eq!(code(&r), "rate_limited", "{r}");
    let r = w.gql(Some(&tb), QUICK, json!({ "q": "alpha" })).await;
    assert!(r["errors"].is_null(), "{r}");
    w.db.close().await;
}

fn roles(r: &Value) -> Vec<(String, String, String)> {
    r["data"]["myEditableChannels"]
        .as_array()
        .unwrap_or_else(|| panic!("{r}"))
        .iter()
        .map(|e| {
            (
                e["role"].as_str().unwrap().to_owned(),
                e["channel"]["login"].as_str().unwrap().to_owned(),
                e["channel"]["platform"].as_str().unwrap().to_owned(),
            )
        })
        .collect()
}

#[tokio::test]
async fn editable_channels_are_ranked_by_role_then_login() {
    let Some(w) = World::new(Rates::default()).await else {
        return;
    };
    let me = w.account("1", "zed").await;
    let bravo = w.account("2", "Bravo").await;
    let alpha = w.account("3", "alpha").await;
    let charlie = w.account("4", "charlie").await;
    let delta = w.account("5", "delta").await;
    let nobody = w.account("6", "nobody").await;
    {
        let mut c = w.db.conn().await;
        // A second channel of mine; Kick sorts before Twitch.
        channels::upsert(&mut c, me, Platform::Kick, "9001", "zed")
            .await
            .unwrap();
        let flags = ["emotes".to_owned()];
        for owner in [bravo, alpha, delta] {
            editors::add(&mut c, owner, me, &flags, None).await.unwrap();
        }
        // A grant the other way round gives me nothing.
        editors::add(&mut c, me, nobody, &flags, None)
            .await
            .unwrap();
        // Set editor of charlie's and delta's sets, and of my own.
        for owner in [charlie, delta, me] {
            let set = sets::create(&mut c, owner, "Main", SetKind::Channel)
                .await
                .unwrap();
            sets::add_editor(&mut c, set.id, me, Some(owner))
                .await
                .unwrap();
        }
    }
    let token = w.token(me).await;
    let r = w.gql(Some(&token), EDITABLE, json!({})).await;
    assert!(r["errors"].is_null(), "{r}");
    let got = roles(&r);
    let row = |role: &str, login: &str, platform: &str| {
        (role.to_owned(), login.to_owned(), platform.to_owned())
    };
    assert_eq!(
        got,
        [
            row("OWNER", "zed", "KICK"),
            row("OWNER", "zed", "TWITCH"),
            row("ACCOUNT_EDITOR", "alpha", "TWITCH"),
            row("ACCOUNT_EDITOR", "Bravo", "TWITCH"),
            // Delta is both; the account editor role wins.
            row("ACCOUNT_EDITOR", "delta", "TWITCH"),
            row("SET_EDITOR", "charlie", "TWITCH"),
        ],
        "{r}"
    );
    assert_eq!(
        r["data"]["myEditableChannels"][2]["channel"]["account"]["id"],
        w.db.public(alpha).await
    );

    // The editor I added sees their own channel, then mine as an account editor.
    let token = w.token(nobody).await;
    let r = w.gql(Some(&token), EDITABLE, json!({})).await;
    assert_eq!(
        roles(&r),
        [
            row("OWNER", "nobody", "TWITCH"),
            row("ACCOUNT_EDITOR", "zed", "KICK"),
            row("ACCOUNT_EDITOR", "zed", "TWITCH"),
        ],
        "{r}"
    );
    w.db.close().await;
}

#[tokio::test]
async fn editable_channels_need_a_sign_in() {
    let Some(w) = World::new(Rates::default()).await else {
        return;
    };
    let r = w.gql(None, EDITABLE, json!({})).await;
    assert_eq!(code(&r), "signed_out", "{r}");
    w.db.close().await;
}
