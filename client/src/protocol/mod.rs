use denim_client::{DenimProtocolClient, DenimSamClient};
use denim_sam_common::buffers::{ReceivingBuffer, SendingBuffer};
use sam_client::net::protocol::WebSocketProtocolClientConfig;
use sam_common::{AccountId, DeviceId};

use crate::error::DenimProtocolError;

pub mod denim_client;

pub struct DenimProtocolClientConfig<T, U> {
    config: WebSocketProtocolClientConfig,
    sending_buffer: T,
    receiving_buffer: U,
}

impl<T: SendingBuffer, U: ReceivingBuffer> DenimProtocolClientConfig<T, U> {
    pub fn new(
        config: WebSocketProtocolClientConfig,
        sending_buffer: T,
        receiving_buffer: U,
    ) -> Self {
        DenimProtocolClientConfig {
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
        Ok(DenimProtocolClient::new(
            self.config
                .to_websocket_client(account_id, device_id, password)?,
            self.sending_buffer,
            self.receiving_buffer,
        ))
    }
}


