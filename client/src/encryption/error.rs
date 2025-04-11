use derive_more::{Display, Error, From};
use libsignal_core::curve::CurveError;
use libsignal_protocol::SignalProtocolError;
use sam_common::api::DecodeError;

#[derive(Debug, Error, Display, From)]
pub enum EncryptionError {
    SignalProtocolError(SignalProtocolError),
    InvalidAccountId,
    FailedToUnpad,
}

#[derive(Debug, Error, Display, From)]
pub enum KeyError {
    SignalProtocolError(SignalProtocolError),
    DecodeError(DecodeError),
    CurveError(CurveError),
}
