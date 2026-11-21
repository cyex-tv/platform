//! Personal lists: "Hide for me" (`hideEmote`, `hideCreator`, their undo, `hiddenEmotes`,
//! `hiddenCreators`) and favourites (`favouriteEmote`, `unfavouriteEmote`, `myFavourites`,
//! `favouriteEmoteIds`): who may, what may be listed, the caps, the rate, and that nobody else
//! can read or change someone's list. Through the real router against a real Postgres; runs when
//! `CYEX_TV_TEST_DATABASE_URL` is set (CI sets it) and skips otherwise.

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
use cyex_tv_db::emotes::{self, NewEmote};
use cyex_tv_db::ids;
use cyex_tv_db::sessions::{self, SessionKind};
use cyex_tv_shared::model::{EmoteState, Platform};
use cyex_tv_shared::ratelimit::Rate;
use http_body_util::BodyExt;
use serde_json::{Value, json};
use tower::ServiceExt;
use uuid::Uuid;

const HIDE: &str = "mutation($id: String!) { hideEmote(id: $id) }";
const UNHIDE: &str = "mutation($id: String!) { unhideEmote(id: $id) }";
const HIDE_CREATOR: &str = "mutation($id: String!) { hideCreator(accountId: $id) }";
const UNHIDE_CREATOR: &str = "mutation($id: String!) { unhideCreator(accountId: $id) }";
const FAV: &str = "mutation($id: String!) { favouriteEmote(id: $id) }";
const UNFAV: &str = "mutation($id: String!) { unfavouriteEmote(id: $id) }";
const HIDDEN: &str = "{
    hiddenEmotes { emoteId hiddenAt emote { id name } }
    hiddenCreators { accountId hiddenAt account { id displayName } }
}";
const FAVS: &str = "query($q: String, $page: Int, $perPage: Int) {
    myFavourites(query: $q, page: $page, perPage: $perPage) { total items { id name } }
    favouriteEmoteIds
}";

struct World {
    db: TestDb,
    app: Router,
}

fn code(response: &Value) -> &str {
    response["errors"][0]["extensions"]["code"]
        .as_str()
        .unwrap_or_default()
}

fn ok(r: &Value) -> &Value {
    assert!(r["errors"].is_null(), "{r}");
    &r["data"]
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

    async fn account(&self, id: &str, login: &str) -> Uuid {
        let mut conn = self.db.conn().await;
        accounts::sign_in(
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

    async fn emote(&self, owner: Uuid, name: &str, state: EmoteState) -> String {
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
        if state != EmoteState::Pending {
            emotes::set_state(&mut conn, e.id, state).await.unwrap();
        }
        self.db.public(e.id).await
    }

    async fn sql(&self, sql: &str, account: Uuid) {
        sqlx::query(sql)
            .bind(account)
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
}

fn names(list: &Value, key: &str) -> Vec<String> {
    list.as_array()
        .unwrap_or_else(|| panic!("{list}"))
        .iter()
        .map(|e| e[key]["name"].as_str().unwrap_or("-").to_owned())
        .collect()
}

#[tokio::test]
async fn hiding_and_unhiding_emotes_and_creators() {
    let Some(w) = World::new(Rates::default()).await else {
        return;
    };
    let me = w.account("1", "viewer").await;
    let maker = w.account("2", "maker").await;
    let t = w.token(me).await;
    let a = w.emote(maker, "HideA", EmoteState::Approved).await;
    let b = w.emote(maker, "HideB", EmoteState::Approved).await;

    // Newly hidden is true, again is false; the list is the latest first.
    assert_eq!(
        ok(&w.gql(Some(&t), HIDE, json!({ "id": a })).await)["hideEmote"],
        true
    );
    assert_eq!(
        ok(&w.gql(Some(&t), HIDE, json!({ "id": a })).await)["hideEmote"],
        false
    );
    ok(&w.gql(Some(&t), HIDE, json!({ "id": b })).await);
    let r = w.gql(Some(&t), HIDDEN, json!({})).await;
    assert_eq!(names(&ok(&r)["hiddenEmotes"], "emote"), ["HideB", "HideA"]);
    assert!(ok(&r)["hiddenEmotes"][0]["hiddenAt"].is_string());

    // Unhide: true once, then false; a malformed ID is just false.
    assert_eq!(
        ok(&w.gql(Some(&t), UNHIDE, json!({ "id": b })).await)["unhideEmote"],
        true
    );
    assert_eq!(
        ok(&w.gql(Some(&t), UNHIDE, json!({ "id": b })).await)["unhideEmote"],
        false
    );
    assert_eq!(
        ok(&w.gql(Some(&t), UNHIDE, json!({ "id": "nope" })).await)["unhideEmote"],
        false
    );

    // A creator: by account, never yourself, never an unknown or deleted account.
    let maker_id = w.db.public(maker).await;
    let r = w
        .gql(Some(&t), HIDE_CREATOR, json!({ "id": maker_id }))
        .await;
    assert_eq!(ok(&r)["hideCreator"], true);
    let r = w
        .gql(Some(&t), HIDE_CREATOR, json!({ "id": ids::to_text(me) }))
        .await;
    assert_eq!(code(&r), "own_account", "{r}");
    let r = w
        .gql(
            Some(&t),
            HIDE_CREATOR,
            json!({ "id": ids::to_text(ids::new_id()) }),
        )
        .await;
    assert_eq!(code(&r), "not_found", "{r}");
    let gone = w.account("3", "gone").await;
    w.sql("UPDATE accounts SET deleted_at = now() WHERE id = $1", gone)
        .await;
    let r = w
        .gql(Some(&t), HIDE_CREATOR, json!({ "id": ids::to_text(gone) }))
        .await;
    assert_eq!(code(&r), "not_found", "{r}");
    let r = w.gql(Some(&t), HIDDEN, json!({})).await;
    let creators = &ok(&r)["hiddenCreators"];
    assert_eq!(creators.as_array().unwrap().len(), 1, "{r}");
    assert_eq!(creators[0]["accountId"], maker_id);
    assert_eq!(creators[0]["account"]["displayName"], "maker");
    let r = w
        .gql(Some(&t), UNHIDE_CREATOR, json!({ "id": maker_id }))
        .await;
    assert_eq!(ok(&r)["unhideCreator"], true);

    // A deleted emote stays listed (so it can be unhidden) without its view.
    w.sql(
        "UPDATE emotes SET deleted_at = now() WHERE owner_account_id = $1",
        maker,
    )
    .await;
    let r = w.gql(Some(&t), HIDDEN, json!({})).await;
    assert_eq!(ok(&r)["hiddenEmotes"][0]["emoteId"], a);
    assert!(ok(&r)["hiddenEmotes"][0]["emote"].is_null(), "{r}");
    w.db.close().await;
}

#[tokio::test]
async fn only_emotes_you_can_see_can_be_listed() {
    let Some(w) = World::new(Rates::default()).await else {
        return;
    };
    let me = w.account("1", "viewer").await;
    let maker = w.account("2", "maker").await;
    let t = w.token(me).await;
    let pending = w.emote(maker, "NotYet", EmoteState::Pending).await;
    let rejected = w.emote(maker, "Nope", EmoteState::Rejected).await;
    let mine = w.emote(me, "MinePending", EmoteState::Pending).await;
    for q in [HIDE, FAV] {
        for id in [
            pending.as_str(),
            rejected.as_str(),
            "nope",
            &ids::to_text(ids::new_id()),
        ] {
            let r = w.gql(Some(&t), q, json!({ "id": id })).await;
            assert_eq!(code(&r), "not_found", "{q} {id}: {r}");
        }
        // Your own emote, even pending, is yours to see.
        assert!(w.gql(Some(&t), q, json!({ "id": mine })).await["errors"].is_null());
    }
    w.db.close().await;
}

#[tokio::test]
async fn favourites_page_filter_and_ids() {
    let Some(w) = World::new(Rates::default()).await else {
        return;
    };
    let me = w.account("1", "viewer").await;
    let maker = w.account("2", "maker").await;
    let t = w.token(me).await;
    let mut all = Vec::new();
    for name in ["FavOne", "FavTwo", "OtherThree"] {
        let id = w.emote(maker, name, EmoteState::Approved).await;
        assert_eq!(
            ok(&w.gql(Some(&t), FAV, json!({ "id": id })).await)["favouriteEmote"],
            true
        );
        all.push(id);
    }
    assert_eq!(
        ok(&w.gql(Some(&t), FAV, json!({ "id": all[0] })).await)["favouriteEmote"],
        false
    );

    let r = w.gql(Some(&t), FAVS, json!({})).await;
    let d = ok(&r);
    assert_eq!(d["myFavourites"]["total"], 3);
    let order: Vec<&str> = d["myFavourites"]["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["name"].as_str().unwrap())
        .collect();
    assert_eq!(order, ["OtherThree", "FavTwo", "FavOne"]);
    let ids_out: Vec<&str> = d["favouriteEmoteIds"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    assert_eq!(ids_out, [all[2].as_str(), all[1].as_str(), all[0].as_str()]);

    // The name filter (any case) and paging.
    let r = w.gql(Some(&t), FAVS, json!({ "q": "fav" })).await;
    assert_eq!(ok(&r)["myFavourites"]["total"], 2);
    let r = w
        .gql(Some(&t), FAVS, json!({ "page": 2, "perPage": 2 }))
        .await;
    assert_eq!(ok(&r)["myFavourites"]["items"][0]["name"], "FavOne");
    // `%` means itself.
    let r = w.gql(Some(&t), FAVS, json!({ "q": "%" })).await;
    assert_eq!(ok(&r)["myFavourites"]["total"], 0);

    // Unfavourite.
    let r = w.gql(Some(&t), UNFAV, json!({ "id": all[1] })).await;
    assert_eq!(ok(&r)["unfavouriteEmote"], true);
    let r = w.gql(Some(&t), UNFAV, json!({ "id": all[1] })).await;
    assert_eq!(ok(&r)["unfavouriteEmote"], false);
    let r = w.gql(Some(&t), FAVS, json!({})).await;
    assert_eq!(ok(&r)["myFavourites"]["total"], 2);
    w.db.close().await;
}

#[tokio::test]
async fn favourites_total_matches_visible_items() {
    let Some(w) = World::new(Rates::default()).await else {
        return;
    };
    let me = w.account("1", "viewer").await;
    let maker = w.account("2", "maker").await;
    let t = w.token(me).await;
    let mut ids_made = Vec::new();
    for i in 0..4 {
        let id = w
            .emote(maker, &format!("Fav{i}"), EmoteState::Approved)
            .await;
        w.gql(Some(&t), FAV, json!({ "id": id })).await;
        ids_made.push(id);
    }
    // One is taken down after it was favourited: it is neither listed nor counted.
    sqlx::query("UPDATE emotes SET state = 'taken_down' WHERE id = $1")
        .bind(w.db.row(&ids_made[1]).await)
        .execute(&w.db.pool)
        .await
        .unwrap();
    let r = w
        .gql(Some(&t), FAVS, json!({ "page": 1, "perPage": 2 }))
        .await;
    let d = ok(&r);
    assert_eq!(d["myFavourites"]["total"], 3);
    assert_eq!(d["myFavourites"]["items"].as_array().unwrap().len(), 2);
    let r = w
        .gql(Some(&t), FAVS, json!({ "page": 2, "perPage": 2 }))
        .await;
    let d = ok(&r);
    assert_eq!(d["myFavourites"]["total"], 3);
    assert_eq!(d["myFavourites"]["items"].as_array().unwrap().len(), 1);
    w.db.close().await;
}

#[tokio::test]
async fn lists_are_private_to_their_account() {
    let Some(w) = World::new(Rates::default()).await else {
        return;
    };
    let alice = w.account("1", "alice").await;
    let bob = w.account("2", "bob").await;
    let maker = w.account("3", "maker").await;
    let ta = w.token(alice).await;
    let tb = w.token(bob).await;
    let e = w.emote(maker, "Secret", EmoteState::Approved).await;
    ok(&w.gql(Some(&ta), HIDE, json!({ "id": e })).await);
    ok(&w.gql(Some(&ta), FAV, json!({ "id": e })).await);
    ok(&w
        .gql(
            Some(&ta),
            HIDE_CREATOR,
            json!({ "id": ids::to_text(maker) }),
        )
        .await);

    // Bob sees his own (empty) lists, never Alice's; there is no account argument to ask with.
    let r = w.gql(Some(&tb), HIDDEN, json!({})).await;
    assert!(ok(&r)["hiddenEmotes"].as_array().unwrap().is_empty(), "{r}");
    assert!(
        ok(&r)["hiddenCreators"].as_array().unwrap().is_empty(),
        "{r}"
    );
    let r = w.gql(Some(&tb), FAVS, json!({})).await;
    assert_eq!(ok(&r)["myFavourites"]["total"], 0);
    let r = w
        .gql(
            Some(&tb),
            "{ hiddenEmotes(accountId: \"x\") { emoteId } }",
            json!({}),
        )
        .await;
    assert!(r["errors"].is_array(), "{r}");
    // Bob's undo touches only his list: Alice's entry stays.
    let r = w.gql(Some(&tb), UNHIDE, json!({ "id": e })).await;
    assert_eq!(ok(&r)["unhideEmote"], false);
    let r = w.gql(Some(&tb), UNFAV, json!({ "id": e })).await;
    assert_eq!(ok(&r)["unfavouriteEmote"], false);
    let r = w.gql(Some(&ta), HIDDEN, json!({})).await;
    assert_eq!(ok(&r)["hiddenEmotes"].as_array().unwrap().len(), 1);

    // Signed out: every field answers signed_out.
    for (q, v) in [
        (HIDDEN, json!({})),
        (FAVS, json!({})),
        (HIDE, json!({ "id": e })),
        (UNHIDE, json!({ "id": e })),
        (HIDE_CREATOR, json!({ "id": ids::to_text(maker) })),
        (UNHIDE_CREATOR, json!({ "id": ids::to_text(maker) })),
        (FAV, json!({ "id": e })),
        (UNFAV, json!({ "id": e })),
    ] {
        let r = w.gql(None, q, v).await;
        assert_eq!(code(&r), "signed_out", "{q}: {r}");
    }

    // The public reads are the same for everyone: Alice's hide doesn't change search.
    let r = w
        .gql(
            Some(&ta),
            "{ searchEmotes(query: \"Secret\") { items { name } } }",
            json!({}),
        )
        .await;
    assert_eq!(ok(&r)["searchEmotes"]["items"][0]["name"], "Secret");

    // Erasing an account takes its lists with it.
    let mut conn = w.db.conn().await;
    cyex_tv_db::privacy::erase(&mut conn, alice).await.unwrap();
    drop(conn);
    for table in ["hidden_emotes", "hidden_creators", "favourite_emotes"] {
        let n: i64 = sqlx::query_scalar(&format!(
            "SELECT count(*) FROM {table} WHERE account_id = $1"
        ))
        .bind(alice)
        .fetch_one(&w.db.pool)
        .await
        .unwrap();
        assert_eq!(n, 0, "{table}");
    }
    w.db.close().await;
}

#[tokio::test]
async fn lists_stop_at_their_caps() {
    let Some(w) = World::new(Rates::default()).await else {
        return;
    };
    let me = w.account("1", "viewer").await;
    let maker = w.account("2", "maker").await;
    let t = w.token(me).await;
    // 1000 emotes, hidden and favourited straight in the database.
    w.sql(
        "INSERT INTO emotes (id, owner_account_id, name, state)
         SELECT gen_random_uuid(), $1, 'Bulk' || n, 'approved' FROM generate_series(1, 1000) n",
        maker,
    )
    .await;
    for table in ["hidden_emotes", "favourite_emotes"] {
        sqlx::query(&format!(
            "INSERT INTO {table} (account_id, emote_id)
             SELECT $1, id FROM emotes WHERE owner_account_id = $2"
        ))
        .bind(me)
        .bind(maker)
        .execute(&w.db.pool)
        .await
        .unwrap();
    }
    let extra = w.emote(maker, "OneTooMany", EmoteState::Approved).await;
    for q in [HIDE, FAV] {
        let r = w.gql(Some(&t), q, json!({ "id": extra })).await;
        assert_eq!(code(&r), "limit_reached", "{q}: {r}");
    }
    // One already listed still answers false at the cap, without an error.
    let listed: Uuid = sqlx::query_scalar("SELECT emote_id FROM hidden_emotes LIMIT 1")
        .fetch_one(&w.db.pool)
        .await
        .unwrap();
    let r = w
        .gql(Some(&t), HIDE, json!({ "id": ids::to_text(listed) }))
        .await;
    assert_eq!(ok(&r)["hideEmote"], false);
    // Unhiding one makes room.
    ok(&w
        .gql(Some(&t), UNHIDE, json!({ "id": ids::to_text(listed) }))
        .await);
    assert_eq!(
        ok(&w.gql(Some(&t), HIDE, json!({ "id": extra })).await)["hideEmote"],
        true
    );
    let r = w.gql(Some(&t), HIDDEN, json!({})).await;
    assert_eq!(ok(&r)["hiddenEmotes"].as_array().unwrap().len(), 1000);

    // 200 creators.
    w.sql(
        "WITH made AS (INSERT INTO accounts (id) SELECT gen_random_uuid() FROM generate_series(1, 200) RETURNING id)
         INSERT INTO hidden_creators (account_id, creator_id) SELECT $1, id FROM made",
        me,
    )
    .await;
    let r = w
        .gql(Some(&t), HIDE_CREATOR, json!({ "id": ids::to_text(maker) }))
        .await;
    assert_eq!(code(&r), "limit_reached", "{r}");
    w.db.close().await;
}

#[tokio::test]
async fn list_changes_are_rate_limited_per_account() {
    let Some(w) = World::new(Rates {
        personal_lists: Rate::per_minute(2),
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
    let e = w.emote(b, "Rated", EmoteState::Approved).await;
    ok(&w.gql(Some(&ta), HIDE, json!({ "id": e })).await);
    ok(&w.gql(Some(&ta), UNHIDE, json!({ "id": e })).await);
    let r = w.gql(Some(&ta), FAV, json!({ "id": e })).await;
    assert_eq!(code(&r), "rate_limited", "{r}");
    ok(&w.gql(Some(&tb), FAV, json!({ "id": e })).await);
    w.db.close().await;
}
