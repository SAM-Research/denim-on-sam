use async_trait::async_trait;
use denim_sam_common::ChaChaRngState;
use libsignal_protocol::{InMemPreKeyStore, InMemSessionStore};
use sam_client::storage::{InMemoryContactStore, InMemoryMessageStore};

use crate::DenimClientError;

use super::{seed::InMemoryPreKeySeedStore, DeniableStore, DeniableStoreConfig, DeniableStoreType};

pub struct InMemoryDeniableStoreType;

impl DeniableStoreType for InMemoryDeniableStoreType {
    type ContactStore = InMemoryContactStore;
    type MessageStore = InMemoryMessageStore;
    type SessionStore = InMemSessionStore;
    type PreKeyStore = InMemPreKeyStore;
    type SeedStore = InMemoryPreKeySeedStore<ChaChaRngState>;
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
            .contact_store(InMemoryContactStore::default())
            .message_store(InMemoryMessageStore::new(10))
            .seed_store(InMemoryPreKeySeedStore::default())
            .build())
    }
}
