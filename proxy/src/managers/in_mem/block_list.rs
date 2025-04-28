use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;
use sam_common::AccountId;
use tokio::sync::Mutex;

use crate::managers::traits::BlockList;

#[derive(Clone, Default)]
pub struct InMemoryBlockList {
    block_list: Arc<Mutex<HashMap<AccountId, Vec<AccountId>>>>,
}

#[async_trait]
impl BlockList for InMemoryBlockList {
    async fn block_user(&mut self, users_account_id: AccountId, blocked_account_id: AccountId) {
        if let Some(vec) = self.block_list.lock().await.get_mut(&users_account_id) {
            vec.push(blocked_account_id);
        } else {
            self.block_list
                .lock()
                .await
                .insert(users_account_id, vec![blocked_account_id]);
        }
    }

    async fn check_for_blocked_user(
        &self,
        user_account_id: &AccountId,
        blocked_account_id: &AccountId,
    ) -> bool {
        if let Some(list) = self.block_list.lock().await.get(user_account_id) {
            return list.contains(blocked_account_id);
        }
        false
    }
}
