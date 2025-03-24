use std::hash::Hash;
use std::{collections::HashMap, mem::take};

use prost::Message;

use crate::denim_message::{DeniablePayload, DenimChunk, Flag};

#[derive(Debug, Default)]
pub struct InMemoryReceivingBuffer<T: Eq + Hash> {
    buffers: HashMap<T, Vec<DenimChunk>>,
}

impl<T: Eq + Hash> ReceivingBuffer<T> for InMemoryReceivingBuffer<T> {
    async fn process_chunks(
        &mut self,
        sender: T,
        mut chunks: Vec<DenimChunk>,
    ) -> Vec<DeniablePayload> {
        let completed: Vec<u32> = chunks
            .iter()
            .filter(|chunk| chunk.flag.is_some_and(|flag| flag == Flag::Final.into()))
            .map(|chunk| chunk.message_id)
            .collect();

        let buffer = self.buffers.entry(sender).or_default();
        buffer.append(&mut chunks);

        let mut messages = Vec::new();
        for message_id in completed {
            let (mut message_chunks, chunks): (Vec<_>, Vec<_>) = take(buffer)
                .into_iter()
                .partition(|chunk| chunk.message_id == message_id);
            buffer.extend(chunks);

            message_chunks.sort_unstable_by(|a, b| a.sequence_number.cmp(&b.sequence_number));
            // Todo: check missing sequence numbers
            // Todo: check duplicate sequence numbers

            let message_bytes: Vec<u8> = message_chunks
                .into_iter()
                .map(|chunk| chunk.chunk)
                .collect::<Vec<Vec<u8>>>()
                .concat();

            let message = DeniablePayload::decode(message_bytes.as_slice()).unwrap();
            messages.push(message);
        }

        messages
    }
}

pub trait ReceivingBuffer<T: Eq + Hash> {
    async fn process_chunks(&mut self, sender: T, chunks: Vec<DenimChunk>) -> Vec<DeniablePayload>;
}

#[cfg(test)]
mod test {
    use crate::{
        denim_message::{DeniablePayload, DenimChunk, Flag},
        receiving_buffer::{InMemoryReceivingBuffer, ReceivingBuffer},
    };
    use bon::vec;
    use prost::Message;

    #[tokio::test]
    async fn in_memory_receiving_buffer() {
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
            .build();

        let chunk2 = DenimChunk::builder()
            .message_id(0)
            .sequence_number(1)
            .flag(Flag::Final.into())
            .chunk(part2.to_vec())
            .build();

        let actual = buffer.process_chunks(1, vec![chunk1, chunk2]).await;
        let expect = vec![payload];

        assert!(actual == expect);
    }
}
