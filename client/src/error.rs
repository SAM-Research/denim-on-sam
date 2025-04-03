use derive_more::derive::From;
use libsignal_protocol::SignalProtocolError;
use sam_client::{net::ApiClientError, ClientError};

#[derive(From)]
pub enum DenimClientError {
    Client(ClientError),
    Api(ApiClientError),
    SignalProtocol(SignalProtocolError),
}
