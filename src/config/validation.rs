use std::path::Path;
use super::error::ConfigError;
use super::Config;

impl Config {
    /// Validate configuration with detailed error messages
    pub fn validate(&self) -> Result<(), ConfigError> {
        // Validate server address format
        if self.server.address.is_empty() {
            return Err(ConfigError::InvalidAddress {
                address: "(empty)".to_string(),
            });
        }

        // Try to parse the address to ensure it's valid
        if self.server.address.parse::<std::net::SocketAddr>().is_err() {
            return Err(ConfigError::InvalidAddress {
                address: self.server.address.clone(),
            });
        }

        // Validate TLS certificate file exists
        if !Path::new(&self.tls.cert_file).exists() {
            return Err(ConfigError::CertFileNotFound {
                path: self.tls.cert_file.clone(),
            });
        }

        // Validate TLS private key file exists
        if !Path::new(&self.tls.key_file).exists() {
            return Err(ConfigError::KeyFileNotFound {
                path: self.tls.key_file.clone(),
            });
        }

        // Validate log level
        let valid_levels = ["trace", "debug", "info", "warn", "error"];
        if !valid_levels.contains(&self.logging.level.as_str()) {
            return Err(ConfigError::InvalidLogLevel {
                level: self.logging.level.clone(),
            });
        }

        // Validate congestion control algorithm
        let valid_cc = ["cubic", "bbr", "reno"];
        if !valid_cc.contains(&self.quic.congestion_control.as_str()) {
            return Err(ConfigError::InvalidCongestionControl {
                algorithm: self.quic.congestion_control.clone(),
            });
        }

        // Validate QUIC parameters ranges
        if self.quic.max_idle_timeout == 0 {
            return Err(ConfigError::InvalidParameter {
                parameter: "max_idle_timeout".to_string(),
                value: "0".to_string(),
                message: "Timeout must be greater than 0".to_string(),
            });
        }

        if self.quic.max_udp_payload_size < 1200 {
            return Err(ConfigError::InvalidParameter {
                parameter: "max_udp_payload_size".to_string(),
                value: self.quic.max_udp_payload_size.to_string(),
                message: "Must be at least 1200 bytes (QUIC minimum)".to_string(),
            });
        }

        if self.quic.max_udp_payload_size > 65535 {
            return Err(ConfigError::InvalidParameter {
                parameter: "max_udp_payload_size".to_string(),
                value: self.quic.max_udp_payload_size.to_string(),
                message: "Must not exceed 65535 bytes (UDP maximum)".to_string(),
            });
        }

        // Validate Server parameters ranges
        if self.server.read_buffer_size < self.quic.max_udp_payload_size {
            return Err(ConfigError::InvalidParameter {
                parameter: "read_buffer_size".to_string(),
                value: self.server.read_buffer_size.to_string(),
                message: format!(
                    "Buffer size must be at least as large as max_udp_payload_size ({})",
                    self.quic.max_udp_payload_size
                ),
            });
        }

        if self.server.event_capacity == 0 {
            return Err(ConfigError::InvalidParameter {
                parameter: "event_capacity".to_string(),
                value: "0".to_string(),
                message: "Event capacity must be greater than 0".to_string(),
            });
        }

        Ok(())
    }
}
