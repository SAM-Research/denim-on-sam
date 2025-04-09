use crate::error::DenimBufferError;
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
    pub fn set_garbage_flag(&mut self) {
        self.flag = match self.flag {
            Flag::None => Flag::NoneNextGarbage,
            Flag::Final => Flag::FinalNextGarbage,
            Flag::DummyPadding => Flag::DummyPaddingNextGarbage,
            _ => self.flag,
        }
    }
    pub fn has_garbage_flag(&self) -> bool {
        match self.flag {
            Flag::NoneNextGarbage | Flag::FinalNextGarbage | Flag::DummyPaddingNextGarbage => true,
            Flag::Final | Flag::DummyPadding | Flag::None => false,
        }
    }
    pub fn remove_garbage_flag(&mut self) {
        self.flag = match self.flag {
            Flag::NoneNextGarbage => Flag::None,
            Flag::FinalNextGarbage => Flag::Final,
            Flag::DummyPaddingNextGarbage => Flag::DummyPadding,
            _ => self.flag,
        }
    }
    pub fn chunk(&self) -> &Vec<u8> {
        &self.chunk
    }
    pub fn chunk_mut(&mut self) -> &mut Vec<u8> {
        &mut self.chunk
    }

    pub fn get_size_without_payload() -> Result<usize, DenimBufferError> {
        let chunk = DenimChunk::new(Vec::new(), 0, 0, Flag::None);
        bincode::encode_to_vec(chunk, config::standard().with_fixed_int_encoding())
            .map_err(|_| DenimBufferError::ChunkEncodeError)
            .map(|encoded| encoded.len())
    }

    pub fn get_size(&self) -> Result<usize, DenimBufferError> {
        bincode::encode_to_vec(self, config::standard().with_fixed_int_encoding())
            .map_err(|_| DenimBufferError::ChunkEncodeError)
            .map(|encoded| encoded.len())
    }
}

#[derive(Encode, Decode, Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum Flag {
    None = 0,
    NoneNextGarbage = 1,
    Final = 2,
    FinalNextGarbage = 3,
    DummyPadding = 4,
    DummyPaddingNextGarbage = 5,
}

#[derive(Encode, Decode, Builder)]
pub struct DeniablePayload {
    denim_chunks: Vec<DenimChunk>,
    garbage: Option<Vec<u8>>,
}

impl DeniablePayload {
    pub fn denim_chunks(&self) -> &Vec<DenimChunk> {
        &self.denim_chunks
    }

    pub fn denim_chunks_mut(&mut self) -> &mut Vec<DenimChunk> {
        &mut self.denim_chunks
    }

    pub fn garbage(&self) -> &Option<Vec<u8>> {
        &self.garbage
    }

    pub fn to_bytes(self) -> Result<Vec<Vec<u8>>, DenimBufferError> {
        let mut encoded_chunks = Vec::new();
        for chunk in self.denim_chunks {
            encoded_chunks.push(
                bincode::encode_to_vec(chunk, config::standard().with_fixed_int_encoding())
                    .map_err(|_| DenimBufferError::ChunkEncodeError)?,
            );
        }
        if let Some(garbage) = self.garbage {
            encoded_chunks.push(garbage);
        }
        Ok(encoded_chunks)
    }

    pub fn decode(bytes: Vec<Vec<u8>>) -> Result<Vec<DenimChunk>, DenimBufferError> {
        let mut denim_chunks = Vec::new();
        for chunk in bytes {
            let (mut denim_chunk, _): (DenimChunk, usize) =
                bincode::decode_from_slice(&chunk, config::standard().with_fixed_int_encoding())
                    .map_err(|_| DenimBufferError::ChunkDecodeError)?;

            if denim_chunk.has_garbage_flag() {
                denim_chunk.remove_garbage_flag();
                denim_chunks.push(denim_chunk);
                return Ok(denim_chunks);
            }
            denim_chunks.push(denim_chunk);
        }

        Ok(denim_chunks)
    }
}
