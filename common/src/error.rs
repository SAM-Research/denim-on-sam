use derive_more::{Display, Error, From};

#[derive(Debug, Display, Error, From)]
pub enum DenimBufferError {
    ChunkDecodeError,
    ChunkEncodeError,
    MinPayloadLengthTooHighError,
    ChunkBufferNotFound,
    NoChunksInDeniablePayloadError,
}
