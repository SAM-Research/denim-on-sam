use bincode::config;
use denim_sam_common::buffers::{
    DeniablePayload, DenimChunk, Flag, InMemoryReceivingBuffer, InMemorySendingBuffer,
    ReceivingBuffer, SendingBuffer,
};
use denim_sam_common::denim_message::deniable_message::MessageKind;
use denim_sam_common::denim_message::{DeniableMessage, MessageType, UserMessage};
use rand::seq::SliceRandom;
use rand::SeedableRng;
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
        let random_bytes = vec![0u8; length];
        deniable_messages.push_back(DeniableMessage {
            message_id: i as u32,
            message_kind: Some(MessageKind::DeniableMessage(UserMessage {
                destination_account_id: vec![i as u8],

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
#[case(30, 1.0, vec![150, 31, 90])]
#[case(40, 0.6, vec![92, 300, 15])]
#[tokio::test]
pub async fn send_recv_buffer_send_in_order(
    #[case] regular_msg_len: u32,
    #[case] q: f32,
    #[case] message_lengths: Vec<usize>,
) {
    let deniable_messages = make_deniable_messages(message_lengths.clone());
    let mut sending_buffer = InMemorySendingBuffer::new(q);

    for message in deniable_messages {
        sending_buffer.queue_message(message);
    }

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

    assert_eq!(messages.len(), message_lengths.len());
    for (i, message) in messages.iter().enumerate() {
        let random_bytes = vec![0u8; message_lengths[i]];
        assert_eq!(
            message.clone().message_kind.expect("Should be a message"),
            MessageKind::DeniableMessage(UserMessage {
                destination_account_id: vec![i as u8],
                message_type: MessageType::SignalMessage.into(),
                content: random_bytes,
            })
        )
    }
}

#[rstest]
#[case(150, 1.0, vec![10, 30, 16], 423842379)]
#[case(30, 1.0, vec![150, 31, 90], 423423)]
#[case(40, 0.6, vec![92, 300, 15], 409034902402)]
#[case(40, 0.6, vec![92, 300, 15,230, 1500, 3000], 389482394)]
#[tokio::test]
pub async fn send_recv_buffer_send_out_of_order(
    #[case] regular_msg_len: u32,
    #[case] q: f32,
    #[case] message_lengths: Vec<usize>,
    #[case] seed: u64,
) {
    let deniable_messages = make_deniable_messages(message_lengths.clone());
    let mut sending_buffer = InMemorySendingBuffer::new(q);

    for message in deniable_messages {
        sending_buffer.queue_message(message.clone());
    }

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
    let mut rng = rand::rngs::StdRng::seed_from_u64(seed);

    deniable_payloads.shuffle(&mut rng);
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

    assert_eq!(messages.len(), message_lengths.len());
}
