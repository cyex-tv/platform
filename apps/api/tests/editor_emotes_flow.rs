//! The account editors' emote permissions (ADR 0014, amended 2026/11/19): `EMOTE_UPLOAD` uploads
//! for the account, `EMOTE_MANAGE` reads, changes and deletes its emotes, `EMOTE_TRANSFER` gives
//! them away. Each is checked for the allowed case and refused without the flag, on another
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
            app: cyex_tv_api::app(state),
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
            SessionKind::App,
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
        ids::to_text(emote.id)
    }

    async fn gql(&self, user: &User, query: &str, variables: Value) -> Value {
        let request = Request::post("/v1/gql")
            .header("content-type", "application/json")
            .header("authorization", format!("Bearer {}", user.token))
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
            .header("authorization", format!("Bearer {}", user.token))
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

    /// The one open transfer request of `emote`, as `user` sees it.
    async fn open(&self, user: &User, emote: &str) -> String {
        let r = self.gql(user, REQUESTS, json!({ "e": emote })).await;
        let list = r["data"]["emoteTransferRequests"]
            .as_array()
            .unwrap_or_else(|| panic!("{r}"));
        assert_eq!(list.len(), 1, "{r}");
        assert_eq!(list[0]["state"], "PENDING", "{r}");
        list[0]["id"].as_str().unwrap().to_owned()
    }

    async fn owner_and_uploader(&self, emote: &str) -> (Option<Uuid>, Option<Uuid>) {
        let mut conn = self.db.conn().await;
        sqlx::query_as("SELECT owner_account_id, uploaded_by FROM emotes WHERE id = $1")
            .bind(ids::from_text(emote).unwrap())
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
    assert_eq!(rows[0].0.target_id, id);
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
    assert_eq!(line["actor"]["id"], bob.tid().as_str());
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

#[tokio::test]
async fn editor_transfers_wait_for_the_owner_and_go_only_to_trusting_accounts() {
    let Some(w) = World::new().await else { return };
    let alice = w.user("7301").await; // the owner
    let bob = w.user("7302").await; // EMOTE_TRANSFER on Alice, EMOTE_UPLOAD on Gina
    let carol = w.user("7303").await; // trusts nobody
    let dave = w.user("7304").await; // EMOTE_MANAGE on Alice only
    let erin = w.user("7305").await; // EMOTE_TRANSFER on one set of Alice
    let gina = w.user("7306").await;
    let set = w.set(&alice).await;
    w.grant(&alice, &bob, &["EMOTE_TRANSFER"], &[]).await;
    w.grant(&gina, &bob, &["EMOTE_UPLOAD"], &[]).await;
    w.grant(&alice, &dave, &["EMOTE_MANAGE"], &[]).await;
    w.grant(&alice, &erin, &["EMOTE_TRANSFER"], &[set.as_str()])
        .await;
    w.grant(&gina, &erin, &["EMOTE_UPLOAD"], &[]).await;
    w.grant(&gina, &dave, &["EMOTE_UPLOAD"], &[]).await;
    let first = w.emote(&alice, "first", EmoteState::Approved, false).await;
    let second = w.emote(&alice, "second", EmoteState::Approved, false).await;

    // To an account that never made Bob an uploader: refused.
    let r = w
        .gql(&bob, TRANSFER, json!({ "id": first, "to": carol.tid() }))
        .await;
    assert_eq!(code(&r), "forbidden", "{r}");
    // Without the flag, or with it on one set only: refused.
    for who in [&dave, &erin, &carol] {
        let r = w
            .gql(who, TRANSFER, json!({ "id": first, "to": gina.tid() }))
            .await;
        assert_eq!(code(&r), "forbidden", "{r}");
    }
    // To the account that owns it already.
    let r = w
        .gql(&bob, TRANSFER, json!({ "id": first, "to": alice.tid() }))
        .await;
    assert_eq!(code(&r), "same_owner", "{r}");

    // Restricted: the editor, the owner, or the receiver.
    for held in [&bob, &alice, &gina] {
        w.restrict(held, true).await;
        let r = w
            .gql(&bob, TRANSFER, json!({ "id": first, "to": gina.tid() }))
            .await;
        assert_eq!(code(&r), "restricted", "{r}");
        w.restrict(held, false).await;
    }

    // Allowed: a request waits for Alice; nothing moves yet.
    let r = w
        .gql(&bob, TRANSFER, json!({ "id": first, "to": gina.tid() }))
        .await;
    ok(&r);
    assert_eq!(
        r["data"]["transferEmote"]["owner"]["id"],
        alice.tid().as_str()
    );
    assert_eq!(w.owner_and_uploader(&first).await.0, Some(alice.id));
    let rows = w.audits("emote.transfer_requested").await;
    assert_eq!(rows.len(), 1);
    assert_editor_row(&rows[0], &bob, &alice);
    assert!(w.audits("emote.transferred").await.is_empty());
    assert!(
        w.notes(&alice)
            .await
            .contains(&"emote.transfer_requested".to_owned())
    );
    // One open request per emote.
    let r = w
        .gql(&bob, TRANSFER, json!({ "id": first, "to": bob.tid() }))
        .await;
    assert_eq!(code(&r), "transfer_pending", "{r}");
    // Both sides see it; nobody else does.
    let request = w.open(&alice, &first).await;
    assert_eq!(w.open(&bob, &first).await, request);
    let r = w.gql(&carol, REQUESTS, json!({ "e": first })).await;
    assert_eq!(r["data"]["emoteTransferRequests"], json!([]), "{r}");

    // Only the owner accepts: not the editor who asked. To anyone else (the receiver too) the
    // request doesn't exist, whatever they try.
    let r = w.gql(&bob, ACCEPT, json!({ "id": request })).await;
    assert_eq!(code(&r), "forbidden", "{r}");
    for who in [&gina, &carol] {
        for q in [ACCEPT, DECLINE, CANCEL] {
            let r = w.gql(who, q, json!({ "id": request })).await;
            assert_eq!(code(&r), "not_found", "{r}");
        }
    }
    // Holds still refuse at accept time: the receiver, or the editor who asked.
    for held in [&gina, &bob] {
        w.restrict(held, true).await;
        let r = w.gql(&alice, ACCEPT, json!({ "id": request })).await;
        assert_eq!(code(&r), "restricted", "{r}");
        w.restrict(held, false).await;
    }
    // Accepted: Gina owns it, both steps are audited, the editor and the receiver hear.
    let r = w.gql(&alice, ACCEPT, json!({ "id": request })).await;
    ok(&r);
    assert_eq!(
        r["data"]["acceptEmoteTransfer"]["owner"]["id"],
        gina.tid().as_str()
    );
    assert_eq!(w.owner_and_uploader(&first).await.0, Some(gina.id));
    let rows = w.audits("emote.transfer_accepted").await;
    assert_eq!(
        (rows.len(), rows[0].0.actor_account_id),
        (1, Some(alice.id))
    );
    assert_eq!(rows[0].1, Some(alice.id));
    let rows = w.audits("emote.transferred").await;
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].0.actor_account_id, Some(alice.id));
    let after = rows[0].0.after_json.as_ref().unwrap();
    assert_eq!(after["owner_id"], gina.tid().as_str());
    assert_eq!(after["requested_by"], bob.tid().as_str());
    for who in [&bob, &gina] {
        assert!(w.notes(who).await.contains(&"emote.transferred".to_owned()));
    }
    let r = w.gql(&alice, ACCEPT, json!({ "id": request })).await;
    assert_eq!(code(&r), "not_pending", "{r}");
    // Alice's editor has no say over it any more.
    let r = w
        .gql(&bob, TRANSFER, json!({ "id": first, "to": bob.tid() }))
        .await;
    assert_eq!(code(&r), "forbidden", "{r}");

    // Declined: it stays, the editor hears.
    ok(&w
        .gql(&bob, TRANSFER, json!({ "id": second, "to": bob.tid() }))
        .await);
    let request = w.open(&alice, &second).await;
    let r = w.gql(&bob, DECLINE, json!({ "id": request })).await;
    assert_eq!(code(&r), "forbidden", "{r}");
    let r = w.gql(&alice, DECLINE, json!({ "id": request })).await;
    assert_eq!(
        r["data"]["declineEmoteTransfer"]["state"], "DECLINED",
        "{r}"
    );
    assert_eq!(w.owner_and_uploader(&second).await.0, Some(alice.id));
    assert!(
        w.notes(&bob)
            .await
            .contains(&"emote.transfer_declined".to_owned())
    );
    assert_eq!(w.audits("emote.transfer_declined").await.len(), 1);

    // Cancelled by the editor: it stays, and can't be accepted afterwards.
    ok(&w
        .gql(&bob, TRANSFER, json!({ "id": second, "to": bob.tid() }))
        .await);
    let request = w.open(&bob, &second).await;
    let r = w.gql(&alice, CANCEL, json!({ "id": request })).await;
    assert_eq!(code(&r), "forbidden", "{r}");
    let r = w.gql(&bob, CANCEL, json!({ "id": request })).await;
    assert_eq!(
        r["data"]["cancelEmoteTransfer"]["state"], "CANCELLED",
        "{r}"
    );
    let r = w.gql(&alice, ACCEPT, json!({ "id": request })).await;
    assert_eq!(code(&r), "not_pending", "{r}");
    assert_eq!(w.owner_and_uploader(&second).await.0, Some(alice.id));
    assert_eq!(w.audits("emote.transfer_cancelled").await.len(), 1);

    // Expired after 7 days: it stays, the expiry is audited, and the editor can ask again.
    ok(&w
        .gql(&bob, TRANSFER, json!({ "id": second, "to": gina.tid() }))
        .await);
    let request = w.open(&alice, &second).await;
    let mut conn = w.db.conn().await;
    sqlx::query(
        "UPDATE emote_transfer_requests SET expires_at = now() - interval '1 minute' WHERE id = $1",
    )
    .bind(ids::from_text(&request).unwrap())
    .execute(&mut *conn)
    .await
    .unwrap();
    drop(conn);
    let r = w.gql(&alice, ACCEPT, json!({ "id": request })).await;
    assert_eq!(code(&r), "not_pending", "{r}");
    assert_eq!(w.owner_and_uploader(&second).await.0, Some(alice.id));
    let rows = w.audits("emote.transfer_expired").await;
    assert_eq!((rows.len(), rows[0].0.actor_account_id), (1, None));
    let r = w.gql(&alice, REQUESTS, json!({ "e": second })).await;
    assert_eq!(r["data"]["emoteTransferRequests"], json!([]), "{r}");
    ok(&w
        .gql(&bob, TRANSFER, json!({ "id": second, "to": gina.tid() }))
        .await);

    // The owner's own transfer is immediate, under the same receiver rule, and closes the
    // request that was waiting.
    let r = w
        .gql(&alice, TRANSFER, json!({ "id": second, "to": carol.tid() }))
        .await;
    assert_eq!(code(&r), "forbidden", "{r}");
    w.grant(&carol, &alice, &["EMOTE_UPLOAD"], &[]).await;
    let r = w
        .gql(&alice, TRANSFER, json!({ "id": second, "to": carol.tid() }))
        .await;
    ok(&r);
    assert_eq!(
        r["data"]["transferEmote"]["owner"]["id"],
        carol.tid().as_str()
    );
    let rows = w.audits("emote.transferred").await;
    assert_eq!(rows[0].0.actor_account_id, Some(alice.id));
    assert!(rows[0].0.after_json.as_ref().unwrap().get("via").is_none());
    let r = w.gql(&bob, REQUESTS, json!({ "e": second })).await;
    assert_eq!(r["data"]["emoteTransferRequests"], json!([]), "{r}");
    w.db.close().await;
}

/// A request lasts only while the editor who made it can transfer the owner's emotes: accepting
/// after their `emote_transfer` went (or their grant did) closes it instead, audited.
#[tokio::test]
async fn accepting_rechecks_the_editors_transfer_permission() {
    let Some(w) = World::new().await else { return };
    let alice = w.user("7401").await; // the owner
    let bob = w.user("7402").await; // EMOTE_TRANSFER on Alice, then only EMOTE_MANAGE
    let gina = w.user("7403").await; // trusts Bob with uploads
    w.grant(&alice, &bob, &["EMOTE_TRANSFER"], &[]).await;
    w.grant(&gina, &bob, &["EMOTE_UPLOAD"], &[]).await;
    let first = w.emote(&alice, "first", EmoteState::Approved, false).await;
    let second = w.emote(&alice, "second", EmoteState::Approved, false).await;
    for emote in [&first, &second] {
        ok(&w
            .gql(&bob, TRANSFER, json!({ "id": emote, "to": gina.tid() }))
            .await);
    }
    let one = w.open(&alice, &first).await;
    let two = w.open(&alice, &second).await;

    // The flag is taken away: accepting closes the request and nothing moves.
    let r = w
        .gql(
            &alice,
            "mutation($a: String!, $f: [EditorPermission!]!) {
                updateAccountEditor(accountId: $a, permissions: $f) { permissions }
            }",
            json!({ "a": bob.tid(), "f": ["EMOTE_MANAGE"] }),
        )
        .await;
    ok(&r);
    let r = w.gql(&alice, ACCEPT, json!({ "id": one })).await;
    assert_eq!(code(&r), "not_pending", "{r}");
    assert_eq!(w.owner_and_uploader(&first).await.0, Some(alice.id));
    let rows = w.audits("emote.transfer_cancelled").await;
    assert_eq!(rows.len(), 1);
    assert_eq!(
        rows[0].0.after_json.as_ref().unwrap()["reason"],
        "editor_permission"
    );
    let r = w.gql(&alice, ACCEPT, json!({ "id": one })).await;
    assert_eq!(code(&r), "not_pending", "{r}");

    // The grant is removed altogether: the same.
    let r = w
        .gql(
            &alice,
            "mutation($a: String!) { removeAccountEditor(accountId: $a) }",
            json!({ "a": bob.tid() }),
        )
        .await;
    ok(&r);
    let r = w.gql(&alice, ACCEPT, json!({ "id": two })).await;
    assert_eq!(code(&r), "not_pending", "{r}");
    assert_eq!(w.owner_and_uploader(&second).await.0, Some(alice.id));
    assert_eq!(w.audits("emote.transfer_cancelled").await.len(), 2);
    assert!(w.audits("emote.transferred").await.is_empty());
    w.db.close().await;
}

/// Accepting a request and the owner's own transfer of the same emote at once take their locks
/// in the same order (the emote, then its requests), so neither deadlocks into a 500: one wins,
/// the other is refused cleanly, and the emote ends with one owner.
#[tokio::test]
async fn accept_and_owner_transfer_at_once_dont_deadlock() {
    let Some(w) = World::new().await else { return };
    let alice = w.user("7501").await; // the owner
    let bob = w.user("7502").await; // EMOTE_TRANSFER on Alice
    let carol = w.user("7503").await; // trusts Alice with uploads
    w.grant(&alice, &bob, &["EMOTE_TRANSFER"], &[]).await;
    w.grant(&carol, &alice, &["EMOTE_UPLOAD"], &[]).await;
    for round in 0..8 {
        let emote = w
            .emote(&alice, &format!("race{round}"), EmoteState::Approved, false)
            .await;
        ok(&w
            .gql(&bob, TRANSFER, json!({ "id": emote, "to": bob.tid() }))
            .await);
        let request = w.open(&alice, &emote).await;
        let (accepted, moved) = tokio::join!(
            w.gql(&alice, ACCEPT, json!({ "id": request })),
            w.gql(&alice, TRANSFER, json!({ "id": emote, "to": carol.tid() })),
        );
        for r in [&accepted, &moved] {
            assert_ne!(code(r), "internal", "round {round}: {r}");
        }
        let wins = [&accepted, &moved]
            .iter()
            .filter(|r| r["errors"].is_null())
            .count();
        assert!(wins >= 1, "round {round}: {accepted} {moved}");
        let owner = w.owner_and_uploader(&emote).await.0;
        assert!(
            owner == Some(bob.id) || owner == Some(carol.id),
            "round {round}: {owner:?}"
        );
    }
    w.db.close().await;
}
