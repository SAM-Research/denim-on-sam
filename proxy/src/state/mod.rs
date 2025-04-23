use crate::managers::traits::CryptoProvider;
use crate::managers::{
    traits::MessageIdProvider, BufferManager, DenimKeyManager, DenimKeyManagerType,
};
use denim_sam_common::buffers::{ReceivingBufferConfig, SendingBufferConfig};
use rand::CryptoRng;
use rand::Rng;
use sam_server::managers::traits::{
    account_manager::AccountManager, device_manager::DeviceManager,
};
use std::sync::Arc;

mod in_mem;

pub use in_mem::InMemoryBufferManagerType;
pub use in_mem::InMemoryStateType;

pub trait BufferManagerType: 'static + Clone {
    type ReceivingBufferConfig: ReceivingBufferConfig;
    type SendingBufferConfig: SendingBufferConfig;
    type MessageIdProvider: MessageIdProvider;
}

pub trait StateType: 'static + Clone {
    type BufferManager: BufferManagerType;
    type DenimKeyManagerType: DenimKeyManagerType;
    type AccountManager: AccountManager;
    type DeviceManger: DeviceManager;
    type CryptoProvider: CryptoProvider<Self::Rng>;
    type Rng: CryptoRng + Rng + Send;
}

#[derive(Clone)]
pub struct DenimState<T: StateType> {
    pub buffer_manager: BufferManager<T::BufferManager>,
    pub keys: DenimKeyManager<T::DenimKeyManagerType>,
    pub devices: T::DeviceManger,
    pub accounts: T::AccountManager,
    pub crypto_provider: T::CryptoProvider,

    sam_addr: String,
    channel_buffer_size: usize,
    ws_proxy_tls_config: Option<Arc<rustls::ClientConfig>>,
}

impl<T: StateType> DenimState<T> {
    pub fn new(
        sam_addr: String,
        channel_buffer_size: usize,
        ws_proxy_tls_config: Option<rustls::ClientConfig>,
        buffer_manager: BufferManager<T::BufferManager>,
        keys: DenimKeyManager<T::DenimKeyManagerType>,
        accounts: T::AccountManager,
        devices: T::DeviceManger,
        crypto_provider: T::CryptoProvider,
    ) -> Self {
        Self {
            sam_addr,
            channel_buffer_size,
            ws_proxy_tls_config: ws_proxy_tls_config.map(Arc::new),
            buffer_manager,
            keys,
            devices,
            accounts,
            crypto_provider,
        }
    }

    pub fn sam_address(&self) -> &String {
        &self.sam_addr
    }

    pub fn channel_buffer_size(&self) -> usize {
        self.channel_buffer_size
    }

    pub fn ws_proxy_tls_config(&self) -> Option<Arc<rustls::ClientConfig>> {
        self.ws_proxy_tls_config.clone()
    }

    #[cfg(test)]
    pub fn in_memory_test(sam_addr: String) -> DenimState<InMemoryStateType> {
        use denim_sam_common::buffers::in_mem::{
            InMemoryReceivingBufferConfig, InMemorySendingBufferConfig,
        };
        use sam_server::managers::in_memory::{
            account::InMemoryAccountManager, device::InMemoryDeviceManager,
            keys::InMemorySignedPreKeyManager,
        };

        use crate::managers::{
            default::ChaChaCryptoProvider, in_mem::InMemoryDenimEcPreKeyManager,
            InMemoryMessageIdProvider,
        };
        let rcfg = InMemoryReceivingBufferConfig;
        let scfg = InMemorySendingBufferConfig::builder().q(1.0).build();
        let id_provider = InMemoryMessageIdProvider::default();
        let buffer_mgr = BufferManager::new(rcfg, scfg, id_provider);

        DenimState::new(
            sam_addr.to_string(),
            10,
            None,
            buffer_mgr,
            DenimKeyManager::new(
                InMemoryDenimEcPreKeyManager::default(),
                InMemorySignedPreKeyManager::default(),
            ),
            InMemoryAccountManager::default(),
            InMemoryDeviceManager::new("Test".to_owned(), 120),
            ChaChaCryptoProvider,
        )
    }
}
