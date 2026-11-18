//! Error helpers the api modules share: the 400, 409 and 429 answers and the database error.

use axum::http::StatusCode;
use cyex_tv_db::DbError;
use cyex_tv_shared::error::ApiError;

use crate::catalog::db_error;

pub fn bad(code: &'static str, message: &str) -> ApiError {
    ApiError::new(StatusCode::BAD_REQUEST, code, message)
}

pub fn conflict(code: &'static str, message: &str) -> ApiError {
    ApiError::new(StatusCode::CONFLICT, code, message)
}

/// `429 rate_limited` with what to tell the person.
pub fn rate_limited(message: &str) -> ApiError {
    ApiError::new(StatusCode::TOO_MANY_REQUESTS, "rate_limited", message)
}

pub fn sqlx_error(err: sqlx::Error) -> ApiError {
    db_error(DbError::Sqlx(err))
}
