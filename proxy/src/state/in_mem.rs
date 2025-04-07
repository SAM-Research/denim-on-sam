use sam_server::managers::in_memory::{
    account::InMemoryAccountManager, device::InMemoryDeviceManager,
};

use crate::managers::in_mem::{InMemoryBufferManager, InMemoryDenimKeyManager};

use super::StateType;

#[derive(Clone)]
pub struct InMemory;

impl StateType for InMemory {
    type BufferManager = InMemoryBufferManager;

    type DenimKeyManagerType = InMemoryDenimKeyManager;

    type AccountManager = InMemoryAccountManager;

    type DeviceManger = InMemoryDeviceManager;
}
