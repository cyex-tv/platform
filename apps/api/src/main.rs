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
    cyex_tv_db::migrate(&db).await?;

    let trust_cloudflare = matches!(
        std::env::var("CYEX_TV_TRUST_CLOUDFLARE").as_deref(),
        Ok("true" | "1")
    );
    // Every limit can be set from the environment (`CYEX_TV_LIMIT_*`, see `.env.example`).
    let rates = cyex_tv_api::limits::Rates::from_env(|name| std::env::var(name).ok())?;
    // The media limits and size ladder too; the worker reads the same variables.
    cyex_tv_media::config::MediaConfig::init_from_env()?;
    let mut state = AppState::new(db, auth)
        .with_limits(cyex_tv_api::limits::Limits::new(rates, trust_cloudflare));
    // Chat-client reads shared for a few seconds per instance (`read_cache`); 0 turns it off.
    let read_cache_ms = match std::env::var("CYEX_TV_READ_CACHE_MS") {
        Ok(raw) => raw
            .trim()
            .parse::<u64>()
            .map_err(|_| "CYEX_TV_READ_CACHE_MS must be a whole number of milliseconds")?,
        Err(_) => cyex_tv_api::read_cache::DEFAULT_TTL.as_millis() as u64,
    };
    state = state.with_read_cache(std::time::Duration::from_millis(read_cache_ms));
    if let Ok(cdn) = std::env::var("CYEX_TV_CDN_URL") {
        state = state.with_cdn_url(&cdn);
    }
    match cyex_tv_storage::S3Config::from_source(|name| std::env::var(name).ok())? {
        Some(s3) => state = state.with_store(cyex_tv_storage::Store::s3(s3)?),
        None => tracing::warn!("CYEX_TV_S3_ENDPOINT not set; uploads answer 503"),
    }
    let paddle = cyex_tv_api::paddle::PaddleConfig::from_source(|name| std::env::var(name).ok())?;
    if paddle.api_key.is_none() {
        tracing::warn!(
            "CYEX_TV_PADDLE_API_KEY not set; the shop lists items but checkout is unavailable"
        );
    }
    if paddle.webhook_secret.is_none() {
        tracing::warn!("CYEX_TV_PADDLE_WEBHOOK_SECRET not set; the Paddle webhook answers 503");
    }
    tracing::info!(env = paddle.env.as_str(), "paddle");
    state = state.with_paddle(cyex_tv_api::paddle::Paddle::new(paddle)?);
    // Editors can't ask for a transfer any more (ADR 0014, amendment 2026/11/22): requests still
    // open from before are withdrawn and both sides told. Finds nothing once done; a failure is
    // logged and retried at the next start.
    match cyex_tv_api::transfers::withdraw_open(&state).await {
        Ok(0) => {}
        Ok(n) => tracing::info!(withdrawn = n, "open emote transfer requests withdrawn"),
        Err(err) => tracing::warn!(error = ?err, "couldn't withdraw open emote transfer requests"),
    }
    // The GraphQL `events` subscription reads the outbox through its own feeder.
    let (stop_tx, mut stop_rx) = tokio::sync::watch::channel(false);
    let feeder = tokio::spawn(cyex_tv_hub::feeder::run(
        state.db.clone(),
        state.hub.clone(),
        async move {
            let _ = stop_rx.wait_for(|stopping| *stopping).await;
        },
    ));
    if let Some(addr) = cyex_tv_shared::metrics::bind_from(|name| std::env::var(name).ok())? {
        let scraped = state.clone();
        let extra: cyex_tv_shared::metrics::Extra = Arc::new(move || {
            let state = scraped.clone();
            Box::pin(async move { cyex_tv_api::scrape_gauges(&state).await })
        });
        tokio::spawn(cyex_tv_shared::metrics::serve(
            addr,
            state.metrics.clone(),
            Some(extra),
            shutdown::signal(),
        ));
    }
    // Developer API key usage (ADR 0033), counted in memory and written every minute.
    let usage = state.clone();
    tokio::spawn(async move {
        let mut every = tokio::time::interval(std::time::Duration::from_secs(60));
        loop {
            every.tick().await;
            if let Err(err) = usage.key_usage.flush(&usage.db).await {
                tracing::warn!(error = %err, "writing api key usage failed");
            }
        }
    });
    let flush_at_exit = state.clone();
    let app = cyex_tv_api::app(state);
    let listener = tokio::net::TcpListener::bind(cfg.bind).await?;
    tracing::info!(addr = %cfg.bind, "api listening");
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown::signal())
    .await?;
    let _ = stop_tx.send(true);
    if let Err(err) = flush_at_exit.key_usage.flush(&flush_at_exit.db).await {
        tracing::warn!(error = %err, "writing api key usage failed");
    }
    feeder.await??;
    Ok(())
}
