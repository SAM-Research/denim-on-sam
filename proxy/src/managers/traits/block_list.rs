use sam_common::AccountId;

pub trait BlockListManager: Send + Sync + Clone {
    fn add_to_block_list(&mut self, users_account_id: AccountId, blocked_account_id: AccountId);
    fn get_list_for_user(&mut self, account_id: &AccountId) -> Option<&Vec<AccountId>>;
}
