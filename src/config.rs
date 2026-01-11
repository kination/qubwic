use serde::Deserialize;
use std::fs;
use anyhow::Result;

#[derive(Deserialize, Clone)]
pub struct ServerConfig {
    pub address: String,
}

#[derive(Deserialize, Clone)]
pub struct TlsConfig {
    pub cert_file: String,
    pub key_file: String,
}

#[derive(Deserialize, Clone)]
pub struct Config {
    pub server: ServerConfig,
    pub tls: TlsConfig
}

pub fn load(path: &str) -> Result<Config> {
    let content = fs::read_to_string(path)?;
    let config: Config = toml::from_str(&content)?;
    Ok(config)
}