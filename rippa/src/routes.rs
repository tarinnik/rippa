use crate::{
    error::RippaError,
    state::RippaState,
    templates::{AxumAskama, IndexPage, MakeMkvDiscDataPage, MakeMkvInfoPage},
};
use axum::{
    Router,
    extract::State,
    response::Html,
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
        .route(
            "/makemkv-disc-data",
            post(makemkv_disc_data).get(makemkv_disc_data_get),
        )
        .route("/makemkv-disc-data-check", post(makemkv_disc_data_check))
        .with_state(state)
}

async fn index(State(state): RS) -> Response {
    let state = state.read().await;
    IndexPage::new(&state).render_response()
}

async fn makemkv_init(State(state): RS) -> Response {
    let mut state = state.write().await;
    state.makemkv_init().await?;

    MakeMkvInfoPage::new(&state).render_response()
}

async fn makemkv_init_check(State(state): RS) -> Response {
    let mut state = state.write().await;
    state.makemkv_init_check().await?;

    MakeMkvInfoPage::new(&state).render_response()
}

async fn makemkv_disc_data_get(State(state): RS) -> Response {
    let state = state.read().await;
    MakeMkvDiscDataPage::new(&state).render_response()
}

async fn makemkv_disc_data(State(state): RS) -> Response {
    let mut state = state.write().await;
    state.makemkv_disc_data().await?;
    MakeMkvDiscDataPage::new(&state).render_response()
}

async fn makemkv_disc_data_check(State(state): RS) -> Response {
    let mut state = state.write().await;
    state.makemkv_disc_data_check().await?;
    MakeMkvDiscDataPage::new(&state).render_response()
}
