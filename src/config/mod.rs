mod error;
mod cli;
mod env;
mod validation;

pub use error::ConfigError;
pub use cli::CliArgs;

use serde::Deserialize;
use std::fs;
use std::path::Path;
use anyhow::Result;
use clap::Parser;

/// Server-level configuration
#[derive(Deserialize, Clone, Debug)]
pub struct ServerConfig {
    pub address: String,
    #[serde(default = "default_worker_threads")]
    pub worker_threads: usize,
    #[serde(default = "default_read_buffer_size")]
    pub read_buffer_size: usize,
    #[serde(default = "default_event_capacity")]
    pub event_capacity: usize,
    #[serde(default)]
    pub pid_file: Option<String>,
}

fn default_read_buffer_size() -> usize {
    65535
}

fn default_event_capacity() -> usize {
    1024
}

fn default_worker_threads() -> usize {
    num_cpus::get()
}

/// QUIC protocol configuration
#[derive(Deserialize, Clone, Debug)]
pub struct QuicConfig {
    /// Maximum idle timeout in milliseconds
    #[serde(default = "default_max_idle_timeout")]
    pub max_idle_timeout: u64,

    /// Initial maximum data (flow control)
    #[serde(default = "default_initial_max_data")]
    pub initial_max_data: u64,

    /// Initial maximum stream data for bidirectional streams (local)
    #[serde(default = "default_initial_max_stream_data_bidi")]
    pub initial_max_stream_data_bidi_local: u64,

    /// Initial maximum stream data for bidirectional streams (remote)
    #[serde(default = "default_initial_max_stream_data_bidi")]
    pub initial_max_stream_data_bidi_remote: u64,

    /// Initial maximum stream data for unidirectional streams
    #[serde(default = "default_initial_max_stream_data_uni")]
    pub initial_max_stream_data_uni: u64,

    /// Maximum UDP payload size
    #[serde(default = "default_max_udp_payload_size")]
    pub max_udp_payload_size: usize,

    /// Congestion control algorithm (cubic, bbr, reno)
    #[serde(default = "default_congestion_control")]
    pub congestion_control: String,

    /// Enable 0-RTT early data
    #[serde(default)]
    pub enable_early_data: bool,

    /// Maximum bidirectional streams
    #[serde(default = "default_max_streams_bidi")]
    pub max_streams_bidi: u64,

    /// Maximum unidirectional streams
    #[serde(default = "default_max_streams_uni")]
    pub max_streams_uni: u64,
}

fn default_max_idle_timeout() -> u64 { 30_000 }
fn default_initial_max_data() -> u64 { 10_000_000 }
fn default_initial_max_stream_data_bidi() -> u64 { 1_000_000 }
fn default_initial_max_stream_data_uni() -> u64 { 1_000_000 }
fn default_max_udp_payload_size() -> usize { 1350 }
fn default_congestion_control() -> String { "cubic".to_string() }
fn default_max_streams_bidi() -> u64 { 100 }
fn default_max_streams_uni() -> u64 { 100 }

impl Default for QuicConfig {
    fn default() -> Self {
        Self {
            max_idle_timeout: default_max_idle_timeout(),
            initial_max_data: default_initial_max_data(),
            initial_max_stream_data_bidi_local: default_initial_max_stream_data_bidi(),
            initial_max_stream_data_bidi_remote: default_initial_max_stream_data_bidi(),
            initial_max_stream_data_uni: default_initial_max_stream_data_uni(),
            max_udp_payload_size: default_max_udp_payload_size(),
            congestion_control: default_congestion_control(),
            enable_early_data: false,
            max_streams_bidi: default_max_streams_bidi(),
            max_streams_uni: default_max_streams_uni(),
        }
    }
}

/// TLS configuration
#[derive(Deserialize, Clone, Debug)]
pub struct TlsConfig {
    pub cert_file: String,
    pub key_file: String,

    /// ALPN protocols
    #[serde(default = "default_alpn")]
    pub alpn: Vec<String>,

    /// Minimum TLS version (always 1.3 for QUIC)
    #[serde(default = "default_min_version")]
    pub min_version: String,
}

fn default_alpn() -> Vec<String> {
    vec!["h3".to_string()]
}

fn default_min_version() -> String {
    "1.3".to_string()
}

/// Logging configuration
#[derive(Deserialize, Clone, Debug)]
pub struct LogConfig {
    /// Log level (trace, debug, info, warn, error)
    #[serde(default = "default_log_level")]
    pub level: String,

    /// Log format (combined, common, json)
    #[serde(default = "default_log_format")]
    pub format: String,

    /// Access log file path
    #[serde(default)]
    pub access_log: Option<String>,

    /// Error log file path
    #[serde(default)]
    pub error_log: Option<String>,
}

fn default_log_level() -> String { "info".to_string() }
fn default_log_format() -> String { "combined".to_string() }

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            level: default_log_level(),
            format: default_log_format(),
            access_log: None,
            error_log: None,
        }
    }
}

/// Root configuration
#[derive(Deserialize, Clone, Debug)]
pub struct Config {
    pub server: ServerConfig,

    #[serde(default)]
    pub quic: QuicConfig,

    pub tls: TlsConfig,

    #[serde(default)]
    pub logging: LogConfig,
}

/// Load configuration from file with environment variable overrides
pub fn load(path: &str) -> Result<Config> {
    // Check if file exists
    if !Path::new(path).exists() {
        return Err(ConfigError::FileNotFound {
            path: path.to_string(),
        }
        .into());
    }

    // Read file content
    let content = fs::read_to_string(path).map_err(|e| ConfigError::ReadError {
        path: path.to_string(),
        source: e,
    })?;

    // Parse TOML
    let mut config: Config = toml::from_str(&content).map_err(|e| ConfigError::ParseError {
        path: path.to_string(),
        source: e,
    })?;

    // Apply environment variable overrides
    config.apply_env_overrides();

    // Validate configuration
    config.validate()?;

    Ok(config)
}

/// Load configuration with custom path from CLI or default
pub fn load_with_args() -> Result<Config> {
    let args = CliArgs::parse();
    load(&args.config)
}
