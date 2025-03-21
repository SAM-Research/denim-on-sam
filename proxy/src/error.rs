use axum::{http::StatusCode, response::IntoResponse};
use derive_more::{Display, Error, From};
use log::error;

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
