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
}

impl IndexPage {
    pub fn new(state: &RippaState) -> Self {
        Self {
            makemkv_info: MakeMkvInfoPage::new(state),
            makemkv_disc_data: MakeMkvDiscDataPage::new(state),
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
