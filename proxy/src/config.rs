use std::io::BufReader;

use sam_client::net::tls::{create_tls_config, MutualTLSConfig};
use sam_server::{create_tls_config as create_server_tls_config, error::TLSConfigError};
use serde::{Deserialize, Serialize};

use crate::error::TLSError;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TlsConfig {
    pub ca_cert_path: String,
    pub proxy_cert_path: String,
    pub proxy_key_path: String,
    pub proxy_client: Option<MtlsClientConfig>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MtlsClientConfig {
    pub cert_path: String,
    pub key_path: String,
}

impl TlsConfig {
    pub fn load(path: String) -> Result<Self, TLSConfigError> {
        let file = std::fs::File::open(path)?;
        let reader = BufReader::new(file);
        Ok(serde_json::from_reader(reader)?)
    }

    pub fn create(self) -> Result<(rustls::ServerConfig, rustls::ClientConfig), TLSError> {
        let server = create_server_tls_config(
            &self.proxy_cert_path,
            &self.proxy_key_path,
            Some(&self.ca_cert_path),
        )?;
        let mutual = if let Some(config) = self.proxy_client {
            Some(MutualTLSConfig::new(config.key_path, config.cert_path))
        } else {
            None
        };

        let client = create_tls_config(&self.ca_cert_path, mutual)?;
        Ok((server, client))
    }
}
