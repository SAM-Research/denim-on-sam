use async_trait::async_trait;
use bon::Builder;
use denim_sam_common::rng::chacha::ChaChaRngState;
use libsignal_protocol::{PreKeyId, PreKeyStore, SessionStore};
use sam_client::storage::{ContactStore, MessageStore, ProvidesKeyId};

use crate::DenimClientError;

pub mod inmem;
pub use inmem::InMemoryDeniableStoreConfig;
mod seed;
pub use seed::{DenimPreKeySeedStore, InMemoryPreKeySeedStore, SeedStoreError};

#[async_trait]
pub trait DeniableStoreConfig {
    type DeniableStoreType: DeniableStoreType;

    async fn create_store(self)
        -> Result<DeniableStore<Self::DeniableStoreType>, DenimClientError>;
}

pub trait DeniableStoreType {
    type ContactStore: ContactStore;
    type MessageStore: MessageStore;
    type SessionStore: SessionStore;
    type PreKeyStore: PreKeyStore + ProvidesKeyId<PreKeyId>;
    type SeedStore: DenimPreKeySeedStore<ChaChaRngState>;
}

#[derive(Builder)]
pub struct DeniableStore<T: DeniableStoreType> {
    pub contact_store: T::ContactStore,
    pub message_store: T::MessageStore,
    pub session_store: T::SessionStore,
    pub pre_key_store: T::PreKeyStore,
    pub seed_store: T::SeedStore,
}
