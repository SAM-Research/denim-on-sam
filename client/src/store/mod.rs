use async_trait::async_trait;
use bon::Builder;
use libsignal_protocol::{PreKeyStore, SessionStore};
use sam_client::storage::{ContactStore, MessageStore};

use crate::DenimClientError;

pub mod inmem;
pub use inmem::InMemoryDeniableStoreConfig;

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
    type PreKeyStore: PreKeyStore;
}

#[derive(Builder)]
pub struct DeniableStore<T: DeniableStoreType> {
    pub contact_store: T::ContactStore,
    pub message_store: T::MessageStore,
    pub session_store: T::SessionStore,
    pub pre_key_store: T::PreKeyStore,
}
