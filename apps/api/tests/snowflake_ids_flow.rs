//! Snowflake public IDs (ADR 0027) through the real router against a real Postgres: answers show
//! accounts, emotes and sets by Snowflake everywhere, and every input takes the Snowflake or the
//! old ULID. Runs when `CYEX_TV_TEST_DATABASE_URL` is set (CI sets it) and skips otherwise.

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
use cyex_tv_db::{channels, ids, public_ids};
use cyex_tv_shared::model::{EmoteState, Platform};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use tower::ServiceExt;
use uuid::Uuid;

/// A row by both of its names.
#[derive(Clone)]
struct Named {
    id: Uuid,
    ulid: String,
    public: String,
}

struct User {
    named: Named,
    token: String,
}

struct World {
    db: TestDb,
    app: Router,
}

fn ok(r: &Value) {
    assert!(r["errors"].is_null(), "{r}");
}

fn code(r: &Value) -> &str {
    r["errors"][0]["extensions"]["code"]
        .as_str()
        .unwrap_or_default()
}

fn digits(v: &Value) -> bool {
    v.as_str().is_some_and(|s| public_ids::parse(s).is_some())
}

impl World {
    async fn new() -> Option<Self> {
        let db = TestDb::new().await?;
        let auth = Arc::new(Auth::new(AuthConfig::from_source(|_| None).unwrap()).unwrap());
        let state = AppState::new(db.pool.clone(), auth);
        Some(Self {
            app: cyex_tv_api::app(state),
            db,
        })
    }

    async fn named(&self, id: Uuid) -> Named {
        let mut conn = self.db.conn().await;
        let public = public_ids::public_text(&mut conn, id)
            .await
            .unwrap()
            .expect("an account, emote or set");
        Named {
            id,
            ulid: ids::to_text(id),
            public,
        }
    }

    async fn named_text(&self, text: &str) -> Named {
        let mut conn = self.db.conn().await;
        let ulid = public_ids::canonical(&mut conn, text).await.unwrap();
        drop(conn);
        self.named(ids::from_text(&ulid).expect("known")).await
    }

    async fn user(&self, twitch_id: &str) -> User {
        let mut conn = self.db.conn().await;
        let login = format!("user{twitch_id}");
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
        drop(conn);
        User {
            named: self.named(signed.account.id).await,
            token,
        }
    }

    async fn emote(&self, owner: &User, name: &str) -> Named {
        let mut conn = self.db.conn().await;
        let emote = emotes::create(
            &mut conn,
            &NewEmote {
                owner_account_id: owner.named.id,
                name,
                ..NewEmote::default()
            },
        )
        .await
        .unwrap();
        emotes::set_state(&mut conn, emote.id, EmoteState::Approved)
            .await
            .unwrap();
        drop(conn);
        self.named(emote.id).await
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
        assert_eq!(response.status(), StatusCode::OK);
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        serde_json::from_slice(&bytes).unwrap()
    }

    async fn rest(&self, uri: &str) -> (StatusCode, Value) {
        let response = self
            .app
            .clone()
            .oneshot(Request::get(uri).body(Body::empty()).unwrap())
            .await
            .unwrap();
        let status = response.status();
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        (
            status,
            serde_json::from_slice(&bytes).unwrap_or(Value::Null),
        )
    }

    async fn new_set(&self, owner: &User, name: &str) -> Named {
        let r = self
            .gql(
                Some(owner),
                "mutation($n: String!) { createSet(input: { name: $n, kind: CHANNEL }) { id owner { id } } }",
                json!({ "n": name }),
            )
            .await;
        ok(&r);
        let made = &r["data"]["createSet"];
        assert!(digits(&made["id"]), "{r}");
        assert_eq!(made["owner"]["id"], owner.named.public.as_str());
        self.named_text(made["id"].as_str().unwrap()).await
    }
}

const SET: &str = "query($id: String!) {
    set(id: $id) { id owner { id } emotes { addedBy emote { id owner { id } } } }
}";
const EMOTE: &str = "query($id: String!) { emote(id: $id) { id owner { id } } }";
const ADD_EMOTE: &str = "mutation($s: String!, $e: String!) {
    addSetEmote(setId: $s, emoteId: $e) { name addedBy emote { id } }
}";

#[tokio::test]
async fn reads_show_snowflakes_by_either_name() {
    let Some(w) = World::new().await else { return };
    let alice = w.user("8101").await;
    let emote = w.emote(&alice, "flakePog").await;
    let set = w.new_set(&alice, "Main").await;
    assert_ne!(set.public, set.ulid);
    let r = w
        .gql(
            Some(&alice),
            ADD_EMOTE,
            json!({ "s": set.public, "e": emote.public }),
        )
        .await;
    ok(&r);
    assert_eq!(
        r["data"]["addSetEmote"]["emote"]["id"],
        emote.public.as_str()
    );
    assert_eq!(
        r["data"]["addSetEmote"]["addedBy"],
        alice.named.public.as_str()
    );

    for name in [&emote.public, &emote.ulid] {
        let r = w.gql(None, EMOTE, json!({ "id": name })).await;
        ok(&r);
        assert_eq!(r["data"]["emote"]["id"], emote.public.as_str(), "{r}");
        assert_eq!(
            r["data"]["emote"]["owner"]["id"],
            alice.named.public.as_str()
        );
    }
    for name in [&set.public, &set.ulid] {
        let r = w.gql(None, SET, json!({ "id": name })).await;
        ok(&r);
        let s = &r["data"]["set"];
        assert_eq!(s["id"], set.public.as_str(), "{r}");
        assert_eq!(s["owner"]["id"], alice.named.public.as_str());
        assert_eq!(s["emotes"][0]["emote"]["id"], emote.public.as_str());
        assert_eq!(s["emotes"][0]["addedBy"], alice.named.public.as_str());
    }
    // An ID written into the query text, not a variable.
    let r = w
        .gql(
            None,
            &format!(r#"{{ set(id: "{}") {{ id }} }}"#, set.public),
            json!({}),
        )
        .await;
    assert_eq!(r["data"]["set"]["id"], set.public.as_str(), "{r}");

    for name in [&alice.named.public, &alice.named.ulid] {
        let r = w
            .gql(
                None,
                "query($a: String!) { accountProfile(accountId: $a) { account { id } } accountCosmetics(accountIds: [$a]) { accountId } }",
                json!({ "a": name }),
            )
            .await;
        ok(&r);
        assert_eq!(
            r["data"]["accountProfile"]["account"]["id"],
            alice.named.public.as_str(),
            "{r}"
        );
    }
    let r = w.gql(Some(&alice), "{ me { id } }", json!({})).await;
    assert_eq!(r["data"]["me"]["id"], alice.named.public.as_str(), "{r}");

    // The channel's active set, set by the old name, reads back as the Snowflake.
    let r = w
        .gql(
            Some(&alice),
            "mutation($s: String) { updateChannelSettings(platform: TWITCH, id: \"8101\", input: { activeSetId: $s }) { activeSetId } }",
            json!({ "s": set.ulid }),
        )
        .await;
    ok(&r);
    assert_eq!(
        r["data"]["updateChannelSettings"]["activeSetId"],
        set.public.as_str()
    );
    let r = w
        .gql(
            Some(&alice),
            "{ channelEditableSets(platform: TWITCH, id: \"8101\") { channel { activeSetId account { id } } sets { set { id } } } }",
            json!({}),
        )
        .await;
    ok(&r);
    let c = &r["data"]["channelEditableSets"];
    assert_eq!(c["channel"]["activeSetId"], set.public.as_str(), "{r}");
    assert_eq!(c["channel"]["account"]["id"], alice.named.public.as_str());
    assert!(
        c["sets"]
            .as_array()
            .unwrap()
            .iter()
            .any(|s| s["set"]["id"] == set.public.as_str())
    );

    // A Snowflake that names nothing is not found, like a ULID that names nothing.
    let r = w
        .gql(None, EMOTE, json!({ "id": "1314628736410763264" }))
        .await;
    assert!(r["data"]["emote"].is_null(), "{r}");

    // REST: both names, the answer by Snowflake.
    for name in [&set.public, &set.ulid] {
        let (status, body) = w.rest(&format!("/v1/sets/{name}")).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["id"], set.public.as_str(), "{body}");
        assert_eq!(body["owner"]["id"], alice.named.public.as_str());
        assert_eq!(body["emotes"][0]["emote"]["id"], emote.public.as_str());
    }
    for name in [&emote.public, &emote.ulid] {
        let (status, body) = w.rest(&format!("/v1/emotes/{name}")).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["id"], emote.public.as_str(), "{body}");
    }
    let (status, body) = w.rest("/v1/channels/twitch/8101").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["active_set_id"], set.public.as_str(), "{body}");
    w.db.close().await;
}

#[tokio::test]
async fn every_mutation_family_takes_the_old_ulid() {
    let Some(w) = World::new().await else { return };
    let alice = w.user("8201").await;
    let bob = w.user("8202").await;
    let emote = w.emote(&alice, "oldPog").await;
    let other = w.emote(&alice, "newPog").await;
    let set = w.new_set(&alice, "Main").await;
    let spare = w.new_set(&alice, "Spare").await;

    // Set emotes: add, rename, remove, each mixing the two names.
    let r = w
        .gql(
            Some(&alice),
            ADD_EMOTE,
            json!({ "s": set.ulid, "e": emote.public }),
        )
        .await;
    ok(&r);
    let r = w
        .gql(
            Some(&alice),
            ADD_EMOTE,
            json!({ "s": set.public, "e": other.ulid }),
        )
        .await;
    ok(&r);
    let r = w
        .gql(
            Some(&alice),
            "mutation($s: String!, $e: String!) { updateSetEmote(setId: $s, emoteId: $e, input: { name: \"renamed\" }) { name emote { id } } }",
            json!({ "s": set.ulid, "e": emote.ulid }),
        )
        .await;
    ok(&r);
    assert_eq!(
        r["data"]["updateSetEmote"]["emote"]["id"],
        emote.public.as_str()
    );
    let r = w
        .gql(
            Some(&alice),
            "mutation($s: String!, $e: String!) { removeSetEmote(setId: $s, emoteId: $e) }",
            json!({ "s": set.ulid, "e": other.ulid }),
        )
        .await;
    ok(&r);

    // Sets: update, clone, delete.
    let r = w
        .gql(
            Some(&alice),
            "mutation($id: String!) { updateSet(id: $id, input: { name: \"Renamed\" }) { id name } }",
            json!({ "id": set.ulid }),
        )
        .await;
    ok(&r);
    assert_eq!(r["data"]["updateSet"]["id"], set.public.as_str());
    let r = w
        .gql(
            Some(&alice),
            "mutation($id: String!) { cloneSet(id: $id) { set { id } } }",
            json!({ "id": set.ulid }),
        )
        .await;
    ok(&r);
    assert!(digits(&r["data"]["cloneSet"]["set"]["id"]), "{r}");

    // Set editors and account editors, by the account's old name.
    let r = w
        .gql(
            Some(&alice),
            "mutation($s: String!, $a: String!) { addSetEditor(setId: $s, accountId: $a) }",
            json!({ "s": set.ulid, "a": bob.named.ulid }),
        )
        .await;
    ok(&r);
    let r = w
        .gql(
            Some(&alice),
            "query($s: String!) { setEditors(setId: $s) { id } }",
            json!({ "s": set.public }),
        )
        .await;
    ok(&r);
    assert_eq!(
        r["data"]["setEditors"][0]["id"],
        bob.named.public.as_str(),
        "{r}"
    );
    let r = w
        .gql(
            Some(&alice),
            "mutation($s: String!, $a: String!) { removeSetEditor(setId: $s, accountId: $a) }",
            json!({ "s": set.public, "a": bob.named.ulid }),
        )
        .await;
    ok(&r);
    let r = w
        .gql(
            Some(&alice),
            "mutation($a: String, $s: [String!]) { addAccountEditor(input: { accountId: $a, permissions: [EMOTES], setIds: $s }) { account { id } setIds } }",
            json!({ "a": bob.named.ulid, "s": [spare.ulid] }),
        )
        .await;
    ok(&r);
    let grant = &r["data"]["addAccountEditor"];
    assert_eq!(grant["account"]["id"], bob.named.public.as_str(), "{r}");
    assert_eq!(grant["setIds"], json!([spare.public]), "{r}");
    let r = w
        .gql(
            Some(&alice),
            "mutation($a: String!, $s: [String!]) { updateAccountEditor(accountId: $a, permissions: [EMOTES, SETS], setIds: $s) { account { id } setIds } }",
            json!({ "a": bob.named.public, "s": [spare.ulid, set.public] }),
        )
        .await;
    ok(&r);
    let mut granted: Vec<String> = r["data"]["updateAccountEditor"]["setIds"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_owned())
        .collect();
    granted.sort();
    let mut want = vec![spare.public.clone(), set.public.clone()];
    want.sort();
    assert_eq!(granted, want, "{r}");
    // Bob, an editor now, works in alice's set by its old name.
    let r = w
        .gql(
            Some(&bob),
            ADD_EMOTE,
            json!({ "s": spare.ulid, "e": other.public }),
        )
        .await;
    ok(&r);
    let r = w
        .gql(
            Some(&alice),
            "mutation($a: String!) { removeAccountEditor(accountId: $a) }",
            json!({ "a": bob.named.ulid }),
        )
        .await;
    ok(&r);

    // Emotes: update (a rename goes back to review), favourite another (listed back by
    // Snowflake), hide their creator.
    let r = w
        .gql(
            Some(&alice),
            "mutation($id: String!) { updateEmote(id: $id, input: { name: \"stillOld\" }) { id } }",
            json!({ "id": emote.ulid }),
        )
        .await;
    ok(&r);
    assert_eq!(r["data"]["updateEmote"]["id"], emote.public.as_str());
    let r = w
        .gql(
            Some(&bob),
            "mutation($id: String!) { favouriteEmote(id: $id) }",
            json!({ "id": other.ulid }),
        )
        .await;
    ok(&r);
    let r = w.gql(Some(&bob), "{ favouriteEmoteIds }", json!({})).await;
    ok(&r);
    assert_eq!(r["data"]["favouriteEmoteIds"], json!([other.public]), "{r}");
    let r = w
        .gql(
            Some(&bob),
            "mutation($a: String!) { hideCreator(accountId: $a) }",
            json!({ "a": alice.named.ulid }),
        )
        .await;
    ok(&r);

    // Activity reads by either name and shows Snowflakes.
    for name in [&alice.named.ulid, &alice.named.public] {
        let r = w
            .gql(
                None,
                "query($a: String!) { accountActivity(accountId: $a) { actor { id } set { id } emote { id } } }",
                json!({ "a": name }),
            )
            .await;
        ok(&r);
        let entries = r["data"]["accountActivity"].as_array().unwrap();
        assert!(!entries.is_empty(), "{r}");
        for entry in entries {
            for field in ["set", "emote"] {
                if !entry[field].is_null() {
                    assert!(digits(&entry[field]["id"]), "{r}");
                }
            }
            assert!(digits(&entry["actor"]["id"]), "{r}");
        }
    }

    // Delete, by the old name; an unknown Snowflake is not found.
    let r = w
        .gql(
            Some(&alice),
            "mutation($id: String!) { deleteSet(id: $id) }",
            json!({ "id": spare.ulid }),
        )
        .await;
    ok(&r);
    let r = w
        .gql(
            Some(&alice),
            "mutation($id: String!) { deleteSet(id: $id) }",
            json!({ "id": "1314628736410763264" }),
        )
        .await;
    assert_eq!(code(&r), "not_found", "{r}");
    w.db.close().await;
}
