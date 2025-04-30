use derive_more::{Display, Error, From};
use libsignal_core::curve::CurveError;
use libsignal_protocol::SignalProtocolError;
use sam_common::api::DecodeError;

use crate::store::SeedStoreError;

#[derive(Debug, Error, Display, From)]
pub enum EncryptionError {
    SignalProtocolError(SignalProtocolError),
    InvalidAccountId,
    FailedToUnpad,
    NoPreKeyInMessage,
    Key(KeyError),
}

#[derive(Debug, Error, Display, From)]
pub enum KeyError {
    SignalProtocolError(SignalProtocolError),
    DecodeError(DecodeError),
    CurveError(CurveError),
    SeedStore(SeedStoreError),
    CouldNotGenerateKey,
}
