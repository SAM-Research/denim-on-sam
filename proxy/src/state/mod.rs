use crate::managers::{BufferManager, DenimKeyManager, DenimKeyManagerType};
use std::sync::Arc;
mod in_mem;

pub use in_mem::InMemory;
use sam_server::managers::traits::account_manager::AccountManager;

pub trait StateType: 'static + Clone {
    type BufferManager: BufferManager;
    type DenimKeyManagerType: DenimKeyManagerType;
    type AccountManager: AccountManager;
}

#[derive(Clone)]
pub struct DenimState<T: StateType> {
    _buffer_manager: T::BufferManager,
    _kdc: DenimKeyManager<T::DenimKeyManagerType>,
    _accounts: T::AccountManager,
    sam_addr: String,
    channel_buffer: usize,
    ws_proxy_tls_config: Option<Arc<rustls::ClientConfig>>,
}

impl<T: StateType> DenimState<T> {
    pub fn new(
        sam_addr: String,
        channel_buffer: usize,
        ws_proxy_tls_config: Option<rustls::ClientConfig>,
        buffer_manager: T::BufferManager,
        kdc: DenimKeyManager<T::DenimKeyManagerType>,
        accounts: T::AccountManager,
    ) -> Self {
        Self {
            sam_addr,
            channel_buffer,
            ws_proxy_tls_config: ws_proxy_tls_config.map(Arc::new),
            _buffer_manager: buffer_manager,
            _kdc: kdc,
            _accounts: accounts,
        }
    }

    pub fn sam_address(&self) -> &String {
        &self.sam_addr
    }

    pub fn channel_buffer(&self) -> usize {
        self.channel_buffer
    }

    pub fn ws_proxy_tls_config(&self) -> Option<Arc<rustls::ClientConfig>> {
        self.ws_proxy_tls_config.clone()
    }
}
