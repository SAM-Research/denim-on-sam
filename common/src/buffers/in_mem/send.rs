use crate::buffers::{DeniablePayload, DenimChunk, Flag, SendingBuffer};
use crate::denim_message::DeniableMessage;
use crate::error::LibError;
use async_trait::async_trait;
use bincode::config;
use log::info;
use prost::Message;
use rand::RngCore;
use std::collections::VecDeque;
use std::mem::take;

const DENIM_CHUNK_WITHOUT_PAYLOAD: usize = 4;

const DENIABLE_PAYLOAD_MIN_LENGTH: usize = 20;

struct Buffer {
    content: Vec<u8>,
    message_id: u32,
    next_sequence_number: u8,
}

pub struct InMemorySendingBuffer {
    q: f32,
    outgoing_messages: VecDeque<DeniableMessage>,
    buffer: Buffer,
}

#[async_trait]
impl SendingBuffer for InMemorySendingBuffer {
    async fn get_deniable_payload(
        &mut self,
        reg_message_len: u32,
    ) -> Result<Option<DeniablePayload>, LibError> {
        if self.q == 0.0 {
            return Ok(None);
        }

        let mut available_bytes = self.calculate_deniable_payload_length(reg_message_len);

        if available_bytes < DENIABLE_PAYLOAD_MIN_LENGTH {
            return Ok(Some(
                DeniablePayload::builder()
                    .denim_chunks(vec![])
                    .garbage(InMemorySendingBuffer::create_n_random_bytes(
                        available_bytes,
                    ))
                    .build(),
            ));
        }

        let mut denim_chunks: Vec<Vec<u8>> = Vec::new();

        while available_bytes > DENIM_CHUNK_WITHOUT_PAYLOAD {
            let deniable_payload_len = available_bytes - DENIM_CHUNK_WITHOUT_PAYLOAD;

            let chunk = self.get_next_chunk(deniable_payload_len);

            match chunk {
                None => {
                    break;
                }
                Some(chunk) => {
                    let encoded_chunk = bincode::encode_to_vec(chunk, config::standard())
                        .map_err(|_| LibError::ChunkEncode)?;
                    available_bytes -= encoded_chunk.len();

                    self.buffer.next_sequence_number += 1; // todo() do this inside get_next_chunk
                    denim_chunks.push(encoded_chunk);
                }
            }
        }
        if available_bytes >= DENIM_CHUNK_WITHOUT_PAYLOAD {
            // Send deniable payload
            let dummy_chunk_length = available_bytes - DENIM_CHUNK_WITHOUT_PAYLOAD;

            let dummy_chunk = self.create_dummy_chunk(dummy_chunk_length);
            let encoded_chunk = bincode::encode_to_vec(dummy_chunk, config::standard())
                .map_err(|_| LibError::ChunkEncode)?;
            available_bytes -= encoded_chunk.len();
            denim_chunks.push(encoded_chunk);
        }

        if available_bytes > 0 {
            let mut random_bytes = vec![0u8; available_bytes];
            rand::rng().fill_bytes(&mut random_bytes);
            return Ok(Some(
                DeniablePayload::builder()
                    .denim_chunks(denim_chunks)
                    .garbage(random_bytes)
                    .build(),
            ));
        }

        Ok(Some(
            DeniablePayload::builder()
                .denim_chunks(denim_chunks)
                .build(),
        ))
    }
}

impl InMemorySendingBuffer {
    pub fn new(q: f32, outgoing_messages: VecDeque<DeniableMessage>) -> Self {
        Self {
            q,
            outgoing_messages,
            buffer: Buffer {
                content: vec![],
                message_id: 0,
                next_sequence_number: 0,
            },
        }
    }
    fn calculate_deniable_payload_length(&self, reg_message_len: u32) -> usize {
        (reg_message_len as f32 * self.q).ceil() as usize
    }

    fn get_next_chunk(&mut self, available_bytes: usize) -> Option<DenimChunk> {
        if self.buffer.content.is_empty() {
            self.buffer = match self.outgoing_messages.pop_front() {
                None => return None,
                Some(message) => Buffer {
                    content: message.encode_to_vec(),
                    message_id: message.message_id,
                    next_sequence_number: 0,
                },
            }
        }
        if available_bytes >= self.buffer.content.len() {
            return Some(
                DenimChunk::builder()
                    .message_id(self.buffer.message_id)
                    .sequence_number(self.buffer.next_sequence_number as u32)
                    .flag(Flag::Final)
                    .chunk(take(&mut self.buffer.content))
                    .build(),
            );
        };

        let next_chunk: Vec<u8> = self.buffer.content.drain(..available_bytes).collect();

        Some(
            DenimChunk::builder()
                .message_id(self.buffer.message_id)
                .sequence_number(self.buffer.next_sequence_number as u32)
                .flag(Flag::None)
                .chunk(next_chunk)
                .build(),
        )
    }

    fn create_dummy_chunk(&self, available_bytes: usize) -> DenimChunk {
        let random_bytes = InMemorySendingBuffer::create_n_random_bytes(available_bytes);

        DenimChunk::builder()
            .chunk(random_bytes)
            .flag(Flag::DummyPadding)
            .sequence_number(0)
            .message_id(0)
            .build()
    }

    fn create_n_random_bytes(n: usize) -> Vec<u8> {
        let mut random_bytes = vec![0u8; n];
        rand::rng().fill_bytes(&mut random_bytes);
        random_bytes
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::denim_message::deniable_message::MessageKind;
    use crate::denim_message::{MessageType, UserMessage};
    use rand::RngCore;
    use rstest::rstest;

    fn make_deniable_messages(lengths: Vec<usize>) -> VecDeque<DeniableMessage> {
        let mut deniable_messages: VecDeque<DeniableMessage> = VecDeque::new();
        let mut i = 0;
        for length in lengths {
            let mut random_bytes = vec![0u8; length];
            rand::rng().fill_bytes(&mut random_bytes);
            deniable_messages.push_back(DeniableMessage {
                message_id: i as u32,
                message_kind: Some(MessageKind::DeniableMessage(UserMessage {
                    destination_account_id: vec![i as u8],
                    destination_device_id: 1,
                    message_type: MessageType::SignalMessage.into(),
                    content: random_bytes,
                })),
            });
            i += 1;
        }
        deniable_messages
    }

    #[rstest]
    #[case(150, 0.32, vec![20, 30, 40])]
    #[case(150, 0.625, vec![23, 31])]
    #[case(300, 0.721, vec![21])]
    #[tokio::test]
    async fn get_deniable_payload(
        #[case] regular_msg_len: u32,
        #[case] q: f32,
        #[case] message_lengths: Vec<usize>,
    ) {
        let deniable_messages = make_deniable_messages(message_lengths);

        let mut sending_buffer = InMemorySendingBuffer::new(q, deniable_messages);

        let deniable_payload = sending_buffer
            .get_deniable_payload(regular_msg_len)
            .await
            .unwrap()
            .expect("Should be Some");

        let mut total_size: usize = deniable_payload
            .denim_chunks()
            .iter()
            .map(|chunk| chunk.len())
            .sum::<usize>();

        if let Some(garbage) = deniable_payload.garbage() {
            total_size += garbage.len();
        }

        assert_eq!(total_size, (regular_msg_len as f32 * q).ceil() as usize);
    }
}
