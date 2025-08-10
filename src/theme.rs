use clap::ValueEnum;
use std::default::Default;

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum ThemeName {
    Unicode,
    Ascii,
}

#[derive(Debug, Clone)]
pub struct Theme {
    pub hr: char,
    pub quote_prefix: &'static str,
    pub bullets: [char; 3],
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            hr: '─',
            quote_prefix: "┃",
            bullets: ['•', '–', '◦'],
        }
    }
}

impl Theme {
    pub fn ascii() -> Self {
        Self {
            hr: '-',
            quote_prefix: ">",
            bullets: ['*', '-', 'o'],
        }
    }
    pub fn from_name(name: ThemeName) -> Self {
        match name {
            ThemeName::Unicode => Default::default(),
            ThemeName::Ascii => Self::ascii(),
        }
    }
    pub fn bullet_for_depth(&self, depth: usize) -> char {
        let idx = depth.min(self.bullets.len().saturating_sub(1));
        self.bullets[idx]
    }
}
