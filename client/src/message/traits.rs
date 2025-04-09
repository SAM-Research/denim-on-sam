use async_trait::async_trait;
use sam_common::AccountId;

#[async_trait]
pub trait MessageQueue {
    async fn enqueue(&mut self, account_id: AccountId, msg: Vec<u8>);
    async fn dequeue(&mut self, account_id: AccountId) -> Option<Vec<u8>>;
    async fn len(&mut self, account_id: AccountId) -> usize;
}

#[async_trait]
pub trait MessageQueueConfig {
    type MessageQueue: MessageQueue;

    async fn create(self) -> Self::MessageQueue;
}
