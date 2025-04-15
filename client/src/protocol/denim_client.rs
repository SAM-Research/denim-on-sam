use std::sync::{
    atomic::{AtomicU32, Ordering},
    Arc,
};

use denim_sam_common::{
    buffers::{ReceivingBuffer, SendingBuffer},
    denim_message::{deniable_message::MessageKind, DeniableMessage},
};
use log::error;
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

pub struct DenimProtocolClient<T: SendingBuffer, U: ReceivingBuffer> {
    client: Arc<Mutex<WebSocketClient>>,
    status_messages: Option<Receiver<ServerStatus>>,
    channel_buffer_size: usize,
    sending_buffer: T,
    receiving_buffer: U,
    denim_id: AtomicU32,
}

impl<T: SendingBuffer, U: ReceivingBuffer> DenimProtocolClient<T, U> {
    pub fn new(
        client: WebSocketClient,
        channel_buffer_size: usize,
        sending_buffer: T,
        receiving_buffer: U,
    ) -> Self {
        Self {
            client: Arc::new(Mutex::new(client)),
            status_messages: None,
            channel_buffer_size,
            sending_buffer,
            receiving_buffer,
            denim_id: AtomicU32::new(0),
        }
    }
}

#[async_trait::async_trait]
impl<T: SendingBuffer, U: ReceivingBuffer> DenimSamClient for DenimProtocolClient<T, U> {
    async fn connect(&mut self) -> Result<Receiver<SamDenimMessage>, DenimProtocolError> {
        let (status_tx, status_rx) = channel(self.channel_buffer_size);
        self.status_messages = Some(status_rx);
        let (tx, rx) = channel(self.channel_buffer_size);
        let handler = DenimReceiver::new(
            self.client.clone(),
            status_tx,
            tx,
            self.sending_buffer.clone(),
            self.receiving_buffer.clone(),
        );
        self.client
            .lock()
            .await
            .connect(handler)
            .await
            .inspect_err(|e| error!("DenimProtocolClient Error: {e}"))
            .map_err(DenimProtocolError::WebSocketError)?;
        Ok(rx)
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
            .send(Message::Binary(msg.to_bytes()?.into()))
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

#[cfg(test)]
mod test {
    use std::time::Duration;

    use crate::{
        protocol::denim_client::{DenimProtocolClient, DenimSamClient},
        receiver::{
            test::{get_payload, make_user_message},
            SamDenimMessage,
        },
    };
    use denim_sam_common::{
        buffers::{
            types::DenimMessage, InMemoryReceivingBuffer, InMemorySendingBuffer, ReceivingBuffer,
        },
        denim_message::DeniableMessage,
    };
    use futures_util::{SinkExt, StreamExt};
    use prost::{bytes::Bytes, Message as PMessage};
    use rstest::rstest;
    use sam_client::net::protocol::{websocket::WebSocketClientConfig, MessageStatus};
    use sam_common::{
        address::MessageId,
        sam_message::{
            server_message::Content, ClientEnvelope, ClientMessage, SamMessage, SamMessageType,
            ServerEnvelope, ServerMessage, ServerMessageType,
        },
        AccountId,
    };
    use test_utils::get_next_port;
    use tokio::{
        net::{TcpListener, TcpStream},
        sync::mpsc::Receiver as MpscReceiver,
        sync::oneshot::{self, Receiver},
    };
    use tokio_tungstenite::{
        accept_async,
        tungstenite::{Error, Message},
        WebSocketStream,
    };

    #[derive(Clone)]
    enum ServerAction {
        SendDenim,
        SendRegular,
        RecvDenim,
        RecvRegular,
    }
    #[rstest]
    #[case(vec![ServerAction::SendDenim], get_next_port())]
    #[case(vec![ServerAction::SendRegular], get_next_port())]
    #[case(vec![ServerAction::RecvDenim], get_next_port())]
    #[case(vec![ServerAction::RecvRegular], get_next_port())]
    #[case(vec![ServerAction::RecvRegular, ServerAction::RecvRegular], get_next_port())]
    #[case(vec![ServerAction::SendRegular, ServerAction::SendRegular], get_next_port())]
    #[case(vec![ServerAction::SendDenim, ServerAction::SendDenim], get_next_port())]
    #[case(vec![ServerAction::RecvDenim, ServerAction::RecvDenim], get_next_port())]
    #[case(vec![ServerAction::SendDenim, ServerAction::RecvDenim], get_next_port())]
    #[case(vec![ServerAction::SendRegular, ServerAction::RecvRegular], get_next_port())]
    #[case(vec![ServerAction::RecvRegular, ServerAction::SendRegular], get_next_port())]
    #[tokio::test]
    async fn deniable_communication(#[case] actions: Vec<ServerAction>, #[case] port: u16) {
        let addr = format!("127.0.0.1:{port}");
        let (stop_tx, stop_rx) = oneshot::channel();

        let server_result = test_server(&addr, actions.clone(), stop_rx).await;

        let mut client = DenimProtocolClient::new(
            WebSocketClientConfig::builder()
                .url(format!("ws://{}", addr))
                .build()
                .into(),
            10,
            InMemorySendingBuffer::new(1.0).expect("can create sending buffer"),
            InMemoryReceivingBuffer::default(),
        );

        let mut receiver = client.connect().await.expect("can connect");
        let actual = perform_client_actions(&mut client, &mut receiver, actions.clone()).await;

        stop_tx.send(()).expect("Can stop server");
        server_result
            .await
            .expect("Server stops")
            .expect("Server works");

        // allow client to update state
        tokio::time::sleep(Duration::from_millis(200)).await;

        assert!(!client.is_connected().await);
        for (action, env, den, status_ok) in actual {
            match action {
                ServerAction::SendDenim => {
                    // client receives denim
                    if let Some(vec) = env {
                        assert_eq!(vec, vec![1, 3, 3, 7, 4, 20]);
                    } else {
                        panic!("Expected Some(vec![1, 3, 3, 7, 4, 20]), found {:?}", env);
                    }
                    assert!(den.is_some());
                    assert!(!status_ok);
                }
                ServerAction::SendRegular => {
                    // client receives regular
                    if let Some(vec) = env {
                        assert_eq!(vec, vec![1, 3, 3, 7, 4, 20]);
                    } else {
                        panic!("Expected Some(vec![1, 3, 3, 7, 4, 20]), found {:?}", env);
                    }
                    assert!(den.is_none());
                    assert!(!status_ok);
                }
                ServerAction::RecvDenim => {
                    // client sends denim
                    assert!(env.is_none());
                    assert!(den.is_none());
                    assert!(status_ok)
                }
                ServerAction::RecvRegular => {
                    // client sends regular
                    assert!(env.is_none());
                    assert!(den.is_none());
                    assert!(status_ok)
                }
            }
        }
    }

    async fn create_server_msg(
        sending: &mut InMemorySendingBuffer,
        denim: bool,
        msg: Vec<u8>,
    ) -> Result<DenimMessage, String> {
        let payload = get_payload(
            sending,
            denim,
            msg.len().try_into().map_err(|_| "Message fits")?,
        )
        .await?;
        Ok(DenimMessage::builder()
            .regular_payload(msg)
            .deniable_payload(payload)
            .build())
    }

    fn server_envelope(content: Vec<u8>) -> (MessageId, Vec<u8>) {
        let id = MessageId::generate();
        let aid = AccountId::generate();
        let msg = ServerMessage::builder()
            .id(id.into())
            .r#type(ServerMessageType::ServerMessage.into())
            .content(Content::ServerEnvelope(
                ServerEnvelope::builder()
                    .id(id.into())
                    .content(content)
                    .destination_device_id(1)
                    .source_device_id(1)
                    .source_account_id(aid.into())
                    .destination_account_id(aid.into())
                    .r#type(SamMessageType::PlaintextContent.into())
                    .build(),
            ))
            .build()
            .encode_to_vec();
        (id, msg)
    }

    fn unpack_client_msg(
        msg: Result<Option<Result<Message, Error>>, String>,
    ) -> Result<DenimMessage, String> {
        Ok(match msg? {
            Some(Ok(Message::Binary(x))) => DenimMessage::decode(x.to_vec()),
            _ => Err("Failed to receive message from client")?,
        }
        .map_err(|_| "Failed to decode client message")?)
    }

    async fn create_server_ack(
        sending: &mut InMemorySendingBuffer,
        receiving: &mut InMemoryReceivingBuffer,
        denim: bool,
        msg: Result<Option<Result<Message, Error>>, String>,
    ) -> Result<DenimMessage, String> {
        let msg = unpack_client_msg(msg)?;
        let chunks = msg.deniable_payload.denim_chunks().to_owned();
        let sam =
            ClientMessage::decode(Bytes::from(msg.regular_payload)).map_err(|e| format!("{e}"))?;
        let results = receiving.process_chunks(chunks).await;

        if denim {
            results
                .first()
                .ok_or("Expected denim message from client")?
                .as_ref()
                .map_err(|_| "Expected successful parse of deniable message")?;
        }
        create_server_msg(
            sending,
            denim,
            ServerMessage::builder()
                .id(sam.id)
                .r#type(ServerMessageType::ServerAck.into())
                .build()
                .encode_to_vec(),
        )
        .await
    }

    async fn prepare_server_message(
        sending: &mut InMemorySendingBuffer,
        action: ServerAction,
    ) -> Result<(DenimMessage, Option<MessageId>), String> {
        let denim = matches!(action, ServerAction::SendDenim);
        let (id, msg) = server_envelope(vec![1, 3, 3, 7, 4, 20]);
        create_server_msg(sending, denim, msg)
            .await
            .map(|msg| (msg, Some(id)))
    }

    async fn prepare_server_ack(
        ws_stream: &mut WebSocketStream<TcpStream>,
        sending: &mut InMemorySendingBuffer,
        receiving: &mut InMemoryReceivingBuffer,
        action: ServerAction,
    ) -> Result<(DenimMessage, Option<MessageId>), String> {
        let denim = matches!(action, ServerAction::RecvDenim);
        let res = tokio::time::timeout(Duration::from_secs(5), ws_stream.next())
            .await
            .map_err(|_| "Client failed to send in time".to_string());
        create_server_ack(sending, receiving, denim, res)
            .await
            .map(|msg| (msg, None))
    }

    async fn wait_for_ack(ws_stream: &mut WebSocketStream<TcpStream>) -> Result<MessageId, String> {
        let res = tokio::time::timeout(Duration::from_secs(5), ws_stream.next())
            .await
            .map_err(|_| "Client failed to send in time".to_string());
        let msg = unpack_client_msg(res)?;
        let msg =
            ClientMessage::decode(Bytes::from(msg.regular_payload)).map_err(|e| format!("{e}"))?;
        MessageId::try_from(msg.id).map_err(|_| "Failed to decode message id".to_string())
    }

    async fn test_server(
        addr: &str,
        actions: Vec<ServerAction>,
        stop_signal: Receiver<()>,
    ) -> Receiver<Result<(), String>> {
        let listener = TcpListener::bind(addr).await.unwrap();
        let (tx, rx) = oneshot::channel();

        tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut ws_stream = accept_async(stream).await.unwrap();
            let mut sending = InMemorySendingBuffer::new(1.0).expect("can create sending buffer");
            let mut receiving = InMemoryReceivingBuffer::default();

            let mut error = Ok(());
            for action in actions {
                let res = match action {
                    ServerAction::SendDenim | ServerAction::SendRegular => {
                        prepare_server_message(&mut sending, action).await
                    }
                    ServerAction::RecvDenim | ServerAction::RecvRegular => {
                        prepare_server_ack(&mut ws_stream, &mut sending, &mut receiving, action)
                            .await
                    }
                };

                let (msg, id) = match res {
                    Ok(x) => x,
                    Err(e) => {
                        error = Err(e);
                        break;
                    }
                };

                let encoded_msg = match msg.to_bytes() {
                    Ok(bytes) => bytes,
                    Err(_) => {
                        error = Err("Failed to encode DenimMessage".to_string());
                        break;
                    }
                };

                if ws_stream
                    .send(Message::Binary(Bytes::from(encoded_msg)))
                    .await
                    .is_err()
                {
                    error = Err("Failed to send to client".to_string());
                    break;
                }

                if let Some(id) = id {
                    let is_match = wait_for_ack(&mut ws_stream)
                        .await
                        .map(|res_id| res_id == id);

                    let res = match is_match {
                        Ok(true) => Ok(()),
                        Ok(false) => Err("Client ack did not match".to_string()),
                        Err(e) => Err(e),
                    };

                    match res {
                        Ok(_) => continue,
                        Err(e) => {
                            error = Err(e);
                            break;
                        }
                    }
                }
            }
            let _ = tokio::time::timeout(Duration::from_secs(5), stop_signal).await;
            let _ = tx.send(error);
        });
        rx
    }

    fn get_actual(
        msg_1: Option<SamDenimMessage>,
        msg_2: Option<SamDenimMessage>,
    ) -> (Vec<u8>, Option<DeniableMessage>) {
        let (env, den) = match (&msg_1, &msg_2) {
            (None, Some(SamDenimMessage::Sam(env))) => (env, None),
            (Some(SamDenimMessage::Sam(env)), None) => (env, None),
            (Some(SamDenimMessage::Sam(env)), Some(SamDenimMessage::Denim(den))) => {
                (env, Some(den.clone()))
            }
            (Some(SamDenimMessage::Denim(den)), Some(SamDenimMessage::Sam(env))) => {
                (env, Some(den.clone()))
            }
            _ => panic!(
                "Unexpected Sam and denim message combination {:?}, {:?}",
                msg_1, msg_2
            ),
        };
        (env.content.clone(), den)
    }

    fn client_envelope() -> ClientEnvelope {
        let aid = AccountId::generate();
        let msg = SamMessage::builder()
            .content(vec![69; 100])
            .destination_account_id(aid.into())
            .destination_device_id(1)
            .r#type(SamMessageType::PlaintextContent.into())
            .build();
        ClientEnvelope::builder().messages(vec![msg]).build()
    }

    async fn perform_client_actions(
        client: &mut DenimProtocolClient<InMemorySendingBuffer, InMemoryReceivingBuffer>,
        receiver: &mut MpscReceiver<SamDenimMessage>,
        actions: Vec<ServerAction>,
    ) -> Vec<(ServerAction, Option<Vec<u8>>, Option<DeniableMessage>, bool)> {
        let mut actual = Vec::new();
        for action in actions {
            match action {
                ServerAction::SendDenim | ServerAction::SendRegular => {
                    let denim = matches!(action, ServerAction::SendDenim);
                    let msg_1 = tokio::time::timeout(Duration::from_millis(300), receiver.recv())
                        .await
                        .expect("msg 1 does not timeout");
                    let msg_2 = if denim {
                        tokio::time::timeout(Duration::from_millis(300), receiver.recv())
                            .await
                            .expect("msg 2 does not timeout")
                    } else {
                        None
                    };

                    let (msg, den) = get_actual(msg_1, msg_2);
                    actual.push((action, Some(msg), den, false));
                }
                ServerAction::RecvDenim | ServerAction::RecvRegular => {
                    client.enqueue_deniable(make_user_message(10)).await;
                    let status = client
                        .send_message(client_envelope())
                        .await
                        .expect("Can send message");
                    let is_ok = matches!(status, MessageStatus::Ok);

                    actual.push((action, None, None, is_ok));
                }
            }
        }
        actual
    }
}
