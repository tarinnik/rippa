use crate::{
    error::RippaError,
    state::{MakeMkvState, RippaState},
};
use askama::Template;
use axum::response::Html;
use makemkv::MakeMkvInfo;

pub trait AxumAskama: Template {
    fn render_response(&self) -> Result<Html<String>, RippaError> {
        Ok(Html(self.render()?))
    }
}

impl<T> AxumAskama for T where T: Template {}

#[derive(Template)]
#[template(path = "index.html")]
pub struct IndexPage {
    pub makemkv_info: MakeMkvInfoPage,
}

#[derive(Template)]
#[template(path = "makemkv-info.html")]
pub struct MakeMkvInfoPage {
    pub info: Option<MakeMkvInfo>,
    pub state: MakeMkvState,
}

impl MakeMkvInfoPage {
    pub fn new(state: &RippaState) -> Self {
        Self {
            info: state.makemkv_info.clone(),
            state: state.makemkv_state,
        }
    }
}
