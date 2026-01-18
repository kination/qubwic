use clap::Parser;

/// QuBWic - QUIC/HTTP/3 Web Server
#[derive(Parser, Debug)]
#[command(name = "qubwic")]
#[command(version, about, long_about = None)]
pub struct CliArgs {
    /// Path to configuration file
    #[arg(short, long, default_value = "server.toml", env = "QUBIC_CONFIG")]
    pub config: String,

    /// Override log level (trace, debug, info, warn, error)
    #[arg(short, long, env = "QUBIC_LOG_LEVEL")]
    pub log_level: Option<String>,

    /// Override server address
    #[arg(short, long, env = "QUBIC_SERVER_ADDRESS")]
    pub address: Option<String>,
}
