//! The cyex.tv database (ADR 0002): one Postgres, its own pool limits, forward-only migrations.
//!
//! Each module holds the queries for one area. They take `&mut PgConnection`, so the api can run
//! several in one transaction (a change, its audit row and its outbox event commit together).
//! Functions that need more than one statement open a nested transaction (a savepoint) themselves.

pub mod accounts;
pub mod announcements;
pub mod app_auth;
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
pub mod idempotency;
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
pub mod review;
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

use std::collections::BTreeSet;

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
/// Afterwards invalid indexes our migrations create are repaired (see [`repair_invalid_indexes`]):
/// a build interrupted halfway leaves an invalid index that `CREATE INDEX IF NOT EXISTS` would
/// never rebuild. Invalid indexes the migrations don't name are logged and left alone.
pub async fn migrate(pool: &PgPool) -> Result<(), sqlx::migrate::MigrateError> {
    let mut conn = pool.acquire().await?.detach();
    conn.execute("SET statement_timeout = 0").await?;
    let mut done = MIGRATOR.run(&mut conn).await;
    if done.is_ok() {
        done = repair_invalid_indexes(&mut conn)
            .await
            .map(|repair| {
                for name in &repair.rebuilt {
                    tracing::info!(index = %name, "rebuilt an invalid index");
                }
                for name in &repair.left_alone {
                    tracing::warn!(
                        index = %name,
                        "invalid index our migrations don't create; left alone"
                    );
                }
            })
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

/// What [`repair_invalid_indexes`] did.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct IndexRepair {
    /// Invalid indexes our migrations create, dropped and built again.
    pub rebuilt: Vec<String>,
    /// Invalid indexes the migrations don't name (made by hand, an extension or another tool).
    /// Never touched, only reported.
    pub left_alone: Vec<String>,
}

/// The names of the indexes the migrations create (`CREATE [UNIQUE] INDEX [CONCURRENTLY]
/// [IF NOT EXISTS] <name>`), read from the embedded migration files. Every index the migrations
/// make is named, so this is the whole list.
pub fn migration_index_names() -> BTreeSet<String> {
    MIGRATOR
        .iter()
        .flat_map(|migration| index_names_in(&migration.sql))
        .collect()
}

/// The index names `sql` creates. `--` comments are skipped; unquoted names are folded to lower
/// case as Postgres does, quoted ones keep their case; a schema prefix is dropped.
fn index_names_in(sql: &str) -> Vec<String> {
    let code: Vec<&str> = sql
        .lines()
        .map(|line| line.split_once("--").map_or(line, |(code, _)| code))
        .collect();
    let code = code.join("\n");
    let words: Vec<&str> = code
        .split(|c: char| c.is_whitespace() || c == ';' || c == '(')
        .filter(|w| !w.is_empty())
        .collect();
    let is = |at: usize, word: &str| words.get(at).is_some_and(|w| w.eq_ignore_ascii_case(word));
    let mut names = Vec::new();
    for at in 0..words.len() {
        if !is(at, "create") {
            continue;
        }
        let mut next = at + 1;
        if is(next, "unique") {
            next += 1;
        }
        if !is(next, "index") {
            continue;
        }
        next += 1;
        if is(next, "concurrently") {
            next += 1;
        }
        if is(next, "if") && is(next + 1, "not") && is(next + 2, "exists") {
            next += 3;
        }
        let Some(name) = words.get(next) else {
            continue;
        };
        if name.eq_ignore_ascii_case("on") {
            continue; // unnamed: Postgres picks the name
        }
        let name = name.rsplit('.').next().unwrap_or(name);
        names.push(
            match name.strip_prefix('"').and_then(|n| n.strip_suffix('"')) {
                Some(quoted) => quoted.to_owned(),
                None => name.to_ascii_lowercase(),
            },
        );
    }
    names
}

/// Drops every invalid index our migrations create (left by an interrupted `CREATE INDEX
/// CONCURRENTLY`) and builds it again from its own definition, concurrently. A rebuild that fails
/// again (for example a unique index over duplicate rows) is an error, so the service refuses to
/// start instead of running slow on a missing index. Invalid indexes the migrations don't name
/// belong to someone else: they are reported in [`IndexRepair::left_alone`] and never dropped.
/// An advisory lock keeps two starting instances from dropping an index the other is rebuilding.
pub async fn repair_invalid_indexes(
    conn: &mut sqlx::PgConnection,
) -> Result<IndexRepair, sqlx::Error> {
    const LOCK: i64 = 0x6379_6578_7476_6978; // "cyextvix"
    sqlx::query("SELECT pg_advisory_lock($1)")
        .bind(LOCK)
        .execute(&mut *conn)
        .await?;
    let done = rebuild_invalid(&mut *conn, &migration_index_names()).await;
    let _ = sqlx::query("SELECT pg_advisory_unlock($1)")
        .bind(LOCK)
        .execute(&mut *conn)
        .await;
    done
}

async fn rebuild_invalid(
    conn: &mut sqlx::PgConnection,
    ours: &BTreeSet<String>,
) -> Result<IndexRepair, sqlx::Error> {
    let broken: Vec<(String, String)> = sqlx::query_as(INVALID_INDEXES)
        .fetch_all(&mut *conn)
        .await?;
    let mut repair = IndexRepair::default();
    for (name, def) in broken {
        if !ours.contains(&name) {
            repair.left_alone.push(name);
            continue;
        }
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
        repair.rebuilt.push(name);
    }
    Ok(repair)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn index_names_are_read_from_sql() {
        let sql = r#"-- CREATE INDEX not_this ON t (a);
            CREATE INDEX a_b ON t (b);
            create unique index if not exists Uniq_C on t (c);
            CREATE INDEX CONCURRENTLY IF NOT EXISTS "Quoted" ON t (d);
            CREATE INDEX public.with_schema ON t (e);
            CREATE INDEX ON t (f);
            CREATE TABLE x (id int); -- CREATE UNIQUE INDEX nor_this ON x (id)"#;
        assert_eq!(
            index_names_in(sql),
            ["a_b", "uniq_c", "Quoted", "with_schema"]
        );
    }

    #[test]
    fn the_migrations_name_their_indexes() {
        let ours = migration_index_names();
        assert!(ours.contains("emotes_file_id"));
        assert!(ours.contains("emote_set_members_emote_added"));
        assert!(ours.contains("app_refresh_tokens_session"));
        assert!(!ours.contains("on"));
    }
}
