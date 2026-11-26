//! What every cyex.tv service shares: config, errors, logging, shutdown, permission names and the
//! public API types.

pub mod config;
pub mod cosmetics;
pub mod developers;
pub mod error;
pub mod error_codes;
pub mod limit_errors;
pub mod logging;
pub mod metrics;
pub mod model;
pub mod permissions;
pub mod public_limits;
pub mod ratelimit;
pub mod shop;
pub mod shutdown;
pub mod site;
#[cfg(feature = "sqlx")]
mod sql;
pub mod text;
pub mod upload;
pub mod webhooks;
