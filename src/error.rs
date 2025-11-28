use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};

#[derive(Debug, thiserror::Error)]
pub enum RippaError {
    #[error("Unable to render template")]
    Render(#[from] askama::Error),
}

impl IntoResponse for RippaError {
    fn into_response(self) -> Response {
        match &self {
            Self::Render(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}
