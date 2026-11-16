use std::sync::Arc;

use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use cyex_tv_auth::{Auth, Caller};
use cyex_tv_shared::error::ApiError;
use sqlx::PgPool;

/// Where emote files are served, unless `CYEX_TV_CDN_URL` says otherwise.
pub const DEFAULT_CDN_URL: &str = "https://cdn.cyex.tv";

/// What every handler can reach.
#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub auth: Arc<Auth>,
    /// The CDN origin emote `host`s are built on, without a trailing slash.
    pub cdn_url: Arc<str>,
}

impl AppState {
    pub fn new(db: PgPool, auth: Arc<Auth>) -> Self {
        Self {
            db,
            auth,
            cdn_url: DEFAULT_CDN_URL.into(),
        }
    }

    pub fn with_cdn_url(mut self, url: &str) -> Self {
        self.cdn_url = url.trim_end_matches('/').into();
        self
    }
}

/// The signed-in caller, if any. Fails only when a cookie write comes from the wrong `Origin`
/// (`bad_origin`) or the database can't be reached.
pub struct MaybeCaller(pub Option<Caller>);

impl FromRequestParts<AppState> for MaybeCaller {
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, ApiError> {
        state
            .auth
            .caller(&state.db, &parts.method, &parts.headers)
            .await
            .map(MaybeCaller)
            .map_err(ApiError::from)
    }
}
