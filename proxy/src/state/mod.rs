use crate::managers::traits::{BlockList, KeyRequestManager, MessageIdProvider};
use crate::managers::{BufferManager, DenimKeyManager, DenimKeyManagerType};
use bon::bon;
use denim_sam_common::buffers::{ReceivingBufferConfig, SendingBufferConfig};

use sam_server::managers::traits::{
    account_manager::AccountManager, device_manager::DeviceManager,
};
use std::sync::Arc;

mod in_mem;
mod postgres;

pub use in_mem::InMemoryBufferManagerType;
pub use in_mem::InMemoryDenimStateType;
pub use postgres::PostgresDenimStateType;

pub trait BufferManagerType: 'static + Clone {
    type ReceivingBufferConfig: ReceivingBufferConfig;
    type SendingBufferConfig: SendingBufferConfig;
}

pub trait DenimStateType: 'static + Clone {
    type KeyRequestManager: KeyRequestManager;
    type BufferManager: BufferManagerType;
    type DenimKeyManagerType: DenimKeyManagerType;
    type AccountManager: AccountManager;
    type DeviceManger: DeviceManager;
    type BlockList: BlockList;
    type MessageIdProvider: MessageIdProvider;
}

#[derive(Clone)]
pub struct DenimState<T: DenimStateType> {
    pub key_request_manager: T::KeyRequestManager,
    pub buffer_manager: BufferManager<T::BufferManager>,
    pub message_id_provider: T::MessageIdProvider,
    pub block_list: T::BlockList,
    pub keys: DenimKeyManager<T::DenimKeyManagerType>,
    pub devices: T::DeviceManger,
    pub accounts: T::AccountManager,

    sam_addr: String,
    channel_buffer_size: usize,
    ws_proxy_tls_config: Option<Arc<rustls::ClientConfig>>,
}

#[bon]
impl<T: DenimStateType> DenimState<T> {
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
        message_id_provider: T::MessageIdProvider,
        block_list: T::BlockList,
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
            message_id_provider,
            block_list,
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
    pub fn in_memory_test(sam_addr: String) -> DenimState<InMemoryDenimStateType> {
        use denim_sam_common::buffers::in_mem::{
            InMemoryReceivingBufferConfig, InMemorySendingBufferConfig,
        };
        use sam_server::managers::in_memory::{
            account::InMemoryAccountManager, device::InMemoryDeviceManager,
            keys::InMemorySignedPreKeyManager,
        };

        use crate::managers::in_mem::InMemoryBlockList;
        use crate::managers::{
            in_mem::{InMemoryDenimEcPreKeyManager, InMemoryKeyRequestManager},
            InMemoryMessageIdProvider,
        };
        let rcfg = InMemoryReceivingBufferConfig;
        let scfg = InMemorySendingBufferConfig::default();

        let buffer_mgr = BufferManager::new(rcfg, scfg, 1.0);
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
            .message_id_provider(InMemoryMessageIdProvider::default())
            .block_list(InMemoryBlockList::default())
            .build()
    }
}
