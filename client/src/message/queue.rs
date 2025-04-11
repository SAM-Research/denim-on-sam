use std::{
    collections::{HashMap, VecDeque},
    sync::Arc,
};

use async_trait::async_trait;
use futures_util::lock::Mutex;
use sam_common::AccountId;

use super::traits::{MessageQueue, MessageQueueConfig};

type Messages = HashMap<AccountId, VecDeque<Vec<u8>>>;
#[derive(Default)]
pub struct InMemoryMessageQueue {
    messages: Arc<Mutex<Messages>>,
}

#[derive(Default)]
pub struct InMemoryMessageQueueConfig;

#[async_trait]
impl MessageQueueConfig for InMemoryMessageQueueConfig {
    type MessageQueue = InMemoryMessageQueue;
    async fn create(self) -> Self::MessageQueue {
        InMemoryMessageQueue::default()
    }
}

fn create_bucket(messages: &mut Messages, account_id: AccountId) -> &mut VecDeque<Vec<u8>> {
    messages.entry(account_id).or_default()
}

#[async_trait]
impl MessageQueue for InMemoryMessageQueue {
    async fn enqueue(&mut self, account_id: AccountId, msg: Vec<u8>) {
        let mut messages = self.messages.lock().await;
        create_bucket(&mut messages, account_id).push_back(msg)
    }

    async fn dequeue(&mut self, account_id: AccountId) -> Option<Vec<u8>> {
        let mut messages = self.messages.lock().await;
        create_bucket(&mut messages, account_id).pop_front()
    }

    async fn len(&mut self, account_id: AccountId) -> usize {
        let mut messages = self.messages.lock().await;
        create_bucket(&mut messages, account_id).len()
    }
}

#[cfg(test)]
mod test {
    use sam_common::AccountId;

    use crate::message::{queue::InMemoryMessageQueue, traits::MessageQueue};

    #[tokio::test]
    async fn inmem_msg_queue_enqueue_dequeue() {
        let mut msg_q = InMemoryMessageQueue::default();
        let account_id = AccountId::generate();
        msg_q.enqueue(account_id, vec![1, 3, 3, 7]).await;
        let msg = msg_q.dequeue(account_id).await.expect("is some");
        assert!(msg == vec![1, 3, 3, 7])
    }
}
