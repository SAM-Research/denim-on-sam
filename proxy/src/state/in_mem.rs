use denim_sam_common::buffers::in_mem::{
    InMemoryReceivingBufferConfig, InMemorySendingBufferConfig,
};

use sam_server::managers::in_memory::{
    account::InMemoryAccountManager, device::InMemoryDeviceManager,
};

use crate::managers::{
    default::ChaChaCryptoProvider,
    in_mem::{InMemoryDenimKeyManager, InMemoryKeyRequestManager},
    InMemoryMessageIdProvider,
};

use super::{BufferManagerType, StateType};

#[derive(Debug, Clone)]
pub struct InMemoryBufferManagerType;

impl BufferManagerType for InMemoryBufferManagerType {
    type ReceivingBufferConfig = InMemoryReceivingBufferConfig;

    type SendingBufferConfig = InMemorySendingBufferConfig;

    type MessageIdProvider = InMemoryMessageIdProvider;
}

#[derive(Clone)]
pub struct InMemoryStateType;

impl StateType for InMemoryStateType {
    type BufferManager = InMemoryBufferManagerType;

    type DenimKeyManagerType = InMemoryDenimKeyManager;

    type AccountManager = InMemoryAccountManager;

    type DeviceManger = InMemoryDeviceManager;

    type CryptoProvider = ChaChaCryptoProvider;

    type KeyRequestManager = InMemoryKeyRequestManager;
}
