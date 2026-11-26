//! REST and GraphQL answer the same things the same way (ADR 0008, API review 3.4 and 3.8).
//!
//! The chat-client reads (`GET /v1/sets/{id}`, `/v1/emotes/{id}`, ...) and the GraphQL queries
//! (`set`, `emote`, ...) are built from the same Rust structs (`cyex-tv-shared`), so a field can
//! only be on one side when somebody hides it there (`#[serde(skip)]`, `#[graphql(skip)]`, a
//! hand-written resolver). This test catches that:
//!
//! - **Names** (`naming` below, unit tests, no database): REST is snake_case, GraphQL camelCase,
//!   REST enums are lower case, GraphQL enums SCREAMING_SNAKE. The rules are checked against the
//!   real serde output and the real schema.
//! - **Data** (needs `CYEX_TV_TEST_DATABASE_URL`, CI sets it): for each resource (sets, set
//!   members, emotes, channels, cosmetics, personal sets, limits, meta) a seeded database is read
//!   over REST and over GraphQL, the GraphQL selection being *every field* of the type in the
//!   schema. After mapping names, both sides must hold equal values. A field on one side only, or
//!   a different value, fails the test unless `DIFFERENCES` lists it with a reason.

mod common;

use std::collections::{BTreeMap, BTreeSet};
use std::io::Cursor;
use std::sync::Arc;

use async_graphql::parser::types::{TypeKind, TypeSystemDefinition};
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
use cyex_tv_db::{channels, cosmetics, files, ids, sets};
use cyex_tv_shared::model::{
    AccountSummary, ChannelsLookup, EmoteFile, EmoteFlags, EmoteState, ImageFormat, PersonalLookup,
    Platform, SetEmotesQuery, SetKind, SetQuery, SetsLookup,
};
use cyex_tv_storage::Store;
use http_body_util::BodyExt;
use image::{ImageFormat as Png, Rgba, RgbaImage};
use serde_json::{Value, json};
use tower::ServiceExt;
use uuid::Uuid;

// The differences that are on purpose. One table; every place that tolerates a difference looks
// its key up here, so nothing is allowed that isn't written down with its reason.

struct Difference {
    key: &'static str,
    reason: &'static str,
}

const DIFFERENCES: &[Difference] = &[
    Difference {
        key: "wrapper:list",
        reason: "REST wraps a list in an object (`{ sets }`, `{ channels }`, `{ users }`) so the answer can grow keys; GraphQL returns the bare list. The items are compared.",
    },
    Difference {
        key: "arg:userCosmetics.userIds",
        reason: "`GET /v1/cosmetics/users?ids=` is a comma-separated query string; GraphQL takes a list named `userIds`, like `personalSets`. Same IDs, one name differs beyond case.",
    },
    Difference {
        key: "number:f32",
        reason: "A paint's `at`, `x`, `y`, `radius` are f32: serde_json prints the shortest digits (0.3), GraphQL widens to f64 (0.30000001192...). Compared within 1e-6.",
    },
    Difference {
        key: "viewer",
        reason: "REST reads always answer as signed out (approved emotes only, `account` limits empty), as chat clients and CDNs need; GraphQL answers for the caller. The test reads both signed out.",
    },
    Difference {
        key: "type:EffectiveLimit",
        reason: "`limits.account` lists the signed-in caller's own limits; signed out (REST always) it is empty, so no item is ever compared.",
    },
    Difference {
        key: "only-graphql",
        reason: "Reads that REST doesn't offer on purpose (the chat-client API stays small, ADR 0008): `emotes(ids)`, search, profiles, activity, everything that needs a session.",
    },
    Difference {
        key: "only-rest",
        reason: "Sign-in redirects, the multipart uploads, the webhooks, health checks and the conditional-request machinery (ETag, Cache-Control, Link, RateLimit-*) are HTTP only.",
    },
    Difference {
        key: "errors",
        reason: "REST errors are `{ code, message, details }` with a status; GraphQL's are `errors[].extensions` with the same `code`, `limit` and `current` (ADR 0008, 0032).",
    },
];

fn intentional(key: &str) -> &'static str {
    DIFFERENCES
        .iter()
        .find(|d| d.key == key)
        .unwrap_or_else(|| panic!("`{key}` is not in DIFFERENCES"))
        .reason
}

// Names

mod naming {
    //! The mapping between the two spellings.

    /// `is_static` -> `isStatic`, `per_page` -> `perPage`.
    pub fn snake_to_camel(snake: &str) -> String {
        let mut out = String::new();
        let mut up = false;
        for c in snake.chars() {
            if c == '_' {
                up = true;
            } else if up {
                out.extend(c.to_uppercase());
                up = false;
            } else {
                out.push(c);
            }
        }
        out
    }

    /// `isStatic` -> `is_static`, `perPage` -> `per_page`.
    pub fn camel_to_snake(camel: &str) -> String {
        let mut out = String::new();
        for c in camel.chars() {
            if c.is_uppercase() {
                out.push('_');
                out.extend(c.to_lowercase());
            } else {
                out.push(c);
            }
        }
        out
    }

    /// REST `set_full` or `twitch` -> GraphQL `SET_FULL` or `TWITCH`.
    pub fn enum_to_graphql(rest: &str) -> String {
        rest.to_uppercase()
    }
}

// The schema, read from the SDL the API generates (the introspection query is too costly for the
// complexity limit, and this is what clients generate their types from).

struct Field {
    name: String,
    base: String,
    list: bool,
    args: Vec<(String, bool)>,
}

struct Schema {
    objects: BTreeMap<String, Vec<Field>>,
    enums: BTreeMap<String, Vec<String>>,
}

impl Schema {
    fn load() -> Self {
        let doc = async_graphql::parser::parse_schema(cyex_tv_api::gql::sdl()).expect("the SDL");
        let mut objects = BTreeMap::new();
        let mut enums = BTreeMap::new();
        for def in doc.definitions {
            let TypeSystemDefinition::Type(def) = def else {
                continue;
            };
            let def = def.node;
            let name = def.name.node.to_string();
            match def.kind {
                TypeKind::Object(object) => {
                    let fields = object
                        .fields
                        .into_iter()
                        .map(|f| {
                            let f = f.node;
                            let text = f.ty.node.to_string();
                            Field {
                                name: f.name.node.to_string(),
                                base: text
                                    .trim_matches(|c| c == '[' || c == ']' || c == '!')
                                    .to_owned(),
                                list: text.contains('['),
                                args: f
                                    .arguments
                                    .into_iter()
                                    .map(|a| {
                                        let a = a.node;
                                        (
                                            a.name.node.to_string(),
                                            !a.ty.node.nullable && a.default_value.is_none(),
                                        )
                                    })
                                    .collect(),
                            }
                        })
                        .collect();
                    objects.insert(name, fields);
                }
                TypeKind::Enum(e) => {
                    enums.insert(
                        name,
                        e.values
                            .into_iter()
                            .map(|v| v.node.value.node.to_string())
                            .collect(),
                    );
                }
                _ => {}
            }
        }
        Self { objects, enums }
    }

    fn field(&self, ty: &str, name: &str) -> &Field {
        self.objects[ty]
            .iter()
            .find(|f| f.name == name)
            .unwrap_or_else(|| panic!("{ty}.{name} is not in the schema"))
    }

    /// Every field of `ty`, nested, as a GraphQL selection. A field that needs an argument can't
    /// be selected in general; it must be one the table of differences knows.
    fn selection(
        &self,
        ty: &str,
        stack: &mut Vec<String>,
        reached: &mut BTreeSet<String>,
    ) -> String {
        assert!(
            !stack.iter().any(|t| t == ty),
            "{ty} contains itself ({stack:?}): the conformance selection can't follow it"
        );
        reached.insert(ty.to_owned());
        stack.push(ty.to_owned());
        let mut out = String::new();
        for f in &self.objects[ty] {
            assert!(
                !f.args.iter().any(|(_, required)| *required),
                "{ty}.{} needs arguments; select it by hand and list the difference",
                f.name
            );
            out.push_str(&f.name);
            if self.objects.contains_key(&f.base) {
                out.push_str(&format!(
                    " {{ {} }}",
                    self.selection(&f.base, stack, reached)
                ));
            }
            out.push(' ');
        }
        stack.pop();
        out
    }
}

// Comparing

struct Comparison<'a> {
    schema: &'a Schema,
    seen: BTreeSet<String>,
    diffs: Vec<String>,
}

impl Comparison<'_> {
    fn value(&mut self, base: &str, list: bool, rest: &Value, gql: &Value, path: &str) {
        if list {
            match (rest.as_array(), gql.as_array()) {
                (Some(r), Some(g)) => {
                    if r.len() != g.len() {
                        self.diffs.push(format!(
                            "{path}: REST has {} items, GraphQL {}",
                            r.len(),
                            g.len()
                        ));
                        return;
                    }
                    for (i, (r, g)) in r.iter().zip(g).enumerate() {
                        self.value(base, false, r, g, &format!("{path}[{i}]"));
                    }
                }
                _ => self
                    .diffs
                    .push(format!("{path}: not a list on both sides: {rest} / {gql}")),
            }
        } else if rest.is_null() || gql.is_null() {
            if rest != gql {
                self.diffs
                    .push(format!("{path}: REST {rest}, GraphQL {gql}"));
            }
        } else if self.schema.objects.contains_key(base) {
            self.object(base, rest, gql, path);
        } else if self.schema.enums.contains_key(base) {
            let want = rest.as_str().map(naming::enum_to_graphql);
            if want.as_deref() != gql.as_str() {
                self.diffs
                    .push(format!("{path}: enum REST {rest}, GraphQL {gql}"));
            }
        } else if let (Some(r), Some(g)) = (rest.as_f64(), gql.as_f64()) {
            if (r - g).abs() > 1e-6 {
                self.diffs
                    .push(format!("{path}: REST {rest}, GraphQL {gql}"));
            } else if rest != gql {
                // Same number, different digits: the `number:f32` row.
                let _ = intentional("number:f32");
            }
        } else if rest != gql {
            self.diffs
                .push(format!("{path}: REST {rest}, GraphQL {gql}"));
        }
    }

    fn object(&mut self, ty: &str, rest: &Value, gql: &Value, path: &str) {
        let (Some(rest), Some(gql)) = (rest.as_object(), gql.as_object()) else {
            self.diffs
                .push(format!("{path}: {ty} is not an object on both sides"));
            return;
        };
        self.seen.insert(ty.to_owned());
        let mut mapped = BTreeSet::new();
        for f in &self.schema.objects[ty] {
            let key = naming::camel_to_snake(&f.name);
            mapped.insert(key.clone());
            let at = format!("{path}.{} ({ty}.{})", key, f.name);
            let Some(g) = gql.get(&f.name) else {
                self.diffs
                    .push(format!("{at}: GraphQL answered without it"));
                continue;
            };
            let Some(r) = rest.get(&key) else {
                self.diffs.push(format!("{at}: only GraphQL has it ({g})"));
                continue;
            };
            self.value(&f.base, f.list, r, g, &at);
        }
        for key in rest.keys() {
            if !mapped.contains(key) {
                self.diffs.push(format!(
                    "{path}.{key} ({ty}): only REST has it, GraphQL has no field {}",
                    naming::snake_to_camel(key)
                ));
            }
        }
    }
}

// The world

const OWNER: &str = "900";

struct User {
    id: Uuid,
    token: String,
}

struct World {
    db: TestDb,
    app: Router,
    schema: Schema,
}

impl World {
    async fn new() -> Option<Self> {
        let db = TestDb::new().await?;
        let cfg = AuthConfig::from_source(|name| {
            (name == "CYEX_TV_OWNER_IDS").then(|| format!("twitch:{OWNER}"))
        })
        .unwrap();
        let state = AppState::new(db.pool.clone(), Arc::new(Auth::new(cfg).unwrap()))
            .with_store(Store::memory())
            .with_cdn_url("https://cdn.example.test");
        Some(Self {
            app: cyex_tv_api::app(state),
            db,
            schema: Schema::load(),
        })
    }

    async fn user(&self, twitch_id: &str, login: &str) -> User {
        let mut conn = self.db.conn().await;
        let signed = accounts::sign_in(
            &mut conn,
            Profile {
                platform: Platform::Twitch,
                platform_user_id: twitch_id,
                login,
                display_name: login,
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

    async fn send(&self, request: Request<Body>) -> (StatusCode, Value) {
        let response = self.app.clone().oneshot(request).await.unwrap();
        let status = response.status();
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        (
            status,
            serde_json::from_slice(&bytes).unwrap_or(Value::Null),
        )
    }

    async fn rest_get(&self, path: &str) -> Value {
        let (status, body) = self
            .send(Request::get(path).body(Body::empty()).unwrap())
            .await;
        assert_eq!(status, StatusCode::OK, "GET {path}: {body}");
        body
    }

    async fn rest_post(&self, path: &str, body: Value) -> Value {
        let (status, body) = self
            .send(
                Request::post(path)
                    .header("content-type", "application/json")
                    .body(Body::from(body.to_string()))
                    .unwrap(),
            )
            .await;
        assert_eq!(status, StatusCode::OK, "POST {path}: {body}");
        body
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
        let (status, body) = self
            .send(
                request
                    .body(Body::from(
                        json!({ "query": query, "variables": variables }).to_string(),
                    ))
                    .unwrap(),
            )
            .await;
        assert_eq!(status, StatusCode::OK);
        body
    }

    /// `field_call` (for example `set(id: "1")`) with every field of `root` selected, read signed
    /// out.
    async fn gql_all(&self, field_call: &str, root: &str, reached: &mut BTreeSet<String>) -> Value {
        let selection = self.schema.selection(root, &mut Vec::new(), reached);
        let query = format!("{{ answer: {field_call} {{ {selection} }} }}");
        let answer = self.gql(None, &query, json!({})).await;
        assert!(answer["errors"].is_null(), "{field_call}: {answer}");
        answer["data"]["answer"].clone()
    }
}

/// One resource: what REST answered and what GraphQL answered for every field of `root`.
struct Case {
    name: &'static str,
    root: &'static str,
    list: bool,
    rest: Value,
    gql_call: String,
}

// Seeding

fn png(w: u32, h: u32) -> Vec<u8> {
    let mut out = Cursor::new(Vec::new());
    RgbaImage::from_fn(w, h, |x, y| {
        Rgba([(x % 256) as u8, (y % 256) as u8, 90, 255])
    })
    .write_to(&mut out, Png::Png)
    .unwrap();
    out.into_inner()
}

const BOUNDARY: &str = "cyextvconformanceboundary";

fn form(kind: &str, file: &[u8]) -> Vec<u8> {
    let mut body = format!(
        "--{BOUNDARY}\r\nContent-Disposition: form-data; name=\"kind\"\r\n\r\n{kind}\r\n--{BOUNDARY}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"art.png\"\r\nContent-Type: application/octet-stream\r\n\r\n"
    )
    .into_bytes();
    body.extend_from_slice(file);
    body.extend_from_slice(format!("\r\n--{BOUNDARY}--\r\n").as_bytes());
    body
}

impl World {
    /// An emote with its files (WebP and AVIF at 1x and 2x), tags and flags, approved.
    async fn emote(&self, owner: &User, name: &str, seed: u8, zero_width: bool) -> (Uuid, String) {
        let mut conn = self.db.conn().await;
        let file = files::insert_or_get(
            &mut conn,
            &files::NewFile {
                sha256: &[seed; 32],
                dhash: None,
                mime: "image/png",
                width: 64,
                height: 32,
                frame_count: 1,
                storage_key: &format!("uploads/conformance-{seed}"),
            },
        )
        .await
        .unwrap()
        .0;
        let variants = json!([
            { "name": "1x.webp", "format": "webp", "scale": 1, "width": 64, "height": 32, "size": 900, "is_static": false },
            { "name": "2x.webp", "format": "webp", "scale": 2, "width": 128, "height": 64, "size": 1800, "is_static": false },
            { "name": "1x.avif", "format": "avif", "scale": 1, "width": 64, "height": 32, "size": 500, "is_static": true }
        ]);
        files::set_variants(&mut conn, file.id, &variants)
            .await
            .unwrap();
        let e = emotes::create(
            &mut conn,
            &NewEmote {
                owner_account_id: owner.id,
                file_id: Some(file.id),
                name,
                animated: seed.is_multiple_of(2),
                zero_width,
                ..NewEmote::default()
            },
        )
        .await
        .unwrap();
        emotes::set_tags(
            &mut conn,
            e.id,
            &["conformance".to_owned(), format!("tag{seed}")],
        )
        .await
        .unwrap();
        emotes::set_state(&mut conn, e.id, EmoteState::Approved)
            .await
            .unwrap();
        drop(conn);
        (e.id, self.db.public(e.id).await)
    }

    async fn add(&self, set: Uuid, emote: Uuid, by: &User, name: Option<&str>) {
        let mut conn = self.db.conn().await;
        sets::add_member(
            &mut conn,
            set,
            sets::NewMember {
                emote_id: emote,
                name,
                zero_width: None,
                added_by: Some(by.id),
            },
        )
        .await
        .unwrap();
    }

    async fn upload(&self, owner: &User, kind: &str, side: u32) -> String {
        let (w, h) = if kind == "badge" {
            (side, side)
        } else {
            (side * 4, side)
        };
        let request = Request::post("/v1/cosmetics/images")
            .header(
                "content-type",
                format!("multipart/form-data; boundary={BOUNDARY}"),
            )
            .header(
                "cookie",
                format!("{}={}", cyex_tv_auth::cookies::SESSION, owner.token),
            )
            .header("origin", "https://cyex.tv")
            .body(Body::from(form(kind, &png(w, h))))
            .unwrap();
        let (status, body) = self.send(request).await;
        assert_eq!(status, StatusCode::ACCEPTED, "{body}");
        let id = body["id"].as_str().unwrap().to_owned();
        // What the worker records when it has made the files.
        let mut conn = self.db.conn().await;
        cosmetics::set_image_variants(
            &mut conn,
            ids::from_text(&id).unwrap(),
            &json!([
                { "name": "1x.webp", "format": "webp", "scale": 1, "width": 18, "height": 18, "size": 100, "is_static": false },
                { "name": "2x.webp", "format": "webp", "scale": 2, "width": 36, "height": 36, "size": 200, "is_static": false }
            ]),
        )
        .await
        .unwrap();
        id
    }
}

#[tokio::test]
async fn rest_and_graphql_answer_the_same() {
    let Some(w) = World::new().await else {
        return;
    };
    let staff = w.user(OWNER, "ownerlogin").await;
    let streamer = w.user("100", "streamerlogin").await;
    let chatter = w.user("200", "chatterlogin").await;

    // Emotes with files, tags and flags.
    let (e1, e1_id) = w.emote(&streamer, "Alpha", 1, false).await;
    let (e2, _) = w.emote(&streamer, "Beta", 2, true).await;
    let (e3, _) = w.emote(&chatter, "Gamma", 3, false).await;

    // A channel set of three members; the channel uses it. A delta since the version after the
    // first member: a member added, one renamed in the set, one emote renamed itself.
    let (set, set_id) = {
        let mut conn = w.db.conn().await;
        let set = sets::create(&mut conn, streamer.id, "Main", SetKind::Channel)
            .await
            .unwrap();
        drop(conn);
        (set.id, w.db.public(set.id).await)
    };
    w.add(set, e1, &streamer, Some("AlphaInSet")).await;
    let since = {
        let mut conn = w.db.conn().await;
        sets::get(&mut conn, set).await.unwrap().unwrap().version
    };
    w.add(set, e2, &streamer, None).await;
    w.add(set, e3, &streamer, None).await;
    {
        let mut conn = w.db.conn().await;
        sets::update_member(&mut conn, set, e1, Some("AlphaRenamed"), Some(true))
            .await
            .unwrap();
        emotes::update(
            &mut conn,
            e2,
            &emotes::EmotePatch {
                name: Some("BetaRenamed"),
                ..emotes::EmotePatch::default()
            },
        )
        .await
        .unwrap();
        let channel = channels::upsert(
            &mut conn,
            streamer.id,
            Platform::Twitch,
            "100",
            "streamerlogin",
        )
        .await
        .unwrap();
        channels::set_active_set(&mut conn, channel.id, Some(set))
            .await
            .unwrap();
    }

    // The global set and a personal set.
    {
        let mut conn = w.db.conn().await;
        let global = sets::ensure_global(&mut conn).await.unwrap();
        sets::add_member(
            &mut conn,
            global.id,
            sets::NewMember {
                emote_id: e1,
                name: None,
                zero_width: None,
                added_by: Some(streamer.id),
            },
        )
        .await
        .unwrap();
        let personal = sets::create(&mut conn, chatter.id, "Mine", SetKind::Personal)
            .await
            .unwrap();
        sets::add_member(
            &mut conn,
            personal.id,
            sets::NewMember {
                emote_id: e2,
                name: Some("MineBeta"),
                zero_width: None,
                added_by: Some(chatter.id),
            },
        )
        .await
        .unwrap();
    }

    // Cosmetics: a badge and a gradient paint made by the owner (staff), given to the chatter,
    // who shows both.
    let badge_image = w.upload(&staff, "badge", 90).await;
    let made = w
        .gql(
            Some(&staff),
            "mutation($i: String!) { createBadge(input: { name: \"Founder\", tooltip: \"The first\", imageId: $i }) { id } }",
            json!({ "i": badge_image }),
        )
        .await;
    let badge = made["data"]["createBadge"]["id"]
        .as_str()
        .expect("badge")
        .to_owned();
    let made = w
        .gql(
            Some(&staff),
            "mutation($p: PaintInput!) { createPaint(input: $p) { id } }",
            json!({ "p": {
                "name": "Sunset", "kind": "LINEAR_GRADIENT", "angle": 45, "repeat": false,
                "color": "#ff8800",
                "stops": [{ "at": 0.0, "color": "#ff8800" }, { "at": 0.5, "color": "#8800ffcc" }, { "at": 1.0, "color": "#00ff00" }],
                "shadows": [{ "x": 0.0, "y": 1.0, "radius": 2.5, "color": "#00000080" }]
            } }),
        )
        .await;
    let paint = made["data"]["createPaint"]["id"]
        .as_str()
        .expect("paint")
        .to_owned();
    for (kind, id) in [("BADGE", &badge), ("PAINT", &paint)] {
        let r = w
            .gql(
                Some(&staff),
                "mutation($a: String!, $k: CosmeticKind!, $c: String!) { grantCosmetic(input: { accountId: $a, kind: $k, cosmeticId: $c, reason: \"conformance\" }) { kind } }",
                json!({ "a": ids::to_text(chatter.id), "k": kind, "c": id }),
            )
            .await;
        assert!(r["errors"].is_null(), "{r}");
    }
    let r = w
        .gql(
            Some(&chatter),
            "mutation($b: String, $p: String) { selectCosmetics(badgeId: $b, paintId: $p) { badgeId } }",
            json!({ "b": badge, "p": paint }),
        )
        .await;
    assert!(r["errors"].is_null(), "{r}");

    // Every resource, over REST and over GraphQL.
    let _ = intentional("viewer");
    let cases = vec![
        Case {
            name: "set",
            root: "EmoteSet",
            list: false,
            rest: w.rest_get(&format!("/v1/sets/{set_id}")).await,
            gql_call: format!("set(id: \"{set_id}\")"),
        },
        Case {
            name: "sets lookup",
            root: "EmoteSet",
            list: true,
            rest: w
                .rest_post("/v1/sets/lookup", json!({ "ids": [set_id] }))
                .await["sets"]
                .clone(),
            gql_call: format!("sets(ids: [\"{set_id}\"])"),
        },
        Case {
            name: "global set",
            root: "EmoteSet",
            list: false,
            rest: w.rest_get("/v1/sets/global").await,
            gql_call: "globalSet".to_owned(),
        },
        Case {
            name: "set members, first page",
            root: "SetEmotePage",
            list: false,
            rest: w
                .rest_get(&format!("/v1/sets/{set_id}/emotes?per_page=2"))
                .await,
            gql_call: format!("setEmotes(id: \"{set_id}\", perPage: 2)"),
        },
        Case {
            name: "set changes",
            root: "SetChanges",
            list: false,
            rest: w
                .rest_get(&format!("/v1/sets/{set_id}?since={since}"))
                .await,
            gql_call: format!("setChanges(id: \"{set_id}\", since: {since})"),
        },
        Case {
            name: "set changes, full resync",
            root: "SetChanges",
            list: false,
            rest: w.rest_get(&format!("/v1/sets/{set_id}?since=99999")).await,
            gql_call: format!("setChanges(id: \"{set_id}\", since: 99999)"),
        },
        Case {
            name: "emote",
            root: "Emote",
            list: false,
            rest: w.rest_get(&format!("/v1/emotes/{e1_id}")).await,
            gql_call: format!("emote(id: \"{e1_id}\")"),
        },
        Case {
            name: "channel",
            root: "Channel",
            list: false,
            rest: w.rest_get("/v1/channels/twitch/100").await,
            gql_call: "channel(platform: TWITCH, id: \"100\")".to_owned(),
        },
        Case {
            name: "channels lookup",
            root: "Channel",
            list: true,
            rest: w
                .rest_post(
                    "/v1/channels/lookup",
                    json!({ "platform": "twitch", "ids": ["100"] }),
                )
                .await["channels"]
                .clone(),
            gql_call: "channels(platform: TWITCH, ids: [\"100\"])".to_owned(),
        },
        Case {
            name: "personal sets",
            root: "PersonalSetEntry",
            list: true,
            rest: w
                .rest_post(
                    "/v1/personal-sets/lookup",
                    json!({ "platform": "twitch", "user_ids": ["200"] }),
                )
                .await["sets"]
                .clone(),
            gql_call: "personalSets(platform: TWITCH, userIds: [\"200\"])".to_owned(),
        },
        Case {
            name: "cosmetics catalog",
            root: "CosmeticCatalog",
            list: false,
            rest: w.rest_get("/v1/cosmetics").await,
            gql_call: "cosmetics".to_owned(),
        },
        Case {
            name: "chatters' cosmetics",
            root: "UserCosmetics",
            list: true,
            rest: w
                .rest_get("/v1/cosmetics/users?platform=twitch&ids=200")
                .await["users"]
                .clone(),
            gql_call: "userCosmetics(platform: TWITCH, userIds: [\"200\"])".to_owned(),
        },
        Case {
            name: "limits",
            root: "PublicLimits",
            list: false,
            rest: w.rest_get("/v1/limits").await,
            gql_call: "limits".to_owned(),
        },
        Case {
            name: "meta",
            root: "ApiMeta",
            list: false,
            rest: w.rest_get("/v1/meta").await,
            gql_call: "meta".to_owned(),
        },
    ];
    let _ = (
        intentional("wrapper:list"),
        intentional("only-graphql"),
        intentional("only-rest"),
        intentional("errors"),
    );

    let mut reached = BTreeSet::new();
    let mut cmp = Comparison {
        schema: &w.schema,
        seen: BTreeSet::new(),
        diffs: Vec::new(),
    };
    for case in &cases {
        let gql = w.gql_all(&case.gql_call, case.root, &mut reached).await;
        assert!(!gql.is_null(), "{}: GraphQL answered null", case.name);
        assert!(!case.rest.is_null(), "{}: REST answered null", case.name);
        cmp.value(case.root, case.list, &case.rest, &gql, case.name);
    }
    assert!(
        cmp.diffs.is_empty(),
        "REST and GraphQL differ (add the field on the lacking side, or list a deliberate difference in DIFFERENCES):\n{}",
        cmp.diffs.join("\n")
    );

    // Every type the selections reach held a value at least once, so none was compared only as an
    // empty list.
    let _ = intentional("type:EffectiveLimit");
    let unseen: Vec<_> = reached
        .difference(&cmp.seen)
        .filter(|t| t.as_str() != "EffectiveLimit")
        .collect();
    assert!(unseen.is_empty(), "never compared with a value: {unseen:?}");
    w.db.close().await;
}

// The naming rules, without a database.

#[test]
fn name_mapping_rules() {
    use naming::*;
    for (snake, camel) in [
        ("is_static", "isStatic"),
        ("per_page", "perPage"),
        ("platform_user_id", "platformUserId"),
        ("removed_ids", "removedIds"),
        ("created_at", "createdAt"),
        ("id", "id"),
        ("name", "name"),
    ] {
        assert_eq!(snake_to_camel(snake), camel);
        assert_eq!(camel_to_snake(camel), snake);
    }
    for (rest, graphql) in [
        ("twitch", "TWITCH"),
        ("approved", "APPROVED"),
        ("set_full", "SET_FULL"),
        ("linear_gradient", "LINEAR_GRADIENT"),
    ] {
        assert_eq!(enum_to_graphql(rest), graphql);
    }
}

/// The rules hold for the real types: every key REST serialises for these structs is a field of
/// the GraphQL type of the same name once mapped, and the other way round.
#[test]
fn rest_keys_are_the_graphql_fields_by_the_rules() {
    let schema = Schema::load();
    let file = EmoteFile::with_urls(
        vec![EmoteFile {
            name: "1x.webp".into(),
            format: ImageFormat::Webp,
            scale: 1,
            width: 32,
            height: 32,
            size: 10,
            is_static: true,
            url: String::new(),
            mime: String::new(),
        }],
        "https://cdn.example.test/emotes/ab",
    );
    let samples = [
        ("EmoteFile", serde_json::to_value(&file[0]).unwrap()),
        (
            "EmoteFlags",
            serde_json::to_value(EmoteFlags {
                zero_width: true,
                private: false,
                nsfw: false,
                wide: false,
            })
            .unwrap(),
        ),
        (
            "AccountSummary",
            serde_json::to_value(AccountSummary {
                id: "1".into(),
                display_name: "x".into(),
                avatar: Some("a".into()),
            })
            .unwrap(),
        ),
    ];
    for (ty, value) in samples {
        let rest: BTreeSet<String> = value.as_object().unwrap().keys().cloned().collect();
        let gql: BTreeSet<String> = schema.objects[ty]
            .iter()
            .map(|f| naming::camel_to_snake(&f.name))
            .collect();
        assert_eq!(rest, gql, "{ty}: REST keys against GraphQL fields");
    }
}

/// Request names: a REST query string or body field is the GraphQL argument of the same read
/// (`per_page`/`perPage`, `user_ids`/`userIds`), except the one row in `DIFFERENCES`.
#[test]
fn rest_inputs_are_the_graphql_arguments_by_the_rules() {
    let schema = Schema::load();
    let args = |field: &str| -> BTreeSet<String> {
        schema
            .field("Query", field)
            .args
            .iter()
            .map(|(name, _)| naming::camel_to_snake(name))
            .collect()
    };
    let keys =
        |value: Value| -> BTreeSet<String> { value.as_object().unwrap().keys().cloned().collect() };
    let _ = intentional("arg:userCosmetics.userIds");

    // Every REST input must be an argument of its GraphQL twin.
    let inputs: [(&str, BTreeSet<String>); 5] = [
        (
            "setEmotes",
            keys(serde_json::to_value(SetEmotesQuery::default()).unwrap()),
        ),
        (
            "setChanges",
            keys(serde_json::to_value(SetQuery::default()).unwrap()),
        ),
        (
            "personalSets",
            keys(
                serde_json::to_value(PersonalLookup {
                    platform: Platform::Twitch,
                    user_ids: vec![],
                })
                .unwrap(),
            ),
        ),
        (
            "channels",
            keys(
                serde_json::to_value(ChannelsLookup {
                    platform: Platform::Twitch,
                    ids: vec![],
                })
                .unwrap(),
            ),
        ),
        (
            "sets",
            keys(serde_json::to_value(SetsLookup { ids: vec![] }).unwrap()),
        ),
    ];
    for (field, rest) in inputs {
        let gql = args(field);
        let missing: Vec<_> = rest.difference(&gql).collect();
        assert!(
            missing.is_empty(),
            "{field}: REST inputs {missing:?} are no GraphQL arguments"
        );
    }
}
