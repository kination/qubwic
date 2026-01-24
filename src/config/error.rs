use thiserror::Error;

/// Configuration errors with detailed context
#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Configuration file not found: {path}")]
    FileNotFound { path: String },

    #[error("Failed to read configuration file: {path}\n  Reason: {source}")]
    ReadError { path: String, source: std::io::Error },

    #[error("Invalid TOML syntax in configuration file: {path}\n  {source}")]
    ParseError { path: String, source: toml::de::Error },

    #[error("Invalid server address: '{address}'\n  Expected format: 'IP:PORT' (e.g., '127.0.0.1:4433')")]
    InvalidAddress { address: String },

    #[error("Certificate file not found: {path}\n  Please create a certificate or specify a valid path")]
    CertFileNotFound { path: String },

    #[error("Private key file not found: {path}\n  Please create a private key or specify a valid path")]
    KeyFileNotFound { path: String },

    #[error("Invalid log level: '{level}'\n  Valid levels: trace, debug, info, warn, error")]
    InvalidLogLevel { level: String },

    #[error("Invalid congestion control algorithm: '{algorithm}'\n  Valid algorithms: cubic, bbr, reno")]
    InvalidCongestionControl { algorithm: String },

    #[error("Invalid configuration parameter: {parameter}\n  Value: {value}\n  {message}")]
    InvalidParameter {
        parameter: String,
        value: String,
        message: String,
    },
}
