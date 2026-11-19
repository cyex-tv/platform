//! Input rules of the staff and editor mutations that no other test sends a bad value to: each
//! call below is made by the owner (every permission) on real rows, with one bad input, and must
//! answer the rule's error code and change nothing.

use cyex_tv_db::emotes::{self, NewEmote};
use cyex_tv_db::ids;
use cyex_tv_shared::model::EmoteState;
use serde_json::{Value, json};

use crate::world::{As, OWNER, World, error_code, form, png};

fn id_at(r: &Value, pointer: &str) -> String {
    r.pointer(pointer)
        .and_then(Value::as_str)
        .unwrap_or_else(|| panic!("no {pointer}: {r}"))
        .to_owned()
}

#[tokio::test]
async fn bad_inputs_answer_their_rule() {
    let Some(w) = World::new().await else {
        return;
    };
    let o = w.user(OWNER).await;
    let f = w.user("7402").await;
    let os = As::App(&o);

    // Real rows to aim the bad inputs at.
    let r = w
        .post_form(
            As::App(&o),
            "/v1/cosmetics/images",
            form(&[("kind", "badge")], Some(&png(72, 72, 9))),
        )
        .await;
    let image = r.body["id"]
        .as_str()
        .unwrap_or_else(|| panic!("{}", r.body))
        .to_owned();
    w.finish_picture(&image).await;
    let badge = id_at(
        &w.gql(
            os,
            "mutation($i: String!) { createBadge(input: { name: \"Val\", tooltip: \"Val\", imageId: $i }) { id } }",
            json!({ "i": image }),
        )
        .await,
        "/data/createBadge/id",
    );
    let paint_input = json!({
        "name": "Val paint", "kind": "LINEAR_GRADIENT", "angle": 45,
        "stops": [{ "at": 0.0, "color": "#ff0000" }, { "at": 1.0, "color": "#0000ff" }]
    });
    let paint = id_at(
        &w.gql(
            os,
            "mutation($p: PaintInput!) { createPaint(input: $p) { id } }",
            json!({ "p": paint_input }),
        )
        .await,
        "/data/createPaint/id",
    );
    let item = id_at(
        &w.gql(
            os,
            "mutation($b: String!) { createShopItem(input: { name: \"Val item\", priceCents: 100, badgeIds: [$b] }) { id } }",
            json!({ "b": badge }),
        )
        .await,
        "/data/createShopItem/id",
    );
    let (pending, approved) = {
        let mut conn = w.db.conn().await;
        let p = emotes::create(
            &mut conn,
            &NewEmote {
                owner_account_id: o.id,
                name: "valPending",
                ..NewEmote::default()
            },
        )
        .await
        .unwrap();
        let a = emotes::create(
            &mut conn,
            &NewEmote {
                owner_account_id: o.id,
                name: "valApproved",
                ..NewEmote::default()
            },
        )
        .await
        .unwrap();
        emotes::set_state(&mut conn, a.id, EmoteState::Approved)
            .await
            .unwrap();
        (ids::to_text(p.id), ids::to_text(a.id))
    };
    let r = w
        .gql(
            os,
            "mutation($a: String!) { addAccountEditor(input: { accountId: $a, permissions: [EMOTES] }) { allSets } }",
            json!({ "a": f.tid() }),
        )
        .await;
    assert!(r["errors"].is_null(), "{r}");

    let long = "x".repeat(101);
    let many: Vec<String> = (0..101).map(|i| format!("{i}")).collect();
    let eleven: Vec<String> = (0..11).map(|i| format!("tag{i}")).collect();
    let stops1 = json!([{ "at": 0.0, "color": "#ff0000" }]);
    let cases: Vec<(&str, &str, Value, &str)> = vec![
        (
            "updateBadge: empty tooltip",
            "mutation($id: String!) { updateBadge(id: $id, input: { tooltip: \"\" }) { id } }",
            json!({ "id": badge }),
            "bad_tooltip",
        ),
        (
            "updateBadge: name over 32",
            "mutation($id: String!, $n: String!) { updateBadge(id: $id, input: { name: $n }) { id } }",
            json!({ "id": badge, "n": "n".repeat(33) }),
            "bad_name",
        ),
        (
            "updatePaint: angle 400",
            "mutation($id: String!, $p: PaintInput!) { updatePaint(id: $id, input: $p) { id } }",
            json!({ "id": paint, "p": { "name": "Val paint", "kind": "LINEAR_GRADIENT", "angle": 400, "stops": paint_input["stops"] } }),
            "bad_angle",
        ),
        (
            "updatePaint: one stop",
            "mutation($id: String!, $p: PaintInput!) { updatePaint(id: $id, input: $p) { id } }",
            json!({ "id": paint, "p": { "name": "Val paint", "kind": "LINEAR_GRADIENT", "stops": stops1 } }),
            "bad_stops",
        ),
        (
            "updateShopItem: negative price",
            "mutation($id: String!, $b: String!) { updateShopItem(id: $id, input: { name: \"Val item\", priceCents: -1, badgeIds: [$b] }) { id } }",
            json!({ "id": item, "b": badge }),
            "bad_price",
        ),
        (
            "updateShopItem: ends before it starts",
            "mutation($id: String!, $b: String!) { updateShopItem(id: $id, input: { name: \"Val item\", priceCents: 100, activeFrom: \"2030-02-01T00:00:00Z\", activeTo: \"2030-01-01T00:00:00Z\", badgeIds: [$b] }) { id } }",
            json!({ "id": item, "b": badge }),
            "bad_time",
        ),
        (
            "updateShopItem: empty name",
            "mutation($id: String!, $b: String!) { updateShopItem(id: $id, input: { name: \"\", priceCents: 100, badgeIds: [$b] }) { id } }",
            json!({ "id": item, "b": badge }),
            "bad_name",
        ),
        (
            "createCodeBatch: bad prefix",
            "mutation($i: String!) { createCodeBatch(input: { name: \"Val\", prefix: \"!!\", count: 1, itemId: $i }) { codes } }",
            json!({ "i": item }),
            "bad_prefix",
        ),
        (
            "createCodeBatch: no codes",
            "mutation($i: String!) { createCodeBatch(input: { name: \"Val\", prefix: \"VAL\", count: 0, itemId: $i }) { codes } }",
            json!({ "i": item }),
            "bad_count",
        ),
        (
            "createCodeBatch: zero uses",
            "mutation($i: String!) { createCodeBatch(input: { name: \"Val\", prefix: \"VAL\", count: 1, usesPerCode: 0, itemId: $i }) { codes } }",
            json!({ "i": item }),
            "bad_uses",
        ),
        (
            "createCodeBatch: expired",
            "mutation($i: String!) { createCodeBatch(input: { name: \"Val\", prefix: \"VAL\", count: 1, expiresAt: \"2020-01-01T00:00:00Z\", itemId: $i }) { codes } }",
            json!({ "i": item }),
            "bad_time",
        ),
        (
            "grantCosmetic: empty reason",
            "mutation($a: String!, $c: String!) { grantCosmetic(input: { accountId: $a, kind: BADGE, cosmeticId: $c, reason: \"\" }) { cosmeticId } }",
            json!({ "a": f.tid(), "c": badge }),
            "bad_reason",
        ),
        (
            "userCosmetics: 101 users",
            "query($u: [String!]!) { userCosmetics(platform: TWITCH, userIds: $u) { platformUserId } }",
            json!({ "u": many }),
            "too_many",
        ),
        (
            "markNotificationsRead: 101 IDs",
            "mutation($ids: [String!]) { markNotificationsRead(ids: $ids) }",
            json!({ "ids": many }),
            "too_many",
        ),
        (
            "reviewQueue: search over 100",
            "query($s: String!) { reviewQueue(filter: { search: $s }) { total } }",
            json!({ "s": long }),
            "bad_search",
        ),
        (
            "rejectEmote: empty reason",
            "mutation($id: String!) { rejectEmote(id: $id, reason: \"\") { id } }",
            json!({ "id": pending }),
            "bad_reason",
        ),
        (
            "addEmoteNote: empty body",
            "mutation($id: String!) { addEmoteNote(emoteId: $id, body: \"\") { id } }",
            json!({ "id": pending }),
            "bad_note",
        ),
        (
            "updateEmote: eleven tags",
            "mutation($id: String!, $t: [String!]) { updateEmote(id: $id, input: { tags: $t }) { id } }",
            json!({ "id": approved, "t": eleven }),
            "too_many_tags",
        ),
        (
            "updateEmote: a bad tag",
            "mutation($id: String!) { updateEmote(id: $id, input: { tags: [\"no spaces!\"] }) { id } }",
            json!({ "id": approved }),
            "bad_tag",
        ),
        (
            "placeRestriction: ends in the past",
            "mutation($a: String!) { placeRestriction(accountId: $a, input: { kind: UPLOADS, until: \"2020-01-01T00:00:00Z\", reason: \"Val\" }) { uploadsBlocked } }",
            json!({ "a": f.tid() }),
            "bad_time",
        ),
        (
            "updateAccountEditor: a set of another account",
            "mutation($a: String!, $s: [String!]) { updateAccountEditor(accountId: $a, permissions: [EMOTES], setIds: $s) { allSets } }",
            json!({ "a": f.tid(), "s": [ids::to_text(ids::new_id())] }),
            "bad_set",
        ),
        (
            "createRole: empty name",
            "mutation { createRole(input: { name: \"\", position: 3, permissions: [] }) { id } }",
            json!({}),
            "bad_name",
        ),
    ];

    let mut failures = Vec::new();
    for (what, query, variables, expected) in &cases {
        let r = w.gql(os, query, variables.clone()).await;
        let code = error_code(&r);
        if code != *expected {
            failures.push(format!("{what}: expected {expected}, got {code:?}: {r}"));
        }
    }
    // Nothing above changed the rows.
    let r = w
        .gql(
            os,
            "query($id: String!) { shopItem(id: $id) { name priceCents } }",
            json!({ "id": item }),
        )
        .await;
    if r["data"]["shopItem"]["priceCents"] != 100 || r["data"]["shopItem"]["name"] != "Val item" {
        failures.push(format!("the shop item changed: {r}"));
    }
    let r = w
        .gql(
            os,
            "query($id: String!) { emote(id: $id) { tags } }",
            json!({ "id": approved }),
        )
        .await;
    if r["data"]["emote"]["tags"] != json!([]) {
        failures.push(format!("the emote's tags changed: {r}"));
    }
    eprintln!("{} bad inputs sent", cases.len());
    w.close().await;
    assert!(
        failures.is_empty(),
        "{} problems:\n{}",
        failures.len(),
        failures.join("\n")
    );
}
