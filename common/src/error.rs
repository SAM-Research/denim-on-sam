use derive_more::{Display, Error};

use crate::receiving_buffer::ChunkDecodeError;

#[derive(Debug, Display, Error)]
pub enum LibError {
    ChunkDecode(ChunkDecodeError),
}
