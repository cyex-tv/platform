//! The website's side of the contract. Every GraphQL operation the site sends is read from its
//! source (`apps/web/src/lib/queries/*.ts` and `paint-studio/queries.ts`, not copied here), with
//! the fragments it uses, and run against the real schema through the real router: as the site
//! sends it (the cookie and its `Origin`), with seeded data and real variables. A field or an
//! operation the API doesn't have fails validation; a read or a main mutation that answers an
//! error fails too, unless it is listed below with the one error it may give here and why. The
//! REST calls the site makes run the same way: the emote and cosmetic picture uploads, the
//! sign-in redirects and signing out.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::PathBuf;

use axum::http::StatusCode;
use cyex_tv_db::accounts::{self, Profile};
use cyex_tv_db::emotes::{self, NewEmote};
use cyex_tv_db::sessions::SessionKind;
use cyex_tv_db::{ids, shop};
use cyex_tv_shared::model::{EmoteState, Platform};
use serde_json::{Value, json};

use crate::world::{As, OWNER, User, World, error_code, form, png};

/// A parsed document of the site: an operation or a fragment.
struct Doc {
    kind: &'static str,
    name: String,
    text: String,
}

fn web_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../web/src/lib")
}

/// Every `graphql(`...`)` template in the site's query files.
fn site_documents() -> Vec<Doc> {
    let mut docs = Vec::new();
    let mut files: Vec<String> = std::fs::read_dir(web_dir().join("queries"))
        .expect("the queries folder")
        .map(|e| {
            e.expect("a directory entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .filter(|n| n.ends_with(".ts") && n != "index.ts")
        .map(|n| format!("queries/{n}"))
        .collect();
    files.sort();
    files.push("paint-studio/queries.ts".to_owned());
    for file in &files {
        let path = web_dir().join(file);
        let source = std::fs::read_to_string(&path)
            .unwrap_or_else(|err| panic!("read {}: {err}", path.display()));
        let mut rest = source.as_str();
        while let Some(start) = rest.find("graphql(`") {
            let body = &rest[start + "graphql(`".len()..];
            let end = body.find('`').expect("a closing backtick");
            let text = body[..end].trim().to_owned();
            assert!(
                !text.contains("${"),
                "{file}: an interpolated GraphQL document can't be checked"
            );
            let mut words = text.split(|c: char| !(c.is_alphanumeric() || c == '_'));
            let kind = match words.next() {
                Some("query") => "query",
                Some("mutation") => "mutation",
                Some("subscription") => "subscription",
                Some("fragment") => "fragment",
                other => panic!("{file}: unknown document start {other:?}"),
            };
            let name = words
                .find(|w| !w.is_empty())
                .expect("a named document")
                .to_owned();
            docs.push(Doc { kind, name, text });
            rest = &body[end + 1..];
        }
    }
    docs
}

/// `...Name` spreads in a document.
fn spreads(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = text;
    while let Some(i) = rest.find("...") {
        let after = &rest[i + 3..];
        let name: String = after
            .trim_start()
            .chars()
            .take_while(|c| c.is_alphanumeric() || *c == '_')
            .collect();
        // `... on Type` is an inline fragment, not a spread.
        if !name.is_empty() && name != "on" {
            out.push(name);
        }
        rest = after;
    }
    out
}

/// The operation with every fragment it uses, the way the site's client sends it.
fn with_fragments(op: &Doc, fragments: &HashMap<String, String>) -> String {
    let mut needed = BTreeSet::new();
    let mut todo = spreads(&op.text);
    while let Some(name) = todo.pop() {
        if needed.insert(name.clone()) {
            let text = fragments
                .get(&name)
                .unwrap_or_else(|| panic!("{}: fragment {name} isn't defined", op.name));
            todo.extend(spreads(text));
        }
    }
    let mut out = op.text.clone();
    for name in needed {
        out.push_str("\n\n");
        out.push_str(&fragments[&name]);
    }
    out
}

/// Errors that a call may answer here, with why. Everything else must answer without errors.
const EXPECTED: &[(&str, &str, &str)] = &[
    (
        "StartCheckout",
        "checkout_unavailable",
        "no Paddle account in tests; shop_flow covers checkout against a stub",
    ),
    (
        "RefundOrder",
        "refund_unavailable",
        "no Paddle account in tests; shop_flow covers refunds against a stub",
    ),
];

struct Contract<'a> {
    w: &'a World,
    ops: BTreeMap<String, String>,
    run: BTreeSet<String>,
    failures: Vec<String>,
}

impl Contract<'_> {
    /// Runs the site's operation `name` and checks the answer; returns its `data`.
    async fn op(&mut self, who: As<'_>, name: &str, variables: Value) -> Value {
        let Some(query) = self.ops.get(name).cloned() else {
            self.failures
                .push(format!("{name}: not in the site's query files any more"));
            return Value::Null;
        };
        self.run.insert(name.to_owned());
        let r = self.w.gql(who, &query, variables).await;
        if let Some(errors) = r["errors"].as_array() {
            let invalid: Vec<&Value> = errors.iter().filter(|e| e.get("path").is_none()).collect();
            if !invalid.is_empty() {
                self.failures.push(format!(
                    "{name}: the API refuses the document: {}",
                    Value::Array(invalid.into_iter().cloned().collect())
                ));
                return Value::Null;
            }
            let code = error_code(&r);
            let allowed = EXPECTED
                .iter()
                .any(|(op, expected, _)| *op == name && *expected == code);
            if !allowed {
                self.failures
                    .push(format!("{name}: answered an error: {r}"));
            }
        }
        r["data"].clone()
    }

    async fn id(&mut self, who: As<'_>, name: &str, variables: Value, pointer: &str) -> String {
        let data = self.op(who, name, variables).await;
        match data.pointer(pointer).and_then(Value::as_str) {
            Some(id) => id.to_owned(),
            None => {
                self.failures
                    .push(format!("{name}: no {pointer} in the answer: {data}"));
                String::new()
            }
        }
    }

    fn check(&mut self, ok: bool, what: impl Into<String>) {
        if !ok {
            self.failures.push(what.into());
        }
    }
}

async fn emote(w: &World, owner: &User, name: &str, state: EmoteState) -> String {
    let mut conn = w.db.conn().await;
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
    if state != EmoteState::Pending {
        emotes::set_state(&mut conn, e.id, state).await.unwrap();
    }
    drop(conn);
    // As the API shows it (ADR 0027), so answers compare equal.
    w.db.public(e.id).await
}

#[tokio::test]
async fn every_site_operation_runs_against_the_real_api() {
    let Some(w) = World::with_config(&[
        ("CYEX_TV_TWITCH_CLIENT_ID", "twitch-test-client"),
        ("CYEX_TV_TWITCH_CLIENT_SECRET", "twitch-test-secret"),
        ("CYEX_TV_KICK_CLIENT_ID", "kick-test-client"),
        ("CYEX_TV_KICK_CLIENT_SECRET", "kick-test-secret"),
    ])
    .await
    else {
        return;
    };

    // The documents, parsed from the site's files.
    let docs = site_documents();
    assert!(docs.len() > 100, "only {} documents parsed", docs.len());
    let fragments: HashMap<String, String> = docs
        .iter()
        .filter(|d| d.kind == "fragment")
        .map(|d| (d.name.clone(), d.text.clone()))
        .collect();
    let mut ops = BTreeMap::new();
    for d in docs.iter().filter(|d| d.kind != "fragment") {
        let full = with_fragments(d, &fragments);
        assert!(
            ops.insert(d.name.clone(), full).is_none(),
            "two operations named {}",
            d.name
        );
    }

    // The people: the owner (all permissions), an editor of theirs, a staff target, and an
    // account that deletes itself.
    let o = w.user(OWNER).await;
    let f = w.user("7002").await;
    let t = w.user("7003").await;
    let v = w.user("7004").await;
    let (os, fs, ts, vs) = (As::Site(&o), As::Site(&f), As::Site(&t), As::Site(&v));
    {
        let mut conn = w.db.conn().await;
        accounts::link(
            &mut conn,
            o.id,
            Profile {
                platform: Platform::Kick,
                platform_user_id: "7101",
                login: "kickowner",
                display_name: "kickowner",
                avatar: None,
            },
        )
        .await
        .unwrap();
    }
    let e1 = emote(&w, &o, "covPog", EmoteState::Approved).await;
    let e2 = emote(&w, &o, "covKek", EmoteState::Approved).await;
    let e3 = emote(&w, &o, "covGive", EmoteState::Approved).await;
    let e_del = emote(&w, &o, "covGone", EmoteState::Approved).await;
    let e_reject = emote(&w, &o, "covNope", EmoteState::Pending).await;
    let e_report = emote(&w, &t, "covBad", EmoteState::Approved).await;
    let spare_session = w.session(o.id, SessionKind::Site).await;

    let mut c = Contract {
        w: &w,
        ops,
        run: BTreeSet::new(),
        failures: Vec::new(),
    };

    // REST: the uploads, as the site sends them (cookie and Origin).
    let r = w
        .post_form(
            os,
            "/v1/emotes",
            form(
                &[("name", "covUpload"), ("tags", "cov, test")],
                Some(&png(64, 64, 1)),
            ),
        )
        .await;
    c.check(
        r.status.is_success() && r.body["id"].is_string(),
        format!("POST /v1/emotes: {} {}", r.status, r.body),
    );
    let uploaded = r.body["id"].as_str().unwrap_or_default().to_owned();
    let r = w
        .post_form(
            os,
            "/v1/cosmetics/images",
            form(&[("kind", "badge")], Some(&png(72, 72, 2))),
        )
        .await;
    c.check(
        r.status == StatusCode::ACCEPTED && r.body["id"].is_string(),
        format!("POST /v1/cosmetics/images (badge): {} {}", r.status, r.body),
    );
    let badge_image = r.body["id"].as_str().unwrap_or_default().to_owned();
    // The same bytes again: the same picture, as the paint studio expects.
    let again = w
        .post_form(
            os,
            "/v1/cosmetics/images",
            form(&[("kind", "badge")], Some(&png(72, 72, 2))),
        )
        .await;
    c.check(
        again.status.is_success() && again.body["id"] == r.body["id"],
        format!(
            "POST /v1/cosmetics/images again: {} {}",
            again.status, again.body
        ),
    );
    let r = w
        .post_form(
            os,
            "/v1/cosmetics/images",
            form(&[("kind", "paint")], Some(&png(256, 64, 3))),
        )
        .await;
    c.check(
        r.status == StatusCode::ACCEPTED && r.body["id"].is_string(),
        format!("POST /v1/cosmetics/images (paint): {} {}", r.status, r.body),
    );
    let paint_image = r.body["id"].as_str().unwrap_or_default().to_owned();
    if !badge_image.is_empty() {
        w.finish_picture(&badge_image).await;
    }
    if !paint_image.is_empty() {
        w.finish_picture(&paint_image).await;
    }
    // The sign-in links the site builds (`loginUrl`).
    for platform in ["twitch", "kick"] {
        let r = w
            .get(
                As::Nobody,
                &format!("/v1/auth/{platform}/login?return_to=%2Femotes"),
            )
            .await;
        let location = r
            .headers
            .get("location")
            .and_then(|l| l.to_str().ok())
            .unwrap_or_default()
            .to_owned();
        c.check(
            r.status == StatusCode::FOUND && location.starts_with("https://"),
            format!("GET /v1/auth/{platform}/login: {} {location}", r.status),
        );
    }

    // Public and own reads.
    let tid = o.tid();
    c.op(os, "Me", json!({})).await;
    c.op(os, "MyRoles", json!({})).await;
    c.op(os, "MyEmotes", json!({ "page": 1, "perPage": 20 }))
        .await;
    c.op(
        os,
        "Search",
        json!({ "query": "cov", "page": 1, "perPage": 20, "sort": "RELEVANCE" }),
    )
    .await;
    c.op(
        As::Nobody,
        "Search",
        json!({ "query": "", "sort": "TRENDING", "window": "WEEK", "flags": ["ZERO_WIDTH"], "tags": ["cov"], "exact": false, "animated": false }),
    )
    .await;
    c.op(
        os,
        "SearchByOwner",
        json!({ "owner": tid, "page": 1, "perPage": 20, "sort": "NEW" }),
    )
    .await;
    c.op(
        os,
        "FindPeople",
        json!({ "query": "user70", "page": 1, "perPage": 10 }),
    )
    .await;
    let data = c.op(As::Nobody, "Emote", json!({ "id": e1 })).await;
    c.check(data["emote"]["name"] == "covPog", format!("Emote: {data}"));
    c.op(
        As::Nobody,
        "EmoteTrending",
        json!({ "id": e1, "window": "DAY" }),
    )
    .await;
    let login = json!({ "platform": "TWITCH", "login": o.login });
    c.op(As::Nobody, "Channel", login.clone()).await;
    c.op(os, "User", login).await;
    c.op(As::Nobody, "PublicAccount", json!({ "id": tid }))
        .await;
    c.op(As::Nobody, "AccountProfile", json!({ "accountId": tid }))
        .await;
    c.op(As::Nobody, "QuickSearch", json!({ "query": "cov" }))
        .await;

    // Sets.
    let set1 = c
        .id(
            os,
            "CreateSet",
            json!({ "input": { "name": "Cov main", "kind": "CHANNEL" } }),
            "/createSet/id",
        )
        .await;
    c.op(os, "AddSetEmote", json!({ "setId": set1, "emoteId": e1 }))
        .await;
    c.op(
        os,
        "AddSetEmote",
        json!({ "setId": set1, "emoteId": e2, "input": { "name": "covKekw", "zeroWidth": false } }),
    )
    .await;
    c.op(
        os,
        "UpdateSetEmote",
        json!({ "setId": set1, "emoteId": e2, "input": { "zeroWidth": true } }),
    )
    .await;
    c.op(
        os,
        "RemoveSetEmote",
        json!({ "setId": set1, "emoteId": e2 }),
    )
    .await;
    c.op(
        os,
        "UpdateSet",
        json!({ "id": set1, "input": { "name": "Cov renamed", "visibility": "LISTED" } }),
    )
    .await;
    let range = c.op(os, "SetCapacityRange", json!({ "id": set1 })).await;
    assert!(
        range["setCapacityRange"]["max"]
            .as_u64()
            .is_some_and(|m| m >= 1000),
        "{range}"
    );
    let menu = c.op(os, "SetMenu", json!({ "id": set1 })).await;
    assert!(menu["setCapacityRange"]["min"].as_u64().is_some(), "{menu}");
    let set2 = c
        .id(
            os,
            "CloneSet",
            json!({ "id": set1, "name": "Cov copy", "visibility": "LISTED" }),
            "/cloneSet/set/id",
        )
        .await;
    c.op(
        os,
        "UpdateChannel",
        json!({ "platform": "TWITCH", "id": o.twitch_id, "input": { "activeSetId": set1, "allowPersonalEmotes": true } }),
    )
    .await;
    c.op(As::Nobody, "PublicSet", json!({ "id": set1 })).await;
    c.op(os, "EditSet", json!({ "id": set1 })).await;
    c.op(os, "SetEditors", json!({ "id": set1 })).await;
    c.op(
        os,
        "SetEmotes",
        json!({ "id": set1, "query": "a", "page": 1, "perPage": 120 }),
    )
    .await;
    c.op(
        os,
        "SetMembers",
        json!({ "id": set1, "page": 1, "perPage": 200 }),
    )
    .await;
    c.op(os, "MySets", json!({})).await;
    c.op(os, "MySetNames", json!({})).await;
    c.op(os, "MyChannels", json!({})).await;
    // The developer platform (ADR 0033, 0034): a key on the Account tab, a webhook on the channel.
    let key = c
        .id(
            os,
            "CreateApiKey",
            json!({ "name": "cov bot" }),
            "/createApiKey/apiKey/id",
        )
        .await;
    let keys = c.op(os, "MyApiKeys", json!({})).await;
    c.check(
        keys["myApiKeys"][0]["id"] == key.as_str() && keys["myApiKeys"][0]["prefix"].is_string(),
        format!("MyApiKeys: {keys}"),
    );
    c.op(os, "RevokeApiKey", json!({ "id": key })).await;
    let hook = c
        .id(
            os,
            "CreateWebhook",
            json!({ "platform": "TWITCH", "id": o.twitch_id, "input": { "url": "https://hooks.example.com/cyex", "events": ["SET_EMOTE_ADDED"] } }),
            "/createWebhook/webhook/id",
        )
        .await;
    c.op(
        os,
        "ChannelWebhooks",
        json!({ "platform": "TWITCH", "id": o.twitch_id }),
    )
    .await;
    c.op(
        os,
        "UpdateWebhook",
        json!({ "id": hook, "input": { "enabled": false, "events": ["SET_EMOTE_ADDED", "EMOTE_UPDATED"] } }),
    )
    .await;
    c.op(os, "TestWebhook", json!({ "id": hook })).await;
    c.op(os, "WebhookDeliveries", json!({ "id": hook })).await;
    c.op(os, "RotateWebhookSecret", json!({ "id": hook })).await;
    c.op(os, "DeleteWebhook", json!({ "id": hook })).await;
    c.op(os, "ProfileMine", json!({})).await;
    c.op(As::Nobody, "AccountSets", json!({ "accountId": tid }))
        .await;
    c.op(
        As::Nobody,
        "EmoteChannels",
        json!({ "id": e1, "limit": 10, "offset": 0 }),
    )
    .await;
    c.op(
        As::Nobody,
        "AccountActivity",
        json!({ "accountId": tid, "limit": 20, "filter": { "groups": null } }),
    )
    .await;
    c.op(
        As::Nobody,
        "EmoteActivity",
        json!({ "emoteId": e1, "limit": 20 }),
    )
    .await;
    c.op(
        As::Nobody,
        "ActivityTopActors",
        json!({ "accountId": tid, "filter": { "groups": null }, "limit": 5 }),
    )
    .await;
    c.op(
        As::Nobody,
        "ActivityTopActors",
        json!({ "emoteId": e1, "limit": 5 }),
    )
    .await;

    // Account editors, and the owner giving an emote to an account that trusts them with uploads
    // (editors can't ask for a transfer: ADR 0014, amendment 2026/11/22). ADR 0038: the owner here
    // holds `emotes.transfer` (the site owner holds every permission), so the move still happens.
    c.op(
        os,
        "AddAccountEditor",
        json!({ "input": { "accountId": f.tid(), "permissions": ["EMOTES", "SETS", "PROFILE"] } }),
    )
    .await;
    c.op(
        os,
        "UpdateAccountEditor",
        json!({ "accountId": f.tid(), "permissions": ["EMOTES", "SETS", "CHANNELS", "PROFILE", "EMOTE_UPLOAD"] }),
    )
    .await;
    c.op(os, "Editors", json!({})).await;
    c.op(os, "AccountEditors", json!({ "accountId": tid }))
        .await;
    c.op(fs, "EditorOf", json!({})).await;
    c.op(fs, "ProfileEdited", json!({})).await;
    c.op(fs, "EditableChannels", json!({})).await;
    c.op(
        os,
        "UpdateEmote",
        json!({ "id": e1, "input": { "tags": ["cov", "pog"], "zeroWidth": false } }),
    )
    .await;
    c.op(
        fs,
        "AddAccountEditor",
        json!({ "input": { "accountId": o.tid(), "permissions": ["EMOTE_UPLOAD"] } }),
    )
    .await;
    let data = c
        .op(
            os,
            "TransferEmote",
            json!({ "id": e3, "toAccountId": f.tid() }),
        )
        .await;
    c.check(
        data["transferEmote"]["owner"]["id"] == c.w.db.public(f.id).await.as_str(),
        format!("TransferEmote: the owner's emote moves at once: {data}"),
    );

    // Cosmetics (staff), and picking them.
    c.op(os, "CosmeticImage", json!({ "id": badge_image }))
        .await;
    // Its original is stored: making its files again is queued.
    let data = c
        .op(os, "RemakeCosmeticImage", json!({ "id": badge_image }))
        .await;
    c.check(
        data["remakeCosmeticImage"]["id"] == badge_image.as_str(),
        format!("RemakeCosmeticImage: the same picture: {data}"),
    );
    c.op(os, "StudioCosmeticImage", json!({ "id": paint_image }))
        .await;
    let badge = c
        .id(
            os,
            "CreateBadge",
            json!({ "input": { "name": "Cov badge", "tooltip": "Cov", "imageId": badge_image } }),
            "/createBadge/id",
        )
        .await;
    c.op(
        os,
        "UpdateBadge",
        json!({ "id": badge, "input": { "tooltip": "Cov badge" } }),
    )
    .await;
    let gradient = |name: &str| {
        json!({
            "name": name, "kind": "LINEAR_GRADIENT", "angle": 90, "repeat": false,
            "color": "#ff8800",
            "stops": [{ "at": 0.0, "color": "#ff8800" }, { "at": 1.0, "color": "#8800ff" }],
            "shadows": [{ "x": 0.0, "y": 1.0, "radius": 2.0, "color": "#00000080" }]
        })
    };
    let paint = c
        .id(
            os,
            "StudioCreatePaint",
            json!({ "input": gradient("Cov paint") }),
            "/createPaint/id",
        )
        .await;
    c.op(
        os,
        "StudioUpdatePaint",
        json!({ "id": paint, "input": gradient("Cov paint two") }),
    )
    .await;
    let image_paint = c
        .id(
            os,
            "StudioCreatePaint",
            json!({ "input": { "name": "Cov image paint", "kind": "IMAGE", "imageId": paint_image } }),
            "/createPaint/id",
        )
        .await;
    c.op(os, "StudioPaints", json!({})).await;
    c.op(As::Nobody, "Cosmetics", json!({})).await;
    for (kind, id) in [("BADGE", &badge), ("PAINT", &paint)] {
        c.op(
            os,
            "GrantCosmetic",
            json!({ "input": { "accountId": tid, "kind": kind, "cosmeticId": id, "reason": "contract test" } }),
        )
        .await;
    }
    c.op(os, "MyCosmetics", json!({})).await;
    c.op(fs, "MyCosmetics", json!({ "accountId": tid })).await;
    let data = c
        .op(
            os,
            "SelectCosmetics",
            json!({ "badgeId": badge, "paintId": paint }),
        )
        .await;
    c.check(
        data["selectCosmetics"]["badgeId"] == badge.as_str(),
        format!("SelectCosmetics: {data}"),
    );
    c.op(
        fs,
        "SelectCosmetics",
        json!({ "badgeId": badge, "ownerId": tid }),
    )
    .await;
    c.op(
        As::Nobody,
        "UserCosmetics",
        json!({ "platform": "TWITCH", "userIds": [OWNER] }),
    )
    .await;
    c.op(
        As::Nobody,
        "AccountCosmetics",
        json!({ "accountIds": [tid] }),
    )
    .await;
    c.op(
        os,
        "Entitlements",
        json!({ "kind": "BADGE", "cosmeticId": badge, "page": 1, "perPage": 20 }),
    )
    .await;
    c.op(os, "EntitlementGraph", json!({ "accountId": tid }))
        .await;
    c.op(os, "EntitlementTopHolders", json!({ "limit": 5 }))
        .await;
    c.op(
        os,
        "EntitlementHolders",
        json!({ "kind": "BADGE", "id": badge }),
    )
    .await;
    c.op(
        os,
        "RevokeCosmetic",
        json!({ "accountId": tid, "kind": "PAINT", "cosmeticId": paint, "reason": "contract test" }),
    )
    .await;

    // The shop.
    let item_input = |name: &str| json!({ "name": name, "description": "A test item", "priceCents": 500, "currency": "EUR", "badgeIds": [badge], "paintIds": [], "visible": true, "sortOrder": 1 });
    let item = c
        .id(
            os,
            "CreateShopItem",
            json!({ "input": item_input("Cov item") }),
            "/createShopItem/id",
        )
        .await;
    c.op(
        os,
        "UpdateShopItem",
        json!({ "id": item, "input": item_input("Cov item two") }),
    )
    .await;
    c.op(fs, "ShopItems", json!({})).await;
    c.op(os, "ShopItems", json!({ "all": true })).await;
    c.op(fs, "ShopItem", json!({ "id": item })).await;
    c.op(
        os,
        "ShopCounts",
        json!({ "items": true, "orders": true, "codes": true }),
    )
    .await;
    c.op(fs, "StartCheckout", json!({ "itemId": item })).await;
    c.op(
        fs,
        "StartCheckout",
        json!({ "itemId": item, "recipient": { "platform": "TWITCH", "login": t.login } }),
    )
    .await;
    // A paid order, as Paddle's webhook leaves it (shop_flow tests that path).
    let order = if let Some(item_id) = ids::from_text(&item) {
        let mut conn = w.db.conn().await;
        let row = shop::get_item(&mut conn, item_id).await.unwrap().unwrap();
        let bundle = shop::item_bundles(&mut conn, &[item_id])
            .await
            .unwrap()
            .remove(&item_id)
            .unwrap_or_default();
        let order = shop::create_order(&mut conn, f.id, f.id, &row, &bundle)
            .await
            .unwrap();
        shop::mark_paid(&mut conn, order.id, Some(500), Some("EUR"))
            .await
            .unwrap();
        ids::to_text(order.id)
    } else {
        String::new()
    };
    c.op(fs, "Order", json!({ "id": order })).await;
    c.op(fs, "MyOrders", json!({ "page": 1, "perPage": 20 }))
        .await;
    c.op(
        os,
        "AdminOrders",
        json!({ "status": "PAID", "itemId": item, "page": 1, "perPage": 20 }),
    )
    .await;
    c.op(
        os,
        "OrderTopBuyers",
        json!({ "status": "PAID", "itemId": item, "limit": 5 }),
    )
    .await;
    c.op(
        os,
        "RefundOrder",
        json!({ "id": order, "reason": "contract test" }),
    )
    .await;
    let data = c
        .op(
            os,
            "CreateCodeBatch",
            json!({ "input": { "name": "Cov codes", "prefix": "COV", "count": 2, "usesPerCode": 1, "itemId": item } }),
        )
        .await;
    let batch = data["createCodeBatch"]["batch"]["id"]
        .as_str()
        .unwrap_or_default()
        .to_owned();
    let code = data["createCodeBatch"]["codes"][0]
        .as_str()
        .unwrap_or_default()
        .to_owned();
    c.check(
        !batch.is_empty() && !code.is_empty(),
        format!("CreateCodeBatch: {data}"),
    );
    c.op(os, "CodeBatches", json!({ "page": 1, "perPage": 20 }))
        .await;
    c.op(ts, "RedeemCode", json!({ "code": code })).await;
    c.op(os, "DisableCodeBatch", json!({ "id": batch })).await;

    // Review and reports.
    c.op(
        os,
        "ReviewQueue",
        json!({ "page": 1, "perPage": 20, "filter": { "search": "cov", "descending": true } }),
    )
    .await;
    c.op(
        os,
        "ReviewTopUploaders",
        json!({ "filter": { "tab": "PUBLIC", "search": "cov" }, "limit": 5 }),
    )
    .await;
    c.op(
        os,
        "AdminCounts",
        json!({ "review": true, "reports": true }),
    )
    .await;
    let note = c
        .id(
            os,
            "AddEmoteNote",
            json!({ "emoteId": uploaded, "body": "Looks fine." }),
            "/addEmoteNote/id",
        )
        .await;
    c.op(os, "EmoteNotes", json!({ "emoteId": uploaded })).await;
    c.op(os, "DeleteEmoteNote", json!({ "id": note })).await;
    let data = c
        .op(
            os,
            "Approve",
            json!({ "id": uploaded, "visibility": "PUBLIC" }),
        )
        .await;
    c.check(
        data["approveEmote"]["state"] == "APPROVED",
        format!("Approve: {data}"),
    );
    c.op(os, "EmoteHistory", json!({ "emoteId": uploaded }))
        .await;
    c.op(
        os,
        "Reject",
        json!({ "id": e_reject, "reason": "Not for this site." }),
    )
    .await;
    let report = c
        .id(
            fs,
            "ReportEmote",
            json!({ "id": e_report, "reason": "This one is not okay." }),
            "/reportEmote/id",
        )
        .await;
    c.op(os, "ReportQueue", json!({ "page": 1, "perPage": 20 }))
        .await;
    c.op(
        os,
        "ResolveReport",
        json!({ "id": report, "action": "DISMISS", "note": "Fine." }),
    )
    .await;

    // Many at once, possible duplicates, the ready reasons, pasted logins, the same-file check.
    let e_batch = emote(&w, &o, "covBatch", EmoteState::Pending).await;
    let data = c
        .op(
            os,
            "ReviewEmotes",
            json!({ "ids": [e_batch], "verdict": "APPROVE", "visibility": "UNLISTED" }),
        )
        .await;
    c.check(
        data["reviewEmotes"]["done"] == json!([e_batch]),
        format!("ReviewEmotes: {data}"),
    );
    c.op(os, "EmoteDuplicates", json!({ "emoteId": e_batch }))
        .await;
    // ADR 0038: both approvals of a pending upload in one call (the queues), then a decision taken
    // back on an approved one (`onlyPending: false`, the edit dialog).
    let e_approvals = emote(&w, &o, "covApprovals", EmoteState::Pending).await;
    let data = c
        .op(
            os,
            "ReviewApprovals",
            json!({ "ids": [e_approvals], "input": { "publicListing": "APPROVE", "personalUse": "APPROVE" } }),
        )
        .await;
    c.check(
        data["reviewApprovals"]["done"] == json!([e_approvals]),
        format!("ReviewApprovals: {data}"),
    );
    let data = c
        .op(
            os,
            "EmoteReviewApprovals",
            json!({ "ids": [e_approvals], "input": { "publicListing": "DENY", "reason": "Not for the listings.", "onlyPending": false } }),
        )
        .await;
    c.check(
        data["reviewApprovals"]["done"] == json!([e_approvals]),
        format!("EmoteReviewApprovals: {data}"),
    );
    // An undone approval goes back last; flags changed after review (ADR 0024).
    let e_requeue = emote(&w, &o, "covRequeue", EmoteState::Pending).await;
    let data = c
        .op(os, "RequeueEmotes", json!({ "ids": [e_requeue] }))
        .await;
    c.check(
        data["requeueEmotes"]["done"] == json!([e_requeue]),
        format!("RequeueEmotes: {data}"),
    );
    let data = c
        .op(
            os,
            "SetEmoteFlags",
            json!({ "id": e_batch, "input": { "visibility": "PUBLIC", "zeroWidth": true } }),
        )
        .await;
    c.check(
        data["setEmoteFlags"]["visibility"] == "PUBLIC"
            && data["setEmoteFlags"]["flags"]["zeroWidth"] == true,
        format!("SetEmoteFlags: {data}"),
    );
    let data = c.op(os, "ReviewReasons", json!({})).await;
    c.check(
        data["reviewReasons"]
            .as_array()
            .is_some_and(|r| r.len() == 6),
        format!("ReviewReasons: {data}"),
    );
    c.op(
        os,
        "SetReviewReasons",
        json!({ "reasons": ["Blurry", "Not an emote"] }),
    )
    .await;
    let e_report2 = emote(&w, &t, "covWorse", EmoteState::Approved).await;
    let report2 = c
        .id(
            fs,
            "ReportEmote",
            json!({ "id": e_report2, "reason": "This one too." }),
            "/reportEmote/id",
        )
        .await;
    let data = c
        .op(
            os,
            "DismissReports",
            json!({ "ids": [report2], "note": "Fine." }),
        )
        .await;
    c.check(
        data["dismissReports"]["done"] == json!([report2]),
        format!("DismissReports: {data}"),
    );
    let data = c
        .op(
            os,
            "AccountsByLogin",
            json!({ "logins": ["kickowner", "nobody_here"] }),
        )
        .await;
    c.check(
        data["accountsByLogin"][1]["problem"] == "not_found",
        format!("AccountsByLogin: {data}"),
    );
    c.op(
        fs,
        "EmotesWithSameFile",
        json!({ "sha256": "00".repeat(32) }),
    )
    .await;

    // Staff pages about an account.
    let ttid = t.tid();
    c.op(os, "AdminAccount", json!({ "accountId": ttid })).await;
    c.op(
        os,
        "UserDetails",
        json!({ "accountId": ttid, "status": true, "roles": true }),
    )
    .await;
    c.op(
        os,
        "AccountEmotes",
        json!({ "accountId": ttid, "page": 1, "perPage": 20 }),
    )
    .await;
    c.op(os, "AccountReports", json!({ "accountId": ttid }))
        .await;
    c.op(os, "AccountRoles", json!({ "accountId": ttid })).await;
    let decided = c
        .op(os, "AccountPermissions", json!({ "accountId": ttid }))
        .await;
    c.check(
        decided["accountPermissions"].as_array().is_some_and(|l| {
            l.iter()
                .any(|d| d["permission"] == "emotes.upload" && d["granted"] == true)
        }),
        "AccountPermissions: the User role allows emotes.upload",
    );
    let staff_note = c
        .id(
            os,
            "AddStaffNote",
            json!({ "accountId": ttid, "body": "A note from a test." }),
            "/addStaffNote/id",
        )
        .await;
    c.op(os, "StaffNotes", json!({ "accountId": ttid })).await;
    c.op(os, "DeleteStaffNote", json!({ "id": staff_note }))
        .await;
    c.op(os, "AccountSessions", json!({ "accountId": ttid }))
        .await;
    c.op(os, "AccountSubscriptions", json!({ "accountId": ttid }))
        .await;
    c.op(
        os,
        "PlaceRestriction",
        json!({ "accountId": ttid, "input": { "kind": "TICKETS", "reason": "A test." } }),
    )
    .await;
    c.op(os, "AccountRestrictions", json!({ "accountId": ttid }))
        .await;
    c.op(
        os,
        "LiftRestriction",
        json!({ "accountId": ttid, "kind": "TICKETS", "reason": "Done." }),
    )
    .await;

    // Roles.
    let data = c.op(os, "Roles", json!({})).await;
    let perm = data["permissions"][0]
        .as_str()
        .unwrap_or("emotes.review")
        .to_owned();
    c.op(os, "AllRoles", json!({})).await;
    let role = c
        .id(
            os,
            "CreateRole",
            json!({ "input": { "name": "Cov role", "description": "From a test", "position": 5, "permissions": [perm], "public": false } }),
            "/createRole/id",
        )
        .await;
    c.op(
        os,
        "UpdateRole",
        json!({ "id": role, "input": { "description": "", "public": true, "invisible": true, "denies": ["reports.file"] } }),
    )
    .await;
    c.op(os, "LimitDefaults", json!({})).await;
    c.op(
        os,
        "SetRoleLimits",
        json!({ "id": role, "limits": [{ "key": "pending_emotes", "value": 40 }] }),
    )
    .await;
    c.op(os, "RoleLimits", json!({ "id": role })).await;
    c.op(os, "UploadQuota", json!({ "ownerId": null })).await;
    c.op(
        os,
        "AssignRole",
        json!({ "accountId": ttid, "roleId": role }),
    )
    .await;
    c.op(
        os,
        "UnassignRole",
        json!({ "accountId": ttid, "roleId": role }),
    )
    .await;
    c.op(os, "DeleteRole", json!({ "id": role })).await;
    c.op(
        os,
        "Audit",
        json!({ "targetKind": "account", "targetId": ttid, "limit": 20 }),
    )
    .await;
    c.op(os, "Audit", json!({ "limit": 20 })).await;
    c.op(
        os,
        "AuditTopPeople",
        json!({ "by": "TARGET", "targetKind": "account", "limit": 5 }),
    )
    .await;

    // The site announcement.
    let announcement = c
        .id(
            os,
            "CreateAnnouncement",
            json!({ "input": { "kind": "CAMPAIGN", "text": "Contract test", "linkUrl": "/shop", "linkLabel": "Shop" } }),
            "/createAnnouncement/id",
        )
        .await;
    c.op(
        os,
        "UpdateAnnouncement",
        json!({ "id": announcement, "input": { "kind": "INFO", "text": "Contract test two" } }),
    )
    .await;
    c.op(As::Nobody, "ActiveAnnouncement", json!({})).await;
    c.op(As::Nobody, "PublicLimits", json!({})).await;
    c.op(os, "Announcements", json!({})).await;
    c.op(os, "EndAnnouncement", json!({ "id": announcement }))
        .await;

    // Notification preferences.
    c.op(fs, "NotificationPrefs", json!({})).await;
    c.op(
        fs,
        "SetNotificationPrefs",
        json!({ "prefs": [{ "kind": "emote.approved", "notify": true, "toast": false }] }),
    )
    .await;

    // The account hub.
    c.op(fs, "MyNotifications", json!({})).await;
    c.op(fs, "UnreadCount", json!({})).await;
    c.op(fs, "MarkRead", json!({})).await;
    let data = c.op(os, "MySessions", json!({})).await;
    let other = data["mySessions"]
        .as_array()
        .and_then(|s| s.iter().find(|s| s["current"] == false))
        .and_then(|s| s["id"].as_str())
        .unwrap_or_default()
        .to_owned();
    c.op(os, "RevokeSession", json!({ "id": other })).await;
    c.op(os, "RevokeOtherSessions", json!({})).await;
    c.op(os, "ExportMyData", json!({})).await;
    c.op(os, "SetDisplayPlatform", json!({ "platform": "KICK" }))
        .await;
    c.op(os, "Unlink", json!({ "platform": "KICK" })).await;
    c.op(os, "PendingDeletion", json!({})).await;

    // Personal lists: hide for me and favourites, as the tile menu, the emote page, Settings →
    // Hidden, the Favourites tab and the searches send them.
    c.op(os, "FavouriteEmote", json!({ "id": e2 })).await;
    c.op(os, "HideEmote", json!({ "id": e_report })).await;
    c.op(os, "HideCreator", json!({ "accountId": t.tid() }))
        .await;
    let data = c.op(os, "PersonalLists", json!({})).await;
    let t_public = c.w.db.public(t.id).await;
    c.check(
        data["favouriteEmoteIds"][0] == json!(e2)
            && data["hiddenEmotes"][0]["emoteId"] == json!(e_report)
            && data["hiddenCreators"][0]["accountId"] == json!(t_public),
        format!("PersonalLists: {data}"),
    );
    let data = c.op(os, "HiddenLists", json!({})).await;
    c.check(
        data["hiddenEmotes"][0]["emote"]["name"] == "covBad",
        format!("HiddenLists: {data}"),
    );
    let data = c
        .op(os, "MyFavourites", json!({ "query": "kek", "perPage": 5 }))
        .await;
    c.check(
        data["myFavourites"]["total"] == 1,
        format!("MyFavourites: {data}"),
    );
    c.op(os, "UnhideCreator", json!({ "accountId": t.tid() }))
        .await;
    c.op(os, "UnhideEmote", json!({ "id": e_report })).await;
    c.op(os, "UnfavouriteEmote", json!({ "id": e2 })).await;

    // Cleaning up, as the site does it.
    c.op(os, "DeleteEmote", json!({ "id": e_del })).await;
    c.op(os, "DeleteSet", json!({ "id": set2 })).await;
    c.op(
        os,
        "RemoveSetEditor",
        json!({ "setId": set1, "accountId": f.tid() }),
    )
    .await;
    c.op(os, "RemoveAccountEditor", json!({ "accountId": f.tid() }))
        .await;
    c.op(os, "DeleteShopItem", json!({ "id": item })).await;
    c.op(os, "DeletePaint", json!({ "id": image_paint })).await;
    c.op(os, "DeleteBadge", json!({ "id": badge })).await;

    // Deleting an account, and undoing it from a restorable site session.
    c.op(vs, "DeleteAccount", json!({ "confirm": v.login }))
        .await;
    let restorable = w.session(v.id, SessionKind::Site).await;
    let data = c
        .op(As::Cookie(&restorable), "PendingDeletion", json!({}))
        .await;
    c.check(
        data["pendingDeletion"]["eraseAfter"].is_string(),
        format!("PendingDeletion after deleting: {data}"),
    );
    c.op(As::Cookie(&restorable), "RestoreAccount", json!({}))
        .await;

    // Signing out (REST), then everywhere (GraphQL), last.
    let r = w
        .post_empty(As::Cookie(&spare_session), "/v1/auth/logout")
        .await;
    c.check(
        r.status.is_success(),
        format!("POST /v1/auth/logout: {} {}", r.status, r.body),
    );
    // The console's API token (`cyextv.token()`, ADR 0028): a Bearer token that reads `me`.
    let r = w.post_empty(ts, "/v1/auth/token/export").await;
    c.check(
        r.status == StatusCode::OK && r.body["token"].is_string(),
        format!("POST /v1/auth/token/export: {} {}", r.status, r.body),
    );
    let console = r.body["token"].as_str().unwrap_or_default().to_owned();
    let me = w
        .gql(As::Bearer(&console), "{ me { id } }", json!({}))
        .await;
    c.check(
        me["errors"].is_null(),
        format!("the console token doesn't read me: {me}"),
    );
    // The Account tab shows its expiry (ADR 0029), never the token.
    let data = c.op(ts, "MyConsoleToken", json!({})).await;
    c.check(
        data["myConsoleToken"]["id"] == r.body["session_id"]
            && data["myConsoleToken"]["expiresAt"] == r.body["expires_at"]
            && !data.to_string().contains(&console),
        format!("MyConsoleToken: {data}"),
    );

    c.op(ts, "SignOutEverywhere", json!({})).await;
    let r = w.gql(ts, "{ me { id } }", json!({})).await;
    c.check(
        error_code(&r) == "signed_out",
        format!("SignOutEverywhere left the session working: {r}"),
    );
    let r = w
        .gql(As::Bearer(&console), "{ me { id } }", json!({}))
        .await;
    c.check(
        error_code(&r) == "signed_out",
        format!("SignOutEverywhere left the console token working: {r}"),
    );

    // Every operation of the site ran.
    let not_run: Vec<&String> = c.ops.keys().filter(|k| !c.run.contains(*k)).collect();
    if !not_run.is_empty() {
        c.failures.push(format!(
            "operations of the site this test doesn't run yet (add them above): {not_run:?}"
        ));
    }
    eprintln!("{} site operations run", c.run.len());
    let failures = std::mem::take(&mut c.failures);
    w.close().await;
    assert!(
        failures.is_empty(),
        "{} problems:\n{}",
        failures.len(),
        failures.join("\n")
    );
}
