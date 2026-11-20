//! The cyex.tv database (ADR 0002): one Postgres, its own pool limits, forward-only migrations.
//!
//! Each module holds the queries for one area. They take `&mut PgConnection`, so the api can run
//! several in one transaction (a change, its audit row and its outbox event commit together).
//! Functions that need more than one statement open a nested transaction (a savepoint) themselves.

pub mod accounts;
pub mod announcements;
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
pub mod prefs;
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
///
/// Afterwards invalid indexes are repaired (see [`repair_invalid_indexes`]): a build interrupted
/// halfway leaves an invalid index that `CREATE INDEX IF NOT EXISTS` would never rebuild.
pub async fn migrate(pool: &PgPool) -> Result<(), sqlx::migrate::MigrateError> {
    let mut conn = pool.acquire().await?.detach();
    conn.execute("SET statement_timeout = 0").await?;
    let mut done = MIGRATOR.run(&mut conn).await;
    if done.is_ok() {
        done = repair_invalid_indexes(&mut conn)
            .await
            .map(|_| ())
            .map_err(Into::into);
    }
    let _ = sqlx::Connection::close(conn).await;
    done
}

const INVALID_INDEXES: &str = "SELECT c.relname::text, pg_get_indexdef(i.indexrelid)
    FROM pg_index i
    JOIN pg_class c ON c.oid = i.indexrelid
    JOIN pg_namespace n ON n.oid = c.relnamespace
    WHERE NOT i.indisvalid AND n.nspname = current_schema()
      AND NOT EXISTS (SELECT 1 FROM pg_constraint k WHERE k.conindid = i.indexrelid)
    ORDER BY c.relname";

/// Drops every invalid index (left by an interrupted `CREATE INDEX CONCURRENTLY`) and builds it
/// again from its own definition, concurrently. A rebuild that fails again (for example a unique
/// index over duplicate rows) is an error, so the service refuses to start instead of running
/// slow on a missing index. An advisory lock keeps two starting instances from dropping an index
/// the other is rebuilding. Returns the names rebuilt.
pub async fn repair_invalid_indexes(
    conn: &mut sqlx::PgConnection,
) -> Result<Vec<String>, sqlx::Error> {
    const LOCK: i64 = 0x6379_6578_7476_6978; // "cyextvix"
    sqlx::query("SELECT pg_advisory_lock($1)")
        .bind(LOCK)
        .execute(&mut *conn)
        .await?;
    let done = rebuild_invalid(&mut *conn).await;
    let _ = sqlx::query("SELECT pg_advisory_unlock($1)")
        .bind(LOCK)
        .execute(&mut *conn)
        .await;
    done
}

async fn rebuild_invalid(conn: &mut sqlx::PgConnection) -> Result<Vec<String>, sqlx::Error> {
    let broken: Vec<(String, String)> = sqlx::query_as(INVALID_INDEXES)
        .fetch_all(&mut *conn)
        .await?;
    let mut rebuilt = Vec::new();
    for (name, def) in broken {
        // pg_get_indexdef gives `CREATE [UNIQUE] INDEX name ON ...`.
        let Some(at) = def.find("INDEX ") else {
            continue;
        };
        let create = format!(
            "{}INDEX CONCURRENTLY IF NOT EXISTS {}",
            &def[..at],
            &def[at + "INDEX ".len()..]
        );
        let quoted = name.replace('"', "\"\"");
        conn.execute(format!("DROP INDEX CONCURRENTLY IF EXISTS \"{quoted}\"").as_str())
            .await?;
        conn.execute(create.as_str()).await?;
        rebuilt.push(name);
    }
    Ok(rebuilt)
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
