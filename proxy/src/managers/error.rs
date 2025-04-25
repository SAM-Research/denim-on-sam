use denim_sam_common::{buffers::MessageId, DenimBufferError};
use derive_more::{Display, Error, From};
use sam_server::managers::error::KeyManagerError;

#[derive(Debug, Display, Error)]
pub enum BufferManagerError {
    DenimBufferError(DenimBufferError),
    MalformedMessage(#[error(not(source))] MessageId),
    ClientSendError(#[error(not(source))] MessageId),
    ClientSendServerResponse(#[error(not(source))] MessageId),
    FailedToEnqueueRequest,
    InvalidAccountId,
}

#[derive(Debug, Display, Error, From)]
pub enum DenimKeyManagerError {
    Sam(KeyManagerError),
    NoSeed,
    NoKeyInStore,
    CouldNotGenerateKeyId,
}
