use std::{collections::HashMap, sync::Arc};

use denim_sam_common::{
    buffers::{
        DeniablePayload, DenimChunk, MessageId, ReceivingBuffer, ReceivingBufferConfig,
        SendingBuffer, SendingBufferConfig,
    },
    denim_message::{
        deniable_message::MessageKind, BlockRequest, DeniableMessage, KeyRequest, KeyUpdate,
        SeedUpdate, UserMessage,
    },
};
use log::debug;

use sam_common::AccountId;
use tokio::sync::Mutex;

use crate::managers::{error::BufferManagerError, traits::message_id_provider::MessageIdProvider};

pub enum ClientRequest {
    BlockRequest(MessageId, BlockRequest),
    KeyRequest(MessageId, KeyRequest),
    KeyRefillRequest(MessageId, KeyUpdate),
    SeedUpdateRequest(MessageId, SeedUpdate),
}

#[derive(Clone)]
pub struct BufferManager<T: ReceivingBufferConfig, U: SendingBufferConfig, V: MessageIdProvider> {
    receiving_buffers: Arc<Mutex<HashMap<AccountId, T::Buffer>>>,
    sending_buffers: Arc<Mutex<HashMap<AccountId, U::Buffer>>>,
    id_provider: V,
    receiving_config: T,
    sending_config: U,
}

impl<T: ReceivingBufferConfig, U: SendingBufferConfig, V: MessageIdProvider>
    BufferManager<T, U, V>
{
    pub fn new(receiving_config: T, sending_config: U, id_provider: V) -> Self {
        Self {
            receiving_buffers: Arc::new(Mutex::new(HashMap::new())),
            sending_buffers: Arc::new(Mutex::new(HashMap::new())),
            id_provider,
            receiving_config,
            sending_config,
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
                .create()
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
    ) -> Result<Option<DeniablePayload>, BufferManagerError> {
        let mut guard = self.sending_buffers.lock().await;
        // or_insert_with would be better, but you know async closures
        let buffer = guard.entry(account_id).or_insert(
            self.sending_config
                .create()
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
    ) -> Result<Vec<Result<Option<ClientRequest>, BufferManagerError>>, BufferManagerError> {
        // or_insert_with would be better, but you know async closures
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
                Some(kind) => {
                    self.handle_message_kind(account_id, msg.message_id, kind)
                        .await
                }
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
        account_id: AccountId,
        message_id: MessageId,
        kind: MessageKind,
    ) -> Result<Option<ClientRequest>, BufferManagerError> {
        let request = match kind {
            MessageKind::DeniableMessage(x) => {
                self.handle_user_message(account_id, x).await?;
                return Ok(None);
            }
            MessageKind::BlockRequest(x) => ClientRequest::BlockRequest(message_id, x),
            MessageKind::KeyRequest(x) => ClientRequest::KeyRequest(message_id, x),
            MessageKind::KeyRefill(x) => ClientRequest::KeyRefillRequest(message_id, x),
            MessageKind::SeedUpdate(x) => ClientRequest::SeedUpdateRequest(message_id, x),
            // Client is not allowed to send these
            MessageKind::Error(_) => Err(BufferManagerError::ClientSendError(message_id))?,
            MessageKind::KeyResponse(_) => {
                Err(BufferManagerError::ClientSendServerResponse(message_id))?
            }
        };
        Ok(Some(request))
    }

    async fn handle_user_message(
        &mut self,
        account_id: AccountId,
        message: UserMessage,
    ) -> Result<(), BufferManagerError> {
        let id = self.id_provider.get_message_id(account_id).await;
        self.enqueue_message(
            account_id,
            DeniableMessage {
                message_id: id,
                message_kind: Some(MessageKind::DeniableMessage(message)),
            },
        )
        .await
    }
}

#[cfg(test)]
mod test {
    use denim_sam_common::{
        buffers::{
            in_mem::{InMemoryReceivingBufferConfig, InMemorySendingBufferConfig},
            Flag,
        },
        denim_message::{
            deniable_message::MessageKind, DeniableMessage, KeyRequest, MessageType, UserMessage,
        },
    };

    use rstest::rstest;
    use sam_common::AccountId;

    use crate::managers::{default::ClientRequest, BufferManager, InMemoryMessageIdProvider};

    #[tokio::test]
    async fn buffer_mgr_enqueue_message_and_deqeue() {
        let receiver = InMemoryReceivingBufferConfig;
        let sender = InMemorySendingBufferConfig::builder()
            .q(1.0)
            .min_payload_length(10)
            .build();
        let id_provider = InMemoryMessageIdProvider::default();
        let mut mgr = BufferManager::new(receiver, sender, id_provider);
        let account_id = AccountId::generate();
        let user_msg = UserMessage::builder()
            .content(vec![1, 3, 3, 7])
            .destination_account_id(account_id.into())
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
            .expect("can get payload")
            .expect("payload is some");

        assert!(payload
            .denim_chunks()
            .first()
            .is_some_and(|x| x.flag() == Flag::Final));
    }

    #[rstest]
    #[case(false)]
    #[case(true)]
    #[tokio::test]
    async fn buffer_mgr_enqueue_chunks(#[case] is_request: bool) {
        let _ = env_logger::try_init();
        let receiver = InMemoryReceivingBufferConfig;
        let sender = InMemorySendingBufferConfig::builder()
            .q(1.0)
            .min_payload_length(10)
            .build();
        let id_provider = InMemoryMessageIdProvider::default();
        let mut mgr = BufferManager::new(receiver, sender, id_provider);

        let account_id = AccountId::generate();
        let kind = if is_request {
            MessageKind::KeyRequest(
                KeyRequest::builder()
                    .account_id(account_id.into())
                    .specific_device_ids(vec![1])
                    .build(),
            )
        } else {
            MessageKind::DeniableMessage(
                UserMessage::builder()
                    .content(vec![1, 3, 3, 7])
                    .destination_account_id(account_id.into())
                    .message_type(MessageType::PlaintextContent.into())
                    .build(),
            )
        };
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
            .expect("Can get payload")
            .expect("Payload is some");

        let results = mgr
            .enqueue_chunks(account_id, payload.denim_chunks().to_vec())
            .await
            .expect("Can enqueue");
        assert!(results.len() == 1);
        for res in results {
            let request = res.expect("decoding chunks works");
            if is_request {
                assert!(request.is_some_and(|x| matches!(x, ClientRequest::KeyRequest(_, _))));
            } else {
                assert!(request.is_none());
            }
        }
        if !is_request {
            let payload = mgr
                .get_deniable_payload(account_id, 50)
                .await
                .expect("Can get payload")
                .expect("Payload is some");
            assert!(payload.denim_chunks().len() == 1);
            assert!(payload
                .denim_chunks()
                .first()
                .is_some_and(|x| x.flag() == Flag::Final));
        }
    }
}
