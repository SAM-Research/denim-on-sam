use crate::managers::{BufferManager, DenimKeyManager, DenimKeyManagerType};
use std::sync::Arc;
mod in_mem;

pub use in_mem::InMemory;
use sam_server::managers::traits::{
    account_manager::AccountManager, device_manager::DeviceManager,
};

pub trait StateType: 'static + Clone {
    type BufferManager: BufferManager;
    type DenimKeyManagerType: DenimKeyManagerType;
    type AccountManager: AccountManager;
    type DeviceManger: DeviceManager;
}

#[derive(Clone)]
pub struct DenimState<T: StateType> {
    _buffer_manager: T::BufferManager,
    pub keys: DenimKeyManager<T::DenimKeyManagerType>,
    pub devices: T::DeviceManger,
    pub accounts: T::AccountManager,
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
        keys: DenimKeyManager<T::DenimKeyManagerType>,
        accounts: T::AccountManager,
        devices: T::DeviceManger,
    ) -> Self {
        Self {
            sam_addr,
            channel_buffer,
            ws_proxy_tls_config: ws_proxy_tls_config.map(Arc::new),
            _buffer_manager: buffer_manager,
            keys,
            devices,
            accounts,
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

    #[cfg(test)]
    pub fn in_memory_test(sam_addr: String) -> DenimState<InMemory> {
        use sam_server::managers::in_memory::{
            account::InMemoryAccountManager, device::InMemoryDeviceManager,
        };

        use crate::managers::in_mem::{
            InMemoryBufferManager, InMemoryDenimEcPreKeyManager, InMemoryDenimSignedPreKeyManager,
        };

        DenimState::new(
            sam_addr.to_string(),
            10,
            None,
            InMemoryBufferManager::default(),
            DenimKeyManager::new(
                InMemoryDenimEcPreKeyManager::default(),
                InMemoryDenimSignedPreKeyManager::default(),
            ),
            InMemoryAccountManager::default(),
            InMemoryDeviceManager::new("Test".to_owned(), 120),
        )
    }
}
