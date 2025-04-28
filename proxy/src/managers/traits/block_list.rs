use sam_common::AccountId;

pub trait BlockList: Send + Sync + Clone {
    fn block_user(&mut self, users_account_id: AccountId, blocked_account_id: AccountId);
    fn check_for_blocked_user(
        &mut self,
        user_account_id: &AccountId,
        blocked_account_id: &AccountId,
    ) -> bool;
}
