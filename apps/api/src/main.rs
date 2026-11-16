use std::net::SocketAddr;

use cyex_tv_api::state::AppState;
use cyex_tv_shared::config::Config;
use cyex_tv_shared::{logging, shutdown};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cfg = Config::from_env(SocketAddr::from(([0, 0, 0, 0], 8080)))?;
    logging::init(&cfg.log_filter, cfg.log_format);

    let db = cyex_tv_db::connect(&cfg).await?;
    cyex_tv_db::MIGRATOR.run(&db).await?;

    let app = cyex_tv_api::app(AppState { db });
    let listener = tokio::net::TcpListener::bind(cfg.bind).await?;
    tracing::info!(addr = %cfg.bind, "api listening");
    axum::serve(listener, app).with_graceful_shutdown(shutdown::signal()).await?;
    Ok(())
}
