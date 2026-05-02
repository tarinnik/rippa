use crate::{
    error::RippaError,
    state::{RippaState, makemkv::MakeMkvCommand},
    templates::{AxumAskama, IndexPage, MakeMkvDiscDataPage, MakeMkvInfoPage, MakeMkvProgressPage},
    util::parse_selected_titles,
};
use axum::{
    Router,
    extract::State,
    response::Html,
    routing::{get, post},
};
use static_serve::embed_assets;
use std::sync::Arc;
use tokio::sync::RwLock;

type RS = State<Arc<RwLock<RippaState>>>;
type Response = Result<Html<String>, RippaError>;

embed_assets!("rippa/assets", compress = true);

pub fn get_router() -> Router {
    let state = Arc::new(RwLock::new(RippaState::new()));

    Router::new()
        .route("/", get(index))
        .route("/makemkv", get(makemkv_info))
        .route("/makemkv/init", post(makemkv_init))
        .route("/makemkv/disc", get(makemkv_disc_info))
        .route("/makemkv/disc/load", post(makemkv_load_disc))
        .route("/makemkv/rip", post(makemkv_rip))
        .route("/makemkv/progress", get(makemkv_progress))
        .with_state(state)
        .nest("/assets", static_router())
}

async fn index(State(state): RS) -> Response {
    let state = state.read().await;
    let makemkv_state = state.makemkv.read().await;
    IndexPage::new(&makemkv_state).render_response()
}

async fn makemkv_init(State(state): RS) -> Response {
    let mut state = state.write().await;
    state.send_command(MakeMkvCommand::Init).await?;
    let makemkv_state = state.makemkv.read().await;
    MakeMkvInfoPage::new(&makemkv_state).render_response()
}

async fn makemkv_info(State(state): RS) -> Response {
    let state = state.read().await;
    let makemkv_state = state.makemkv.read().await;
    MakeMkvInfoPage::new(&makemkv_state).render_response()
}

async fn makemkv_load_disc(State(state): RS) -> Response {
    let mut state = state.write().await;
    state.send_command(MakeMkvCommand::Load).await?;
    let makemkv_state = state.makemkv.read().await;
    MakeMkvDiscDataPage::new(&makemkv_state).render_response()
}

async fn makemkv_disc_info(State(state): RS) -> Response {
    let state = state.read().await;
    let makemkv_state = state.makemkv.read().await;

    MakeMkvDiscDataPage::new(&makemkv_state).render_response()
}

async fn makemkv_rip(State(state): RS, body: String) -> Response {
    let title_map = parse_selected_titles(&body).ok_or(RippaError::InvalidTitle)?;
    let mut state = state.write().await;
    state.send_command(MakeMkvCommand::Rip(title_map)).await?;
    let makemkv_state = state.makemkv.read().await;
    MakeMkvProgressPage::new(&makemkv_state).render_response()
}

async fn makemkv_progress(State(state): RS) -> Response {
    let state = state.read().await;
    let makemkv_state = state.makemkv.read().await;
    MakeMkvProgressPage::new(&makemkv_state).render_response()
}
