//! The events service: the event WebSocket at `/v1/events` (ADR 0005), fed from the outbox.

use std::net::SocketAddr;

use axum::routing::get;
use axum::{Json, Router};
use cyex_tv_events::{EventsState, router};
use cyex_tv_hub::Hub;
use cyex_tv_shared::config::Config;
use cyex_tv_shared::model::Health;
use cyex_tv_shared::{logging, shutdown};
use tokio::sync::watch;
use tower_http::trace::TraceLayer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cfg = Config::from_env(SocketAddr::from(([0, 0, 0, 0], 8081)))?;
    logging::init(&cfg.log_filter, cfg.log_format);

    let db = cyex_tv_db::connect(&cfg).await?;
    let hub = Hub::new();
    let (stop_tx, stop_rx) = watch::channel(false);
    let feeder = tokio::spawn(cyex_tv_hub::feeder::run(db.clone(), hub.clone(), {
        let mut rx = stop_rx.clone();
        async move {
            let _ = rx.wait_for(|stopping| *stopping).await;
        }
    }));

    let app = Router::new()
        .route("/health", get(health))
        .merge(router(
            EventsState::new(db, hub, stop_rx).trusting_cloudflare(matches!(
                std::env::var("CYEX_TV_TRUST_CLOUDFLARE").as_deref(),
                Ok("true" | "1")
            )),
        ))
        .layer(TraceLayer::new_for_http());
    let listener = tokio::net::TcpListener::bind(cfg.bind).await?;
    tracing::info!(addr = %cfg.bind, "events listening");
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(async move {
        shutdown::signal().await;
        // Tell every connection to reconnect, then let them close.
        let _ = stop_tx.send(true);
    })
    .await?;
    feeder.await??;
    Ok(())
}

async fn health() -> Json<Health> {
    Json(Health {
        status: "ok".to_owned(),
    })
}
