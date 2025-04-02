use async_trait::async_trait;

use crate::denim_message::DeniableMessage;
use crate::{buffers::DeniablePayload, LibError};

#[async_trait]
pub trait SendingBuffer: Send + 'static {
    async fn get_deniable_payload(
        &mut self,
        reg_message_len: u32,
    ) -> Result<Option<DeniablePayload>, LibError>;

    async fn enqueue_message(&mut self, deniable_message: DeniableMessage);
}
