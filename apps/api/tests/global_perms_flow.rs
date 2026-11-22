//! Role permissions that act on anyone's account (ADR 0010, amendment 2026/11/19, round 2):
//! `emotes.edit_any` edits anyone's emote (name, tags, flags, visibility), `emotes.delete_any`
//! deletes it, `editors.manage_any` adds, changes and removes anyone's account editors. Each is
//! refused without the permission, works with it, and is audited as a staff action. Through the
//! real router against a real Postgres; runs when `CYEX_TV_TEST_DATABASE_URL` is set (CI sets
//! it) and skips otherwise.

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
use cyex_tv_db::{audit, ids, roles, sets};
use cyex_tv_shared::model::{EmoteState, Platform, SetKind};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use tower::ServiceExt;
use uuid::Uuid;

struct User {
    id: Uuid,
    token: String,
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

const UPDATE_EMOTE: &str = "mutation($id: String!, $input: UpdateEmoteInput!) {
    updateEmote(id: $id, input: $input) { name tags flags { zeroWidth nsfw } visibility }
}";
const DELETE_EMOTE: &str = "mutation($id: String!) { deleteEmote(id: $id) }";
const ACCOUNT_EMOTES: &str =
    "query($a: String!) { accountEmotes(accountId: $a) { total items { id } } }";
const ADD: &str = "mutation($o: String, $a: String, $f: [EditorPermission!]!) {
    addAccountEditor(input: { ownerId: $o, accountId: $a, permissions: $f }) { account { id } permissions }
}";
const UPDATE: &str = "mutation($o: String, $a: String!, $f: [EditorPermission!]!) {
    updateAccountEditor(ownerId: $o, accountId: $a, permissions: $f) { account { id } permissions }
}";
const REMOVE: &str =
    "mutation($o: String, $a: String!) { removeAccountEditor(ownerId: $o, accountId: $a) }";
const LIST: &str =
    "query($o: String) { accountEditors(accountId: $o) { account { id } permissions } }";

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

    /// A Twitch user `user<id>` with a site session, holding `permissions` through one role.
    async fn user(&self, twitch_id: &str, permissions: &[&str]) -> User {
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
        if !permissions.is_empty() {
            let role = roles::create(
                &mut conn,
                &roles::NewRole {
                    name: &format!("staff {twitch_id}"),
                    description: None,
                    position: 1,
                    permissions,
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

    /// An approved emote of `owner`, in one of their channel sets.
    async fn emote(&self, owner: &User, name: &str) -> String {
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
        let set = sets::create(&mut conn, owner.id, "Main", SetKind::Channel)
            .await
            .unwrap();
        sets::add_member(
            &mut conn,
            set.id,
            sets::NewMember {
                emote_id: emote.id,
                name: None,
                zero_width: None,
                added_by: Some(owner.id),
            },
        )
        .await
        .unwrap();
        self.db.public(emote.id).await
    }

    async fn gql(&self, user: &User, query: &str, variables: Value) -> Value {
        let request = Request::post("/v1/gql")
            .header("content-type", "application/json")
            .header(
                "cookie",
                format!("{}={}", cyex_tv_auth::cookies::SESSION, user.token),
            )
            .header("origin", "https://cyex.tv")
            .body(Body::from(
                json!({ "query": query, "variables": variables }).to_string(),
            ))
            .unwrap();
        let response = self.app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        serde_json::from_slice(&bytes).unwrap()
    }

    async fn audit_rows(&self, action: &str) -> Vec<audit::Entry> {
        let mut conn = self.db.conn().await;
        audit::list(&mut conn, &audit::Filter::default(), 200)
            .await
            .unwrap()
            .into_iter()
            .filter(|e| e.action == action)
            .collect()
    }

    async fn exists(&self, emote: &str) -> bool {
        let mut conn = self.db.conn().await;
        emotes::get(&mut conn, self.db.row(emote).await)
            .await
            .unwrap()
            .is_some()
    }
}

#[tokio::test]
async fn edit_any_and_delete_any_work_on_anyones_emotes_and_are_audited() {
    let Some(w) = World::new().await else { return };
    let owner = w.user("7001", &[]).await;
    let nobody = w.user("7002", &[]).await;
    let editor = w.user("7003", &["emotes.edit_any"]).await;
    let deleter = w.user("7004", &["emotes.delete_any"]).await;
    let emote = w.emote(&owner, "ownersEmote").await;
    let change = json!({ "id": emote, "input": {
        "name": "staffRenamed", "tags": ["calm"], "zeroWidth": true, "visibility": "UNLISTED"
    } });

    // Deny: neither an unrelated account nor a deleter-only role may edit; nobody may delete or list.
    for who in [&nobody, &deleter] {
        assert_eq!(
            code(&w.gql(who, UPDATE_EMOTE, change.clone()).await),
            "forbidden"
        );
    }
    for who in [&nobody, &editor] {
        assert_eq!(
            code(&w.gql(who, DELETE_EMOTE, json!({ "id": emote })).await),
            "forbidden"
        );
    }
    assert_eq!(
        code(
            &w.gql(&nobody, ACCOUNT_EMOTES, json!({ "a": owner.tid() }))
                .await
        ),
        "forbidden"
    );
    assert!(w.exists(&emote).await);

    // Allow: edit_any changes name, tags, flags and visibility, and stays approved.
    let r = w.gql(&editor, UPDATE_EMOTE, change).await;
    let e = &r["data"]["updateEmote"];
    assert_eq!(e["name"], "staffRenamed", "{r}");
    assert_eq!(e["tags"], json!(["calm"]));
    assert_eq!(e["flags"], json!({ "zeroWidth": true, "nsfw": false }));
    // Staff may take it off the listings (unlisted is a moderator's call, ADR 0020).
    assert_eq!(e["visibility"], "UNLISTED");
    let updated = w.audit_rows("emote.updated").await;
    assert_eq!(updated.len(), 1);
    assert_eq!(updated[0].actor_account_id, Some(editor.id));

    // Both may find the account's emotes on the admin dashboard.
    for who in [&editor, &deleter] {
        let r = w
            .gql(who, ACCOUNT_EMOTES, json!({ "a": owner.tid() }))
            .await;
        assert_eq!(r["data"]["accountEmotes"]["total"], 1, "{r}");
    }

    // Allow: delete_any deletes it; audited with the emote as it was and the permission used.
    let r = w.gql(&deleter, DELETE_EMOTE, json!({ "id": emote })).await;
    assert_eq!(r["data"]["deleteEmote"], true, "{r}");
    assert!(!w.exists(&emote).await);
    let deleted = w.audit_rows("emote.deleted").await;
    assert_eq!(deleted.len(), 1);
    assert_eq!(deleted[0].actor_account_id, Some(deleter.id));
    assert_eq!(deleted[0].target_id, ids::to_text(w.db.row(&emote).await));
    let before = deleted[0].before_json.as_ref().unwrap();
    assert_eq!(before["name"], "staffRenamed");
    assert_eq!(before["owner_id"], owner.tid().as_str());
    assert_eq!(
        deleted[0].after_json.as_ref().unwrap()["via"],
        "emotes.delete_any"
    );

    // The owner deleting their own emote is not a staff action.
    let own = w.emote(&owner, "ownDelete").await;
    let r = w.gql(&owner, DELETE_EMOTE, json!({ "id": own })).await;
    assert_eq!(r["data"]["deleteEmote"], true, "{r}");
    assert_eq!(w.audit_rows("emote.deleted").await.len(), 1);
    w.db.close().await;
}

#[tokio::test]
async fn editors_manage_any_manages_anyones_editors_and_is_audited() {
    let Some(w) = World::new().await else { return };
    let owner = w.user("7101", &[]).await;
    let helper = w.user("7102", &[]).await;
    let nobody = w.user("7103", &[]).await;
    let viewer = w.user("7104", &["users.view", "emotes.edit_any"]).await;
    let staff = w.user("7105", &["editors.manage_any"]).await;
    let o = owner.tid();

    // The owner's own editor: the starting point.
    let r = w
        .gql(
            &owner,
            ADD,
            json!({ "o": null, "a": helper.tid(), "f": ["EMOTES", "EDITORS"] }),
        )
        .await;
    assert_eq!(
        r["data"]["addAccountEditor"]["account"]["id"],
        w.db.public(helper.id).await.as_str(),
        "{r}"
    );

    // Deny: without editors.manage_any (other staff permissions don't count) nothing goes through.
    for who in [&nobody, &viewer] {
        assert_eq!(
            code(&w.gql(who, LIST, json!({ "o": o })).await),
            "forbidden"
        );
        assert_eq!(
            code(
                &w.gql(
                    who,
                    ADD,
                    json!({ "o": o, "a": nobody.tid(), "f": ["EMOTES"] })
                )
                .await
            ),
            "forbidden"
        );
        assert_eq!(
            code(
                &w.gql(
                    who,
                    UPDATE,
                    json!({ "o": o, "a": helper.tid(), "f": ["EMOTES"] })
                )
                .await
            ),
            "forbidden"
        );
        assert_eq!(
            code(
                &w.gql(who, REMOVE, json!({ "o": o, "a": helper.tid() }))
                    .await
            ),
            "forbidden"
        );
    }
    assert_eq!(w.audit_rows("account.editor_updated").await.len(), 0);

    // Allow: list, add (every flag, as the owner could), change and remove on the owner's behalf.
    let r = w.gql(&staff, LIST, json!({ "o": o })).await;
    assert_eq!(
        r["data"]["accountEditors"].as_array().unwrap().len(),
        1,
        "{r}"
    );
    let r = w
        .gql(
            &staff,
            ADD,
            json!({ "o": o, "a": nobody.tid(), "f": ["SETS", "PROFILE", "EDITORS"] }),
        )
        .await;
    assert_eq!(
        r["data"]["addAccountEditor"]["account"]["id"],
        w.db.public(nobody.id).await.as_str(),
        "{r}"
    );
    let r = w
        .gql(
            &staff,
            UPDATE,
            json!({ "o": o, "a": helper.tid(), "f": ["EMOTES"] }),
        )
        .await;
    assert_eq!(
        r["data"]["updateAccountEditor"]["permissions"],
        json!(["EMOTES"]),
        "{r}"
    );
    let r = w
        .gql(&staff, REMOVE, json!({ "o": o, "a": helper.tid() }))
        .await;
    assert_eq!(r["data"]["removeAccountEditor"], true, "{r}");
    let r = w.gql(&owner, LIST, json!({ "o": null })).await;
    let left: Vec<&str> = r["data"]["accountEditors"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["account"]["id"].as_str().unwrap())
        .collect();
    assert_eq!(left, [w.db.public(nobody.id).await.as_str()]);

    // Each change is audited on the owner's account, by the staff member, marked with the permission.
    for action in [
        "account.editor_added",
        "account.editor_updated",
        "account.editor_removed",
    ] {
        let rows: Vec<_> = w
            .audit_rows(action)
            .await
            .into_iter()
            .filter(|e| e.actor_account_id == Some(staff.id))
            .collect();
        assert_eq!(rows.len(), 1, "{action}");
        assert_eq!(rows[0].target_id, o);
        assert_eq!(
            rows[0].after_json.as_ref().unwrap()["via"],
            "editors.manage_any",
            "{action}"
        );
    }
    // The owner's own change carries no such mark.
    let own = w.audit_rows("account.editor_added").await;
    let own = own
        .iter()
        .find(|e| e.actor_account_id == Some(owner.id))
        .unwrap();
    assert!(own.after_json.as_ref().unwrap().get("via").is_none());

    // An account that doesn't exist is not found, not created.
    let ghost = ids::to_text(Uuid::from_u128(0x0123_4567_89ab_cdef));
    assert_eq!(
        code(
            &w.gql(
                &staff,
                ADD,
                json!({ "o": ghost, "a": nobody.tid(), "f": ["EMOTES"] })
            )
            .await
        ),
        "not_found"
    );
    w.db.close().await;
}
