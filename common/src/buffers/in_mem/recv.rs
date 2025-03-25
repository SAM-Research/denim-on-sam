use async_trait::async_trait;
use log::error;
use prost::Message;
use std::collections::HashMap;
use std::collections::HashSet;
use std::fmt::Display;
use std::hash::Hash;

use crate::buffers::ChunkDecodeError;
use crate::buffers::ReceivingBuffer;
use crate::denim_message::{DeniablePayload, DenimChunk, Flag};

#[derive(Debug)]
pub struct ChunkBuffer {
    chunks: HashMap<u32, Vec<u8>>,
    waiting_for: HashSet<u32>,
}

impl Default for ChunkBuffer {
    fn default() -> Self {
        let mut waiting_for = HashSet::new();
        waiting_for.insert(0);
        ChunkBuffer {
            chunks: Default::default(),
            waiting_for,
        }
    }
}

#[derive(Debug, Default)]
pub struct InMemoryReceivingBuffer<Sender: Eq + Hash> {
    buffers: HashMap<Sender, HashMap<u32, ChunkBuffer>>,
}

#[async_trait]
impl<T: Send + Eq + Hash + Copy + Display> ReceivingBuffer<T> for InMemoryReceivingBuffer<T> {
    async fn process_chunks(
        &mut self,
        sender: T,
        chunks: Vec<DenimChunk>,
    ) -> Vec<Result<DeniablePayload, ChunkDecodeError>> {
        let buffer = self.buffers.entry(sender).or_default();
        let mut messages = Vec::new();
        for chunk in chunks {
            let message_id = chunk.message_id;
            let chunk_buffer = buffer.entry(message_id).or_default();

            if !chunk_buffer.waiting_for.contains(&chunk.sequence_number) {
                todo!("Handle duplicate sequence_number");
            } else {
                chunk_buffer.waiting_for.remove(&chunk.sequence_number);
                let next = chunk.sequence_number + 1;
                chunk_buffer
                    .chunks
                    .insert(chunk.sequence_number, chunk.chunk);
                if chunk.flag != i32::from(Flag::Final) {
                    chunk_buffer.waiting_for.insert(next);
                    println!("next {}", next);
                }
            }
            if chunk_buffer.waiting_for.is_empty() {
                let chunk_buffer = buffer.remove(&message_id).unwrap();

                let mut completed: Vec<(u32, Vec<u8>)> = chunk_buffer.chunks.into_iter().collect();
                completed.sort_by_key(|(seq, _)| *seq);
                let size = completed.len();

                let bytes =
                    completed
                        .into_iter()
                        .fold(Vec::with_capacity(size), |mut acc, (_, bytes)| {
                            acc.extend(bytes);
                            acc
                        });

                let payload = DeniablePayload::decode(bytes.as_slice())
                    .inspect_err(|err| error!("{err}"))
                    .map_err(|_| ChunkDecodeError::new(sender.to_string()));
                messages.push(payload);
            }
        }
        messages
    }
}

#[cfg(test)]
mod test {
    use crate::buffers::{InMemoryReceivingBuffer, ReceivingBuffer};
    use crate::denim_message::{DeniablePayload, DenimChunk, Flag};
    use bon::vec;
    use prost::Message;

    #[tokio::test]
    async fn in_memory_receiving_buffer() {
        _ = env_logger::try_init();
        let mut buffer = InMemoryReceivingBuffer::default();

        let payload = DeniablePayload::builder()
            .message_kind(
                crate::denim_message::deniable_payload::MessageKind::SeedUpdate(
                    crate::denim_message::SeedUpdate {
                        pre_key_seed: vec![1],
                        pq_pre_key_seed: vec![2],
                    },
                ),
            )
            .build();

        let bytes = payload.encode_to_vec();

        let (part1, part2) = bytes.split_at(bytes.len() / 2);

        let chunk1 = DenimChunk::builder()
            .message_id(0)
            .sequence_number(0)
            .chunk(part1.to_vec())
            .flag(Flag::None.into())
            .build();

        let chunk2 = DenimChunk::builder()
            .message_id(0)
            .sequence_number(1)
            .flag(Flag::Final.into())
            .chunk(part2.to_vec())
            .build();

        let actual: Vec<DeniablePayload> = buffer
            .process_chunks(1, vec![chunk1, chunk2])
            .await
            .into_iter()
            .map(|payload| payload.expect("can decode payload"))
            .collect();

        let expect = vec![payload];

        assert!(actual == expect);
    }
}
