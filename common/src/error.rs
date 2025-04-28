use derive_more::{Display, Error, From};

#[derive(Debug, Display, Error, From)]
pub enum DenimBufferError {
    MinPayloadLengthTooHighError,
    ChunkBufferNotFound,
    EncodingDecoding(DenimEncodeDecodeError),
}

#[derive(Debug, Display, Error, From)]
pub enum DenimEncodeDecodeError {
    DenimMessageEncode,
    DenimMessageDecode,
    ChunkEncode,
    DeniableMessageDecode,
}

#[derive(Debug, Display, Error, From)]
pub enum ConversionError {
    SeedConversionError,
}
