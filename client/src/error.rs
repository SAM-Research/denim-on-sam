use derive_more::derive::{Debug, From};
use libsignal_protocol::SignalProtocolError;
use sam_client::{net::ApiClientError, ClientError};

#[derive(From, Debug)]
pub enum DenimClientError {
    Client(ClientError),
    Api(ApiClientError),
    SignalProtocol(SignalProtocolError),
}
