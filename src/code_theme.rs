use clap::ValueEnum;

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum CodeThemeName {
    OneHalfDark,
    OneHalfLight,
    Dracula,
    Nord,
}

impl CodeThemeName {
    pub fn as_bat_name(&self) -> &'static str {
        match self {
            CodeThemeName::OneHalfDark => "OneHalfDark",
            CodeThemeName::OneHalfLight => "OneHalfLight",
            CodeThemeName::Dracula => "Dracula",
            CodeThemeName::Nord => "Nord",
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub enum CodeThemeSetting {
    #[default]
    Auto,
    Named(CodeThemeName),
}
