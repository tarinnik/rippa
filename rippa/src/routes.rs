use crate::{
    error::RippaError,
    templates::{AxumAskama, HelloWorld},
};
use axum::{
    Router,
    response::{Html, IntoResponse},
    routing::get,
};

pub fn get_router() -> Router {
    Router::new().route("/", get(index))
}

async fn index() -> Result<impl IntoResponse, RippaError> {
    Ok(Html(
        HelloWorld {
            name: "world".to_string(),
        }
        .render_response()?,
    ))
}
