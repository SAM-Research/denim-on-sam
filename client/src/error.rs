use denim_sam_common::DenimEncodeDecodeError;
use derive_more::{Display, Error, From};
use libsignal_protocol::SignalProtocolError;
use sam_client::logic::LogicError;
use sam_client::net::protocol::error::DecodeError;
use sam_client::net::protocol::error::ProtocolError;
use sam_client::net::ApiClientError;
use sam_client::storage::error::AccountStoreError;
use sam_client::storage::error::ContactStoreError;
use sam_client::storage::error::StoreCreationError;
use sam_net::error::WebSocketError;

use crate::encryption::error::EncryptionError;
use crate::message::error::{MessageError, MessageProcessingError};
use crate::store::SeedStoreError;

#[derive(Debug, Error, Display, From)]
pub enum DenimProtocolError {
    SamDecodeError(DecodeError),
    WebSocketError(WebSocketError),
    DenimEncodeDecode(DenimEncodeDecodeError),
    MessageError(MessageError),
    Protocol(ProtocolError),
    ReceivedWrongResponseId,
    InvalidCredentials,
    FailedToReceiveQStatus,
}

#[derive(Debug, Error, Display, From)]
pub enum DenimClientError {
    StoreCreation(StoreCreationError),
    Logic(LogicError),
    AccountStore(AccountStoreError),
    ContactStore(ContactStoreError),
    SeedStore(SeedStoreError),
    Api(ApiClientError),
    MessageProcessingError(MessageProcessingError),
    EncryptionError(EncryptionError),
    SignalProtocol(SignalProtocolError),
    Protocol(DenimProtocolError),
    NotSupported,
}
