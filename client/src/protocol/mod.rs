use crate::error::DenimProtocolError;
use denim_client::{DenimProtocolClient, DenimSamClient};
use denim_sam_common::buffers::{ReceivingBuffer, SendingBuffer};
use log::debug;
use rustls::ClientConfig;
use sam_client::net::protocol::websocket::WebSocketClientConfig;
use sam_client::net::protocol::{get_ws_auth, get_ws_url_and_connector};
use sam_common::{AccountId, DeviceId};
use tokio_tungstenite::tungstenite::http;

pub mod denim_client;

pub struct DenimProtocolClientConfig<T, U> {
    base_url: String,
    config: Option<ClientConfig>,
    sending_buffer: T,
    receiving_buffer: U,
}

impl<T: SendingBuffer, U: ReceivingBuffer> DenimProtocolClientConfig<T, U> {
    pub fn new(
        base_url: String,
        config: Option<ClientConfig>,
        sending_buffer: T,
        receiving_buffer: U,
    ) -> DenimProtocolClientConfig<T, U> {
        DenimProtocolClientConfig {
            base_url,
            config,
            sending_buffer,
            receiving_buffer,
        }
    }
}

pub trait DenimProtocolConfig {
    type ProtocolClient: DenimSamClient;

    fn create(
        self,
        account_id: AccountId,
        device_id: DeviceId,
        password: String,
    ) -> Result<Self::ProtocolClient, DenimProtocolError>;
}

impl<T: SendingBuffer, U: ReceivingBuffer> DenimProtocolConfig for DenimProtocolClientConfig<T, U> {
    type ProtocolClient = DenimProtocolClient<T, U>;

    fn create(
        self,
        account_id: AccountId,
        device_id: DeviceId,
        password: String,
    ) -> Result<Self::ProtocolClient, DenimProtocolError> {
        let (url, connector) = get_ws_url_and_connector(self.config, self.base_url);
        let basic = get_ws_auth(account_id, device_id, password);
        let ws_client = WebSocketClientConfig::builder()
            .maybe_tls(connector)
            .url(format!("{}/api/v1/websocket", url))
            .headers(vec![(
                http::header::AUTHORIZATION,
                http::HeaderValue::from_str(&basic)
                    .inspect_err(|e| debug!("{e}"))
                    .map_err(|_| DenimProtocolError::InvalidCredentials)?,
            )])
            .build()
            .into();

        Ok(DenimProtocolClient::new(
            ws_client,
            self.sending_buffer,
            self.receiving_buffer,
        ))
    }
}
