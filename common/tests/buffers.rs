use denim_sam_common::buffers::{
    DeniablePayload, Flag, InMemoryReceivingBuffer, InMemorySendingBuffer, ReceivingBuffer,
    SendingBuffer,
};
use denim_sam_common::denim_message::deniable_message::MessageKind;
use denim_sam_common::denim_message::{DeniableMessage, MessageType, UserMessage};
use rand::seq::SliceRandom;
use rand::SeedableRng;
use rstest::rstest;
use std::collections::VecDeque;

pub fn make_deniable_messages(lengths: Vec<usize>) -> VecDeque<DeniableMessage> {
    let mut deniable_messages: VecDeque<DeniableMessage> = VecDeque::new();
    for (i, length) in lengths.into_iter().enumerate() {
        let content = vec![0u8; length];
        deniable_messages.push_back(DeniableMessage {
            message_id: i as u32,
            message_kind: Some(MessageKind::DeniableMessage(UserMessage {
                account_id: vec![i as u8],

                message_type: MessageType::SignalMessage.into(),
                content,
            })),
        });
    }
    deniable_messages
}

#[rstest]
#[case(150, 1.0, vec![10, 30, 16], None)]
#[case(30, 1.0, vec![150, 31, 90], None)]
#[case(40, 0.6, vec![92, 300, 15], None)]
#[case(150, 1.0, vec![10, 30, 16], Some(42384722223219))]
#[case(50, 1.0, vec![150, 31, 90], Some(423423))]
#[case(40, 0.6, vec![92, 300, 15], Some(409034902402))]
#[case(40, 0.6, vec![92, 300, 15,230, 1500, 3000], Some(389482394))]
#[tokio::test]
pub async fn send_recv_buffer(
    #[case] regular_msg_len: u32,
    #[case] q: f32,
    #[case] message_lengths: Vec<usize>,
    #[case] seed: Option<u64>,
) {
    let _ = env_logger::try_init();
    let deniable_messages = make_deniable_messages(message_lengths.clone());
    let mut sending_buffer = InMemorySendingBuffer::new(q, 10).expect("Can make SendingBuffer");

    for message in deniable_messages {
        sending_buffer.enqueue_message(message).await;
    }

    let mut deniable_payloads: Vec<DeniablePayload> = Vec::new();

    while let Some(deniable_payload) = sending_buffer
        .get_deniable_payload(regular_msg_len)
        .await
        .expect("Should produce a deniable payload")
    {
        if deniable_payload
            .denim_chunks()
            .first()
            .expect("Should be at least one chunk")
            .flag()
            == Flag::DummyPadding
        {
            break;
        }
        deniable_payloads.push(deniable_payload);
    }

    let mut receiving_buffer = InMemoryReceivingBuffer::default();
    let mut messages = Vec::new();

    if let Some(rng_seed) = seed {
        let mut rng = rand::rngs::StdRng::seed_from_u64(rng_seed);
        deniable_payloads.shuffle(&mut rng);
    }

    for deniable_payload in deniable_payloads {
        let bytes = deniable_payload.to_bytes().expect("Can make it to bytes");
        let denim_chunks = DeniablePayload::decode(bytes).expect("Can decode bytes");

        let result_from_process = receiving_buffer.process_chunks(denim_chunks).await;
        for result in result_from_process {
            let deniable_message = result.expect("Can decode message");
            messages.push(deniable_message);
        }
    }

    assert_eq!(messages.len(), message_lengths.len());
    if seed.is_none() {
        for (i, message) in messages.iter().enumerate() {
            let content = vec![0u8; message_lengths[i]];
            assert_eq!(
                message.clone().message_kind.expect("Should be a message"),
                MessageKind::DeniableMessage(UserMessage {
                    account_id: vec![i as u8],
                    message_type: MessageType::SignalMessage.into(),
                    content,
                })
            )
        }
    }
}
