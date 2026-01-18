use qubwic::config::{Config, ServerConfig, QuicConfig, TlsConfig, LogConfig};

mod common;

#[test]
fn test_validate_valid_config() {
    let cert_path = common::create_dummy_file("dummy_cert_config.pem");
    let key_path = common::create_dummy_file("dummy_key_config.pem");

    let config = Config {
        server: ServerConfig {
            address: "127.0.0.1:4433".to_string(),
            worker_threads: 1,
            pid_file: None,
        },
        quic: QuicConfig::default(),
        tls: TlsConfig {
            cert_file: cert_path.to_str().unwrap().to_string(),
            key_file: key_path.to_str().unwrap().to_string(),
            alpn: vec!["h3".to_string()],
            min_version: "1.3".to_string(),
        },
        logging: LogConfig::default(),
    };

    assert!(config.validate().is_ok());
    
    // Cleanup
    common::remove_file(cert_path);
    common::remove_file(key_path);
}

#[test]
fn test_validate_invalid_address() {
    // We don't need real cert files here if address validation comes first
    let config = Config {
        server: ServerConfig {
            address: "invalid-ip".to_string(),
            worker_threads: 1,
            pid_file: None,
        },
        quic: QuicConfig::default(),
        tls: TlsConfig {
            cert_file: "cert".to_string(),
            key_file: "key".to_string(),
            alpn: vec![],
            min_version: "1.3".to_string(),
        },
        logging: LogConfig::default(),
    };
    
    assert!(config.validate().is_err());
}

#[test]
fn test_validate_missing_cert() {
    let config = Config {
        server: ServerConfig {
            address: "127.0.0.1:4433".to_string(),
            worker_threads: 1,
            pid_file: None,
        },
        quic: QuicConfig::default(),
        tls: TlsConfig {
            cert_file: "/non/existent/path/cert.pem".to_string(),
            key_file: "/non/existent/path/key.pem".to_string(),
            alpn: vec![],
            min_version: "1.3".to_string(),
        },
        logging: LogConfig::default(),
    };

    assert!(config.validate().is_err());
}
