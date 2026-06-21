use crate::{
    VERSION,
    error::RippaError,
    state::makemkv::{MakeMkvState, MakeMkvStatus},
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
    pub makemkv_rip: MakeMkvProgressPage,
}

impl IndexPage {
    pub fn new(state: &MakeMkvState) -> Self {
        Self {
            makemkv_info: MakeMkvInfoPage::new(state),
            makemkv_disc_data: MakeMkvDiscDataPage::new(state),
            makemkv_rip: MakeMkvProgressPage::new(state),
        }
    }
}

#[derive(Template)]
#[template(path = "makemkv-info.html")]
pub struct MakeMkvInfoPage {
    pub info: Option<MakeMkvInfo>,
    pub status: MakeMkvStatus,
    pub version: String,
}

impl MakeMkvInfoPage {
    pub fn new(state: &MakeMkvState) -> Self {
        Self {
            info: state.info.clone(),
            status: state.status,
            version: VERSION.into(),
        }
    }
}

#[derive(Template)]
#[template(path = "makemkv-disc-data.html")]
pub struct MakeMkvDiscDataPage {
    pub title_list: Option<TitleList>,
    pub status: MakeMkvStatus,
}

impl MakeMkvDiscDataPage {
    pub fn new(state: &MakeMkvState) -> Self {
        Self {
            title_list: state.titles.clone(),
            status: state.status,
        }
    }
}

#[derive(Template)]
#[template(path = "makemkv-progress.html")]
pub struct MakeMkvProgressPage {
    pub current_progress: String,
    pub total_progress: String,
    pub status: MakeMkvStatus,
}

impl MakeMkvProgressPage {
    pub fn new(state: &MakeMkvState) -> Self {
        Self {
            current_progress: format!("{:.2}", state.rip_progress.current),
            total_progress: format!("{:.2}", state.rip_progress.total),
            status: state.status,
        }
    }
}
