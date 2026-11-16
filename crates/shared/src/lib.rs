//! What every cyex.tv service shares: config, errors, logging, shutdown, permission names and the
//! public API types.

pub mod config;
pub mod error;
pub mod logging;
pub mod model;
pub mod permissions;
pub mod shutdown;
#[cfg(feature = "sqlx")]
mod sql;
