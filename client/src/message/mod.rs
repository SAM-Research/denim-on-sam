use denim_sam_common::buffers::{DenimMessage, SendingBuffer};
use error::MessageError;
use prost::Message;
use sam_common::sam_message::ClientMessage;

pub mod error;
pub mod process;
pub mod queue;
pub mod traits;

pub async fn create_message<T: SendingBuffer>(
    sending_buffer: &mut T,
    message: ClientMessage,
) -> Result<DenimMessage, MessageError> {
    let message = message.encode_to_vec();
    let size = message
        .len()
        .try_into()
        .map_err(|_| MessageError::MessageTooBig)?;
    let deniable_payload = sending_buffer
        .get_deniable_payload(size)
        .await?;

    Ok(DenimMessage::builder()
        .regular_payload(message)
        .deniable_payload(deniable_payload)
        .build())
}
