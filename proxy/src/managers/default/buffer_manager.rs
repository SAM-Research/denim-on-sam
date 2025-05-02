use std::{collections::HashMap, sync::Arc};

use denim_sam_common::{
    buffers::{
        DeniablePayload, DenimChunk, MessageId, ReceivingBuffer, ReceivingBufferConfig,
        SendingBuffer, SendingBufferConfig,
    },
    denim_message::{
        deniable_message::MessageKind, BlockRequest, DeniableMessage, KeyRequest, SeedUpdate,
        UserMessage,
    },
};
use log::debug;

use sam_common::AccountId;
use tokio::sync::Mutex;

use crate::{managers::error::BufferManagerError, state::BufferManagerType};

pub enum ClientRequest {
    BlockRequest(MessageId, BlockRequest),
    KeyRequest(MessageId, KeyRequest),
    SeedUpdateRequest(MessageId, SeedUpdate),
    UserMessage(MessageId, UserMessage),
}

#[derive(Clone)]
pub struct BufferManager<T: BufferManagerType> {
    receiving_buffers:
        Arc<Mutex<HashMap<AccountId, <T::ReceivingBufferConfig as ReceivingBufferConfig>::Buffer>>>,
    sending_buffers:
        Arc<Mutex<HashMap<AccountId, <T::SendingBufferConfig as SendingBufferConfig>::Buffer>>>,
    receiving_config: T::ReceivingBufferConfig,
    sending_config: T::SendingBufferConfig,
    q: f32,
}

impl<T: BufferManagerType> BufferManager<T> {
    pub fn new(
        receiving_config: T::ReceivingBufferConfig,
        sending_config: T::SendingBufferConfig,
        q: f32,
    ) -> Self {
        Self {
            receiving_buffers: Arc::new(Mutex::new(HashMap::new())),
            sending_buffers: Arc::new(Mutex::new(HashMap::new())),
            receiving_config,
            sending_config,
            q,
        }
    }

    pub async fn get_q(&self) -> f32 {
        self.q
    }

    pub async fn set_q(&mut self, q: f32) {
        self.q = q;
        for buffer in self.sending_buffers.lock().await.values_mut() {
            buffer.set_q(q).await;
        }
    }

    pub async fn enqueue_message(
        &mut self,
        account_id: AccountId,
        deniable_message: DeniableMessage,
    ) -> Result<(), BufferManagerError> {
        let mut guard = self.sending_buffers.lock().await;
        let buffer = guard.entry(account_id).or_insert(
            self.sending_config
                .create(self.q)
                .await
                .map_err(BufferManagerError::DenimBufferError)?,
        );
        buffer.enqueue_message(deniable_message).await;
        Ok(())
    }

    pub async fn get_deniable_payload(
        &mut self,
        account_id: AccountId,
        reg_message_len: u32,
    ) -> Result<DeniablePayload, BufferManagerError> {
        let mut guard = self.sending_buffers.lock().await;
        // or_insert_with would be better, but you know async closures
        let buffer = guard.entry(account_id).or_insert(
            self.sending_config
                .create(self.q)
                .await
                .map_err(BufferManagerError::DenimBufferError)?,
        );

        buffer
            .get_deniable_payload(reg_message_len)
            .await
            .map_err(BufferManagerError::DenimBufferError)
    }

    pub async fn enqueue_chunks(
        &mut self,
        account_id: AccountId,
        chunks: Vec<DenimChunk>,
    ) -> Result<Vec<Result<ClientRequest, BufferManagerError>>, BufferManagerError> {
        let chunks = {
            let mut guard = self.receiving_buffers.lock().await;
            guard
                .entry(account_id)
                .or_insert(
                    self.receiving_config
                        .create()
                        .await
                        .map_err(BufferManagerError::DenimBufferError)?,
                )
                .process_chunks(chunks)
                .await
        };

        let mut results = Vec::new();
        for res in chunks {
            let msg = match res {
                Ok(msg) => msg,
                Err(e) => {
                    debug!("Failed to process Deniable Message '{e}' for account '{account_id}'");
                    results.push(Err(BufferManagerError::DenimBufferError(e)));
                    continue;
                }
            };
            let res = match msg.message_kind {
                Some(kind) => self.handle_message_kind(msg.message_id, kind).await,
                None => {
                    results.push(Err(BufferManagerError::MalformedMessage(msg.message_id)));
                    debug!("Malformed message from account '{account_id}'");
                    continue;
                }
            };
            results.push(res);
        }
        Ok(results)
    }

    async fn handle_message_kind(
        &mut self,
        message_id: MessageId,
        kind: MessageKind,
    ) -> Result<ClientRequest, BufferManagerError> {
        let request = match kind {
            MessageKind::DeniableMessage(x) => ClientRequest::UserMessage(message_id, x),
            MessageKind::BlockRequest(x) => ClientRequest::BlockRequest(message_id, x),
            MessageKind::KeyRequest(x) => ClientRequest::KeyRequest(message_id, x),
            MessageKind::SeedUpdate(x) => ClientRequest::SeedUpdateRequest(message_id, x),
            // Client is not allowed to send these
            MessageKind::Error(_) => Err(BufferManagerError::ClientSendError(message_id))?,
            MessageKind::KeyResponse(_) => {
                Err(BufferManagerError::ClientSendServerResponse(message_id))?
            }
        };
        Ok(request)
    }
}

#[cfg(test)]
mod test {
    use denim_sam_common::{
        buffers::{
            in_mem::{InMemoryReceivingBufferConfig, InMemorySendingBufferConfig},
            Flag, SendingBuffer,
        },
        denim_message::{
            deniable_message::MessageKind, BlockRequest, DeniableMessage, KeyRequest, MessageType,
            SeedUpdate, UserMessage,
        },
    };

    use rstest::rstest;
    use sam_common::AccountId;

    use crate::{
        managers::{default::ClientRequest, BufferManager},
        state::InMemoryBufferManagerType,
    };

    #[tokio::test]
    async fn buffer_mgr_enqueue_message_and_deqeue() {
        let receiver = InMemoryReceivingBufferConfig;
        let sender = InMemorySendingBufferConfig::default();

        let q = 1.0;
        let mut mgr: BufferManager<InMemoryBufferManagerType> =
            BufferManager::new(receiver, sender, q);
        let account_id = AccountId::generate();
        let user_msg = UserMessage::builder()
            .content(vec![1, 3, 3, 7])
            .account_id(account_id.into())
            .message_type(MessageType::PlaintextContent.into())
            .build();
        mgr.enqueue_message(
            account_id,
            DeniableMessage::builder()
                .message_id(1)
                .message_kind(MessageKind::DeniableMessage(user_msg))
                .build(),
        )
        .await
        .expect("Can enqueue");
        let payload = mgr
            .get_deniable_payload(account_id, 50)
            .await
            .expect("can get payload");

        assert!(payload
            .denim_chunks()
            .first()
            .is_some_and(|x| x.flag() == Flag::Final));
    }

    enum Request {
        Key,
        Block,
        Seed,
        Message,
    }
    impl Request {
        fn kind(self) -> (AccountId, MessageKind) {
            let acid = AccountId::generate();
            let kind = match self {
                Request::Key => MessageKind::KeyRequest(
                    KeyRequest::builder()
                        .account_id(acid.into())
                        .specific_device_ids(vec![1])
                        .build(),
                ),
                Request::Block => MessageKind::BlockRequest(
                    BlockRequest::builder()
                        .account_id(AccountId::generate().into())
                        .build(),
                ),
                Request::Seed => MessageKind::SeedUpdate(
                    SeedUpdate::builder()
                        .pre_key_id_seed(vec![1, 2, 3])
                        .pre_key_seed(vec![5, 3, 1])
                        .build(),
                ),
                Request::Message => MessageKind::DeniableMessage(
                    UserMessage::builder()
                        .account_id(AccountId::generate().into())
                        .content(vec![1, 2, 3])
                        .message_type(MessageType::PlaintextContent.into())
                        .build(),
                ),
            };
            (acid, kind)
        }
    }

    #[rstest]
    #[case(Request::Key, |req: &ClientRequest| matches!(req, ClientRequest::KeyRequest(_, _)))]
    #[case(Request::Block, |req: &ClientRequest| matches!(req, ClientRequest::BlockRequest(_, _)))]
    #[case(Request::Seed, |req: &ClientRequest| matches!(req, ClientRequest::SeedUpdateRequest(_, _)))]
    #[case(Request::Message, |req: &ClientRequest| matches!(req, ClientRequest::UserMessage(_, _)))]
    #[tokio::test]
    async fn buffer_mgr_enqueue_chunks(
        #[case] req: Request,
        #[case] expected_pattern: fn(&ClientRequest) -> bool,
    ) {
        let receiver = InMemoryReceivingBufferConfig;
        let sender = InMemorySendingBufferConfig::default();

        let q = 1.0;
        let mut mgr: BufferManager<InMemoryBufferManagerType> =
            BufferManager::new(receiver, sender, q);

        let (account_id, kind) = req.kind();

        let msg = DeniableMessage::builder()
            .message_id(1)
            .message_kind(kind)
            .build();

        mgr.enqueue_message(account_id, msg)
            .await
            .expect("Can enqueue");
        let payload = mgr
            .get_deniable_payload(account_id, 200)
            .await
            .expect("Can get payload");

        let results = mgr
            .enqueue_chunks(account_id, payload.denim_chunks().to_vec())
            .await
            .expect("Can enqueue");
        assert!(results.len() == 1);
        for res in results {
            let request = res.expect("decoding chunks works");
            assert!(expected_pattern(&request))
        }
    }

    #[tokio::test]
    async fn set_q_updates_all_sending_buffers() {
        let init_q = 1.0;
        let expected_q = 2.3;
        let receiver = InMemoryReceivingBufferConfig;
        let sender = InMemorySendingBufferConfig::default();

        let mut mgr: BufferManager<InMemoryBufferManagerType> =
            BufferManager::new(receiver, sender, init_q);

        let accounts = vec![AccountId::generate(); 32];

        for account in accounts {
            mgr.enqueue_message(
                account,
                DeniableMessage {
                    message_id: 1u32,
                    message_kind: Some(MessageKind::BlockRequest(BlockRequest {
                        account_id: account.into(),
                    })),
                },
            )
            .await
            .expect("Can enqueue message");
        }

        for buffer in mgr.sending_buffers.lock().await.values() {
            let actual_q = buffer.get_q().await;
            assert_eq!(
                actual_q, init_q,
                "Expected initial q '{}', Actual q '{}'",
                init_q, actual_q
            );
        }
        mgr.set_q(expected_q).await;
        for buffer in mgr.sending_buffers.lock().await.values() {
            let actual_q = buffer.get_q().await;
            assert_eq!(
                actual_q, expected_q,
                "Expected updated q '{}', Actual q '{}'",
                expected_q, actual_q
            );
        }
    }
}
