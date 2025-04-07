use denim_sam_common::DenimBufferError;
use derive_more::{Display, Error, From};
use libsignal_protocol::SignalProtocolError;
use sam_client::net::protocol::{error::DecodeError, websocket::WebSocketError};
use sam_client::{net::ApiClientError, ClientError};

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

#[derive(From, Debug)]
pub enum DenimClientError {
    Client(ClientError),
    Api(ApiClientError),
    SignalProtocol(SignalProtocolError),
}
