//! Common error types for Oracle VM

use thiserror::Error;

#[derive(Error, Debug)]
pub enum OracleVmError {
    #[error("Network error: {0}")]
    Network(String),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Cryptographic error: {0}")]
    Crypto(String),

    #[error("Bitcoin error: {0}")]
    Bitcoin(String),

    #[error("Oracle error: {0}")]
    Oracle(String),

    #[error("Aggregation error: {0}")]
    Aggregation(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Invalid data: {0}")]
    InvalidData(String),

    #[error("Timeout error")]
    Timeout,

    #[error("Internal error: {0}")]
    Internal(String),
}

pub type Result<T> = std::result::Result<T, OracleVmError>;
