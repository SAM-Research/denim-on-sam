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
    async fn store_requester(&mut self, requested: AccountId, requester: AccountId) {
        let mut requests = self.requests.lock().await;
        if let Some(vec) = requests.get_mut(&requested) {
            vec.push(requester);
        } else {
            requests.insert(requested, vec![requester]);
        }
    }

    async fn remove_requesters(&mut self, requested: AccountId) -> Option<Vec<AccountId>> {
        self.requests.lock().await.remove(&requested)
    }
}

#[cfg(test)]
mod test {
    use sam_common::AccountId;

    use crate::managers::{in_mem::InMemoryKeyRequestManager, traits::KeyRequestManager};

    #[tokio::test]
    async fn can_get_stored_requesters() {
        let mut request_manager = InMemoryKeyRequestManager::default();

        let requested_accounts = vec![AccountId::generate(), AccountId::generate()];
        let requester_accounts = vec![AccountId::generate(); 32];

        // accounts requests keys from other accounts that have not uploaded seed
        for requested in requested_accounts.clone() {
            for requester in requester_accounts.clone() {
                request_manager.store_requester(requested, requester).await;
            }
        }

        // requested accounts upload their seed
        for requested in requested_accounts {
            let receivers = request_manager
                .remove_requesters(requested)
                .await
                .expect("Should contain receivers");
            for inserted_receiver in requester_accounts.clone() {
                assert!(receivers.contains(&inserted_receiver))
            }
        }
    }

    #[tokio::test]
    async fn can_get_none_requesters() {
        let mut request_manager = InMemoryKeyRequestManager::default();

        let requested_accounts = vec![AccountId::generate(), AccountId::generate()];

        for requested in requested_accounts {
            let receivers = request_manager.remove_requesters(requested).await;
            assert_eq!(receivers, None)
        }
    }
}
