use async_trait::async_trait;
use derive_more::{Display, Error};
use std::hash::Hash;

use crate::{buffers::DenimChunk, denim_message::DeniableMessage};

#[derive(Debug, Display, Error)]
#[display("Message from {sender} failed to be decoded")]
pub struct ChunkDecodeError {
    #[error(not(source))]
    sender: String,
}

impl ChunkDecodeError {
    pub fn new(sender: String) -> Self {
        Self { sender }
    }
}

#[async_trait]
pub trait ReceivingBuffer<T: Eq + Hash> {
    async fn process_chunks(
        &mut self,
        sender: T,
        chunks: Vec<DenimChunk>,
    ) -> Vec<Result<DeniableMessage, ChunkDecodeError>>;
}
