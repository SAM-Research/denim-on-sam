use denim_sam_common::{
    buffers::{DenimMessage, SendingBuffer},
    denim_message::{denim_envelope::MessageKind, DenimEnvelope},
};
use error::MessageError;
use log::info;
use prost::Message;
use sam_common::sam_message::ClientMessage;

pub mod error;
pub mod process;
pub mod queue;
pub mod traits;

pub async fn create_message<T: SendingBuffer>(
    sending_buffer: &mut T,
    message: ClientMessage,
) -> Result<DenimEnvelope, MessageError> {
    let message = message.encode_to_vec();
    let size = message
        .len()
        .try_into()
        .map_err(|_| MessageError::MessageTooBig)?;
    let deniable_payload = sending_buffer.get_deniable_payload(size).await?;
    let denim_msg = DenimMessage::builder()
        .regular_payload(message)
        .deniable_payload(deniable_payload)
        .q(0.0) // Only server sets this field
        .build()
        .encode()?;
    info!("DENIM MESSAGE: {}", denim_msg.len());
    Ok(DenimEnvelope::builder()
        .message_kind(MessageKind::DenimMessage(denim_msg))
        .build())
}
