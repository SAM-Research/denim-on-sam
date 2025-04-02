use sam_server::managers::in_memory::account::InMemoryAccountManager;

use crate::managers::in_mem::{InMemoryBufferManager, InMemoryKeyDistributionCenter};

use super::StateType;

#[derive(Clone)]
pub struct InMemory;

impl StateType for InMemory {
    type BufferManager = InMemoryBufferManager;

    type KeyDistributionCenter = InMemoryKeyDistributionCenter;

    type AccountManager = InMemoryAccountManager;
}
