use crate::{
    error::RippaError,
    state::{MakeMkvState, RippaState},
};
use askama::Template;
use axum::response::Html;
use makemkv::{MakeMkvInfo, title::TitleList};

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
    pub makemkv_disc_data: MakeMkvDiscDataPage,
    pub makemkv_rip: MakeMkvRipPage,
}

impl IndexPage {
    pub fn new(state: &RippaState) -> Self {
        Self {
            makemkv_info: MakeMkvInfoPage::new(state),
            makemkv_disc_data: MakeMkvDiscDataPage::new(state),
            makemkv_rip: MakeMkvRipPage::new(state),
        }
    }
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

#[derive(Template)]
#[template(path = "makemkv-disc-data.html")]
pub struct MakeMkvDiscDataPage {
    pub title_list: Option<TitleList>,
    pub state: MakeMkvState,
}

impl MakeMkvDiscDataPage {
    pub fn new(state: &RippaState) -> Self {
        Self {
            title_list: state.titles.clone(),
            state: state.makemkv_state,
        }
    }
}

#[derive(Template)]
#[template(path = "makemkv-rip.html")]
pub struct MakeMkvRipPage {
    pub current_progress: String,
    pub total_progress: String,
    pub state: MakeMkvState,
}

impl MakeMkvRipPage {
    pub fn new(state: &RippaState) -> Self {
        Self {
            current_progress: format!("{:.2}", state.makemkv_rip_progress.current),
            total_progress: format!("{:.2}", state.makemkv_rip_progress.total),
            state: state.makemkv_state,
        }
    }
}
