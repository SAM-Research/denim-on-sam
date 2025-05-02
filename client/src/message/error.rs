use denim_sam_common::{DenimBufferError, DenimEncodeDecodeError};
use derive_more::{Display, Error, From};
use libsignal_protocol::SignalProtocolError;
use sam_client::storage::error::{ContactStoreError, MessageStoreError};

use crate::encryption::error::{EncryptionError, KeyError};

#[derive(Debug, Error, Display, From)]
pub enum MessageError {
    MessageTooBig,
    DenimBufferError(DenimBufferError),
    DenimEncodeDecodeError(DenimEncodeDecodeError),
}

#[derive(Debug, Error, Display, From)]
pub enum MessageProcessingError {
    MessageKindWasNone,
    MalformedMessage,
    MessageStore(MessageStoreError),
    EncryptionError(EncryptionError),
    KeyError(KeyError),
    SignalProtocol(SignalProtocolError),
    ServerError(#[error(not(source))] String),
    ContactStore(ContactStoreError),
}
