use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use std::io;
use tokio::time::error::Elapsed;

#[derive(Debug, thiserror::Error)]
pub enum RippaError {
    #[error("Unable to render template: {0}")]
    Render(#[from] askama::Error),
    #[error("Error occurred using MakeMKV: {0}")]
    MakeMkv(String),
    #[error("Received an invalid command from MakeMKV: {0}")]
    InvalidMmkvCommand(String),
}

impl IntoResponse for RippaError {
    fn into_response(self) -> Response {
        match &self {
            Self::Render(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
            Self::MakeMkv(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
            Self::InvalidMmkvCommand(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

impl From<io::Error> for RippaError {
    fn from(value: io::Error) -> Self {
        Self::MakeMkv(format!("IO Error: {}", value))
    }
}

impl From<Elapsed> for RippaError {
    fn from(_value: Elapsed) -> Self {
        Self::MakeMkv(String::from("Timeout receiving data"))
    }
}

impl From<anyhow::Error> for RippaError {
    fn from(value: anyhow::Error) -> Self {
        Self::MakeMkv(format!("{:#}", value))
    }
}
