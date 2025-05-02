pub mod buffers;
mod error;
pub mod rng;

pub use error::{ConversionError, DenimBufferError, DenimEncodeDecodeError};

include!(concat!(env!("OUT_DIR"), "/_includes.rs"));

use denim_message::{MessageType, UserMessage};
use libsignal_protocol::{
    CiphertextMessage, CiphertextMessageType, PlaintextContent, PreKeySignalMessage,
    SenderKeyMessage, SignalMessage, SignalProtocolError,
};

impl From<CiphertextMessageType> for MessageType {
    fn from(val: CiphertextMessageType) -> Self {
        match val {
            CiphertextMessageType::Whisper => MessageType::SignalMessage,
            CiphertextMessageType::PreKey => MessageType::PreKeySignalMessage,
            CiphertextMessageType::SenderKey => MessageType::SenderKeyMessage,
            CiphertextMessageType::Plaintext => MessageType::PlaintextContent,
        }
    }
}

impl From<MessageType> for CiphertextMessageType {
    fn from(val: MessageType) -> Self {
        match val {
            MessageType::SignalMessage => CiphertextMessageType::Whisper,
            MessageType::PreKeySignalMessage => CiphertextMessageType::PreKey,
            MessageType::SenderKeyMessage => CiphertextMessageType::SenderKey,
            MessageType::PlaintextContent => CiphertextMessageType::Plaintext,
        }
    }
}

impl UserMessage {
    pub fn ciphertext(&self) -> Result<CiphertextMessage, SignalProtocolError> {
        Ok(match self.message_type() {
            MessageType::SignalMessage => {
                CiphertextMessage::SignalMessage(SignalMessage::try_from(self.content.as_slice())?)
            }
            MessageType::PreKeySignalMessage => CiphertextMessage::PreKeySignalMessage(
                PreKeySignalMessage::try_from(self.content.as_slice())?,
            ),
            MessageType::SenderKeyMessage => CiphertextMessage::SenderKeyMessage(
                SenderKeyMessage::try_from(self.content.as_slice())?,
            ),
            MessageType::PlaintextContent => CiphertextMessage::PlaintextContent(
                PlaintextContent::try_from(self.content.as_slice())?,
            ),
        })
    }
}
