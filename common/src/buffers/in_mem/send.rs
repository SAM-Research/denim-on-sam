use crate::buffers::{DeniablePayload, DenimChunk, Flag};
use crate::denim_message::DeniableMessage;
use crate::error::LibError;
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

impl InMemorySendingBuffer {
    pub fn get_deniable_payload(
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
                    .garbage(self.create_n_random_bytes(available_bytes))
                    .build(),
            ));
        }

        let mut denim_chunks: Vec<Vec<u8>> = Vec::new();

        while available_bytes > DENIM_CHUNK_WITHOUT_PAYLOAD {
            // we put deniable chunks on

            println!("Available bytes: {:?}", available_bytes);
            // Find out how many bytes deniable payload we can have
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
                    info!("Denim chunk piggybacked has size {:?}", encoded_chunk.len());
                    self.buffer.next_sequence_number += 1;
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

            denim_chunks.push(encoded_chunk);
        }

        info!(
            "Deniable payload has size {:?}, filling {:?} bytes garbage.",
            denim_chunks.iter().map(|chunk| chunk.len()).sum::<usize>(),
            available_bytes
        );

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

    pub fn calculate_deniable_payload_length(&self, reg_message_len: u32) -> usize {
        (reg_message_len as f32 * self.q).ceil() as usize
    }

    pub fn get_next_chunk(&mut self, available_bytes: usize) -> Option<DenimChunk> {
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

    pub fn create_dummy_chunk(&self, available_bytes: usize) -> DenimChunk {
        let random_bytes = self.create_n_random_bytes(available_bytes);

        DenimChunk::builder()
            .chunk(random_bytes)
            .flag(Flag::DummyPadding)
            .sequence_number(0)
            .message_id(0)
            .build()
    }

    fn create_n_random_bytes(&self, n: usize) -> Vec<u8> {
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

    fn make_deniable_messages() -> VecDeque<DeniableMessage> {
        let mut deniable_messages: VecDeque<DeniableMessage> = VecDeque::new();
        for i in 1..10 {
            let mut random_bytes = vec![0u8; i * 15];
            rand::rng().fill_bytes(&mut random_bytes);
            deniable_messages.push_back(DeniableMessage {
                message_id: i as u32,
                message_kind: Some(MessageKind::DeniableMessage(UserMessage {
                    destination_account_id: vec![i as u8],
                    destination_device_id: 1,
                    message_type: i32::from(MessageType::SignalMessage),
                    content: vec![1, 2, 3],
                })),
            })
        }
        deniable_messages
    }

    #[test]
    fn calculate_payload_length() {
        let deniable_messages = make_deniable_messages();

        let sending_buffer = InMemorySendingBuffer {
            q: 0.33,
            outgoing_messages: deniable_messages,
            buffer: Buffer {
                content: vec![1, 2, 3],
                message_id: 0,
                next_sequence_number: 0,
            },
        };

        let length = sending_buffer.calculate_deniable_payload_length(120);

        assert_eq!(length, 40)
    }

    #[test]
    fn get_next_chunk() {
        let q = 1.0;
        let regular_msg_len = 20;
        let deniable_content_length = (regular_msg_len as f32 * q) as usize;
        let mut random_bytes = vec![0u8; 10];
        rand::rng().fill_bytes(&mut random_bytes);

        let deniable_payload = random_bytes.clone();

        let mut sending_buffer = InMemorySendingBuffer {
            q,
            outgoing_messages: VecDeque::new(),
            buffer: Buffer {
                content: deniable_payload,
                message_id: 0,
                next_sequence_number: 0,
            },
        };

        let denim_chunk = sending_buffer
            .get_next_chunk(deniable_content_length)
            .expect("Should return Chunk");

        println!("DenimChunk content size: {:?}", denim_chunk.chunk().len());
        let denim_chunk_serialized =
            bincode::encode_to_vec(denim_chunk, config::standard()).expect("Can encode DenimChunk");

        assert_eq!(
            denim_chunk_serialized.len(),
            random_bytes.len() + DENIM_CHUNK_WITHOUT_PAYLOAD
        );
    }

    #[test]
    fn get_deniable_payload() {
        let q = 1.0;
        let regular_msg_len = 100;
        let mut random_bytes = vec![0u8; 4];
        rand::rng().fill_bytes(&mut random_bytes);

        let deniable_payload = random_bytes.clone();

        let deniable_messages = make_deniable_messages();

        let mut sending_buffer = InMemorySendingBuffer {
            q,
            outgoing_messages: deniable_messages,
            buffer: Buffer {
                content: deniable_payload,
                message_id: 0,
                next_sequence_number: 0,
            },
        };

        let deniable_payload = sending_buffer
            .get_deniable_payload(regular_msg_len)
            .unwrap()
            .expect("Should be Some");
        let first_chunk = deniable_payload.denim_chunks()[0].to_owned();

        assert_eq!(
            first_chunk.len(),
            random_bytes.len() + DENIM_CHUNK_WITHOUT_PAYLOAD
        );

        //let second_chunk = deniable_payload.denim_chunks[1].to_owned();
        //assert_eq!(second_chunk.encode_to_vec().len(), )

        let mut total_size: usize = deniable_payload
            .denim_chunks()
            .iter()
            .map(|chunk| chunk.len())
            .sum::<usize>();

        if let Some(garbage) = deniable_payload.garbage() {
            total_size += garbage.len();
        }

        println!("Deniable payload length: {:?}", total_size);
        assert_eq!(total_size, (q * regular_msg_len as f32) as usize);
    }
}
