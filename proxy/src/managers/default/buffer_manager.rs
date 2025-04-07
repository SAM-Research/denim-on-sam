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
    Block(AccountId, MessageId, BlockRequest),
    Key(AccountId, MessageId, KeyRequest),
    KeyRefill(AccountId, MessageId, KeyUpdate),
    SeedUpdate(AccountId, MessageId, SeedUpdate),
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
            MessageKind::BlockRequest(x) => ClientRequest::Block(account_id, message_id, x), // client
            MessageKind::KeyRequest(x) => ClientRequest::Key(account_id, message_id, x), // client
            MessageKind::KeyResponse(_) => {
                Err(BufferManagerError::ClientSendServerResponse(message_id))?
            } // server
            MessageKind::KeyRefill(x) => ClientRequest::KeyRefill(account_id, message_id, x), // client
            MessageKind::SeedUpdate(x) => ClientRequest::SeedUpdate(account_id, message_id, x), // client
            MessageKind::Error(_) => Err(BufferManagerError::ClientSendError(message_id))?, // server
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
