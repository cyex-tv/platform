//! Structured logging: JSON in production, readable lines in development.

use tracing_subscriber::EnvFilter;
use tracing_subscriber::fmt;
use tracing_subscriber::prelude::*;

use crate::config::LogFormat;

/// Installs the global subscriber. Call once, at the top of `main`.
pub fn init(filter: &str, format: LogFormat) {
    let filter = EnvFilter::try_new(filter).unwrap_or_else(|_| EnvFilter::new("info"));
    let registry = tracing_subscriber::registry().with(filter);
    match format {
        LogFormat::Json => registry
            .with(fmt::layer().json().with_current_span(true))
            .init(),
        LogFormat::Pretty => registry.with(fmt::layer()).init(),
    }
}
