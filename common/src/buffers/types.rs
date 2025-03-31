use crate::LibError;
use bincode::config;
use bincode::{Decode, Encode};
use bon::Builder;

#[derive(Encode, Decode, Builder)]
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

    pub fn get_size_without_payload() -> usize {
        let chunk = DenimChunk::new(Vec::new(), 0, 0, Flag::None);
        let encoded = bincode::encode_to_vec(chunk, config::standard().with_fixed_int_encoding())
            .map_err(|_| LibError::ChunkEncode)
            .expect("Should be able to encode");
        encoded.len()
    }

    pub fn get_size(&self) -> Result<usize, LibError> {
        let encoded = bincode::encode_to_vec(self, config::standard().with_fixed_int_encoding())
            .map_err(|_| LibError::ChunkEncode)?;
        Ok(encoded.len())
    }
}

#[derive(Encode, Decode, Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum Flag {
    None = 0,
    Final = 1,
    DummyPadding = 2,
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

    pub fn to_bytes(self) -> Result<Vec<Vec<u8>>, LibError> {
        let mut encoded_chunks = Vec::new();
        for chunk in self.denim_chunks {
            encoded_chunks.push(
                bincode::encode_to_vec(chunk, config::standard().with_fixed_int_encoding())
                    .map_err(|_| LibError::ChunkEncode)?,
            );
        }
        if let Some(garbage) = self.garbage {
            encoded_chunks.push(garbage);
        }
        Ok(encoded_chunks)
    }
}
