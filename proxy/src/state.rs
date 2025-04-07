use std::sync::Arc;

use denim_sam_common::buffers::{ReceivingBufferConfig, SendingBufferConfig};

use crate::managers::{default::BufferManager, traits::MessageIdProvider};

#[derive(Clone)]
pub struct DenimState<T: ReceivingBufferConfig, U: SendingBufferConfig, V: MessageIdProvider> {
    buffer_manager: BufferManager<T, U, V>,
    sam_url: String,
    channel_buffer: usize,
    ws_proxy_tls_config: Option<Arc<rustls::ClientConfig>>,
}

impl<T: ReceivingBufferConfig, U: SendingBufferConfig, V: MessageIdProvider> DenimState<T, U, V> {
    pub fn new(
        buffer_manager: BufferManager<T, U, V>,
        sam_addr: String,
        channel_buffer: usize,
        ws_proxy_tls_config: Option<rustls::ClientConfig>,
    ) -> Self {
        Self {
            buffer_manager,
            sam_url: sam_addr,
            channel_buffer,
            ws_proxy_tls_config: ws_proxy_tls_config.map(Arc::new),
        }
    }

    pub fn buffer_manager(&self) -> BufferManager<T, U, V> {
        self.buffer_manager.clone()
    }

    pub fn sam_url(&self) -> &String {
        &self.sam_url
    }

    pub fn channel_buffer(&self) -> usize {
        self.channel_buffer
    }

    pub fn ws_proxy_tls_config(&self) -> Option<Arc<rustls::ClientConfig>> {
        self.ws_proxy_tls_config.clone()
    }
}
