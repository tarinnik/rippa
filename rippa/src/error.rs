use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use makemkv::error::MakeMkvError;
use tokio::task::JoinError;

#[derive(Debug, thiserror::Error)]
pub enum RippaError {
    #[error("Unable to render template: {0}")]
    Render(#[from] askama::Error),
    #[error("Error occurred using MakeMKV: {0}")]
    MakeMkv(#[from] MakeMkvError),
    #[error("MakeMKV is already running")]
    MakeMkvAlreadyRunning,
    #[error("MakeMKV is not running")]
    MakeMkvNotRunning,
    #[error("Task exited unexpectedly: {0}")]
    TaskError(#[from] JoinError),
}

impl IntoResponse for RippaError {
    fn into_response(self) -> Response {
        match &self {
            Self::Render(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
            Self::MakeMkv(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
            Self::MakeMkvAlreadyRunning => StatusCode::METHOD_NOT_ALLOWED.into_response(),
            Self::MakeMkvNotRunning => StatusCode::METHOD_NOT_ALLOWED.into_response(),
            Self::TaskError(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}
