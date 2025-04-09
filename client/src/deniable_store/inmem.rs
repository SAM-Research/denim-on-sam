use async_trait::async_trait;
use libsignal_protocol::{InMemPreKeyStore, InMemSessionStore};

use crate::DenimClientError;

use super::{DeniableStore, DeniableStoreConfig, DeniableStoreType};

pub struct InMemoryDeniableStoreType;

impl DeniableStoreType for InMemoryDeniableStoreType {
    type SessionStore = InMemSessionStore;

    type PreKeyStore = InMemPreKeyStore;
}

pub type InMemoryDeniableStore = DeniableStore<InMemoryDeniableStoreType>;

#[derive(Debug, Default)]
pub struct InMemoryDeniableStoreConfig {}

#[async_trait]
impl DeniableStoreConfig for InMemoryDeniableStoreConfig {
    type DeniableStoreType = InMemoryDeniableStoreType;

    async fn create_store(
        self,
    ) -> Result<DeniableStore<Self::DeniableStoreType>, DenimClientError> {
        Ok(InMemoryDeniableStore::builder()
            .session_store(InMemSessionStore::default())
            .pre_key_store(InMemPreKeyStore::default())
            .build())
    }
}
