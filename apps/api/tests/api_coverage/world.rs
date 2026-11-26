//! The shared harness of this test binary: a real router (and, for the socket tests, a real
//! server on 127.0.0.1) over a fresh Postgres, an in-memory object store, the owner configured,
//! and every rate limit far out of the way, so a sweep of hundreds of calls isn't cut short.

use std::io::Cursor;
use std::sync::Arc;

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use chrono::Duration;
use cyex_tv_api::limits::{Limits, Rates};
use cyex_tv_api::state::AppState;
use cyex_tv_auth::{Auth, AuthConfig, tokens};
use cyex_tv_db::accounts::{self, Profile};
use cyex_tv_db::sessions::{self, SessionKind};
use cyex_tv_db::{channels, ids};
use cyex_tv_shared::model::Platform;
use cyex_tv_shared::ratelimit::Rate;
use cyex_tv_storage::Store;
use http_body_util::BodyExt;
use image::{ImageFormat, Rgba, RgbaImage};
use serde_json::{Value, json};
use tower::ServiceExt;
use uuid::Uuid;

use crate::common::TestDb;

/// The site the website is served from in these tests (`CYEX_TV_SITE_URL`).
pub const SITE: &str = "http://localhost:5173";
/// The owner's Twitch ID here (`CYEX_TV_OWNER_IDS`); a made-up number.
pub const OWNER: &str = "7001";

pub struct User {
    pub id: Uuid,
    /// A site session (the cookie), as the website holds it.
    pub cookie: String,
    pub twitch_id: String,
    pub login: String,
}

impl User {
    pub fn tid(&self) -> String {
        ids::to_text(self.id)
    }
}

/// Who sends a request.
#[derive(Clone, Copy)]
pub enum As<'a> {
    Nobody,
    /// The cookie and the site's `Origin`, as the website sends it.
    Site(&'a User),
    /// A raw cookie value with the site's `Origin`.
    Cookie(&'a str),
    /// `Authorization: Bearer`, for a console token (ADR 0028).
    Bearer(&'a str),
}

pub struct Reply {
    pub status: StatusCode,
    pub headers: axum::http::HeaderMap,
    pub body: Value,
}

pub struct World {
    pub db: TestDb,
    pub app: Router,
    pub state: AppState,
}

/// Every limit at a million a minute (and `CF-Connecting-IP` trusted, see `authorize`): these tests count calls, they don't test limits.
pub fn open_rates() -> Rates {
    let r = Rate::per_minute(1_000_000);
    Rates {
        auth: r,
        gql: r,
        gql_account: r,
        upload: r,
        reads: r,
        socket: r,
        editors: r,
        cosmetics: r,
        checkout: r,
        redeem: r,
        set_clones: r,
        data_export: r,
        account_search: r,
        quick_search: r,
        heavy_reads: r,
        delete_account: r,
        console_tokens: r,
        emote_transfers: r,
        personal_lists: r,
        prefs: r,
        api_key: r,
        api_key_account: r,
        uploads_per_hour: 1_000_000,
        pending_emotes: 1_000_000,
        set_capacity_max: 1000,
        sockets_per_ip: 1_000_000,
    }
}

pub fn error_code(r: &Value) -> &str {
    r["errors"][0]["extensions"]["code"]
        .as_str()
        .unwrap_or_default()
}

/// A PNG of `w` x `h`; `seed` makes the bytes differ.
pub fn png(w: u32, h: u32, seed: u8) -> Vec<u8> {
    let mut out = Cursor::new(Vec::new());
    RgbaImage::from_fn(w, h, |x, y| {
        Rgba([(x % 256) as u8, (y % 256) as u8, seed, 255])
    })
    .write_to(&mut out, ImageFormat::Png)
    .unwrap();
    out.into_inner()
}

const BOUNDARY: &str = "cyextvapicoverageboundary";

/// A multipart body: text fields, then `file` when given.
pub fn form(fields: &[(&str, &str)], file: Option<&[u8]>) -> Vec<u8> {
    let mut body = Vec::new();
    for (name, value) in fields {
        body.extend_from_slice(
            format!(
                "--{BOUNDARY}\r\nContent-Disposition: form-data; name=\"{name}\"\r\n\r\n{value}\r\n"
            )
            .as_bytes(),
        );
    }
    if let Some(file) = file {
        body.extend_from_slice(
            format!(
                "--{BOUNDARY}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"art.png\"\r\nContent-Type: image/png\r\n\r\n"
            )
            .as_bytes(),
        );
        body.extend_from_slice(file);
        body.extend_from_slice(b"\r\n");
    }
    body.extend_from_slice(format!("--{BOUNDARY}--\r\n").as_bytes());
    body
}

impl World {
    pub async fn new() -> Option<Self> {
        Self::with_config(&[]).await
    }

    /// With extra auth settings (`CYEX_TV_TWITCH_CLIENT_ID` and so on).
    pub async fn with_config(extra: &[(&str, &str)]) -> Option<Self> {
        let db = TestDb::new().await?;
        let extra: Vec<(String, String)> = extra
            .iter()
            .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
            .collect();
        let cfg = AuthConfig::from_source(|key| match key {
            "CYEX_TV_SITE_URL" => Some(SITE.to_owned()),
            "CYEX_TV_OWNER_IDS" => Some(format!("twitch:{OWNER}")),
            _ => extra.iter().find(|(k, _)| k == key).map(|(_, v)| v.clone()),
        })
        .unwrap();
        let state = AppState::new(db.pool.clone(), Arc::new(Auth::new(cfg).unwrap()))
            .with_limits(Limits::new(open_rates(), true))
            .with_store(Store::memory())
            .with_cdn_url("https://cdn.example.test");
        Some(Self {
            app: cyex_tv_api::app(state.clone()),
            state,
            db,
        })
    }

    pub async fn session(&self, account: Uuid, kind: SessionKind) -> String {
        let mut conn = self.db.conn().await;
        let token = tokens::random_token();
        sessions::create_with(
            &mut conn,
            &tokens::hash(&token),
            account,
            kind,
            Duration::days(1),
            Some("Mozilla/5.0 (X11; Linux x86_64; rv:133.0) Gecko/20100101 Firefox/133.0"),
        )
        .await
        .unwrap();
        token
    }

    /// A Twitch user `user<id>` with a channel and a site session.
    pub async fn user(&self, twitch_id: &str) -> User {
        let login = format!("user{twitch_id}");
        let id = {
            let mut conn = self.db.conn().await;
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
            signed.account.id
        };
        let cookie = self.session(id, SessionKind::Site).await;
        User {
            id,
            cookie,
            twitch_id: twitch_id.to_owned(),
            login,
        }
    }

    fn authorize(
        mut request: axum::http::request::Builder,
        who: As<'_>,
    ) -> axum::http::request::Builder {
        // A known client address, as Cloudflare sends it (`redeemCode` refuses unknown ones).
        request = request.header("cf-connecting-ip", "203.0.113.7");
        match who {
            As::Nobody => {}
            As::Site(user) => {
                request = request
                    .header("cookie", format!("cyex_tv_session={}", user.cookie))
                    .header("origin", SITE);
            }
            As::Cookie(cookie) => {
                request = request
                    .header("cookie", format!("cyex_tv_session={cookie}"))
                    .header("origin", SITE);
            }
            As::Bearer(token) => {
                request = request.header("authorization", format!("Bearer {token}"));
            }
        }
        request
    }

    pub async fn send(&self, request: Request<Body>) -> Reply {
        let response = self.app.clone().oneshot(request).await.unwrap();
        let status = response.status();
        let headers = response.headers().clone();
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        Reply {
            status,
            headers,
            body: serde_json::from_slice(&bytes).unwrap_or(Value::Null),
        }
    }

    pub async fn gql(&self, who: As<'_>, query: &str, variables: Value) -> Value {
        let request = Self::authorize(
            Request::post("/v1/gql").header("content-type", "application/json"),
            who,
        )
        .body(Body::from(
            json!({ "query": query, "variables": variables }).to_string(),
        ))
        .unwrap();
        let reply = self.send(request).await;
        assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
        reply.body
    }

    pub async fn get(&self, who: As<'_>, uri: &str) -> Reply {
        let request = Self::authorize(Request::get(uri), who)
            .body(Body::empty())
            .unwrap();
        self.send(request).await
    }

    pub async fn post_form(&self, who: As<'_>, uri: &str, body: Vec<u8>) -> Reply {
        let request = Self::authorize(
            Request::post(uri).header(
                "content-type",
                format!("multipart/form-data; boundary={BOUNDARY}"),
            ),
            who,
        )
        .body(Body::from(body))
        .unwrap();
        self.send(request).await
    }

    pub async fn post_empty(&self, who: As<'_>, uri: &str) -> Reply {
        let request = Self::authorize(Request::post(uri), who)
            .body(Body::empty())
            .unwrap();
        self.send(request).await
    }

    /// What the worker does when it has made a cosmetic picture's files.
    pub async fn finish_picture(&self, image_id: &str) {
        let mut conn = self.db.conn().await;
        cyex_tv_db::cosmetics::set_image_variants(
            &mut conn,
            ids::from_text(image_id).unwrap(),
            &json!([
                { "name": "1x.webp", "format": "webp", "scale": 1, "width": 18, "height": 18, "size": 100, "is_static": false },
                { "name": "2x.webp", "format": "webp", "scale": 2, "width": 36, "height": 36, "size": 200, "is_static": false },
                { "name": "4x.webp", "format": "webp", "scale": 4, "width": 72, "height": 72, "size": 400, "is_static": false }
            ]),
        )
        .await
        .unwrap();
    }

    pub async fn close(self) {
        self.db.close().await;
    }
}
