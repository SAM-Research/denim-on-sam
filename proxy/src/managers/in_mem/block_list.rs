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
        let mut block_list = self.block_list.lock().await;
        if let Some(vec) = block_list.get_mut(&users_account_id) {
            vec.push(blocked_account_id);
        } else {
            block_list.insert(users_account_id, vec![blocked_account_id]);
        }
    }

    async fn is_user_blocked(
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

#[cfg(test)]
mod test {
    use sam_common::AccountId;

    use crate::managers::{in_mem::InMemoryBlockList, traits::BlockList};

    #[tokio::test]
    async fn can_find_blocked_user() {
        let mut block_list = InMemoryBlockList::default();

        let users = vec![AccountId::generate(), AccountId::generate()];
        let blocked_users = vec![
            AccountId::generate(),
            AccountId::generate(),
            AccountId::generate(),
        ];

        for user in users.clone() {
            for blocked_user in blocked_users.clone() {
                block_list.block_user(user, blocked_user).await;
            }
        }

        for user in users {
            for blocked_user in blocked_users.clone() {
                assert!(block_list.is_user_blocked(&user, &blocked_user).await)
            }
        }
    }

    #[tokio::test]
    async fn cannot_find_not_blocked_user() {
        let block_list = InMemoryBlockList::default();

        let users = vec![AccountId::generate(), AccountId::generate()];
        let not_blocked_users = vec![
            AccountId::generate(),
            AccountId::generate(),
            AccountId::generate(),
        ];

        for user in users {
            for blocked_user in not_blocked_users.clone() {
                assert!(!block_list.is_user_blocked(&user, &blocked_user).await)
            }
        }
    }
}
