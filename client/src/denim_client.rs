use std::sync::{
    atomic::{AtomicU32, Ordering},
    Arc,
};

use denim_sam_common::{
    buffers::{ReceivingBuffer, SendingBuffer},
    denim_message::{deniable_message::MessageKind, DeniableMessage},
};
use log::error;
use prost::Message as PMessage;
use sam_client::net::protocol::{
    decode::ServerStatus,
    websocket::{WebSocketClient, WebSocketError},
    MessageStatus,
};
use sam_common::{
    address::MessageId,
    sam_message::{ClientEnvelope, ClientMessage, ClientMessageType},
};
use tokio::sync::mpsc::channel;
use tokio::sync::{mpsc::Receiver, Mutex};
use tokio_tungstenite::tungstenite::{
    protocol::{frame::coding::CloseCode, CloseFrame},
    Message,
};

use crate::{
    error::DenimProtocolError,
    message::create_message,
    receiver::{DenimReceiver, SamDenimMessage},
};

#[async_trait::async_trait]
pub trait DenimSamClient {
    async fn connect(&mut self) -> Result<Receiver<SamDenimMessage>, DenimProtocolError>;
    async fn disconnect(&mut self) -> Result<(), DenimProtocolError>;
    async fn is_connected(&self) -> bool;
    async fn enqueue_deniable(&mut self, message: MessageKind);
    async fn send_message(
        &mut self,
        message: ClientEnvelope,
    ) -> Result<MessageStatus, DenimProtocolError>;
}

pub struct DenimProtcolClient<T: SendingBuffer, U: ReceivingBuffer> {
    client: Arc<Mutex<WebSocketClient>>,
    status_messages: Option<Receiver<ServerStatus>>,
    sending_buffer: T,
    receiving_buffer: U,
    denim_id: AtomicU32,
}

impl<T: SendingBuffer, U: ReceivingBuffer> DenimProtcolClient<T, U> {
    pub fn new(client: WebSocketClient, sending_buffer: T, receiving_buffer: U) -> Self {
        Self {
            client: Arc::new(Mutex::new(client)),
            status_messages: None,
            sending_buffer: sending_buffer,
            receiving_buffer: receiving_buffer,
            denim_id: AtomicU32::new(0),
        }
    }
}

#[async_trait::async_trait]
impl<T: SendingBuffer, U: ReceivingBuffer> DenimSamClient for DenimProtcolClient<T, U> {
    async fn connect(&mut self) -> Result<Receiver<SamDenimMessage>, DenimProtocolError> {
        // Implement the connection logic here

        let (status_tx, status_rx) = channel(10);
        self.status_messages = Some(status_rx);
        let handler = DenimReceiver::new(
            self.client.clone(),
            status_tx,
            self.sending_buffer.clone(),
            self.receiving_buffer.clone(),
        );
        self.client
            .lock()
            .await
            .connect(handler)
            .await
            .inspect_err(|e| error!("DenimProtocolClient Error: {e}"))
            .map_err(DenimProtocolError::WebSocketError)
    }

    async fn disconnect(&mut self) -> Result<(), DenimProtocolError> {
        self.status_messages = None;
        self.client
            .lock()
            .await
            .send(Message::Close(Some(CloseFrame {
                code: CloseCode::Normal,
                reason: "bye!".into(),
            })))
            .await
            .map_err(DenimProtocolError::WebSocketError)
    }

    async fn is_connected(&self) -> bool {
        self.client.lock().await.is_connected()
    }

    async fn enqueue_deniable(&mut self, message: MessageKind) {
        self.sending_buffer
            .enqueue_message(
                DeniableMessage::builder()
                    .message_id(self.denim_id.fetch_add(1, Ordering::Relaxed))
                    .message_kind(message)
                    .build(),
            )
            .await
    }

    async fn send_message(
        &mut self,
        message: ClientEnvelope,
    ) -> Result<MessageStatus, DenimProtocolError> {
        let id = MessageId::generate();
        // Implement the logic to send a message here
        let message = ClientMessage::builder()
            .message(message)
            .r#type(ClientMessageType::ClientMessage.into())
            .id(id.into())
            .build();
        let msg = create_message(&mut self.sending_buffer, message).await?;
        self.client
            .lock()
            .await
            .send(Message::Binary(msg.encode_to_vec().into()))
            .await
            .map_err(DenimProtocolError::WebSocketError)?;

        let response = match &mut self.status_messages {
            // Client can only send one message at a time, and receive a response to that message
            // This means that the next status in the queue is always for the current message
            Some(status) => status
                .recv()
                .await
                .ok_or(DenimProtocolError::WebSocketError(
                    WebSocketError::Disconnected,
                )),
            None => Err(DenimProtocolError::WebSocketError(
                WebSocketError::Disconnected,
            )),
        }?;

        match response.validate(id)? {
            Some(status) => Ok(status),
            None => {
                let res = self
                    .client
                    .lock()
                    .await
                    .send(Message::Close(Some(CloseFrame {
                        code: CloseCode::Error,
                        reason: "Request and Response Id did not match".into(),
                    })))
                    .await;
                match res {
                    Ok(()) => Err(DenimProtocolError::ReceivedWrongResponseId),
                    Err(err) => Err(DenimProtocolError::WebSocketError(err)),
                }
            }
        }
    }
}
