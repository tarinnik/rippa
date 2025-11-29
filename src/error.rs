use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};

#[derive(Debug, thiserror::Error)]
pub enum RippaError {
    #[error("Unable to render template")]
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
