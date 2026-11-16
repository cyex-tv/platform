//! The image worker. Phase 1: starts, logs and waits. Jobs arrive in Phase 3.

use std::net::SocketAddr;

use cyex_tv_shared::config::Config;
use cyex_tv_shared::{logging, shutdown};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // The worker serves no HTTP; the bind address is unused for now.
    let cfg = Config::from_env(SocketAddr::from(([127, 0, 0, 1], 0)))?;
    logging::init(&cfg.log_filter, cfg.log_format);

    tracing::info!("worker started; no job types yet");
    shutdown::signal().await;
    Ok(())
}
