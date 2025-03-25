use axum::{http::StatusCode, response::IntoResponse};
use derive_more::{Display, Error, From};
use log::error;
use sam_client::net::error::TLSError as ClientTLSError;
use sam_server::error::{TLSConfigError, TLSError as ServerTLSError};

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
pub enum CLIError {
    AddressParseError,
    FailedToStartProxy,
    #[error(ignore)]
    ArgumentError(String),
    TLSError(TLSError),
    TLSConfigError(TLSConfigError),
}

#[derive(Debug, Display, Error, From)]
pub enum TLSError {
    Client(ClientTLSError),
    Server(ServerTLSError),
}
