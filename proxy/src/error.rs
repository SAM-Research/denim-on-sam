use axum::{http::StatusCode, response::IntoResponse};
use derive_more::{Display, Error, From};
use log::error;
use sam_client::net::error::TlsError as ClientTlsError;
use sam_server::{
    error::TlsError as ServerTlsError,
    managers::error::{AccountManagerError, DeviceManagerError},
};

use crate::managers::DenimKeyManagerError;

#[derive(Debug, Display, Error, From)]
pub enum ServerError {
    SAMUnAuth,
    KeyManager(DenimKeyManagerError),
    DeviceManager(DeviceManagerError),
    AccountManager(AccountManagerError),
}

impl IntoResponse for ServerError {
    fn into_response(self) -> axum::response::Response {
        error!("ServerError occured: {}", self);
        match self {
            ServerError::SAMUnAuth => StatusCode::UNAUTHORIZED.into_response(),
            ServerError::KeyManager(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
            ServerError::DeviceManager(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
            ServerError::AccountManager(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

#[derive(Debug, Display, Error, From)]
pub enum CliError {
    AddressParseError,
    FailedToStartProxy,
    #[error(ignore)]
    ArgumentError(String),
    TLSError(TlsError),
    SerdeError(serde_json::Error),
    IoError(std::io::Error),
}

#[derive(Debug, Display, Error, From)]
pub enum TlsError {
    Client(ClientTlsError),
    Server(ServerTlsError),
}
