use axum::http;
use bon::bon;
use log::debug;

use sam_net::{
    tls::{create_tls_client_config, create_tls_server_config, MutualTlsConfig},
    websocket::WebSocketClientConfig,
};

use serde::{Deserialize, Serialize};
use tokio_tungstenite::Connector;

use crate::{
    error::{ServerError, TlsError},
    state::{DenimState, DenimStateType},
};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DenimCliConfig {
    pub database_url: String,
    pub sam_address: Option<String>,
    pub denim_proxy_address: Option<String>,
    pub deniable_ratio: Option<f32>, // q
    pub tls: Option<TlsConfig>,
    pub channel_buffer_size: Option<usize>,
    pub key_generate_amount: Option<usize>,
    pub logging: Option<String>,
}

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

#[bon]
impl DenimCliConfig {
    #[builder]
    pub fn new(
        database_url: String,
        sam_address: Option<String>,
        denim_proxy_address: Option<String>,
        deniable_ratio: Option<f32>,
        tls: Option<TlsConfig>,
        channel_buffer_size: Option<usize>,
        key_generate_amount: Option<usize>,
        logging: Option<String>,
    ) -> Self {
        Self {
            database_url,
            sam_address,
            denim_proxy_address,
            deniable_ratio,
            tls,
            channel_buffer_size,
            key_generate_amount,
            logging,
        }
    }
    pub fn load<R: std::io::Read>(reader: R) -> Result<Self, serde_json::Error> {
        serde_json::from_reader(reader)
    }
}

impl TlsConfig {
    pub fn create(self) -> Result<(rustls::ServerConfig, rustls::ClientConfig), TlsError> {
        let mtls = if self.proxy_mtls {
            Some(self.ca_cert_path.clone())
        } else {
            None
        };
        let server =
            create_tls_server_config(&self.proxy_cert_path, &self.proxy_key_path, mtls.as_deref())?;
        let mutual = if let Some(config) = self.proxy_client {
            Some(MutualTlsConfig::new(config.key_path, config.cert_path))
        } else {
            None
        };

        let client = create_tls_client_config(&self.ca_cert_path, mutual)?;
        Ok((server, client))
    }
}

pub fn websocket_config<T: DenimStateType>(
    basic: String,
    state: &DenimState<T>,
) -> Result<WebSocketClientConfig, ServerError> {
    let (url, connector) = match state.ws_proxy_tls_config() {
        None => (format!("ws://{}", state.sam_address()), None),
        Some(config) => (
            format!("wss://{}", state.sam_address()),
            Some(Connector::Rustls(config)),
        ),
    };
    Ok(WebSocketClientConfig::builder()
        .maybe_tls(connector)
        .url(format!("{}/api/v1/websocket", url))
        .headers(vec![(
            http::header::AUTHORIZATION,
            http::HeaderValue::from_str(&basic)
                .inspect_err(|e| debug!("{e}"))
                .map_err(|_| ServerError::SAMUnAuth)?,
        )])
        .build())
}
