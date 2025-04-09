use denim_sam_common::DenimBufferError;
use derive_more::{Display, Error, From};

#[derive(Debug, Error, Display, From)]
pub enum MessageError {
    MessageTooBig,
    DenimBufferError(DenimBufferError),
}
