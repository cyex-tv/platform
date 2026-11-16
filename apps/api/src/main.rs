use std::net::SocketAddr;
use std::sync::Arc;

use cyex_tv_api::state::AppState;
use cyex_tv_auth::{Auth, AuthConfig};
use cyex_tv_shared::config::Config;
use cyex_tv_shared::{logging, shutdown};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cfg = Config::from_env(SocketAddr::from(([0, 0, 0, 0], 8080)))?;
    logging::init(&cfg.log_filter, cfg.log_format);
    let auth_cfg = AuthConfig::from_source(|name| std::env::var(name).ok())?;
    for (platform, configured) in [
        ("twitch", auth_cfg.twitch.is_some()),
        ("kick", auth_cfg.kick.is_some()),
    ] {
        if !configured {
            tracing::warn!(
                platform,
                "no OAuth client configured; its sign-in answers 503"
            );
        }
    }
    let auth = Arc::new(Auth::new(auth_cfg)?);

    let db = cyex_tv_db::connect(&cfg).await?;
    cyex_tv_db::MIGRATOR.run(&db).await?;

    let mut state = AppState::new(db, auth);
    if let Ok(cdn) = std::env::var("CYEX_TV_CDN_URL") {
        state = state.with_cdn_url(&cdn);
    }
    match cyex_tv_storage::S3Config::from_source(|name| std::env::var(name).ok())? {
        Some(s3) => state = state.with_store(cyex_tv_storage::Store::s3(s3)?),
        None => tracing::warn!("CYEX_TV_S3_ENDPOINT not set; uploads answer 503"),
    }
    let app = cyex_tv_api::app(state);
    let listener = tokio::net::TcpListener::bind(cfg.bind).await?;
    tracing::info!(addr = %cfg.bind, "api listening");
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown::signal())
        .await?;
    Ok(())
}
