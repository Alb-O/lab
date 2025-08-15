use clap::ValueEnum;
use std::default::Default;

// These are about character glyphs, not colors
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum GlyphThemeName {
    Unicode,
    Ascii,
}

#[derive(Debug, Clone)]
pub struct GlyphTheme {
    pub hr: char,
    pub quote_prefix: &'static str,
    pub bullets: [char; 3],
}

impl Default for GlyphTheme {
    fn default() -> Self {
        Self {
            hr: '─',
            quote_prefix: "┃",
            bullets: ['•', '–', '◦'],
        }
    }
}

impl GlyphTheme {
    pub fn ascii() -> Self {
        Self {
            hr: '-',
            quote_prefix: ">",
            bullets: ['*', '-', 'o'],
        }
    }
    pub fn from_name(name: GlyphThemeName) -> Self {
        match name {
            GlyphThemeName::Unicode => Default::default(),
            GlyphThemeName::Ascii => Self::ascii(),
        }
    }
    pub fn bullet_for_depth(&self, depth: usize) -> char {
        let idx = depth.min(self.bullets.len().saturating_sub(1));
        self.bullets[idx]
    }
}
