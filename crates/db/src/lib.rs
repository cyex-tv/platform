//! The cyex.tv database (ADR 0002): one Postgres, its own pool limits, forward-only migrations.
//!
//! Each module holds the queries for one area. They take `&mut PgConnection`, so the api can run
//! several in one transaction (a change, its audit row and its outbox event commit together).
//! Functions that need more than one statement open a nested transaction (a savepoint) themselves.

pub mod accounts;
pub mod audit;
pub mod blocklist;
pub mod channels;
pub mod codes;
pub mod cosmetics;
pub mod editors;
pub mod emotes;
mod error;
pub mod event_sessions;
pub mod files;
pub mod ids;
pub mod jobs;
pub mod notifications;
pub mod oauth_states;
pub mod outbox;
pub mod personal;
pub mod privacy;
pub mod reads;
pub mod reports;
pub mod roles;
pub mod search;
pub mod sessions;
pub mod sets;
pub mod shop;
pub mod staff;
#[cfg(feature = "testing")]
pub mod testing;
pub mod transfers;

pub use error::{DbError, DbResult};

use cyex_tv_shared::config::Config;
use sqlx::Executor;
use sqlx::migrate::Migrator;
use sqlx::postgres::{PgPool, PgPoolOptions};

/// Migrations in `crates/db/migrations`, embedded at build time. The api service runs them on start.
pub static MIGRATOR: Migrator = sqlx::migrate!("./migrations");

/// Runs `MIGRATOR` on a connection of its own without the pool's statement timeout: an index
/// built `CONCURRENTLY` on a large table takes longer than a request may. The connection is
/// closed afterwards, so the pool never gets it back without its timeout.
pub async fn migrate(pool: &PgPool) -> Result<(), sqlx::migrate::MigrateError> {
    let mut conn = pool.acquire().await?.detach();
    conn.execute("SET statement_timeout = 0").await?;
    let done = MIGRATOR.run(&mut conn).await;
    let _ = sqlx::Connection::close(conn).await;
    done
}

/// Opens the pool and checks that one connection works.
pub async fn connect(cfg: &Config) -> Result<PgPool, sqlx::Error> {
    options(cfg).connect(&cfg.database_url).await
}

/// Builds the pool without connecting. Connections open on first use.
pub fn connect_lazy(cfg: &Config) -> Result<PgPool, sqlx::Error> {
    options(cfg).connect_lazy(&cfg.database_url)
}

/// One round trip, for readiness checks.
pub async fn ping(pool: &PgPool) -> Result<(), sqlx::Error> {
    sqlx::query("SELECT 1").execute(pool).await.map(|_| ())
}

fn options(cfg: &Config) -> PgPoolOptions {
    let statement_timeout_ms = cfg.db_statement_timeout.as_millis();
    PgPoolOptions::new()
        .max_connections(cfg.db_max_connections)
        .acquire_timeout(cfg.db_acquire_timeout)
        .after_connect(move |conn, _meta| {
            Box::pin(async move {
                let sql = format!("SET statement_timeout = {statement_timeout_ms}");
                conn.execute(sql.as_str()).await?;
                Ok(())
            })
        })
}
