use async_trait::async_trait;
use sam_common::AccountId;

#[async_trait]
pub trait BlockList: Send + Sync + Clone {
    async fn block_user(&mut self, users_account_id: AccountId, blocked_account_id: AccountId);
    async fn is_user_blocked(
        &self,
        user_account_id: &AccountId,
        blocked_account_id: &AccountId,
    ) -> bool;
}
