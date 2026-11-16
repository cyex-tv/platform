//! The events service. Phase 1: health check only. The SSE stream arrives in Phase 5 (ADR 0005).

use std::net::SocketAddr;

use axum::routing::get;
use axum::{Json, Router};
use cyex_tv_shared::config::Config;
use cyex_tv_shared::model::Health;
use cyex_tv_shared::{logging, shutdown};
use tower_http::trace::TraceLayer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cfg = Config::from_env(SocketAddr::from(([0, 0, 0, 0], 8081)))?;
    logging::init(&cfg.log_filter, cfg.log_format);

    let app = Router::new().route("/health", get(health)).layer(TraceLayer::new_for_http());
    let listener = tokio::net::TcpListener::bind(cfg.bind).await?;
    tracing::info!(addr = %cfg.bind, "events listening");
    axum::serve(listener, app).with_graceful_shutdown(shutdown::signal()).await?;
    Ok(())
}

async fn health() -> Json<Health> {
    Json(Health { status: "ok".to_owned() })
}
