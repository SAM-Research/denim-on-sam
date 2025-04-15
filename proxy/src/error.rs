use axum::{http::StatusCode, response::IntoResponse};

use derive_more::{Display, Error, From};
use log::error;
use sam_server::managers::error::{AccountManagerError, DeviceManagerError};

use crate::managers::DenimKeyManagerError;
use sam_net::error::{ClientTlsError, ServerTlsError};

#[derive(Debug, Display, Error, From)]
pub enum ServerError {
    SAMUnAuth,
}

impl IntoResponse for ServerError {
    fn into_response(self) -> axum::response::Response {
        error!("ServerError occured: {}", self);
        match self {
            ServerError::SAMUnAuth => StatusCode::UNAUTHORIZED.into_response(),
        }
    }
}

#[derive(Debug, Display, Error, From)]
pub enum LogicError {
    Encode,
    KeyManager(DenimKeyManagerError),
    DeviceManager(DeviceManagerError),
    AccountManager(AccountManagerError),
}

#[derive(Debug, Display, Error, From)]
pub enum CliError {
    AddressParseError,
    FailedToStartProxy,

    ArgumentError(#[error(not(source))] String),
    TLSError(TlsError),
    SerdeError(serde_json::Error),
    IoError(std::io::Error),
}

#[derive(Debug, Display, Error, From)]
pub enum TlsError {
    Client(ClientTlsError),
    Server(ServerTlsError),
}

#[derive(Debug, Display, Error, From)]
pub enum DenimRouterError {
    Error, // TODO: fill out this
}
