use quiche;
use anyhow::Result;
use crate::config::{QuicConfig, TlsConfig};

/// Create a quiche::Config from our configuration
pub fn create_quiche_config(tls_config: &TlsConfig, quic_config: &QuicConfig) -> Result<quiche::Config> {
    let mut config = quiche::Config::new(quiche::PROTOCOL_VERSION)?;

    // Load TLS certificates
    config.load_cert_chain_from_pem_file(&tls_config.cert_file)?;
    config.load_priv_key_from_pem_file(&tls_config.key_file)?;

    // Set ALPN protocols
    let alpn_protos: Vec<&[u8]> = tls_config.alpn.iter().map(|s| s.as_bytes()).collect();
    config.set_application_protos(&alpn_protos)?;

    // Connection settings
    config.set_max_idle_timeout(quic_config.max_idle_timeout);
    config.set_max_recv_udp_payload_size(quic_config.max_udp_payload_size);
    config.set_max_send_udp_payload_size(quic_config.max_udp_payload_size);

    // Flow control
    config.set_initial_max_data(quic_config.initial_max_data);
    config.set_initial_max_stream_data_bidi_local(quic_config.initial_max_stream_data_bidi_local);
    config.set_initial_max_stream_data_bidi_remote(quic_config.initial_max_stream_data_bidi_remote);
    config.set_initial_max_stream_data_uni(quic_config.initial_max_stream_data_uni);

    // Stream limits
    config.set_initial_max_streams_bidi(quic_config.max_streams_bidi);
    config.set_initial_max_streams_uni(quic_config.max_streams_uni);

    // Congestion control
    match quic_config.congestion_control.as_str() {
        "cubic" => config.set_cc_algorithm(quiche::CongestionControlAlgorithm::CUBIC),
        "bbr" => config.set_cc_algorithm(quiche::CongestionControlAlgorithm::BBR),
        "reno" => config.set_cc_algorithm(quiche::CongestionControlAlgorithm::Reno),
        _ => config.set_cc_algorithm(quiche::CongestionControlAlgorithm::CUBIC),
    }

    // Early data (0-RTT)
    if quic_config.enable_early_data {
        config.enable_early_data();
    }

    Ok(config)
}
