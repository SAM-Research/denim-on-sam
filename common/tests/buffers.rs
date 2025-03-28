use bincode::config;
use denim_sam_common::buffers::{
    DeniablePayload, DenimChunk, Flag, InMemoryReceivingBuffer, InMemorySendingBuffer,
    ReceivingBuffer, SendingBuffer,
};
use denim_sam_common::denim_message::deniable_message::MessageKind;
use denim_sam_common::denim_message::{DeniableMessage, MessageType, UserMessage};
use rand::RngCore;
use rstest::rstest;
use std::collections::VecDeque;

pub fn is_dummy(denim_chunk: &Vec<u8>) -> bool {
    let (decoded_chunk, _): (DenimChunk, usize) =
        bincode::decode_from_slice(denim_chunk, config::standard())
            .expect("should be able to decode chunk");

    if decoded_chunk.flag() == Flag::DummyPadding {
        return true;
    }
    false
}

pub fn make_deniable_messages(lengths: Vec<usize>) -> VecDeque<DeniableMessage> {
    let mut deniable_messages: VecDeque<DeniableMessage> = VecDeque::new();
    let mut i = 0;
    for length in lengths {
        let mut random_bytes = vec![0u8; length];
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
#[case(150, 1.0, vec![10, 30, 16])]
#[tokio::test]
pub async fn send_in_order(
    #[case] regular_msg_len: u32,
    #[case] q: f32,
    #[case] message_lengths: Vec<usize>,
) {
    let deniable_messages = make_deniable_messages(message_lengths.clone());
    let mut sending_buffer = InMemorySendingBuffer::new(q, deniable_messages);

    let mut deniable_payloads: Vec<DeniablePayload> = Vec::new();

    while let Some(deniable_payload) = sending_buffer
        .get_deniable_payload(regular_msg_len)
        .await
        .expect("Should produce a deniable payload")
    {
        if is_dummy(
            deniable_payload
                .denim_chunks()
                .first()
                .expect("Should have at least one chunk"),
        ) {
            break;
        }
        deniable_payloads.push(deniable_payload);
    }

    let mut receiving_buffer = InMemoryReceivingBuffer::default();
    let mut messages = Vec::new();
    for mut deniable_payload in deniable_payloads {
        let mut chunks = Vec::new();
        for chunk in deniable_payload.denim_chunks_mut() {
            let (denim_chunk, _): (DenimChunk, usize) =
                bincode::decode_from_slice(chunk, config::standard()).unwrap();
            chunks.push(denim_chunk)
        }

        let result_from_process = receiving_buffer.process_chunks("bob", chunks).await;
        for result in result_from_process {
            let deniable_message = result.expect("Can decode message");
            messages.push(deniable_message);
        }
    }

    assert_eq!(messages.len(), 3);
    for (i, message) in messages.iter().enumerate() {
        let mut random_bytes = vec![0u8; message_lengths[i]];
        assert_eq!(
            message.clone().message_kind.expect("Should be a message"),
            MessageKind::DeniableMessage(UserMessage {
                destination_account_id: vec![i as u8],
                destination_device_id: 1,
                message_type: MessageType::SignalMessage.into(),
                content: random_bytes,
            })
        )
    }
}
