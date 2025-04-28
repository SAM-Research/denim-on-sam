use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;
use sam_common::AccountId;
use tokio::sync::Mutex;

use crate::managers::traits::KeyRequestManager;

#[derive(Clone, Default)]
pub struct InMemoryKeyRequestManager {
    requests: Arc<Mutex<HashMap<AccountId, Vec<AccountId>>>>,
}

#[async_trait]
impl KeyRequestManager for InMemoryKeyRequestManager {
    async fn store_receiver(&mut self, sender: AccountId, receiver: AccountId) {
        if let Some(vec) = self.requests.lock().await.get_mut(&sender) {
            vec.push(receiver);
        } else {
            self.requests.lock().await.insert(sender, vec![receiver]);
        }
    }

    async fn get_receivers(&mut self, account_id: AccountId) -> Option<Vec<AccountId>> {
        self.requests.lock().await.remove(&account_id)
    }
}
