use crate::managers::{
    in_mem::{InMemoryBufferManager, InMemoryKeyDistributionCenter},
    BufferManager, KeyDistributionCenter,
};
use std::sync::Arc;

pub trait StateType: 'static + Clone {
    type BufferManager: BufferManager + Send + Sync + Clone;
    type KeyDistributionCenter: KeyDistributionCenter + Send + Sync + Clone;
}

#[derive(Clone)]
pub struct InMemory;

impl StateType for InMemory {
    type BufferManager = InMemoryBufferManager;

    type KeyDistributionCenter = InMemoryKeyDistributionCenter;
}

#[derive(Clone)]
pub struct DenimState<T: StateType> {
    _buffer_manager: T::BufferManager,
    _kdc: T::KeyDistributionCenter,
    sam_url: String,
    channel_buffer: usize,
    ws_proxy_tls_config: Option<Arc<rustls::ClientConfig>>,
}

impl<T: StateType> DenimState<T> {
    pub fn new(
        sam_addr: String,
        channel_buffer: usize,
        ws_proxy_tls_config: Option<rustls::ClientConfig>,
        buffer_manager: T::BufferManager,
        kdc: T::KeyDistributionCenter,
    ) -> Self {
        Self {
            sam_url: sam_addr,
            channel_buffer,
            ws_proxy_tls_config: ws_proxy_tls_config.map(Arc::new),
            _buffer_manager: buffer_manager,
            _kdc: kdc,
        }
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
