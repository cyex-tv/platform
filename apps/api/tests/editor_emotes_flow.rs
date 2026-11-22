//! The account editors' emote permissions (ADR 0014, amended 2026/11/19): `EMOTE_UPLOAD` uploads
//! for the account, `EMOTE_MANAGE` reads, changes and deletes its emotes. `EMOTE_TRANSFER` is
//! retired (amendment 2026/11/22): only an emote's owner gives it away, nobody can ask for someone
//! else's emote, and open requests from before are withdrawn. Each is checked for the allowed case and refused without the flag, on another
//! account, on a grant limited to some sets, and while the editor or the owner is restricted; the
//! audit rows name the editor with `"via": "account_editor"` and the owner as subject. Through the
//! real router against a real Postgres; runs when `CYEX_TV_TEST_DATABASE_URL` is set (CI sets it)
//! and skips otherwise.

mod common;

use std::io::Cursor;
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
use cyex_tv_db::{audit, ids, notifications, roles};
use cyex_tv_shared::model::{EmoteState, Platform};
use cyex_tv_storage::Store;
use http_body_util::BodyExt;
use image::{ImageFormat, Rgba, RgbaImage};
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
    state: AppState,
}

fn code(response: &Value) -> &str {
    response["errors"][0]["extensions"]["code"]
        .as_str()
        .unwrap_or_default()
}

fn ok(response: &Value) {
    assert!(response["errors"].is_null(), "{response}");
}

fn png(seed: u8) -> Vec<u8> {
    let img = RgbaImage::from_fn(32, 32, |x, y| Rgba([x as u8 * 8, y as u8 * 8, seed, 255]));
    let mut out = Cursor::new(Vec::new());
    img.write_to(&mut out, ImageFormat::Png).unwrap();
    out.into_inner()
}

const BOUNDARY: &str = "cyextvboundaryEditorEmotes42";

fn form(fields: &[(&str, &str)], file: &[u8]) -> Vec<u8> {
    let mut body = Vec::new();
    for (name, value) in fields {
        body.extend_from_slice(
            format!(
                "--{BOUNDARY}\r\nContent-Disposition: form-data; name=\"{name}\"\r\n\r\n{value}\r\n"
            )
            .as_bytes(),
        );
    }
    body.extend_from_slice(
        format!(
            "--{BOUNDARY}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"e.png\"\r\nContent-Type: image/png\r\n\r\n"
        )
        .as_bytes(),
    );
    body.extend_from_slice(file);
    body.extend_from_slice(format!("\r\n--{BOUNDARY}--\r\n").as_bytes());
    body
}

const ADD: &str = "mutation($a: String, $f: [EditorPermission!]!, $s: [String!]) {
    addAccountEditor(input: { accountId: $a, permissions: $f, setIds: $s }) { permissions allSets }
}";
const CREATE_SET: &str =
    "mutation($n: String!) { createSet(input: { name: $n, kind: CHANNEL }) { id } }";
const EMOTE: &str = "query($id: String!) { emote(id: $id) { id name state owner { id } } }";
const ACCOUNT_EMOTES: &str =
    "query($a: String!) { accountEmotes(accountId: $a) { total items { id } } }";
const UPDATE: &str = "mutation($id: String!, $n: String, $v: EmoteVisibility) {
    updateEmote(id: $id, input: { name: $n, visibility: $v }) { name state visibility }
}";
const DELETE: &str = "mutation($id: String!) { deleteEmote(id: $id) }";
const TRANSFER: &str = "mutation($id: String!, $to: String!) {
    transferEmote(id: $id, toAccountId: $to) { id owner { id } }
}";
const REQUESTS: &str = "query($e: String) {
    emoteTransferRequests(emoteId: $e) { id state emoteId from { id } to { id } requestedBy { id } }
}";
const ACCEPT: &str = "mutation($id: String!) { acceptEmoteTransfer(id: $id) { id owner { id } } }";
const DECLINE: &str = "mutation($id: String!) { declineEmoteTransfer(id: $id) { id state } }";
const CANCEL: &str = "mutation($id: String!) { cancelEmoteTransfer(id: $id) { id state } }";
const APPROVE: &str = "mutation($id: String!) { approveEmote(id: $id) { id } }";
const ACTIVITY: &str = "query($a: String!) {
    accountActivity(accountId: $a) { kind actor { id } emote { id } }
}";

impl World {
    async fn new() -> Option<Self> {
        let db = TestDb::new().await?;
        let auth = Arc::new(Auth::new(AuthConfig::from_source(|_| None).unwrap()).unwrap());
        let state = AppState::new(db.pool.clone(), auth)
            .with_cdn_url("https://cdn.example.test")
            .with_store(Store::memory())
            .with_limits(Limits::new(Rates::default(), false));
        Some(Self {
            app: cyex_tv_api::app(state.clone()),
            state,
            db,
        })
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

    async fn reviewer(&self, twitch_id: &str) -> User {
        let user = self.user(twitch_id).await;
        let mut conn = self.db.conn().await;
        let role = roles::create(
            &mut conn,
            &roles::NewRole {
                name: "editor emotes reviewer",
                description: None,
                position: 10,
                permissions: &["emotes.review"],
                created_by: None,
            },
        )
        .await
        .unwrap();
        roles::assign(&mut conn, user.id, role.id, None)
            .await
            .unwrap();
        user
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

    async fn upload(
        &self,
        user: &User,
        owner: Option<&User>,
        name: &str,
        seed: u8,
    ) -> (StatusCode, Value) {
        let mut fields = vec![("name", name.to_owned())];
        if let Some(o) = owner {
            fields.push(("owner_id", o.tid()));
        }
        let fields: Vec<(&str, &str)> = fields.iter().map(|(k, v)| (*k, v.as_str())).collect();
        let request = Request::post("/v1/emotes")
            .header(
                "content-type",
                format!("multipart/form-data; boundary={BOUNDARY}"),
            )
            .header(
                "cookie",
                format!("{}={}", cyex_tv_auth::cookies::SESSION, user.token),
            )
            .header("origin", "https://cyex.tv")
            .body(Body::from(form(&fields, &png(seed))))
            .unwrap();
        let response = self.app.clone().oneshot(request).await.unwrap();
        let status = response.status();
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        (
            status,
            serde_json::from_slice(&bytes).unwrap_or(Value::Null),
        )
    }

    /// `editor` becomes an account editor of `owner` with `flags`, on every set or only `only`.
    async fn grant(&self, owner: &User, editor: &User, flags: &[&str], only: &[&str]) {
        let r = self
            .gql(
                owner,
                ADD,
                json!({ "a": editor.tid(), "f": flags, "s": only }),
            )
            .await;
        ok(&r);
    }

    async fn set(&self, owner: &User) -> String {
        let r = self.gql(owner, CREATE_SET, json!({ "n": "Main" })).await;
        r["data"]["createSet"]["id"].as_str().unwrap().to_owned()
    }

    async fn restrict(&self, user: &User, on: bool) {
        let mut conn = self.db.conn().await;
        accounts::set_restricted(&mut conn, user.id, on)
            .await
            .unwrap();
    }

    /// The audit rows with `action`, newest first, with the account whose feed they belong to.
    async fn audits(&self, action: &str) -> Vec<(audit::Entry, Option<Uuid>)> {
        let mut conn = self.db.conn().await;
        let rows = audit::list(
            &mut conn,
            &audit::Filter {
                action: Some(action),
                ..audit::Filter::default()
            },
            100,
        )
        .await
        .unwrap();
        let mut out = Vec::new();
        for row in rows {
            let subject: Option<Uuid> =
                sqlx::query_scalar("SELECT subject_account_id FROM audit_log WHERE id = $1")
                    .bind(row.id)
                    .fetch_one(&mut *conn)
                    .await
                    .unwrap();
            out.push((row, subject));
        }
        out
    }

    /// The kinds of the account's notifications.
    async fn notes(&self, user: &User) -> Vec<String> {
        let mut conn = self.db.conn().await;
        notifications::all_for(&mut conn, user.id)
            .await
            .unwrap()
            .into_iter()
            .map(|n| n.kind)
            .collect()
    }

    async fn owner_and_uploader(&self, emote: &str) -> (Option<Uuid>, Option<Uuid>) {
        let mut conn = self.db.conn().await;
        sqlx::query_as("SELECT owner_account_id, uploaded_by FROM emotes WHERE id = $1")
            .bind(self.db.row(emote).await)
            .fetch_one(&mut *conn)
            .await
            .unwrap()
    }

    /// `n` emotes made in the last hour, owned by `owner`, uploaded by `uploader` if given.
    async fn recent(&self, owner: &User, uploader: Option<&User>, n: usize) {
        let mut conn = self.db.conn().await;
        for i in 0..n {
            let e = emotes::create(
                &mut conn,
                &NewEmote {
                    owner_account_id: owner.id,
                    name: &format!("filler{i}"),
                    ..NewEmote::default()
                },
            )
            .await
            .unwrap();
            if let Some(u) = uploader {
                emotes::set_uploader(&mut conn, e.id, u.id).await.unwrap();
            }
        }
    }
}

fn assert_editor_row(row: &(audit::Entry, Option<Uuid>), editor: &User, owner: &User) {
    assert_eq!(row.0.actor_account_id, Some(editor.id), "{row:?}");
    assert_eq!(
        row.0.after_json.as_ref().unwrap()["via"],
        "account_editor",
        "{row:?}"
    );
    assert_eq!(row.1, Some(owner.id), "{row:?}");
}

#[tokio::test]
async fn upload_as_needs_the_upload_flag_on_every_set() {
    let Some(w) = World::new().await else { return };
    let alice = w.user("7001").await;
    let bob = w.user("7002").await; // EMOTE_UPLOAD
    let carol = w.user("7003").await; // nothing
    let dave = w.user("7004").await; // EMOTES (set emotes) only
    let erin = w.user("7005").await; // EMOTE_UPLOAD on one set only
    let set = w.set(&alice).await;
    w.grant(&alice, &bob, &["EMOTE_UPLOAD"], &[]).await;
    w.grant(&alice, &dave, &["EMOTES"], &[]).await;
    w.grant(&alice, &erin, &["EMOTE_UPLOAD"], &[set.as_str()])
        .await;

    // Allowed: the account owns it, the editor is named, and the row says so.
    let (status, body) = w.upload(&bob, Some(&alice), "forAlice", 1).await;
    assert_eq!(status, StatusCode::ACCEPTED, "{body}");
    let id = body["id"].as_str().unwrap().to_owned();
    assert_eq!(
        w.owner_and_uploader(&id).await,
        (Some(alice.id), Some(bob.id))
    );
    let rows = w.audits("emote.uploaded").await;
    assert_eq!(rows.len(), 1);
    assert_editor_row(&rows[0], &bob, &alice);
    // The audit log keeps the row's ULID; the answer shows the Snowflake.
    assert_eq!(rows[0].0.target_id, ids::to_text(w.db.row(&id).await));
    // Their own uploads are unchanged: theirs, nobody named.
    let (status, body) = w.upload(&bob, None, "bobsOwn", 2).await;
    assert_eq!(status, StatusCode::ACCEPTED, "{body}");
    let own = body["id"].as_str().unwrap().to_owned();
    assert_eq!(w.owner_and_uploader(&own).await, (Some(bob.id), None));
    assert_eq!(w.audits("emote.uploaded").await.len(), 1);

    // Refused: no grant, the wrong flag, a set-limited grant, and an account they don't edit.
    for (who, owner) in [
        (&carol, &alice),
        (&dave, &alice),
        (&erin, &alice),
        (&bob, &carol),
    ] {
        let (status, body) = w.upload(who, Some(owner), "nope", 3).await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{body}");
        assert_eq!(body["error"], "forbidden", "{body}");
    }

    // A restricted editor, or a restricted owner, refuses it.
    w.restrict(&bob, true).await;
    let (status, body) = w.upload(&bob, Some(&alice), "held", 4).await;
    assert_eq!(
        (status, body["error"].as_str()),
        (StatusCode::FORBIDDEN, Some("restricted"))
    );
    w.restrict(&bob, false).await;
    w.restrict(&alice, true).await;
    let (status, body) = w.upload(&bob, Some(&alice), "held", 5).await;
    assert_eq!(
        (status, body["error"].as_str()),
        (StatusCode::FORBIDDEN, Some("restricted"))
    );
    w.restrict(&alice, false).await;
    let (status, body) = w.upload(&bob, Some(&alice), "free", 6).await;
    assert_eq!(status, StatusCode::ACCEPTED, "{body}");

    // Approved, it shows on the owner's feed as uploaded by the editor.
    let reviewer = w.reviewer("7009").await;
    ok(&w.gql(&reviewer, APPROVE, json!({ "id": id })).await);
    let r = w.gql(&alice, ACTIVITY, json!({ "a": alice.tid() })).await;
    let line = r["data"]["accountActivity"]
        .as_array()
        .unwrap()
        .iter()
        .find(|l| l["emote"]["id"] == id.as_str())
        .unwrap_or_else(|| panic!("{r}"))
        .clone();
    assert_eq!(line["kind"], "EMOTE_UPLOADED");
    assert_eq!(line["actor"]["id"], w.db.public(bob.id).await.as_str());
    w.db.close().await;
}

#[tokio::test]
async fn upload_as_counts_against_the_editor_and_the_account() {
    let Some(w) = World::new().await else { return };
    let alice = w.user("7101").await;
    let bob = w.user("7102").await;
    let frank = w.user("7103").await;
    w.grant(&alice, &bob, &["EMOTE_UPLOAD"], &[]).await;
    w.grant(&frank, &bob, &["EMOTE_UPLOAD"], &[]).await;

    // Twenty uploads by Bob for Frank this hour: Bob is done, for Alice too, and for himself.
    w.recent(&frank, Some(&bob), 20).await;
    let (status, _) = w.upload(&bob, Some(&alice), "one", 1).await;
    assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);
    let (status, _) = w.upload(&bob, None, "two", 2).await;
    assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);
    // Frank had twenty: nobody else gets more in today, Frank himself included.
    let (status, _) = w.upload(&frank, None, "three", 3).await;
    assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);
    // Alice is under her limit and so is she as an uploader.
    let (status, body) = w.upload(&alice, None, "four", 4).await;
    assert_eq!(status, StatusCode::ACCEPTED, "{body}");

    // An account that got twenty refuses an editor who has uploaded nothing.
    let carl = w.user("7104").await;
    w.grant(&carl, &alice, &["EMOTE_UPLOAD"], &[]).await;
    w.recent(&carl, None, 20).await;
    let (status, _) = w.upload(&alice, Some(&carl), "five", 5).await;
    assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);
    w.db.close().await;
}

#[tokio::test]
async fn manage_reads_changes_and_deletes_the_accounts_emotes() {
    let Some(w) = World::new().await else { return };
    let alice = w.user("7201").await;
    let bob = w.user("7202").await; // EMOTE_MANAGE
    let carol = w.user("7203").await; // nothing
    let dave = w.user("7204").await; // EMOTE_UPLOAD, EMOTES
    let erin = w.user("7205").await; // EMOTE_MANAGE on one set
    let set = w.set(&alice).await;
    w.grant(&alice, &bob, &["EMOTE_MANAGE"], &[]).await;
    w.grant(&alice, &dave, &["EMOTE_UPLOAD", "EMOTES"], &[])
        .await;
    w.grant(&alice, &erin, &["EMOTE_MANAGE"], &[set.as_str()])
        .await;
    let pending = w.emote(&alice, "secret", EmoteState::Pending, true).await;
    let approved = w.emote(&alice, "public", EmoteState::Approved, false).await;
    let carols = w.emote(&carol, "carols", EmoteState::Approved, false).await;

    // Bob sees the pending private emote and the full list.
    let r = w.gql(&bob, EMOTE, json!({ "id": pending })).await;
    assert_eq!(r["data"]["emote"]["id"], pending.as_str(), "{r}");
    let r = w
        .gql(&bob, ACCOUNT_EMOTES, json!({ "a": alice.tid() }))
        .await;
    assert_eq!(r["data"]["accountEmotes"]["total"], 2, "{r}");
    // Alice may read her own list too.
    ok(&w
        .gql(&alice, ACCOUNT_EMOTES, json!({ "a": alice.tid() }))
        .await);

    // Nobody else does: no grant, the wrong flags, a set-limited grant.
    for who in [&carol, &dave, &erin] {
        let r = w.gql(who, EMOTE, json!({ "id": pending })).await;
        assert_eq!(r["data"]["emote"], Value::Null, "{r}");
        let r = w
            .gql(who, ACCOUNT_EMOTES, json!({ "a": alice.tid() }))
            .await;
        assert_eq!(code(&r), "forbidden", "{r}");
        for (q, v) in [
            (UPDATE, json!({ "id": approved, "n": "nope" })),
            (DELETE, json!({ "id": approved })),
        ] {
            assert_eq!(code(&w.gql(who, q, v).await), "forbidden", "{q}");
        }
    }
    // Bob's grant is on Alice, not on Carol.
    assert_eq!(
        code(
            &w.gql(&bob, UPDATE, json!({ "id": carols, "n": "nope" }))
                .await
        ),
        "forbidden"
    );
    assert_eq!(
        code(&w.gql(&bob, DELETE, json!({ "id": carols })).await),
        "forbidden"
    );
    let r = w
        .gql(&bob, ACCOUNT_EMOTES, json!({ "a": carol.tid() }))
        .await;
    assert_eq!(code(&r), "forbidden", "{r}");

    // Bob renames and makes it unlisted: as the owner, a new name goes back to review.
    let r = w
        .gql(
            &bob,
            UPDATE,
            json!({ "id": approved, "n": "renamed", "v": "UNLISTED" }),
        )
        .await;
    ok(&r);
    assert_eq!(r["data"]["updateEmote"]["name"], "renamed");
    assert_eq!(r["data"]["updateEmote"]["state"], "PENDING");
    assert_eq!(r["data"]["updateEmote"]["visibility"], "UNLISTED");
    let rows = w.audits("emote.updated").await;
    assert_eq!(rows.len(), 1);
    assert_editor_row(&rows[0], &bob, &alice);
    assert_eq!(rows[0].0.before_json.as_ref().unwrap()["name"], "public");
    assert_eq!(rows[0].0.after_json.as_ref().unwrap()["name"], "renamed");
    assert_eq!(w.audits("emote.resubmitted").await.len(), 1);

    // A restricted editor, or a restricted owner, refuses changes and deletes.
    for held in [&bob, &alice] {
        w.restrict(held, true).await;
        assert_eq!(
            code(
                &w.gql(&bob, UPDATE, json!({ "id": pending, "n": "held" }))
                    .await
            ),
            "restricted"
        );
        assert_eq!(
            code(&w.gql(&bob, DELETE, json!({ "id": pending })).await),
            "restricted"
        );
        w.restrict(held, false).await;
    }

    // Bob deletes the pending one; the row keeps what it was.
    let r = w.gql(&bob, DELETE, json!({ "id": pending })).await;
    assert_eq!(r["data"]["deleteEmote"], true, "{r}");
    let rows = w.audits("emote.deleted").await;
    assert_eq!(rows.len(), 1);
    assert_editor_row(&rows[0], &bob, &alice);
    assert_eq!(rows[0].0.before_json.as_ref().unwrap()["name"], "secret");

    // Taking the flag away takes access away at once.
    ok(&w
        .gql(
            &alice,
            "mutation($a: String!) { updateAccountEditor(accountId: $a, permissions: [EMOTES]) { permissions } }",
            json!({ "a": bob.tid() }),
        )
        .await);
    assert_eq!(
        code(
            &w.gql(&bob, UPDATE, json!({ "id": approved, "n": "again" }))
                .await
        ),
        "forbidden"
    );
    w.db.close().await;
}

/// Nobody can ask for someone else's emote to be transferred (owner decision 2026/11/22): an
/// account editor, whatever they hold, is refused, no request is made, the owner hears nothing,
/// and accepting answers `forbidden` for any ID.
#[tokio::test]
async fn editors_cant_transfer_or_ask_for_a_transfer() {
    let Some(w) = World::new().await else { return };
    let alice = w.user("7301").await; // the owner
    let bob = w.user("7302").await; // every emote flag on Alice, EMOTE_UPLOAD on Gina
    let gina = w.user("7306").await;
    let carol = w.user("7303").await; // nobody's editor
    // The retired flag alone is refused; with others it's dropped and never listed.
    let r = w
        .gql(
            &alice,
            ADD,
            json!({ "a": bob.tid(), "f": ["EMOTE_TRANSFER"], "s": [] }),
        )
        .await;
    assert_eq!(code(&r), "retired_permission", "{r}");
    let r = w
        .gql(
            &alice,
            ADD,
            json!({ "a": bob.tid(), "f": ["EMOTE_MANAGE", "EMOTE_UPLOAD", "EMOTE_TRANSFER"], "s": [] }),
        )
        .await;
    ok(&r);
    assert_eq!(
        r["data"]["addAccountEditor"]["permissions"],
        json!(["EMOTE_UPLOAD", "EMOTE_MANAGE"]),
        "{r}"
    );
    w.grant(&gina, &bob, &["EMOTE_UPLOAD"], &[]).await;
    // A grant stored with the old flag (from before) grants nothing more.
    let mut conn = w.db.conn().await;
    sqlx::query(
        "UPDATE account_editors SET permissions = array_append(permissions, 'emote_transfer')
         WHERE account_id = $1 AND editor_id = $2",
    )
    .bind(alice.id)
    .bind(bob.id)
    .execute(&mut *conn)
    .await
    .unwrap();
    drop(conn);
    let first = w.emote(&alice, "first", EmoteState::Approved, false).await;
    let hidden = w.emote(&alice, "hidden", EmoteState::Pending, false).await;

    for (who, emote, to, want) in [
        (&bob, &first, &gina, "forbidden"),
        (&bob, &first, &bob, "forbidden"),
        (&bob, &hidden, &gina, "forbidden"),
        (&carol, &first, &carol, "forbidden"),
        // An emote Carol can't see answers as if it weren't there.
        (&carol, &hidden, &carol, "not_found"),
    ] {
        let r = w
            .gql(who, TRANSFER, json!({ "id": emote, "to": to.tid() }))
            .await;
        assert_eq!(code(&r), want, "{r}");
    }
    let r = w
        .gql(&bob, TRANSFER, json!({ "id": first, "to": gina.tid() }))
        .await;
    assert!(
        r["errors"][0]["message"]
            .as_str()
            .unwrap_or_default()
            .contains("Only an emote's owner"),
        "{r}"
    );
    assert_eq!(w.owner_and_uploader(&first).await.0, Some(alice.id));
    let mut conn = w.db.conn().await;
    let requests: i64 = sqlx::query_scalar("SELECT count(*) FROM emote_transfer_requests")
        .fetch_one(&mut *conn)
        .await
        .unwrap();
    drop(conn);
    assert_eq!(requests, 0);
    assert!(w.audits("emote.transfer_requested").await.is_empty());
    assert!(w.audits("emote.transferred").await.is_empty());
    assert!(
        !w.notes(&alice)
            .await
            .iter()
            .any(|k| k.starts_with("emote.transfer"))
    );
    for who in [&alice, &bob] {
        let r = w.gql(who, REQUESTS, json!({ "e": first })).await;
        assert_eq!(r["data"]["emoteTransferRequests"], json!([]), "{r}");
    }
    // Accepting is retired: the same answer for any ID, to anyone.
    for who in [&alice, &bob, &carol] {
        let r = w
            .gql(who, ACCEPT, json!({ "id": "01JD8XTRANSFER000000000001" }))
            .await;
        assert_eq!(code(&r), "forbidden", "{r}");
    }
    w.db.close().await;
}

/// The owner's own transfer is unchanged: immediate, under the receiver rule (to themselves or an
/// account that trusts them with uploads), refused while anyone involved is held, audited without
/// `via`, and the receiver hears.
#[tokio::test]
async fn the_owner_still_gives_their_emote_away() {
    let Some(w) = World::new().await else { return };
    let alice = w.user("7311").await; // the owner
    let carol = w.user("7313").await; // trusts Alice with uploads, once granted
    let first = w.emote(&alice, "first", EmoteState::Approved, false).await;

    let r = w
        .gql(&alice, TRANSFER, json!({ "id": first, "to": carol.tid() }))
        .await;
    assert_eq!(code(&r), "forbidden", "{r}");
    let r = w
        .gql(&alice, TRANSFER, json!({ "id": first, "to": alice.tid() }))
        .await;
    assert_eq!(code(&r), "same_owner", "{r}");
    w.grant(&carol, &alice, &["EMOTE_UPLOAD"], &[]).await;
    for held in [&alice, &carol] {
        w.restrict(held, true).await;
        let r = w
            .gql(&alice, TRANSFER, json!({ "id": first, "to": carol.tid() }))
            .await;
        assert_eq!(code(&r), "restricted", "{r}");
        w.restrict(held, false).await;
    }
    let r = w
        .gql(&alice, TRANSFER, json!({ "id": first, "to": carol.tid() }))
        .await;
    ok(&r);
    assert_eq!(
        r["data"]["transferEmote"]["owner"]["id"],
        w.db.public(carol.id).await.as_str()
    );
    assert_eq!(w.owner_and_uploader(&first).await.0, Some(carol.id));
    let rows = w.audits("emote.transferred").await;
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].0.actor_account_id, Some(alice.id));
    assert_eq!(rows[0].1, Some(alice.id));
    assert!(rows[0].0.after_json.as_ref().unwrap().get("via").is_none());
    assert!(
        w.notes(&carol)
            .await
            .contains(&"emote.transferred".to_owned())
    );
    // It's Carol's now: Alice has no say over it.
    let r = w
        .gql(&alice, TRANSFER, json!({ "id": first, "to": alice.tid() }))
        .await;
    assert_eq!(code(&r), "forbidden", "{r}");
    w.db.close().await;
}

const NOTES: &str = "query { myNotifications { kind transferState payload } }";

/// Requests made before the change are withdrawn at start-up: cancelled, audited with
/// `"reason": "retired"` and no actor, both the owner and the editor who asked told, the owner's
/// old notification reading as cancelled. The emote stays; a second run finds nothing.
#[tokio::test]
async fn open_requests_from_before_are_withdrawn_and_both_sides_told() {
    let Some(w) = World::new().await else { return };
    let alice = w.user("7321").await; // the owner
    let bob = w.user("7322").await; // the editor who asked, before
    let gina = w.user("7323").await; // where it would have gone
    let first = w.emote(&alice, "first", EmoteState::Approved, false).await;
    let second = w.emote(&alice, "second", EmoteState::Approved, false).await;
    let later = chrono::Utc::now() + Duration::days(7);
    let mut asked = Vec::new();
    for (emote, to) in [(&first, &gina), (&second, &bob)] {
        let row = w.db.row(emote).await;
        let mut conn = w.db.conn().await;
        let r = cyex_tv_db::transfers::create(&mut conn, row, alice.id, to.id, bob.id, later)
            .await
            .unwrap()
            .unwrap();
        notifications::create(
            &mut conn,
            alice.id,
            "emote.transfer_requested",
            &json!({ "request_id": ids::to_text(r.id), "emote_id": emote, "name": "x" }),
        )
        .await
        .unwrap();
        asked.push(r.id);
    }

    assert_eq!(
        cyex_tv_api::transfers::withdraw_open(&w.state)
            .await
            .unwrap(),
        2
    );
    let mut conn = w.db.conn().await;
    for id in &asked {
        let state: String =
            sqlx::query_scalar("SELECT state FROM emote_transfer_requests WHERE id = $1")
                .bind(id)
                .fetch_one(&mut *conn)
                .await
                .unwrap();
        assert_eq!(state, "cancelled");
    }
    drop(conn);
    for emote in [&first, &second] {
        assert_eq!(w.owner_and_uploader(emote).await.0, Some(alice.id));
    }
    let rows = w.audits("emote.transfer_cancelled").await;
    assert_eq!(rows.len(), 2);
    for row in &rows {
        assert_eq!(row.0.actor_account_id, None);
        assert_eq!(row.1, Some(alice.id));
        assert_eq!(row.0.after_json.as_ref().unwrap()["reason"], "retired");
    }
    for who in [&alice, &bob] {
        let kinds = w.notes(who).await;
        assert_eq!(
            kinds
                .iter()
                .filter(|k| *k == "emote.transfer_cancelled")
                .count(),
            2,
            "{kinds:?}"
        );
    }
    let r = w.gql(&alice, NOTES, json!({})).await;
    ok(&r);
    let states: Vec<&Value> = r["data"]["myNotifications"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|n| n["kind"] == "emote.transfer_requested")
        .map(|n| &n["transferState"])
        .collect();
    assert_eq!(states, [&json!("CANCELLED"), &json!("CANCELLED")]);
    let r = w.gql(&alice, REQUESTS, json!({})).await;
    assert_eq!(r["data"]["emoteTransferRequests"], json!([]), "{r}");
    // Declining or cancelling one now: it isn't open any more.
    let old = ids::to_text(asked[0]);
    for (who, q) in [(&alice, DECLINE), (&bob, CANCEL)] {
        let r = w.gql(who, q, json!({ "id": old })).await;
        assert_eq!(code(&r), "not_pending", "{r}");
    }
    // Once done, there's nothing left to withdraw.
    assert_eq!(
        cyex_tv_api::transfers::withdraw_open(&w.state)
            .await
            .unwrap(),
        0
    );
    assert_eq!(w.audits("emote.transfer_cancelled").await.len(), 2);
    w.db.close().await;
}
