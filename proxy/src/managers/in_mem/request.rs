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
        let mut requests = self.requests.lock().await;
        if let Some(vec) = requests.get_mut(&sender) {
            vec.push(receiver);
        } else {
            requests.insert(sender, vec![receiver]);
        }
    }

    async fn get_receivers(&mut self, account_id: AccountId) -> Option<Vec<AccountId>> {
        self.requests.lock().await.remove(&account_id)
    }
}

#[cfg(test)]
mod test {
    use sam_common::AccountId;

    use crate::managers::{in_mem::InMemoryKeyRequestManager, traits::KeyRequestManager};

    #[tokio::test]
    async fn can_get_stored_receivers() {
        let mut request_manager = InMemoryKeyRequestManager::default();

        let senders = vec![AccountId::generate(), AccountId::generate()];
        let inserted_receivers = vec![AccountId::generate(); 32];

        for sender in senders.clone() {
            for receiver in inserted_receivers.clone() {
                request_manager.store_receiver(sender, receiver).await;
            }
        }

        for sender in senders {
            let receivers = request_manager
                .get_receivers(sender)
                .await
                .expect("Should contain receivers");
            for inserted_receiver in inserted_receivers.clone() {
                assert!(receivers.contains(&inserted_receiver))
            }
        }
    }

    #[tokio::test]
    async fn can_get_none_receivers() {
        let mut request_manager = InMemoryKeyRequestManager::default();

        let senders = vec![AccountId::generate(), AccountId::generate()];

        for sender in senders {
            let receivers = request_manager.get_receivers(sender).await;
            assert_eq!(receivers, None)
        }
    }
}
