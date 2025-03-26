use async_trait::async_trait;

use crate::{buffers::DeniablePayload, LibError};

#[async_trait]
pub trait SendingBuffer {
    async fn get_deniable_payload(
        &mut self,
        reg_message_len: u32,
    ) -> Result<Option<DeniablePayload>, LibError>;
}
