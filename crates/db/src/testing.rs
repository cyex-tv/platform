//! For tests (feature `testing`): a fresh, migrated database per test, so tests can run in parallel and see only their own rows.
//! They run when `CYEX_TV_TEST_DATABASE_URL` is set (CI sets it) and skip otherwise, except in
//! CI (`CI=true`, which GitHub Actions sets in every job): there a missing URL panics, so database
//! tests can't silently skip. The role in that URL needs `CREATEDB`.

use std::str::FromStr;

use sqlx::postgres::{PgConnectOptions, PgPool, PgPoolOptions};
use sqlx::{ConnectOptions, Connection, Executor, PgConnection};

pub struct TestDb {
    pub pool: PgPool,
    name: String,
    admin: PgConnectOptions,
}

impl TestDb {
    /// `None` when no test database is configured. Panics instead in CI.
    pub async fn new() -> Option<Self> {
        let Some(url) = test_url(
            std::env::var("CYEX_TV_TEST_DATABASE_URL").ok(),
            std::env::var("CI").ok(),
        ) else {
            eprintln!("CYEX_TV_TEST_DATABASE_URL not set; skipping");
            return None;
        };
        let admin = PgConnectOptions::from_str(&url).expect("test database URL");
        let name = format!(
            "cyextv_test_{}",
            crate::ids::to_text(crate::ids::new_id()).to_lowercase()
        );
        let mut conn = admin.connect().await.expect("connect to the test server");
        conn.execute(format!(r#"CREATE DATABASE "{name}""#).as_str())
            .await
            .expect("create the test database");
        conn.close().await.ok();

        let pool = PgPoolOptions::new()
            .max_connections(8)
            .connect_with(admin.clone().database(&name))
            .await
            .expect("connect to the test database");
        crate::MIGRATOR.run(&pool).await.expect("run migrations");
        Some(Self { pool, name, admin })
    }

    pub async fn conn(&self) -> sqlx::pool::PoolConnection<sqlx::Postgres> {
        self.pool.acquire().await.unwrap()
    }

    /// Drops the database. A test that panics leaves its database behind; that's fine for CI,
    /// and locally `cyextv_test_*` databases can be dropped by hand.
    pub async fn close(self) {
        self.pool.close().await;
        let mut conn: PgConnection = self.admin.connect().await.unwrap();
        conn.execute(format!(r#"DROP DATABASE IF EXISTS "{}" WITH (FORCE)"#, self.name).as_str())
            .await
            .unwrap();
    }
}

/// The test database URL, if any. In CI (`CI` is `true` or `1`) a missing or empty URL is a
/// failure, not a skip.
fn test_url(url: Option<String>, ci: Option<String>) -> Option<String> {
    let url = url.filter(|u| !u.trim().is_empty());
    let in_ci = ci.is_some_and(|v| matches!(v.trim().to_ascii_lowercase().as_str(), "true" | "1"));
    assert!(
        url.is_some() || !in_ci,
        "CYEX_TV_TEST_DATABASE_URL isn't set in CI: database tests would skip. Set it for the job          that runs them."
    );
    url
}

#[cfg(test)]
mod tests {
    use super::test_url;

    #[test]
    fn database_tests_skip_only_outside_ci() {
        assert_eq!(test_url(None, None), None);
        assert_eq!(test_url(None, Some("false".into())), None);
        assert_eq!(
            test_url(Some("postgres://x".into()), Some("true".into())).as_deref(),
            Some("postgres://x")
        );
        for ci in ["true", "1", "TRUE"] {
            let missing = std::panic::catch_unwind(|| test_url(None, Some(ci.into())));
            assert!(missing.is_err(), "{ci}");
            let empty = std::panic::catch_unwind(|| test_url(Some(" ".into()), Some(ci.into())));
            assert!(empty.is_err(), "{ci}");
        }
    }
}
