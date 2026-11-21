//! A fresh, migrated database per test, so tests can run in parallel and see only their own rows.
//! They run when `CYEX_TV_TEST_DATABASE_URL` is set (CI sets it) and skip otherwise. The role in
//! that URL needs `CREATEDB`.

use std::str::FromStr;

use sqlx::postgres::{PgConnectOptions, PgPool, PgPoolOptions};
use sqlx::{ConnectOptions, Connection, Executor, PgConnection};

pub struct TestDb {
    pub pool: PgPool,
    name: String,
    admin: PgConnectOptions,
}

impl TestDb {
    /// `None` when no test database is configured.
    pub async fn new() -> Option<Self> {
        Self::migrated_before(i64::MAX).await
    }

    /// A database with only the migrations older than `version`, to test what a later one does
    /// to existing rows; [`Self::migrate`] runs the rest.
    #[allow(dead_code)]
    pub async fn before(version: i64) -> Option<Self> {
        Self::migrated_before(version).await
    }

    /// Runs every migration not run yet.
    #[allow(dead_code)]
    pub async fn migrate(&self) {
        cyex_tv_db::MIGRATOR
            .run(&self.pool)
            .await
            .expect("run the remaining migrations");
    }

    async fn migrated_before(version: i64) -> Option<Self> {
        let Ok(url) = std::env::var("CYEX_TV_TEST_DATABASE_URL") else {
            eprintln!("CYEX_TV_TEST_DATABASE_URL not set; skipping");
            return None;
        };
        let admin = PgConnectOptions::from_str(&url).expect("test database URL");
        let name = format!(
            "cyextv_test_{}",
            cyex_tv_db::ids::to_text(cyex_tv_db::ids::new_id()).to_lowercase()
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
        let migrator = sqlx::migrate::Migrator {
            migrations: cyex_tv_db::MIGRATOR
                .migrations
                .iter()
                .filter(|m| m.version < version)
                .cloned()
                .collect::<Vec<_>>()
                .into(),
            ..sqlx::migrate::Migrator::DEFAULT
        };
        migrator.run(&pool).await.expect("run migrations");
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
