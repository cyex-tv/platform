//! The worker: turns uploads into emote files (`emote.transcode`), deletes taken-down files and
//! purges the CDN (`files.delete`), and runs the hourly clean-up.

use std::net::SocketAddr;
use std::time::Duration;

use cyex_tv_shared::config::Config;
use cyex_tv_shared::{logging, shutdown};
use cyex_tv_storage::{S3Config, Store};
use cyex_tv_worker::{Ctx, Handlers, Limits};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // The worker serves no HTTP; the bind address is unused.
    let cfg = Config::from_env(SocketAddr::from(([127, 0, 0, 1], 0)))?;
    logging::init(&cfg.log_filter, cfg.log_format);

    let db = cyex_tv_db::connect(&cfg).await?;
    let store = match S3Config::from_source(|name| std::env::var(name).ok())? {
        Some(s3) => Some(Store::s3(s3)?),
        None => {
            tracing::warn!("CYEX_TV_S3_ENDPOINT not set; jobs that need storage will fail");
            None
        }
    };
    let registry = cyex_tv_shared::metrics::Registry::new();
    if let Some(addr) = cyex_tv_shared::metrics::bind_from(|name| std::env::var(name).ok())? {
        let db = db.clone();
        let extra: cyex_tv_shared::metrics::Extra = std::sync::Arc::new(move || {
            let db = db.clone();
            Box::pin(async move { cyex_tv_worker::queue_metrics(&db).await })
        });
        tokio::spawn(cyex_tv_shared::metrics::serve(
            addr,
            registry.clone(),
            Some(extra),
            shutdown::signal(),
        ));
    }
    let handlers = Handlers::default()
        .on(
            cyex_tv_worker::transcode::KIND,
            cyex_tv_worker::transcode::run,
        )
        .on(cyex_tv_worker::purge::KIND, cyex_tv_worker::purge::run)
        .with_metrics(registry);
    let purge = cyex_tv_worker::purge::Purge::from_env(|name| std::env::var(name).ok())?;
    if purge.is_none() {
        tracing::warn!("CYEX_TV_CLOUDFLARE_PURGE_TOKEN not set; takedowns won't purge the CDN");
    }
    tracing::info!(kinds = ?handlers.kinds(), "worker started");
    cyex_tv_worker::run(
        Ctx { db, store, purge },
        handlers,
        Limits::default(),
        Duration::from_secs(3600),
        shutdown::signal(),
    )
    .await?;
    Ok(())
}
