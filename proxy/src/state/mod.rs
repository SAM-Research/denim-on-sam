use crate::managers::traits::BlockList;
use crate::managers::traits::CryptoProvider;
use crate::managers::traits::KeyRequestManager;
use crate::managers::{
    traits::MessageIdProvider, BufferManager, DenimKeyManager, DenimKeyManagerType,
};
use bon::bon;
use denim_sam_common::buffers::{ReceivingBufferConfig, SendingBufferConfig};

use sam_server::managers::traits::{
    account_manager::AccountManager, device_manager::DeviceManager,
};
use std::sync::Arc;

mod in_mem;
mod postgres;

pub use in_mem::InMemoryBufferManagerType;
pub use in_mem::InMemoryStateType;
pub use postgres::PostgresStateType;

pub trait BufferManagerType: 'static + Clone {
    type BlockList: BlockList;
    type ReceivingBufferConfig: ReceivingBufferConfig;
    type SendingBufferConfig: SendingBufferConfig;
    type MessageIdProvider: MessageIdProvider;
}

pub trait StateType: 'static + Clone {
    type KeyRequestManager: KeyRequestManager;
    type BufferManager: BufferManagerType;
    type DenimKeyManagerType: DenimKeyManagerType;
    type AccountManager: AccountManager;
    type DeviceManger: DeviceManager;
    type CryptoProvider: CryptoProvider;
}

#[derive(Clone)]
pub struct DenimState<T: StateType> {
    pub key_request_manager: T::KeyRequestManager,
    pub buffer_manager: BufferManager<T::BufferManager>,
    pub keys: DenimKeyManager<T::DenimKeyManagerType>,
    pub devices: T::DeviceManger,
    pub accounts: T::AccountManager,

    sam_addr: String,
    channel_buffer_size: usize,
    ws_proxy_tls_config: Option<Arc<rustls::ClientConfig>>,
}

#[bon]
impl<T: StateType> DenimState<T> {
    #[builder]
    pub fn new(
        sam_addr: String,
        channel_buffer_size: usize,
        ws_proxy_tls_config: Option<rustls::ClientConfig>,
        buffer_manager: BufferManager<T::BufferManager>,
        keys: DenimKeyManager<T::DenimKeyManagerType>,
        accounts: T::AccountManager,
        devices: T::DeviceManger,
        key_request_manager: T::KeyRequestManager,
    ) -> Self {
        Self {
            key_request_manager,
            sam_addr,
            channel_buffer_size,
            ws_proxy_tls_config: ws_proxy_tls_config.map(Arc::new),
            buffer_manager,
            keys,
            devices,
            accounts,
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
            in_mem::{InMemoryBlockList, InMemoryDenimEcPreKeyManager, InMemoryKeyRequestManager},
            InMemoryMessageIdProvider,
        };
        let rcfg = InMemoryReceivingBufferConfig;
        let scfg = InMemorySendingBufferConfig::default();
        let id_provider = InMemoryMessageIdProvider::default();
        let buffer_mgr =
            BufferManager::new(InMemoryBlockList::default(), rcfg, scfg, id_provider, 1.0);

        DenimState::builder()
            .sam_addr(sam_addr.to_string())
            .channel_buffer_size(10)
            .buffer_manager(buffer_mgr)
            .keys(DenimKeyManager::new(
                InMemoryDenimEcPreKeyManager::default(),
                InMemorySignedPreKeyManager::default(),
            ))
            .accounts(InMemoryAccountManager::default())
            .devices(InMemoryDeviceManager::new("Test".to_owned(), 120))
            .key_request_manager(InMemoryKeyRequestManager::default())
            .build()
    }
}
