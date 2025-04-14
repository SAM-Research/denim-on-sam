use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicU32, Ordering},
        Arc,
    },
};

use async_trait::async_trait;
use denim_sam_common::buffers::MessageId;
use sam_common::AccountId;
use tokio::sync::Mutex;

use crate::managers::traits::MessageIdProvider;

type AtomicMessageId = AtomicU32;

#[derive(Clone, Default)]
pub struct InMemoryMessageIdProvider {
    ids: Arc<Mutex<HashMap<AccountId, AtomicMessageId>>>,
}

#[async_trait]
impl MessageIdProvider for InMemoryMessageIdProvider {
    async fn get_message_id(&mut self, id: AccountId) -> MessageId {
        self.ids
            .lock()
            .await
            .entry(id)
            .or_default()
            .fetch_add(1, Ordering::Relaxed)
    }
}
