use denim_sam_common::{buffers::MessageId, DenimBufferError};
use derive_more::{Display, Error};

#[derive(Debug, Display, Error)]
pub enum BufferManagerError {
    DenimBufferError(DenimBufferError),
    MalformedMessage(#[error(not(source))] MessageId),
    ClientSendError(#[error(not(source))] MessageId),
    ClientSendServerResponse(#[error(not(source))] MessageId),
    FailedToEnqueueRequest,
}
