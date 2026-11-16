use sqlx::PgPool;

/// What every handler can reach.
#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
}
