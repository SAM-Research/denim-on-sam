use bincode::{Decode, Encode};
use bon::Builder;

#[derive(Encode, Decode, Builder)]
pub struct DenimChunk {
    chunk: Vec<u8>,
    message_id: u32,
    sequence_number: u32,
    flag: Flag,
}

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
}

#[derive(Encode, Decode, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Flag {
    None = 0,
    Final = 1,
    DummyPadding = 2,
}

#[derive(Encode, Decode, Builder)]
pub struct DeniablePayload {
    denim_chunks: Vec<Vec<u8>>,
    garbage: Option<Vec<u8>>,
}

impl DeniablePayload {
    pub fn denim_chunks(&self) -> &Vec<Vec<u8>> {
        &self.denim_chunks
    }

    pub fn garbage(&self) -> &Option<Vec<u8>> {
        &self.garbage
    }
}
