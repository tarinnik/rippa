use crate::{
    error::RippaError,
    state::RippaState,
    templates::{AxumAskama, IndexPage, MakeMkvInfoPage},
};
use axum::{
    Router,
    extract::State,
    response::{Html, IntoResponse},
    routing::{get, post},
};
use std::sync::Arc;
use tokio::sync::RwLock;

type RS = State<Arc<RwLock<RippaState>>>;
type Response = Result<Html<String>, RippaError>;

pub fn get_router() -> Router {
    let state = Arc::new(RwLock::new(RippaState::new()));

    Router::new()
        .route("/", get(index))
        .route("/makemkv-init", post(makemkv_init))
        .route("/makemkv-init-check", post(makemkv_init_check))
        .route("/get-disc-data", post(get_disc_data))
        .with_state(state)
}

async fn index(State(state): RS) -> Response {
    let rs = state.read().await;

    IndexPage {
        makemkv_info: MakeMkvInfoPage::new(&rs),
    }
    .render_response()
}

async fn makemkv_init(State(state): RS) -> Response {
    let mut state = state.write().await;
    state.makemkv_init().await?;

    MakeMkvInfoPage::new(&state).render_response()
}

async fn makemkv_init_check(State(state): RS) -> Response {
    let mut state = state.write().await;
    state.makemkv_check_init().await?;

    MakeMkvInfoPage::new(&state).render_response()
}

async fn get_disc_data(State(state): RS) -> Result<(), RippaError> {
    Ok(())
}
