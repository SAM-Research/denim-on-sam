use std::collections::HashMap;

use sam_common::AccountId;

use crate::managers::traits::BlockListManager;

#[derive(Clone, Default)]
pub struct InMemoryBlockListManager {
    block_list: HashMap<AccountId, Vec<AccountId>>,
}

impl BlockListManager for InMemoryBlockListManager {
    fn add_to_block_list(&mut self, users_account_id: AccountId, blocked_account_id: AccountId) {
        if let Some(vec) = self.block_list.get_mut(&users_account_id) {
            vec.push(blocked_account_id);
        } else {
            self.block_list
                .insert(users_account_id, vec![blocked_account_id]);
        }
    }

    fn get_list_for_user(&mut self, account_id: &AccountId) -> Option<&Vec<AccountId>> {
        self.block_list.get(account_id)
    }
}
