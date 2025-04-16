use crate::error::DenimEncodeDecodeError;
use bincode::config;
use bincode::{Decode, Encode};
use bon::Builder;

#[derive(Encode, Decode, Builder, Debug, Clone)]
pub struct DenimChunk {
    chunk: Vec<u8>,
    message_id: MessageId,
    sequence_number: SequenceNumber,
    flag: Flag,
}

pub type SequenceNumber = u32;
pub type MessageId = u32;

impl DenimChunk {
    pub fn new(chunk: Vec<u8>, message_id: u32, sequence_number: u32, flag: Flag) -> Self {
        Self {
            chunk,
            message_id,
            sequence_number,
            flag,
        }
    }
    pub fn message_id(&self) -> u32 {
        self.message_id
    }
    pub fn sequence_number(&self) -> u32 {
        self.sequence_number
    }
    pub fn flag(&self) -> Flag {
        self.flag
    }
    pub fn chunk(&self) -> &Vec<u8> {
        &self.chunk
    }
    pub fn chunk_mut(&mut self) -> &mut Vec<u8> {
        &mut self.chunk
    }

    pub fn get_size_without_payload() -> Result<usize, DenimEncodeDecodeError> {
        let chunk = DenimChunk::new(Vec::new(), 0, 0, Flag::None);
        bincode::encode_to_vec(chunk, config::standard().with_fixed_int_encoding())
            .map_err(|_| DenimEncodeDecodeError::ChunkEncode)
            .map(|encoded| encoded.len())
    }

    pub fn get_size(&self) -> Result<usize, DenimEncodeDecodeError> {
        bincode::encode_to_vec(self, config::standard().with_fixed_int_encoding())
            .map_err(|_| DenimEncodeDecodeError::ChunkEncode)
            .map(|encoded| encoded.len())
    }
}

#[derive(Encode, Decode, Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum Flag {
    None = 0,
    Final = 1,
    DummyPadding = 2,
}

#[derive(Encode, Decode, Builder, Clone, Default)]
pub struct DeniablePayload {
    denim_chunks: Vec<DenimChunk>,
    garbage: Vec<u8>,
}

impl DeniablePayload {
    pub fn denim_chunks(&self) -> &Vec<DenimChunk> {
        &self.denim_chunks
    }

    pub fn denim_chunks_mut(&mut self) -> &mut Vec<DenimChunk> {
        &mut self.denim_chunks
    }

    pub fn garbage(&self) -> &Vec<u8> {
        &self.garbage
    }
}

#[derive(Encode, Decode, Builder, Clone)]
pub struct DenimMessage {
    pub regular_payload: Vec<u8>,
    pub deniable_payload: DeniablePayload,
}

impl DenimMessage {
    pub fn encode(self) -> Result<Vec<u8>, DenimEncodeDecodeError> {
        bincode::encode_to_vec(self, config::standard().with_fixed_int_encoding())
            .map_err(|_| DenimEncodeDecodeError::DenimMessageEncode)
    }

    pub fn decode(bytes: Vec<u8>) -> Result<Self, DenimEncodeDecodeError> {
        let (denim_chunk, _): (DenimMessage, usize) =
            bincode::decode_from_slice(&bytes, config::standard().with_fixed_int_encoding())
                .map_err(|_| DenimEncodeDecodeError::DenimMessageDecode)?;
        Ok(denim_chunk)
    }
}
