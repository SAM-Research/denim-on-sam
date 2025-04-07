use denim_sam_common::{buffers::MessageId, DenimBufferError};
use derive_more::{Display, Error};

#[derive(Debug, Display, Error)]
pub enum BufferManagerError {
    DenimBufferError(DenimBufferError),
    #[error(ignore)]
    MalformedMessage(MessageId),
    #[error(ignore)]
    ClientSendError(MessageId),
    #[error(ignore)]
    ClientSendServerResponse(MessageId),
    FailedToEnqueueRequest,
}
