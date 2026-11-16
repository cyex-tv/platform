//! Service config, read from `CYEX_TV_*` environment variables (see `.env.example`).

use std::fmt;
use std::net::SocketAddr;
use std::str::FromStr;
use std::time::Duration;

#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum ConfigError {
    #[error("{0} is required")]
    Missing(&'static str),
    #[error("{name} has an invalid value: {reason}")]
    Invalid { name: &'static str, reason: String },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LogFormat {
    Json,
    Pretty,
}

#[derive(Clone)]
pub struct Config {
    pub bind: SocketAddr,
    pub database_url: String,
    pub db_max_connections: u32,
    pub db_statement_timeout: Duration,
    pub db_acquire_timeout: Duration,
    pub log_filter: String,
    pub log_format: LogFormat,
}

/// Never prints the database URL: it holds the password.
impl fmt::Debug for Config {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Config")
            .field("bind", &self.bind)
            .field("database_url", &"<redacted>")
            .field("db_max_connections", &self.db_max_connections)
            .field("db_statement_timeout", &self.db_statement_timeout)
            .field("db_acquire_timeout", &self.db_acquire_timeout)
            .field("log_filter", &self.log_filter)
            .field("log_format", &self.log_format)
            .finish()
    }
}

impl Config {
    /// Reads the process environment, after loading a `.env` file if there is one.
    pub fn from_env(default_bind: SocketAddr) -> Result<Self, ConfigError> {
        let _ = dotenvy::dotenv();
        Self::from_source(|name| std::env::var(name).ok(), default_bind)
    }

    /// Reads from any lookup, so tests don't touch the process environment.
    pub fn from_source(
        get: impl Fn(&str) -> Option<String>,
        default_bind: SocketAddr,
    ) -> Result<Self, ConfigError> {
        let bind = parse_or(&get, "CYEX_TV_BIND", default_bind)?;
        let database_url = get("CYEX_TV_DATABASE_URL")
            .filter(|v| !v.trim().is_empty())
            .ok_or(ConfigError::Missing("CYEX_TV_DATABASE_URL"))?;
        let db_max_connections = parse_or(&get, "CYEX_TV_DB_MAX_CONNECTIONS", 10u32)?;
        if db_max_connections == 0 {
            return Err(ConfigError::Invalid {
                name: "CYEX_TV_DB_MAX_CONNECTIONS",
                reason: "must be at least 1".to_owned(),
            });
        }
        let statement_ms = parse_or(&get, "CYEX_TV_DB_STATEMENT_TIMEOUT_MS", 5_000u64)?;
        let acquire_ms = parse_or(&get, "CYEX_TV_DB_ACQUIRE_TIMEOUT_MS", 5_000u64)?;
        let log_filter = get("CYEX_TV_LOG").unwrap_or_else(|| "info".to_owned());
        let log_format = match get("CYEX_TV_LOG_FORMAT").as_deref() {
            None | Some("json") => LogFormat::Json,
            Some("pretty") => LogFormat::Pretty,
            Some(other) => {
                return Err(ConfigError::Invalid {
                    name: "CYEX_TV_LOG_FORMAT",
                    reason: format!("{other:?} is not json or pretty"),
                });
            }
        };

        Ok(Self {
            bind,
            database_url,
            db_max_connections,
            db_statement_timeout: Duration::from_millis(statement_ms),
            db_acquire_timeout: Duration::from_millis(acquire_ms),
            log_filter,
            log_format,
        })
    }
}

fn parse_or<T>(
    get: &impl Fn(&str) -> Option<String>,
    name: &'static str,
    default: T,
) -> Result<T, ConfigError>
where
    T: FromStr,
    T::Err: fmt::Display,
{
    match get(name) {
        None => Ok(default),
        Some(raw) => raw.trim().parse().map_err(|err: T::Err| ConfigError::Invalid {
            name,
            reason: err.to_string(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::net::{IpAddr, Ipv4Addr};

    use super::*;

    const BIND: SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 8080);

    fn load(pairs: &[(&str, &str)]) -> Result<Config, ConfigError> {
        let map: HashMap<String, String> =
            pairs.iter().map(|(k, v)| ((*k).to_owned(), (*v).to_owned())).collect();
        Config::from_source(|name| map.get(name).cloned(), BIND)
    }

    #[test]
    fn defaults_apply() {
        let cfg = load(&[("CYEX_TV_DATABASE_URL", "postgres://localhost/x")]).unwrap();
        assert_eq!(cfg.bind, BIND);
        assert_eq!(cfg.db_max_connections, 10);
        assert_eq!(cfg.db_statement_timeout, Duration::from_secs(5));
        assert_eq!(cfg.log_format, LogFormat::Json);
    }

    #[test]
    fn database_url_is_required() {
        assert_eq!(load(&[]).unwrap_err(), ConfigError::Missing("CYEX_TV_DATABASE_URL"));
        assert_eq!(
            load(&[("CYEX_TV_DATABASE_URL", "  ")]).unwrap_err(),
            ConfigError::Missing("CYEX_TV_DATABASE_URL")
        );
    }

    #[test]
    fn bad_values_are_named() {
        let err = load(&[("CYEX_TV_DATABASE_URL", "x"), ("CYEX_TV_DB_MAX_CONNECTIONS", "ten")])
            .unwrap_err();
        assert!(matches!(err, ConfigError::Invalid { name: "CYEX_TV_DB_MAX_CONNECTIONS", .. }));

        let err = load(&[("CYEX_TV_DATABASE_URL", "x"), ("CYEX_TV_DB_MAX_CONNECTIONS", "0")])
            .unwrap_err();
        assert!(matches!(err, ConfigError::Invalid { name: "CYEX_TV_DB_MAX_CONNECTIONS", .. }));

        let err = load(&[("CYEX_TV_DATABASE_URL", "x"), ("CYEX_TV_LOG_FORMAT", "xml")]).unwrap_err();
        assert!(matches!(err, ConfigError::Invalid { name: "CYEX_TV_LOG_FORMAT", .. }));
    }

    #[test]
    fn debug_hides_the_database_url() {
        let cfg = load(&[("CYEX_TV_DATABASE_URL", "postgres://user:secret@host/db")]).unwrap();
        assert!(!format!("{cfg:?}").contains("secret"));
    }
}
