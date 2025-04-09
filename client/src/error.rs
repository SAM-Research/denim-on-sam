use derive_more::{Display, Error, From};
use libsignal_protocol::SignalProtocolError;
use sam_client::net::protocol::error::ProtocolError;
use sam_client::net::protocol::{error::DecodeError, websocket::WebSocketError};
use sam_client::{net::ApiClientError, ClientError};

use crate::message::error::MessageError;

#[derive(Debug, Error, Display, From)]
pub enum DenimProtocolError {
    SamDecodeError(DecodeError),
    WebSocketError(WebSocketError),
    MessageError(MessageError),
    Protocol(ProtocolError),
    ReceivedWrongResponseId,
    InvalidCredentials,
}

#[derive(From, Debug)]
pub enum DenimClientError {
    Client(ClientError),
    Api(ApiClientError),
    SignalProtocol(SignalProtocolError),
    Protocol(DenimProtocolError),
}
