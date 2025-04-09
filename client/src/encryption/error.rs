use derive_more::{Display, Error, From};
use libsignal_protocol::SignalProtocolError;

#[derive(Debug, Error, Display, From)]
pub enum EncryptionError {
    SignalProtocolError(SignalProtocolError),
    InvalidAccountId,
}
