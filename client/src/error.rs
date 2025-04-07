use denim_sam_common::DenimBufferError;
use derive_more::{Display, Error, From};
use sam_client::net::protocol::{error::DecodeError, websocket::WebSocketError};
#[derive(Debug, Error, Display, From)]
pub enum DenimProtocolError {
    SamDecodeError(DecodeError),
    WebSocketError(WebSocketError),
    MessageError(MessageError),
    ReceivedWrongResponseId,
}

#[derive(Debug, Error, Display, From)]
pub enum MessageError {
    MessageTooBig,
    DenimBufferError(DenimBufferError),
}
