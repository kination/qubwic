use super::{Config, ServerConfig, QuicConfig, TlsConfig, LogConfig};

impl Config {
    /// Apply environment variable overrides
    pub fn apply_env_overrides(&mut self) {
        self.server.apply_env_overrides();
        self.quic.apply_env_overrides();
        self.tls.apply_env_overrides();
        self.logging.apply_env_overrides();
    }
}

impl ServerConfig {
    fn apply_env_overrides(&mut self) {
        if let Ok(val) = std::env::var("QUBIC_SERVER_ADDRESS") {
            log::info!("Overriding server.address from env: {}", val);
            self.address = val;
        }
        if let Ok(val) = std::env::var("QUBIC_SERVER_WORKER_THREADS") {
            if let Ok(threads) = val.parse::<usize>() {
                log::info!("Overriding server.worker_threads from env: {}", threads);
                self.worker_threads = threads;
            }
        }
        if let Ok(val) = std::env::var("QUBIC_SERVER_PID_FILE") {
            log::info!("Overriding server.pid_file from env: {}", val);
            self.pid_file = Some(val);
        }
    }
}

impl QuicConfig {
    fn apply_env_overrides(&mut self) {
        if let Ok(val) = std::env::var("QUBIC_QUIC_MAX_IDLE_TIMEOUT") {
            if let Ok(timeout) = val.parse::<u64>() {
                log::info!("Overriding quic.max_idle_timeout from env: {}", timeout);
                self.max_idle_timeout = timeout;
            }
        }
        if let Ok(val) = std::env::var("QUBIC_QUIC_CONGESTION_CONTROL") {
            log::info!("Overriding quic.congestion_control from env: {}", val);
            self.congestion_control = val;
        }
        if let Ok(val) = std::env::var("QUBIC_QUIC_ENABLE_EARLY_DATA") {
            if let Ok(enabled) = val.parse::<bool>() {
                log::info!("Overriding quic.enable_early_data from env: {}", enabled);
                self.enable_early_data = enabled;
            }
        }
    }
}

impl TlsConfig {
    fn apply_env_overrides(&mut self) {
        if let Ok(val) = std::env::var("QUBIC_TLS_CERT_FILE") {
            log::info!("Overriding tls.cert_file from env: {}", val);
            self.cert_file = val;
        }
        if let Ok(val) = std::env::var("QUBIC_TLS_KEY_FILE") {
            log::info!("Overriding tls.key_file from env: {}", val);
            self.key_file = val;
        }
    }
}

impl LogConfig {
    fn apply_env_overrides(&mut self) {
        if let Ok(val) = std::env::var("QUBIC_LOG_LEVEL") {
            log::info!("Overriding logging.level from env: {}", val);
            self.level = val;
        }
        if let Ok(val) = std::env::var("QUBIC_LOG_FORMAT") {
            log::info!("Overriding logging.format from env: {}", val);
            self.format = val;
        }
        if let Ok(val) = std::env::var("QUBIC_LOG_ACCESS_LOG") {
            log::info!("Overriding logging.access_log from env: {}", val);
            self.access_log = Some(val);
        }
        if let Ok(val) = std::env::var("QUBIC_LOG_ERROR_LOG") {
            log::info!("Overriding logging.error_log from env: {}", val);
            self.error_log = Some(val);
        }
    }
}
