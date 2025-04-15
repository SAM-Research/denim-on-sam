use std::sync::Arc;

use denim_sam_common::{
    buffers::{DenimChunk, DenimMessage, ReceivingBuffer, SendingBuffer},
    denim_message::DeniableMessage,
};
use futures_util::{stream::SplitStream, StreamExt};
use log::{debug, error};

use prost::bytes::Bytes;
use prost::Message as PMessage;
use sam_client::net::protocol::decode::{EnvelopeOrStatus, ServerStatus};
use sam_common::{
    address::MessageId,
    sam_message::{ClientMessage, ClientMessageType, ServerEnvelope, ServerMessage},
};
use sam_net::{
    error::WebSocketError,
    websocket::{WebSocket, WebSocketClient, WebSocketReceiver},
};
use tokio::sync::{mpsc::Sender, Mutex};
use tokio_tungstenite::tungstenite::Message;

use crate::{error::DenimProtocolError, message::create_message};

#[derive(Debug)]
pub enum SamDenimMessage {
    Denim(DeniableMessage),
    Sam(ServerEnvelope),
}

pub struct DenimReceiver<T: SendingBuffer, U: ReceivingBuffer> {
    client: Arc<Mutex<WebSocketClient>>,
    enqueue_sam_status: Sender<ServerStatus>,
    enqueue_message: Sender<SamDenimMessage>,
    sending_buffer: T,
    receiving_buffer: U,
}

impl<T: SendingBuffer, U: ReceivingBuffer> DenimReceiver<T, U> {
    pub fn new(
        client: Arc<Mutex<WebSocketClient>>,
        enqueue_sam_status: Sender<ServerStatus>,
        enqueue_message: Sender<SamDenimMessage>,
        sending_buffer: T,
        receiving_buffer: U,
    ) -> Self {
        Self {
            client,
            enqueue_sam_status,
            enqueue_message,
            sending_buffer,
            receiving_buffer,
        }
    }

    async fn send_ack(&mut self, id: MessageId) -> Result<(), DenimProtocolError> {
        let msg = create_message(
            &mut self.sending_buffer,
            ClientMessage::builder()
                .id(id.into())
                .r#type(ClientMessageType::ClientAck.into())
                .build(),
        )
        .await?;
        self.client
            .lock()
            .await
            .send(Message::Binary(msg.to_bytes()?.into()))
            .await
            .map_err(DenimProtocolError::from)
    }

    async fn handle_sam_message(
        &mut self,
        message: ServerMessage,
    ) -> Result<(), DenimProtocolError> {
        let res = match EnvelopeOrStatus::try_from(message)? {
            EnvelopeOrStatus::Envelope(id, envelope) => self.dispatch_envelope(id, envelope).await,
            EnvelopeOrStatus::Status(status) => self.dispatch_server_status(status).await,
        };

        match res {
            Ok(Some(id)) => self.send_ack(id).await,
            Ok(None) => Ok(()),
            Err(e) => Err(e),
        }
    }

    async fn dispatch_envelope(
        &mut self,
        id: MessageId,
        envelope: ServerEnvelope,
    ) -> Result<Option<MessageId>, DenimProtocolError> {
        self.enqueue_message
            .send(SamDenimMessage::Sam(envelope))
            .await
            .inspect_err(|e| debug!("{e}"))
            .map_err(|_| DenimProtocolError::WebSocketError(WebSocketError::Disconnected))
            .map(|_| Some(id))
    }

    async fn dispatch_server_status(
        &mut self,
        status: ServerStatus,
    ) -> Result<Option<MessageId>, DenimProtocolError> {
        self.enqueue_sam_status
            .send(status)
            .await
            .inspect_err(|e| debug!("{e}"))
            .map_err(|_| DenimProtocolError::WebSocketError(WebSocketError::Disconnected))
            .map(|_| None)
    }

    async fn handle_chunks(&mut self, chunks: Vec<DenimChunk>) {
        let results = self.receiving_buffer.process_chunks(chunks).await;
        for res in results {
            let send_res = match res {
                Ok(msg) => self.enqueue_message.send(SamDenimMessage::Denim(msg)).await,
                Err(e) => {
                    error!("Failed to handle deniable message: '{e}'");
                    continue;
                }
            };
            match send_res {
                Ok(_) => continue,
                Err(e) => error!("Failed to enqueue denim chunk: {e}"),
            }
        }
    }

    async fn validate_and_enqueue(
        &mut self,
        regular: ServerMessage,
        chunks: Vec<DenimChunk>,
    ) -> Result<(), DenimProtocolError> {
        self.handle_chunks(chunks).await;
        self.handle_sam_message(regular).await
    }
}

#[async_trait::async_trait]
impl<T: SendingBuffer, U: ReceivingBuffer> WebSocketReceiver for DenimReceiver<T, U> {
    async fn handler(&mut self, mut receiver: SplitStream<WebSocket>) {
        while let Some(Ok(msg)) = receiver.next().await {
            let res = match msg {
                Message::Binary(b) => DenimMessage::decode(b.to_vec()),
                Message::Close(_) => break,
                _ => continue,
            };
            let (sam_message, denim_chunks) = match res {
                Ok(msg) => {
                    let regular = ServerMessage::decode(Bytes::from(msg.regular_payload));
                    (regular, msg.deniable_payload)
                }
                Err(e) => {
                    error!("Failed to decode DenimMessage from server '{e}', disconnecting...");
                    break;
                }
            };

            let msg = match sam_message {
                Ok(msg) => msg,
                Err(e) => {
                    error!("Failed to decode ServerMessage from server '{e}', disconnecting...");
                    break;
                }
            };

            match self
                .validate_and_enqueue(msg, denim_chunks.denim_chunks().to_owned())
                .await
            {
                Ok(()) => continue,
                Err(DenimProtocolError::WebSocketError(WebSocketError::Disconnected)) => {
                    break;
                }
                Err(x) => {
                    error!("Failed to handle server message '{x}', disconnecting...");
                    break;
                }
            };
        }
    }
}

#[cfg(test)]
pub mod test {
    use std::{sync::Arc, time::Duration};

    use denim_sam_common::{
        buffers::{
            types::DenimMessage, DeniablePayload, InMemoryReceivingBuffer, InMemorySendingBuffer,
            SendingBuffer,
        },
        denim_message::{deniable_message::MessageKind, DeniableMessage, MessageType, UserMessage},
    };
    use futures_util::SinkExt;
    use prost::Message as PMessage;
    use rand::RngCore;
    use rstest::rstest;

    use crate::receiver::DenimReceiver;
    use sam_common::{
        address::MessageId,
        sam_message::{
            server_message::Content, SamMessageType, ServerEnvelope, ServerMessage,
            ServerMessageType,
        },
        AccountId,
    };
    use sam_net::websocket::{WebSocketClient, WebSocketClientConfig};
    use sam_test_utils::get_next_port;
    use tokio::{
        net::TcpListener,
        sync::{
            mpsc::{self, channel},
            oneshot::{self, Receiver},
            Mutex,
        },
    };
    use tokio_tungstenite::{accept_async, tungstenite::Message};

    use super::SamDenimMessage;

    impl SamDenimMessage {
        pub fn some_sam(self) -> Option<ServerEnvelope> {
            if let SamDenimMessage::Sam(envelope) = self {
                Some(envelope)
            } else {
                None
            }
        }

        pub fn some_denim(self) -> Option<DeniableMessage> {
            if let SamDenimMessage::Denim(message) = self {
                Some(message)
            } else {
                None
            }
        }
    }

    pub fn make_user_message(length: usize) -> MessageKind {
        let mut rng = rand::thread_rng();
        let mut random_bytes = vec![0u8; length];
        rng.fill_bytes(&mut random_bytes);
        MessageKind::DeniableMessage(UserMessage {
            account_id: vec![1_u8],
            message_type: MessageType::SignalMessage.into(),
            content: random_bytes,
        })
    }

    fn make_deniable_message(length: usize) -> DeniableMessage {
        DeniableMessage {
            message_id: 1u32,
            message_kind: Some(make_user_message(length)),
        }
    }

    pub async fn get_payload<T: SendingBuffer>(
        buffer: &mut T,
        denim: bool,
        len: u32,
    ) -> Result<DeniablePayload, String> {
        if denim {
            let msg = make_deniable_message(10);
            buffer.enqueue_message(msg).await;
        }
        Ok(buffer
            .get_deniable_payload(len)
            .await
            .expect("Should be able to get_deniable_payload"))
    }

    pub async fn test_server(
        addr: &str,
        actions: Vec<ClientAction>,
        envelope: ServerMessage,
        status: ServerMessage,
        q: f32,
        stop_signal: Receiver<()>,
    ) -> Receiver<Option<String>> {
        let actions = actions.clone();
        let listener = TcpListener::bind(addr).await.expect("can bind tcp");

        let mut sending_buffer = InMemorySendingBuffer::new(q).expect("can create sending buffer");
        let (tx, rx) = oneshot::channel();
        let env_msg = envelope.encode_to_vec();
        let env_len: u32 = env_msg.len().try_into().expect("envelope fits");
        let status_msg = status.encode_to_vec();
        let status_len: u32 = status_msg.len().try_into().expect("message fits");
        tokio::spawn(async move {
            let (stream, _) = listener.accept().await.expect("can  accept");
            let mut ws_stream = accept_async(stream)
                .await
                .expect("can get websocket stream");
            let mut error = None;
            for action in actions {
                let payload = match action {
                    ClientAction::Deniable => get_payload(&mut sending_buffer, true, env_len)
                        .await
                        .map(|x| {
                            DenimMessage::builder()
                                .regular_payload(env_msg.clone())
                                .deniable_payload(x)
                                .build()
                                .to_bytes()
                                .expect("Can encode DenimMessage")
                        }),
                    ClientAction::Regular => get_payload(&mut sending_buffer, false, env_len)
                        .await
                        .map(|x| {
                            DenimMessage::builder()
                                .regular_payload(env_msg.clone())
                                .deniable_payload(x)
                                .build()
                                .to_bytes()
                                .expect("Can encode DenimMessage")
                        }),
                    ClientAction::Status => get_payload(&mut sending_buffer, false, status_len)
                        .await
                        .map(|x| {
                            DenimMessage::builder()
                                .regular_payload(status_msg.clone())
                                .deniable_payload(x)
                                .build()
                                .to_bytes()
                                .expect("Can encode DenimMessage")
                        }),
                };

                let res = match payload {
                    Ok(msg) => ws_stream
                        .send(Message::Binary(msg.into()))
                        .await
                        .map_err(|_| "Failed to send message".to_string()),
                    Err(e) => Err(e),
                };
                match res {
                    Ok(_) => continue,
                    Err(e) => {
                        error = Some(e);
                        break;
                    }
                }
            }
            let _ = tokio::time::timeout(Duration::from_secs(5), stop_signal).await;
            let _ = tx.send(error);
        });
        rx
    }

    #[derive(Clone)]
    pub enum ClientAction {
        Deniable,
        Regular,
        Status,
    }

    fn create_envelope() -> ServerMessage {
        let id = MessageId::generate();
        let envelope = ServerEnvelope::builder()
            .content(vec![69; 100])
            .destination_account_id(AccountId::generate().into())
            .destination_device_id(1)
            .id(id.into())
            .source_account_id(AccountId::generate().into())
            .source_device_id(1)
            .r#type(SamMessageType::PlaintextContent.into())
            .build();
        ServerMessage::builder()
            .content(Content::ServerEnvelope(envelope))
            .id(id.into())
            .r#type(ServerMessageType::ServerMessage.into())
            .build()
    }

    fn create_status() -> ServerMessage {
        ServerMessage::builder()
            .id(MessageId::generate().into())
            .r#type(ServerMessageType::ServerAck.into())
            .build()
    }

    #[rstest]
    #[case(vec![ClientAction::Deniable, ClientAction::Regular, ClientAction::Status], get_next_port())]
    #[case(vec![ClientAction::Deniable, ClientAction::Deniable, ClientAction::Deniable], get_next_port())]
    #[case(vec![ClientAction::Regular, ClientAction::Regular, ClientAction::Regular], get_next_port())]
    #[case(vec![ClientAction::Status, ClientAction::Status, ClientAction::Status], get_next_port())]
    #[tokio::test]
    async fn receive_denim_message(#[case] actions: Vec<ClientAction>, #[case] port: u16) {
        let q = 1.0;
        let addr = format!("127.0.0.1:{port}");

        let envelope = create_envelope();
        let status = create_status();
        let (stop_tx, stop_rx) = oneshot::channel();
        let server_result = test_server(
            &addr,
            actions.clone(),
            envelope.clone(),
            status.clone(),
            1.0,
            stop_rx,
        )
        .await;
        let client: Arc<Mutex<WebSocketClient>> = Arc::new(Mutex::new(
            WebSocketClientConfig::builder()
                .url(format!("ws://{}", addr))
                .build()
                .into(),
        ));

        let (status_tx, mut status_rx) = mpsc::channel(10);
        let send_buffer = InMemorySendingBuffer::new(q).expect("can create sending buffer");
        let recv_buffer = InMemoryReceivingBuffer::default();
        let (tx, mut chunk_rx) = channel(10);
        let receiver = DenimReceiver::new(
            client.clone(),
            status_tx,
            tx,
            send_buffer.clone(),
            recv_buffer,
        );
        client
            .lock()
            .await
            .connect(receiver)
            .await
            .expect("Can connect");

        let mut actual = Vec::new();
        for action in actions {
            let (data, deniable_data) = match action {
                ClientAction::Deniable => {
                    let a_msg_1 = tokio::time::timeout(Duration::from_millis(300), chunk_rx.recv())
                        .await
                        .expect("envelope does not timeout")
                        .expect("Can get envelope");
                    let a_msg_2 = tokio::time::timeout(Duration::from_millis(300), chunk_rx.recv())
                        .await
                        .expect("msg does not timeout")
                        .expect("Can get msg");

                    let (env, den) = match (a_msg_1, a_msg_2) {
                        (SamDenimMessage::Sam(env), SamDenimMessage::Denim(den)) => (env, den),
                        (SamDenimMessage::Denim(den), SamDenimMessage::Sam(env)) => (env, den),
                        _ => panic!("Did not expect two of the same type"),
                    };
                    (
                        Some(env.content),
                        Some(matches!(
                            den.message_kind,
                            Some(MessageKind::DeniableMessage(_))
                        )),
                    )
                }
                ClientAction::Regular => {
                    let a_env = tokio::time::timeout(Duration::from_millis(300), chunk_rx.recv())
                        .await
                        .expect("envelope does not timeout")
                        .expect("Can get envelope");
                    (Some(a_env.some_sam().expect("expects sam").content), None)
                }
                ClientAction::Status => {
                    tokio::time::timeout(Duration::from_millis(300), status_rx.recv())
                        .await
                        .expect("chunk does not timeout")
                        .expect("Can get status");
                    (None, None)
                }
            };

            actual.push((action, data, deniable_data));
        }
        stop_tx.send(()).expect("Can stop server");
        server_result.await.expect("Server works");
        for (action, data, is_denim) in actual {
            match action {
                ClientAction::Deniable => {
                    assert!(
                        data.is_some_and(|x| x == vec![69; 100]),
                        "Deniable data did not match"
                    );
                    assert!(matches!(is_denim, Some(true)));
                }
                ClientAction::Regular => {
                    assert!(
                        data.is_some_and(|x| x == vec![69; 100]),
                        "Regular data did not match"
                    );
                    assert!(is_denim.is_none())
                }
                ClientAction::Status => {
                    assert!(data.is_none());
                    assert!(is_denim.is_none());
                }
            }
        }
    }
}
