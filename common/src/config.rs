//! Configuration utilities for Oracle VM components

use crate::{NodeId, OracleVmError, Result};
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Network configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    pub listen_address: String,
    pub bootstrap_peers: Vec<String>,
    pub max_peers: usize,
    pub connection_timeout: Duration,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            listen_address: "0.0.0.0:9000".to_string(),
            bootstrap_peers: vec![],
            max_peers: 50,
            connection_timeout: Duration::from_secs(30),
        }
    }
}

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogConfig {
    pub level: String,
    pub file: Option<String>,
    pub console: bool,
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            file: None,
            console: true,
        }
    }
}

/// Database configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub path: String,
    pub cache_size: usize,
    pub write_buffer_size: usize,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            path: "./data".to_string(),
            cache_size: 256 * 1024 * 1024,       // 256MB
            write_buffer_size: 64 * 1024 * 1024, // 64MB
        }
    }
}

/// Base configuration shared by all components
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaseConfig {
    pub node_id: NodeId,
    pub network: NetworkConfig,
    pub logging: LogConfig,
    pub database: DatabaseConfig,
}

impl BaseConfig {
    pub fn new(node_id: impl Into<String>) -> Self {
        Self {
            node_id: NodeId::new(node_id),
            network: NetworkConfig::default(),
            logging: LogConfig::default(),
            database: DatabaseConfig::default(),
        }
    }
}

/// Load configuration from file or environment
pub trait ConfigLoader<T> {
    fn load_from_file(path: &str) -> Result<T>;
    fn load_from_env() -> Result<T>;
    fn save_to_file(&self, path: &str) -> Result<()>;
}

/// Environment variable helper
pub fn get_env_var(key: &str) -> Option<String> {
    std::env::var(key).ok()
}

/// Parse duration from string (e.g., "30s", "5m", "1h")
pub fn parse_duration(s: &str) -> Result<Duration> {
    if s.is_empty() {
        return Err(OracleVmError::Config("Empty duration string".to_string()));
    }

    let (value_str, unit) = if let Some(pos) = s.find(|c: char| c.is_alphabetic()) {
        (&s[..pos], &s[pos..])
    } else {
        (s, "")
    };

    let value: u64 = value_str
        .parse()
        .map_err(|_| OracleVmError::Config(format!("Invalid duration value: {}", value_str)))?;

    let duration = match unit {
        "" | "s" => Duration::from_secs(value),
        "ms" => Duration::from_millis(value),
        "m" => Duration::from_secs(value * 60),
        "h" => Duration::from_secs(value * 3600),
        "d" => Duration::from_secs(value * 86400),
        _ => {
            return Err(OracleVmError::Config(format!(
                "Unknown duration unit: {}",
                unit
            )))
        }
    };

    Ok(duration)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_duration() {
        assert_eq!(parse_duration("30s").unwrap(), Duration::from_secs(30));
        assert_eq!(parse_duration("5m").unwrap(), Duration::from_secs(300));
        assert_eq!(parse_duration("1h").unwrap(), Duration::from_secs(3600));
        assert_eq!(
            parse_duration("1000ms").unwrap(),
            Duration::from_millis(1000)
        );
    }

    #[test]
    fn test_base_config() {
        let config = BaseConfig::new("test-node");
        assert_eq!(config.node_id.as_str(), "test-node");
    }
}
