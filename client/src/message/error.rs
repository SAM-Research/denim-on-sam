use denim_sam_common::DenimBufferError;
use derive_more::{Display, Error, From};
use sam_client::{storage::error::MessageStoreError, ClientError};

use crate::encryption::error::EncryptionError;

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
    ClientError(ClientError),
    EncryptionError(EncryptionError),
    ServerError(#[error(not(source))] String),
}
