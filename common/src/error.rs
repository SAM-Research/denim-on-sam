use derive_more::{Display, Error};

use crate::buffers::ChunkDecodeError;

#[derive(Debug, Display, Error)]
pub enum LibError {
    ChunkDecode(ChunkDecodeError),
    ChunkEncode,
}
