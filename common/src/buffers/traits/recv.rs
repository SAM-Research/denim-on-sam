use async_trait::async_trait;

use crate::{buffers::DenimChunk, denim_message::DeniableMessage, error::DenimBufferError};

#[async_trait]
pub trait ReceivingBuffer: Clone + Send + Sync + 'static {
    async fn process_chunks(
        &mut self,
        chunks: Vec<DenimChunk>,
    ) -> Vec<Result<DeniableMessage, DenimBufferError>>;
}

#[async_trait]
pub trait ReceivingBufferConfig: Send + Sync + Clone + 'static {
    type Buffer: ReceivingBuffer;
    async fn create(&self) -> Result<Self::Buffer, DenimBufferError>;
}
