use denim_sam_common::{buffers::SendingBuffer, denim_message::DenimMessage};
use prost::Message;
use sam_common::sam_message::ClientMessage;

use crate::error::MessageError;

pub async fn create_message<T: SendingBuffer>(
    sending_buffer: &mut T,
    message: ClientMessage,
) -> Result<DenimMessage, MessageError> {
    let message = message.encode_to_vec();
    let size = message
        .len()
        .try_into()
        .map_err(|_| MessageError::MessageTooBig)?;
    let denim_payload = sending_buffer
        .get_deniable_payload(size)
        .await?
        .map_or(Ok(Vec::new()), |x| x.to_bytes())?;

    Ok(DenimMessage::builder()
        .regular_payload(message)
        .deniable_payload(denim_payload)
        .build())
}
