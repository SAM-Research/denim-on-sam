use denim_sam_common::DenimBufferError;
use derive_more::{Display, Error, From};
use libsignal_protocol::SignalProtocolError;
use sam_client::storage::error::MessageStoreError;

use crate::encryption::error::{EncryptionError, KeyError};

#[derive(Debug, Error, Display, From)]
pub enum MessageError {
    MessageTooBig,
    DenimBufferError(DenimBufferError),
}

#[derive(Debug, Error, Display, From)]
pub enum MessageProcessingError {
    MessageKindWasNone,
    MalformedMessage,
    MessageStore(MessageStoreError),
    EncryptionError(EncryptionError),
    KeyError(KeyError),
    SignalProtocolError(SignalProtocolError),
    ServerError(#[error(not(source))] String),
}
