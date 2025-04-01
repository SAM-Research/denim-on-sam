use crate::buffers::{DeniablePayload, DenimChunk, Flag, MessageId, SendingBuffer, SequenceNumber};
use crate::denim_message::DeniableMessage;
use crate::error::LibError;
use async_trait::async_trait;
use log::debug;
use prost::Message;
use rand::RngCore;
use std::collections::VecDeque;
use std::mem::take;
use std::sync::Arc;
use tokio::sync::Mutex;

struct Buffer {
    content: Vec<u8>,
    message_id: MessageId,
    next_sequence_number: SequenceNumber,
}

pub struct InMemorySendingBuffer {
    min_payload_length: u8,
    q: f32,
    chunk_size_without_payload: usize,
    outgoing_messages: Arc<Mutex<VecDeque<DeniableMessage>>>,
    buffer: Arc<Mutex<Buffer>>,
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

        if available_bytes < self.min_payload_length.into() {
            return Ok(Some(
                DeniablePayload::builder()
                    .denim_chunks(vec![])
                    .garbage(InMemorySendingBuffer::create_n_random_bytes(
                        available_bytes,
                    ))
                    .build(),
            ));
        }

        let mut denim_chunks: Vec<DenimChunk> = Vec::new();

        while available_bytes > self.chunk_size_without_payload {
            let deniable_payload_len = available_bytes - self.chunk_size_without_payload;

            let chunk = self.get_next_chunk(deniable_payload_len).await;

            match chunk {
                None => {
                    break;
                }
                Some(chunk) => {
                    let encoded_chunk_size = chunk.get_size()?;
                    debug!(
                        "Size of chunk with payload {:?}, content size {:?}",
                        encoded_chunk_size,
                        chunk.chunk().len()
                    );
                    available_bytes -= encoded_chunk_size;

                    denim_chunks.push(chunk);
                }
            }
        }
        if available_bytes >= self.chunk_size_without_payload {
            let dummy_chunk_length = available_bytes - self.chunk_size_without_payload;

            let dummy_chunk = self.create_dummy_chunk(dummy_chunk_length);
            let encoded_chunk_size = dummy_chunk.get_size()?;
            available_bytes -= encoded_chunk_size;
            denim_chunks.push(dummy_chunk);
        }

        if available_bytes > 0 {
            denim_chunks
                .last_mut()
                .ok_or(LibError::NoChunksInDeniablePayload)?
                .set_garbage_flag();
            return Ok(Some(
                DeniablePayload::builder()
                    .denim_chunks(denim_chunks)
                    .garbage(InMemorySendingBuffer::create_n_random_bytes(
                        available_bytes,
                    ))
                    .build(),
            ));
        }

        Ok(Some(
            DeniablePayload::builder()
                .denim_chunks(denim_chunks)
                .build(),
        ))
    }

    async fn enqueue_message(&mut self, deniable_message: DeniableMessage) {
        self.outgoing_messages
            .lock()
            .await
            .push_back(deniable_message);
    }
}

impl InMemorySendingBuffer {
    pub fn new(q: f32, min_payload_length: u8) -> Result<Self, LibError> {
        let chunk_size_without_payload = DenimChunk::get_size_without_payload();

        if min_payload_length as usize > chunk_size_without_payload {
            return Err(LibError::MinPayloadLengthTooHigh);
        }

        Ok(Self {
            min_payload_length,
            q,
            chunk_size_without_payload,
            outgoing_messages: Arc::new(Mutex::new(VecDeque::new())),
            buffer: Arc::new(Mutex::new(Buffer {
                content: Vec::new(),
                message_id: 0,
                next_sequence_number: 0,
            })),
        })
    }
    fn calculate_deniable_payload_length(&self, reg_message_len: u32) -> usize {
        (reg_message_len as f32 * self.q).ceil() as usize
    }

    async fn get_next_chunk(&mut self, available_bytes: usize) -> Option<DenimChunk> {
        if self.buffer.lock().await.content.is_empty() {
            self.buffer = match self.outgoing_messages.lock().await.pop_front() {
                None => return None,
                Some(message) => Arc::new(Mutex::new(Buffer {
                    content: message.encode_to_vec(),
                    message_id: message.message_id,
                    next_sequence_number: 0,
                })),
            }
        }
        let chunk_bytes;
        let flag;
        let sequence_number = self.buffer.lock().await.next_sequence_number;
        self.buffer.lock().await.next_sequence_number += 1;
        if available_bytes >= self.buffer.lock().await.content.len() {
            chunk_bytes = take(&mut self.buffer.lock().await.content);
            flag = Flag::Final;
        } else {
            chunk_bytes = self
                .buffer
                .lock()
                .await
                .content
                .drain(..available_bytes)
                .collect();
            flag = Flag::None;
        }

        Some(
            DenimChunk::builder()
                .message_id(self.buffer.lock().await.message_id)
                .sequence_number(sequence_number)
                .flag(flag)
                .chunk(chunk_bytes)
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
    use crate::denim_message::{DenimMessage, MessageType, UserMessage};
    use rand::RngCore;
    use rstest::rstest;

    fn make_deniable_messages(lengths: Vec<usize>) -> VecDeque<DeniableMessage> {
        let mut deniable_messages: VecDeque<DeniableMessage> = VecDeque::new();
        for (i, length) in lengths.into_iter().enumerate() {
            let mut random_bytes = vec![0u8; length];
            rand::rng().fill_bytes(&mut random_bytes);
            deniable_messages.push_back(DeniableMessage {
                message_id: i as u32,
                message_kind: Some(MessageKind::DeniableMessage(UserMessage {
                    destination_account_id: vec![i as u8],
                    message_type: MessageType::SignalMessage.into(),
                    content: random_bytes,
                })),
            });
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

        let mut sending_buffer = InMemorySendingBuffer::new(q, 10).expect("Can make SendingBuffer");

        for message in deniable_messages {
            sending_buffer.enqueue_message(message).await;
        }

        let deniable_payload = sending_buffer
            .get_deniable_payload(regular_msg_len)
            .await
            .unwrap()
            .expect("Should be Some");

        let total_size: usize = deniable_payload
            .to_bytes()
            .expect("Should get bytes")
            .iter()
            .map(|bytes| bytes.len())
            .sum::<usize>();

        assert_eq!(total_size, (regular_msg_len as f32 * q).ceil() as usize);
    }

    #[rstest]
    #[case(InMemorySendingBuffer::create_n_random_bytes(123), 0.32, vec![20, 30, 40])]
    #[case(InMemorySendingBuffer::create_n_random_bytes(50), 0.625, vec![23, 31,15])]
    #[case(InMemorySendingBuffer::create_n_random_bytes(1023), 0.721, vec![21,3,5,123])]
    #[case(InMemorySendingBuffer::create_n_random_bytes(300), 1.0, vec![260])]
    #[tokio::test]
    async fn encode_and_decode_deniable_payload_in_denim_message(
        #[case] regular_msg: Vec<u8>,
        #[case] q: f32,
        #[case] message_lengths: Vec<usize>,
    ) {
        let deniable_messages = make_deniable_messages(message_lengths);

        let mut sending_buffer = InMemorySendingBuffer::new(q, 10).expect("Can make SendingBuffer");

        for message in deniable_messages {
            sending_buffer.enqueue_message(message).await;
        }

        let deniable_payload = sending_buffer
            .get_deniable_payload(regular_msg.len() as u32)
            .await
            .unwrap()
            .expect("Should be Some");

        let deniable_payload_chunks = deniable_payload.denim_chunks().len();

        let denim_message = DenimMessage::builder()
            .deniable_payload(
                deniable_payload
                    .to_bytes()
                    .expect("Should be able to make it to bytes"),
            )
            .regular_payload(regular_msg.clone())
            .build();

        let deniable_payload = DeniablePayload::decode(denim_message.deniable_payload.clone())
            .expect("Should be able to decode deniable payload");

        let deniable_payload_size = denim_message
            .deniable_payload
            .iter()
            .map(|bytes| bytes.len())
            .sum::<usize>();

        assert_eq!(
            deniable_payload_size,
            (regular_msg.len() as f32 * q).ceil() as usize
        );

        assert_eq!(deniable_payload.len(), deniable_payload_chunks);
    }
}
