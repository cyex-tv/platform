//! For tests (feature `testing`): a fresh, migrated database per test, so tests can run in parallel and see only their own rows.
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
        let Ok(url) = std::env::var("CYEX_TV_TEST_DATABASE_URL") else {
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
