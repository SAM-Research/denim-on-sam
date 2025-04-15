pub mod buffers;
mod error;
mod seed;

pub use error::DenimBufferError;
pub use seed::Seed;

include!(concat!(env!("OUT_DIR"), "/_includes.rs"));

use denim_message::MessageType;
use libsignal_protocol::CiphertextMessageType;

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
