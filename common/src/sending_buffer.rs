use crate::denim_message::{DeniableMessage, UserMessage};
use crate::denim_message_flatbuffer::{DenimChunkArgs, DenimChunkT, Flag};
use flatbuffers::FlatBufferBuilder;
use log::info;
use prost::Message;
use rand::RngCore;
use std::collections::VecDeque;

const PROTOBUF_LENGTH_PREFIX: usize = 1;

const PROTOBUF_TAG_SIZE: usize = 1;
const DENIM_CHUNK_WITHOUT_PAYLOAD: usize = 6;
const DUMMY_PAYLOAD_WITHOUT_CONTENT: usize = 4;

struct Buffer {
    content: Vec<u8>,
    message_id: u32,
    next_sequence_number: u8,
}

struct SendingBuffer {
    q: f32,
    outgoing_messages: VecDeque<DeniableMessage>,
    buffer: Buffer,
}

struct DeniablePayload {
    denim_chunks: Vec<Vec<u8>>,
    garbage: Option<Vec<u8>>,
}

impl SendingBuffer {
    pub fn get_deniable_payload(&mut self, reg_message_len: u32) -> Option<DeniablePayload> {
        if self.q == 0.0 {
            return None;
        } // no deniable traffic

        let deniable_payload_length = self.calculate_deniable_payload_length(reg_message_len);

        let mut available_bytes = deniable_payload_length;

        let mut builder = FlatBufferBuilder::new();

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
                Some(chunk_t) => {
                    let denim_chunk = chunk_t.pack(&mut builder);
                    builder.finish(denim_chunk, None);
                    let chunk_serialized = builder.finished_data();
                    available_bytes -= chunk_serialized.len();
                    info!(
                        "Denim chunk piggybacked has size {:?}",
                        chunk_serialized.len()
                    );
                    self.buffer.next_sequence_number += 1;
                    denim_chunks.push(chunk_serialized.to_owned());
                }
            }
        }
        if available_bytes >= DENIM_CHUNK_WITHOUT_PAYLOAD + DUMMY_PAYLOAD_WITHOUT_CONTENT {
            // Send deniable payload
            let dummy_chunk_length =
                available_bytes - (DENIM_CHUNK_WITHOUT_PAYLOAD + DUMMY_PAYLOAD_WITHOUT_CONTENT);

            let dummy_chunk_t = self.create_dummy_chunk(dummy_chunk_length);

            let denim_chunk = dummy_chunk_t.pack(&mut builder);
            builder.finish(denim_chunk, None);
            let chunk_serialized = builder.finished_data();

            denim_chunks.push(chunk_serialized.to_owned());
        }

        info!(
            "Deniable payload has size {:?}, filling {:?} bytes garbage.",
            denim_chunks.iter().map(|chunk| chunk.len()).sum::<usize>(),
            available_bytes
        );

        if available_bytes > 0 {
            let mut random_bytes = vec![0u8; available_bytes];
            rand::rng().fill_bytes(&mut random_bytes);
            return Some(DeniablePayload {
                denim_chunks,
                garbage: Some(random_bytes),
            });
        }

        Some(DeniablePayload {
            denim_chunks,
            garbage: None,
        })
    }

    pub fn calculate_deniable_payload_length(&self, reg_message_len: u32) -> usize {
        (reg_message_len as f32 * self.q).ceil() as usize
    }

    pub fn get_next_chunk(&mut self, available_bytes: usize) -> Option<DenimChunkT> {
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
            return Some(DenimChunkT {
                chunk: std::mem::take(&mut self.buffer.content),
                message_id: 0,
                sequence_number: self.buffer.next_sequence_number as u32,
                flag: Flag::NOT_FINAL,
            });
        };

        let next_chunk: Vec<u8> = self.buffer.content.drain(..available_bytes).collect();

        Some(DenimChunkT {
            chunk: next_chunk,
            message_id: 0,
            sequence_number: self.buffer.next_sequence_number as u32,
            flag: Flag::IS_FINAL,
        })
    }

    pub fn create_dummy_chunk(&self, available_bytes: usize) -> DenimChunkT {
        let mut random_bytes = vec![0u8; available_bytes];
        rand::rng().fill_bytes(&mut random_bytes);
        DenimChunkT {
            chunk: random_bytes,
            message_id: 0,
            sequence_number: 0,
            flag: Flag::DUMMY_PADDING,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::denim_message::deniable_message::MessageKind;
    use crate::denim_message::{DummyPadding, MessageType};
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

        let sending_buffer = SendingBuffer {
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
        let mut builder = FlatBufferBuilder::new();
        let q = 1.0;
        let regular_msg_len = 20;
        let deniable_content_length = (regular_msg_len as f32 * q) as usize;
        let mut random_bytes = vec![0u8; deniable_content_length * 2];
        rand::rng().fill_bytes(&mut random_bytes);

        let deniable_payload = DeniableMessage::builder()
            .message_kind(MessageKind::Dummy(DummyPadding {
                padding: random_bytes.clone(),
            }))
            .message_id(0)
            .build();

        let mut sending_buffer = SendingBuffer {
            q: q,
            outgoing_messages: VecDeque::new(),
            buffer: Buffer {
                content: deniable_payload.encode_to_vec(),
                message_id: 0,
                next_sequence_number: 0,
            },
        };

        let denim_chunk_t = sending_buffer.get_next_chunk(deniable_content_length);

        let denim_chunk = denim_chunk_t.unwrap().pack(&mut builder);
        builder.finish(denim_chunk, None);
        let chunk_serialized = builder.finished_data();

        assert_eq!(
            chunk_serialized.len(),
            random_bytes.len() / 2 + DENIM_CHUNK_WITHOUT_PAYLOAD
        );
    }

    #[test]
    fn get_deniable_payload() {
        let q = 1.0;
        let regular_msg_len = 100;
        let deniable_content_length = (regular_msg_len as f32 * q) as usize;
        let mut random_bytes = vec![0u8; 4];
        rand::rng().fill_bytes(&mut random_bytes);

        let deniable_payload = DeniableMessage::builder()
            .message_kind(MessageKind::Dummy(DummyPadding {
                padding: random_bytes.clone(),
            }))
            .build();

        let deniable_messages = make_deniable_messages();

        let mut sending_buffer = SendingBuffer {
            q: q,
            outgoing_messages: deniable_messages,
            buffer: Buffer {
                content: deniable_payload.encode_to_vec(),
                message_id: 0,
                next_sequence_number: 0,
            },
        };

        let first_message = sending_buffer.outgoing_messages[0].to_owned();

        let deniable_payload = sending_buffer.get_deniable_payload(regular_msg_len);

        let first_chunk = deniable_payload[0].to_owned();
        assert_eq!(
            first_chunk.encode_to_vec().len(),
            random_bytes.len() + DENIM_CHUNK_WITHOUT_PAYLOAD + DUMMY_PAYLOAD_WITHOUT_CONTENT
        );

        let second_chunk = deniable_payload[1].to_owned();
        //assert_eq!(second_chunk.encode_to_vec().len(), )

        let total_size: usize = deniable_payload
            .unwrap()
            .denim_chunks
            .iter()
            .map(|chunk| chunk.len())
            .sum::<usize>();
        println!("Deniable payload length: {:?}", total_size);
        assert_eq!(total_size, (q * regular_msg_len as f32) as usize);
    }
}
