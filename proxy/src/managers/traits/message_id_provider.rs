use async_trait::async_trait;
use denim_sam_common::buffers::MessageId;
use sam_common::AccountId;

#[async_trait]
pub trait MessageIdProvider: Send + Sync + Clone + 'static {
    async fn get_message_id(&mut self, id: AccountId) -> MessageId;
}
