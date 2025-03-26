use sam_client::net::tls::{create_tls_config, MutualTlsConfig};
use sam_server::create_tls_config as create_server_tls_config;
use serde::{Deserialize, Serialize};

use crate::error::TLSError;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TlsConfig {
    pub ca_cert_path: String,
    pub proxy_cert_path: String,
    pub proxy_key_path: String,
    pub proxy_client: Option<ProxyMtlsConfig>,
    pub proxy_mtls: bool,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProxyMtlsConfig {
    pub cert_path: String,
    pub key_path: String,
}

impl TlsConfig {
    pub fn load<R: std::io::Read>(reader: R) -> Result<Self, serde_json::Error> {
        serde_json::from_reader(reader)
    }

    pub fn create(self) -> Result<(rustls::ServerConfig, rustls::ClientConfig), TLSError> {
        let mtls = if self.proxy_mtls {
            Some(self.ca_cert_path.clone())
        } else {
            None
        };
        let server =
            create_server_tls_config(&self.proxy_cert_path, &self.proxy_key_path, mtls.as_deref())?;
        let mutual = if let Some(config) = self.proxy_client {
            Some(MutualTlsConfig::new(config.key_path, config.cert_path))
        } else {
            None
        };

        let client = create_tls_config(&self.ca_cert_path, mutual)?;
        Ok((server, client))
    }
}
