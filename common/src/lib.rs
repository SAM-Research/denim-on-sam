pub mod buffers;
mod error;

include!(concat!(env!("OUT_DIR"), "/_includes.rs"));

use denim_message::MessageType;
pub use error::DenimBufferError;
use libsignal_protocol::CiphertextMessageType;

impl Into<MessageType> for CiphertextMessageType {
    fn into(self) -> MessageType {
        match self {
            CiphertextMessageType::Whisper => MessageType::SignalMessage,
            CiphertextMessageType::PreKey => MessageType::PreKeySignalMessage,
            CiphertextMessageType::SenderKey => MessageType::SenderKeyMessage,
            CiphertextMessageType::Plaintext => MessageType::PlaintextContent,
        }
    }
}

impl Into<CiphertextMessageType> for MessageType {
    fn into(self) -> CiphertextMessageType {
        match self {
            MessageType::SignalMessage => CiphertextMessageType::Whisper,
            MessageType::PreKeySignalMessage => CiphertextMessageType::PreKey,
            MessageType::SenderKeyMessage => CiphertextMessageType::SenderKey,
            MessageType::PlaintextContent => CiphertextMessageType::Plaintext,
        }
    }
}
