//! SafeClaw error types

use thiserror::Error;

/// SafeClaw error type
#[derive(Error, Debug)]
pub enum Error {
    /// Configuration error
    #[error("Configuration error: {0}")]
    Config(String),

    /// Channel error
    #[error("Channel error: {0}")]
    Channel(String),

    /// Session error
    #[error("Session error: {0}")]
    Session(String),

    /// TEE error
    #[error("TEE error: {0}")]
    Tee(String),

    /// Privacy classification error
    #[error("Privacy error: {0}")]
    Privacy(String),

    /// Cryptographic error
    #[error("Crypto error: {0}")]
    Crypto(String),

    /// Gateway error
    #[error("Gateway error: {0}")]
    Gateway(String),

    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// HTTP error
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    /// Memory error
    #[error("Memory error: {0}")]
    Memory(String),

    /// Internal error
    #[error("Internal error: {0}")]
    Internal(String),
}

/// Result type alias for SafeClaw operations
pub type Result<T> = std::result::Result<T, Error>;
