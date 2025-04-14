use axum::extract::ws::{CloseFrame as ACloseFrame, Message as AMessage, WebSocket};
use tokio_tungstenite::tungstenite::Message;

pub type AxumWebSocket = WebSocket;
pub type AxumMessage = AMessage;
pub type AxumCloseFrame = ACloseFrame;
pub type TungsteniteMessage = Message;

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
