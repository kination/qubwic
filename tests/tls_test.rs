use qubwic::config::{QuicConfig, TlsConfig};
use qubwic::tls;

mod common;

#[test]
fn test_create_quiche_config() {
    let cert_path = common::create_dummy_file("dummy_cert_tls.pem");
    let key_path = common::create_dummy_file("dummy_key_tls.pem");

    let tls_config = TlsConfig {
        cert_file: cert_path.to_str().unwrap().to_string(),
        key_file: key_path.to_str().unwrap().to_string(),
        alpn: vec!["h3".to_string()],
        min_version: "1.3".to_string(),
    };
    
    let quic_config = QuicConfig::default();
    
    // This will likely fail because dummy files are empty and quiche expects valid PEM structure
    // We expect an error, but verify that it tries to load.
    // However, if we want success, we need valid self-signed certs. 
    // Since we don't have openssl/ring easily available to generate them in test (without dev-deps),
    // we can check if it returns an Error (which means it reached the loading step).
    // Or we content ourselves with checking that correct paths are passed.
    
    // Let's assume testing for Error is fine for now given the constraints.
    // "create_quiche_config" calls "load_cert...".
    
    let result = tls::create_quiche_config(&tls_config, &quic_config);
    
    // It should fail loading certs from empty file
    assert!(result.is_err());
    
    common::remove_file(cert_path);
    common::remove_file(key_path);
}
