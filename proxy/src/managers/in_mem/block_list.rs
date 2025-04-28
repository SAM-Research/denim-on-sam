use std::collections::HashMap;

use sam_common::AccountId;

use crate::managers::traits::BlockList;

#[derive(Clone, Default)]
pub struct InMemoryBlockList {
    block_list: HashMap<AccountId, Vec<AccountId>>,
}

impl BlockList for InMemoryBlockList {
    fn block_user(&mut self, users_account_id: AccountId, blocked_account_id: AccountId) {
        if let Some(vec) = self.block_list.get_mut(&users_account_id) {
            vec.push(blocked_account_id);
        } else {
            self.block_list
                .insert(users_account_id, vec![blocked_account_id]);
        }
    }

    fn check_for_blocked_user(
        &mut self,
        user_account_id: &AccountId,
        blocked_account_id: &AccountId,
    ) -> bool {
        if let Some(list) = self.block_list.get(user_account_id) {
            if list.contains(blocked_account_id) {
                return true;
            }
        }
        false
    }
}
