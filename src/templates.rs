use crate::error::RippaError;
use askama::Template;

#[derive(Template)]
#[template(path = "index.html")]
pub struct HelloWorld {
    pub name: String,
}

pub trait AxumAskama: Template {
    fn render_response(&self) -> Result<String, RippaError> {
        match self.render() {
            Ok(s) => Ok(s),
            Err(e) => Err(e.into()),
        }
    }
}

impl<T> AxumAskama for T where T: Template {}
