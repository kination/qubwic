use quiche;
use anyhow::Result;

pub fn create_quiche_config(cert_path: &str, key_path: &str) -> Result<quiche::Config> {

    let mut config = quiche::Config::new(quiche::PROTOCOL_VERSION)?;

    config.load_cert_chain_from_pem_file(cert_path)?;
    config.load_priv_key_from_pem_file(key_path)?;
    config.set_application_protos(&[b"h3"])?;

    // Connection settings
    config.set_max_idle_timeout(30000);
    config.set_max_recv_udp_payload_size(1350);
    config.set_max_send_udp_payload_size(1350);

    // Flow control
    config.set_initial_max_data(10_000_000);
    config.set_initial_max_stream_data_bidi_local(1_000_000);
    config.set_initial_max_stream_data_bidi_remote(1_000_000);
    config.set_initial_max_stream_data_uni(1_000_000);

    // Stream limits
    config.set_initial_max_streams_bidi(100);
    config.set_initial_max_streams_uni(100);

    Ok(config)
}
