use crate::buffers::DenimChunk;
use crate::buffers::Flag;
use crate::buffers::MessageId;
use crate::buffers::ReceivingBuffer;
use crate::buffers::ReceivingBufferConfig;
use crate::buffers::SequenceNumber;
use crate::denim_message::DeniableMessage;
use crate::error::DenimBufferError;
use crate::error::DenimEncodeDecodeError;
use async_trait::async_trait;
use log::{debug, error};

use prost::Message as _;
use std::collections::HashMap;
use std::collections::HashSet;

use std::mem::take;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Debug)]
pub struct ChunkBuffer {
    chunks: Arc<Mutex<HashMap<SequenceNumber, Vec<u8>>>>,
    waiting_for: Arc<Mutex<HashSet<SequenceNumber>>>,
}

impl Default for ChunkBuffer {
    fn default() -> Self {
        let mut waiting_for = HashSet::new();
        waiting_for.insert(0);
        ChunkBuffer {
            chunks: Arc::new(Mutex::new(Default::default())),
            waiting_for: Arc::new(Mutex::new(waiting_for)),
        }
    }
}

#[derive(Clone, Default)]
pub struct InMemoryReceivingBufferConfig;

#[async_trait]
impl ReceivingBufferConfig for InMemoryReceivingBufferConfig {
    type Buffer = InMemoryReceivingBuffer;
    async fn create(&self) -> Result<InMemoryReceivingBuffer, DenimBufferError> {
        Ok(InMemoryReceivingBuffer::default())
    }
}

#[derive(Clone, Debug, Default)]
pub struct InMemoryReceivingBuffer {
    buffers: Arc<Mutex<HashMap<MessageId, ChunkBuffer>>>,
}

#[async_trait]
impl ReceivingBuffer for InMemoryReceivingBuffer {
    async fn process_chunks(
        &mut self,
        chunks: Vec<DenimChunk>,
    ) -> Vec<Result<DeniableMessage, DenimBufferError>> {
        let mut messages = Vec::new();
        for mut chunk in chunks {
            if chunk.flag() == Flag::DummyPadding {
                continue;
            }
            let message_id = chunk.message_id();
            let mut buffer_guard = self.buffers.lock().await;
            let chunk_buffer = buffer_guard.entry(message_id).or_default();
            let seq = chunk.sequence_number();

            if !chunk_buffer.waiting_for.lock().await.contains(&seq) {
                for id in 0..seq {
                    if !chunk_buffer.chunks.lock().await.contains_key(&id) {
                        chunk_buffer.waiting_for.lock().await.insert(id);
                    }
                }
                let next = chunk.sequence_number() + 1;
                if chunk.flag() != Flag::Final
                    && !chunk_buffer.chunks.lock().await.contains_key(&next)
                {
                    chunk_buffer.waiting_for.lock().await.insert(next);
                }
                debug!(
                    "Received message chunk {:?} out of order for message {:?}. Waiting for {:?}",
                    chunk.sequence_number(),
                    chunk.message_id(),
                    chunk_buffer.waiting_for
                )
            } else {
                chunk_buffer.waiting_for.lock().await.remove(&seq);
                let next = seq + 1;
                if chunk.flag() != Flag::Final
                    && !chunk_buffer.chunks.lock().await.contains_key(&next)
                {
                    chunk_buffer.waiting_for.lock().await.insert(next);
                }
            }
            chunk_buffer
                .chunks
                .lock()
                .await
                .insert(seq, take(chunk.chunk_mut()));

            debug!(
                "Message id {:?}: Received Chunks {:?}, waiting for {:?}",
                chunk.message_id(),
                chunk_buffer.chunks.lock().await.keys(),
                chunk_buffer.waiting_for
            );

            if !chunk_buffer.waiting_for.lock().await.is_empty() {
                continue;
            }

            let chunk_buffer = match buffer_guard.remove(&message_id) {
                Some(x) => x,
                None => {
                    messages.push(Err(DenimBufferError::ChunkBufferNotFound));
                    continue;
                }
            };

            let mut completed: Vec<(u32, Vec<u8>)> =
                chunk_buffer.chunks.lock().await.drain().collect();
            completed.sort_by_key(|(seq, _)| *seq);
            let size = completed.len();
            debug!(
                "Completed message with id {:?}: chunks: {:?}",
                chunk.message_id(),
                completed
            );

            let bytes =
                completed
                    .into_iter()
                    .fold(Vec::with_capacity(size), |mut acc, (_, bytes)| {
                        acc.extend(bytes);
                        acc
                    });

            let payload = DeniableMessage::decode(bytes.as_slice())
                .inspect_err(|err| error!("{err}"))
                .map_err(|_| DenimEncodeDecodeError::DeniableMessageDecode.into());
            messages.push(payload);
        }
        messages
    }
}

#[cfg(test)]
mod test {
    use crate::{
        buffers::{DenimChunk, Flag, InMemoryReceivingBuffer, ReceivingBuffer},
        denim_message::{deniable_message::MessageKind, DeniableMessage},
    };
    use bon::vec;
    use prost::Message;

    fn payload() -> DeniableMessage {
        DeniableMessage::builder()
            .message_id(0)
            .message_kind(MessageKind::SeedUpdate(crate::denim_message::SeedUpdate {
                pre_key_seed: vec![1],
                pre_key_id_seed: vec![1],
            }))
            .build()
    }

    fn chunks() -> (DenimChunk, DenimChunk) {
        let bytes = payload().encode_to_vec();

        let (part1, part2) = bytes.split_at(bytes.len() / 2);

        let chunk1 = DenimChunk::builder()
            .message_id(0)
            .sequence_number(0)
            .chunk(part1.to_vec())
            .flag(Flag::None)
            .build();

        let chunk2 = DenimChunk::builder()
            .message_id(0)
            .sequence_number(1)
            .flag(Flag::Final)
            .chunk(part2.to_vec())
            .build();

        (chunk1, chunk2)
    }

    #[tokio::test]
    async fn in_memory_receiving_buffer() {
        let mut buffer = InMemoryReceivingBuffer::default();

        let (chunk1, chunk2) = chunks();

        let actual: Vec<DeniableMessage> = buffer
            .process_chunks(vec![chunk1, chunk2])
            .await
            .into_iter()
            .map(|payload| payload.expect("can decode payload"))
            .collect();

        let expect = vec![payload()];

        assert!(actual == expect);
    }

    #[tokio::test]
    async fn out_of_order() {
        let mut buffer = InMemoryReceivingBuffer::default();

        let (chunk1, chunk2) = chunks();

        let actual: Vec<DeniableMessage> = buffer
            .process_chunks(vec![chunk2, chunk1])
            .await
            .into_iter()
            .map(|payload| payload.expect("can decode payload"))
            .collect();

        let expect = vec![payload()];

        assert!(actual == expect);
    }
}
