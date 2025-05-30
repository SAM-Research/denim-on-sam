use denim_sam_common::buffers::in_mem::{
    InMemoryReceivingBufferConfig, InMemorySendingBufferConfig,
};

use sam_server::managers::in_memory::{
    account::InMemoryAccountManager, device::InMemoryDeviceManager,
};

use crate::managers::in_mem::{InMemoryBlockList, InMemoryKeyRequestManager};
use crate::managers::{in_mem::InMemoryDenimKeyManager, InMemoryMessageIdProvider};

use super::{BufferManagerType, DenimStateType};

#[derive(Debug, Clone)]
pub struct InMemoryBufferManagerType;

impl BufferManagerType for InMemoryBufferManagerType {
    type ReceivingBufferConfig = InMemoryReceivingBufferConfig;

    type SendingBufferConfig = InMemorySendingBufferConfig;
}

#[derive(Clone)]
pub struct InMemoryDenimStateType;

impl DenimStateType for InMemoryDenimStateType {
    type BufferManager = InMemoryBufferManagerType;

    type DenimKeyManagerType = InMemoryDenimKeyManager;

    type AccountManager = InMemoryAccountManager;

    type DeviceManger = InMemoryDeviceManager;

    type KeyRequestManager = InMemoryKeyRequestManager;
    type MessageIdProvider = InMemoryMessageIdProvider;
    type BlockList = InMemoryBlockList;
}
