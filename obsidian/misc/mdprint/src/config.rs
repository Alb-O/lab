use crate::{
    code_theme::CodeThemeSetting, style::ColorThemeName, theme::GlyphThemeName, types::Cells,
};
use clap::ValueEnum;

#[derive(Debug, Clone)]
pub struct Config {
    pub width: Cells,
    pub tab_length: usize,
    pub hide_urls: bool,
    pub no_images: bool,
    pub syncat: bool,
    pub dev: bool,
    pub glyph_theme: GlyphThemeName,
    pub color_theme: ColorThemeName,
    pub code_theme: CodeThemeSetting,
    pub wrap_mode: WrapMode,
}

impl Config {
    pub fn validate(self) -> Self {
        let width = Cells(self.width.0.max(20));
        Self { width, ..self }
    }
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum WrapMode {
    Greedy,
    None,
}
