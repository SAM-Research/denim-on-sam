use axum::{
    extract::ws::{CloseFrame as ACloseFrame, Message as AMessage, WebSocket},
    http,
};
use log::debug;
use sam_client::net::protocol::websocket::WebSocketClientConfig;
use tokio_tungstenite::{
    tungstenite::{protocol::CloseFrame, Message},
    Connector,
};

use crate::{error::ServerError, state::DenimState};

pub type AxumWebSocket = WebSocket;
pub type AxumMessage = AMessage;
pub type AxumCloseFrame = ACloseFrame;
pub type TungsteniteMessage = Message;
pub type TungsteniteCloseFrame = CloseFrame;

pub fn into_axum_message(msg: Message) -> Option<AxumMessage> {
    Some(match msg {
        Message::Text(utf8_bytes) => AxumMessage::Text(
            String::from_utf8(utf8_bytes.as_bytes().to_vec())
                .ok()?
                .into(),
        ),
        Message::Binary(bytes) => AxumMessage::Binary(bytes),
        Message::Ping(bytes) => AxumMessage::Ping(bytes),
        Message::Pong(bytes) => AxumMessage::Pong(bytes),
        Message::Close(Some(close_frame)) => AxumMessage::Close(Some(AxumCloseFrame {
            code: close_frame.code.into(),
            reason: close_frame.reason.to_string().into(),
        })),
        Message::Close(None) => AxumMessage::Close(None),
        Message::Frame(_) => {
            return None;
        }
    })
}

pub fn into_tungstenite_message(msg: AxumMessage) -> Option<TungsteniteMessage> {
    Some(match msg {
        AxumMessage::Text(text) => TungsteniteMessage::Text(text.to_string().into()),
        AxumMessage::Binary(bytes) => TungsteniteMessage::Binary(bytes),
        AxumMessage::Ping(bytes) => TungsteniteMessage::Ping(bytes),
        AxumMessage::Pong(bytes) => TungsteniteMessage::Pong(bytes),
        AxumMessage::Close(Some(close_frame)) => {
            TungsteniteMessage::Close(Some(TungsteniteCloseFrame {
                code: close_frame.code.into(),
                reason: close_frame.reason.to_string().into(),
            }))
        }
        AxumMessage::Close(None) => TungsteniteMessage::Close(None),
    })
}

pub fn websocket_config(
    basic: String,
    state: &DenimState,
) -> Result<WebSocketClientConfig, ServerError> {
    let (url, connector) = match state.ws_proxy_tls_config() {
        None => (format!("ws://{}", state.sam_url()), None),
        Some(config) => (
            format!("wss://{}", state.sam_url()),
            Some(Connector::Rustls(config)),
        ),
    };
    Ok(WebSocketClientConfig::builder()
        .buffer(state.channel_buffer())
        .maybe_tls(connector)
        .url(format!("{}/api/v1/websocket", url))
        .headers(vec![(
            http::header::AUTHORIZATION,
            http::HeaderValue::from_str(&basic)
                .inspect_err(|e| debug!("{e}"))
                .map_err(|_| ServerError::SAMUnAuth)?,
        )])
        .build())
}
