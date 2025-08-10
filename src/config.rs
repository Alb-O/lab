use crate::{theme::ThemeName, types::Cells};

#[derive(Debug, Clone)]
pub struct Config {
    pub width: Cells,
    pub tab_length: usize,
    pub hide_urls: bool,
    pub no_images: bool,
    pub syncat: bool,
    pub dev: bool,
    pub theme: ThemeName,
}

impl Config {
    pub fn validate(self) -> Self {
        let width = Cells(self.width.0.max(20));
        Self { width, ..self }
    }
}
