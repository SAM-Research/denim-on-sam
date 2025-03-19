mod error;

use std::net::SocketAddr;

use axum::{
    extract::{
        ws::{CloseFrame, Message as AxumMessage},
        State, WebSocketUpgrade,
    },
    http,
    response::IntoResponse,
    routing::get,
    Router,
};
use axum_extra::{
    headers::{authorization::Basic, Authorization},
    TypedHeader,
};
use base64::{prelude::BASE64_STANDARD, Engine as _};
use error::ServerError;

use futures_util::{stream::SplitStream, SinkExt, StreamExt};
use log::{debug, error, info};
use sam_client::net::protocol::websocket::{
    WebSocket, WebSocketClient, WebSocketClientConfig, WebSocketReceiver,
};
use tokio::sync::mpsc::Sender;
use tokio_tungstenite::tungstenite::Message;

#[derive(Clone)]
struct DenimState {
    sam_url: String,
}

impl DenimState {
    pub fn new(sam_url: String) -> Self {
        Self { sam_url }
    }
}

struct DenimWebSocketReceiver {}
impl DenimWebSocketReceiver {}
fn into_axum_message(msg: Message) -> Option<AxumMessage> {
    Some(match msg {
        Message::Text(utf8_bytes) => AxumMessage::Text(
            String::from_utf8(utf8_bytes.as_bytes().to_vec())
                .ok()?
                .into(),
        ),
        Message::Binary(bytes) => AxumMessage::Binary(bytes.into()),
        Message::Ping(bytes) => AxumMessage::Ping(bytes.into()),
        Message::Pong(bytes) => AxumMessage::Pong(bytes.into()),
        Message::Close(Some(close_frame)) => AxumMessage::Close(Some(CloseFrame {
            code: close_frame.code.into(),
            reason: close_frame.reason.to_string().into(),
        })),
        Message::Close(None) => AxumMessage::Close(None),
        Message::Frame(_) => {
            return None;
        }
    })
}

fn into_tungstenite_message(msg: AxumMessage) -> Option<Message> {
    Some(match msg {
        AxumMessage::Text(text) => Message::Text(text.to_string().into()),
        AxumMessage::Binary(bytes) => Message::Binary(bytes.into()),
        AxumMessage::Ping(bytes) => Message::Ping(bytes.into()),
        AxumMessage::Pong(bytes) => Message::Pong(bytes.into()),
        AxumMessage::Close(Some(close_frame)) => {
            Message::Close(Some(tokio_tungstenite::tungstenite::protocol::CloseFrame {
                code: close_frame.code.into(),
                reason: close_frame.reason.to_string().into(),
            }))
        }
        AxumMessage::Close(None) => Message::Close(None),
    })
}

#[async_trait::async_trait]
impl WebSocketReceiver<AxumMessage> for DenimWebSocketReceiver {
    async fn handler(
        &mut self,
        mut receiver: SplitStream<WebSocket>,
        enqueue: Sender<AxumMessage>,
    ) {
        while let Some(Ok(msg)) = receiver.next().await {
            let res = match into_axum_message(msg) {
                Some(msg) => enqueue.send(msg).await,
                None => {
                    error!("Failed to convert tungstenite message into axum message");
                    continue;
                }
            };
            if res.is_err() {
                error!("Failed to send")
            }
        }
    }
}

async fn websocket_endpoint(
    State(state): State<DenimState>,
    TypedHeader(Authorization(auth)): TypedHeader<Authorization<Basic>>,
    ws: WebSocketUpgrade,
) -> Result<impl IntoResponse, ServerError> {
    let basic = format!("{}:{}", auth.username(), auth.password());
    let basic = format!("Basic {}", BASE64_STANDARD.encode(basic));
    let mut client: WebSocketClient = WebSocketClientConfig::builder()
        .buffer(10)
        .url(format!("{}/api/v1/websocket", state.sam_url))
        .headers(vec![(
            http::header::AUTHORIZATION,
            http::HeaderValue::from_str(&basic)
                .inspect_err(|e| debug!("{e}"))
                .map_err(|_| ServerError::SAMUnAuth)?,
        )])
        .build()
        .into();

    let receiver = DenimWebSocketReceiver {};

    let queue = client
        .connect(receiver)
        .await
        .map_err(|_| ServerError::SAMUnAuth)?;

    Ok(ws.on_upgrade(move |socket| async move {
        info!("A User Connected");
        let (mut sender, mut receiver) = socket.split();

        tokio::spawn(async move {
            let mut queue = queue;
            while let Some(msg) = queue.recv().await {
                if sender.send(msg).await.is_err() {
                    break;
                }
            }
        });

        tokio::spawn(async move {
            let mut client = client;
            while let Some(Ok(msg)) = receiver.next().await {
                let res = match into_tungstenite_message(msg) {
                    Some(msg) => client.send(msg).await,
                    None => {
                        error!("Failed to convert axum message to tungstenite message");
                        info!("Disconnecting...");

                        break;
                    }
                };
                match res {
                    Ok(_) => continue,
                    Err(e) => {
                        error!("WebSocketError: {e}");
                        info!("Disconnecting...");
                        break;
                    }
                }
            }
        });
    }))
}

#[tokio::main]
async fn main() {
    env_logger::init();
    let addr = "127.0.0.1:8081"
        .parse()
        .expect("Unable to parse socket address");

    let state = DenimState::new("ws://127.0.0.1:8080".to_string());
    let app = Router::new()
        .route("/api/v1/websocket", get(websocket_endpoint))
        .with_state(state);
    info!("Starting DenIM Proxy");
    axum_server::bind(addr)
        .serve(app.into_make_service_with_connect_info::<SocketAddr>())
        .await
        .unwrap();
}
