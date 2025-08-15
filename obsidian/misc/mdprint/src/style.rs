use clap::ValueEnum;
use crossterm::style::Color as CrossColor;

/// Simple color representation for terminal output
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Color {
    Black,
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    White,
    BrightBlack,
    BrightRed,
    BrightGreen,
    BrightYellow,
    BrightBlue,
    BrightMagenta,
    BrightCyan,
    BrightWhite,
}

impl From<Color> for CrossColor {
    fn from(color: Color) -> Self {
        match color {
            Color::Black => CrossColor::Black,
            Color::Red => CrossColor::DarkRed,
            Color::Green => CrossColor::DarkGreen,
            Color::Yellow => CrossColor::DarkYellow,
            Color::Blue => CrossColor::DarkBlue,
            Color::Magenta => CrossColor::DarkMagenta,
            Color::Cyan => CrossColor::DarkCyan,
            Color::White => CrossColor::Grey,
            Color::BrightBlack => CrossColor::DarkGrey,
            Color::BrightRed => CrossColor::Red,
            Color::BrightGreen => CrossColor::Green,
            Color::BrightYellow => CrossColor::Yellow,
            Color::BrightBlue => CrossColor::Blue,
            Color::BrightMagenta => CrossColor::Magenta,
            Color::BrightCyan => CrossColor::Cyan,
            Color::BrightWhite => CrossColor::White,
        }
    }
}

/// Text styling for markdown elements - focused and simple
#[derive(Debug, Clone, Default)]
pub struct TextStyle {
    pub fg: Option<Color>,
    pub bg: Option<Color>,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub strikethrough: bool,
    pub dim: bool,
    pub reverse: bool,
}

impl TextStyle {
    /// Apply this style to text using crossterm
    pub fn apply(&self, text: &str) -> String {
        use crossterm::style::{Attribute, ContentStyle, StyledContent};

        let mut style = ContentStyle::new();

        if let Some(fg) = self.fg {
            style.foreground_color = Some(fg.into());
        }
        if let Some(bg) = self.bg {
            style.background_color = Some(bg.into());
        }

        if self.bold {
            style.attributes.set(Attribute::Bold);
        }
        if self.italic {
            style.attributes.set(Attribute::Italic);
        }
        if self.underline {
            style.attributes.set(Attribute::Underlined);
        }
        if self.strikethrough {
            style.attributes.set(Attribute::CrossedOut);
        }
        if self.dim {
            style.attributes.set(Attribute::Dim);
        }
        if self.reverse {
            style.attributes.set(Attribute::Reverse);
        }

        let styled_content = StyledContent::new(style, text);
        format!("{styled_content}")
    }
}

/// Color palette for consistent theming
#[derive(Debug, Clone)]
pub struct ColorPalette {
    // Primary colors
    pub primary: Color,
    pub secondary: Color,
    pub accent: Color,

    // Text hierarchy
    pub text_primary: Color,
    pub text_secondary: Color,
    pub text_muted: Color,

    // UI elements
    pub border: Color,
    pub background_subtle: Color,

    // Semantic colors
    pub info: Color,
    pub success: Color,
    pub warning: Color,
}

impl ColorPalette {
    /// Light theme palette - suitable for light backgrounds
    pub fn light() -> Self {
        Self {
            primary: Color::Blue,
            secondary: Color::Cyan,
            accent: Color::Magenta,

            text_primary: Color::Black,
            text_secondary: Color::BrightBlack,
            text_muted: Color::BrightBlack,

            border: Color::BrightBlack,
            background_subtle: Color::White,

            info: Color::Blue,
            success: Color::Green,
            warning: Color::Yellow,
        }
    }

    /// Dark theme palette - suitable for dark backgrounds
    pub fn dark() -> Self {
        Self {
            primary: Color::Cyan,
            secondary: Color::Blue,
            accent: Color::Magenta,

            text_primary: Color::White,
            text_secondary: Color::BrightBlack,
            text_muted: Color::BrightBlack,

            border: Color::BrightBlack,
            background_subtle: Color::BrightBlack,

            info: Color::Cyan,
            success: Color::Green,
            warning: Color::Yellow,
        }
    }

    /// No-color palette - attributes only
    pub fn none() -> Self {
        Self {
            primary: Color::White,
            secondary: Color::White,
            accent: Color::White,

            text_primary: Color::White,
            text_secondary: Color::White,
            text_muted: Color::White,

            border: Color::White,
            background_subtle: Color::White,

            info: Color::White,
            success: Color::White,
            warning: Color::White,
        }
    }
}

/// Markdown-focused color theme built on a color palette
#[derive(Debug, Clone)]
pub struct ColorTheme {
    pub name: String,
    pub palette: ColorPalette,

    // Core markdown elements
    pub text: TextStyle,
    pub heading: TextStyle,
    pub emphasis: TextStyle,      // *italic*
    pub strong: TextStyle,        // **bold**
    pub strikethrough: TextStyle, // ~~strike~~
    pub code: TextStyle,          // `inline code`
    pub code_block: TextStyle,    // ```code blocks```
    pub link: TextStyle,          // [links](url)
    pub quote: TextStyle,         // > blockquotes
    pub rule: TextStyle,          // ---

    // UI elements
    pub list_bullet: TextStyle,  // • - bullets
    pub list_number: TextStyle,  // 1. 2. numbers
    pub quote_prefix: TextStyle, // ┃ > prefixes
    pub caption: TextStyle,      // image captions
}

impl ColorTheme {
    /// Light theme - suitable for light backgrounds
    pub fn light() -> Self {
        let palette = ColorPalette::light();
        Self {
            name: "light".to_string(),
            palette: palette.clone(),

            text: TextStyle {
                fg: Some(palette.text_primary),
                ..Default::default()
            },
            heading: TextStyle {
                fg: Some(palette.primary),
                bold: true,
                ..Default::default()
            },
            emphasis: TextStyle {
                fg: Some(palette.text_secondary),
                italic: true,
                ..Default::default()
            },
            strong: TextStyle {
                fg: Some(palette.text_primary),
                bold: true,
                ..Default::default()
            },
            strikethrough: TextStyle {
                fg: Some(palette.text_secondary),
                strikethrough: true,
                ..Default::default()
            },
            code: TextStyle {
                fg: Some(palette.accent),
                bg: Some(palette.background_subtle),
                ..Default::default()
            },
            code_block: TextStyle {
                fg: Some(palette.text_secondary),
                ..Default::default()
            },
            link: TextStyle {
                fg: Some(palette.primary),
                underline: true,
                ..Default::default()
            },
            quote: TextStyle {
                fg: Some(palette.text_secondary),
                italic: true,
                ..Default::default()
            },
            rule: TextStyle {
                fg: Some(palette.border),
                ..Default::default()
            },
            list_bullet: TextStyle {
                fg: Some(palette.accent),
                bold: true,
                ..Default::default()
            },
            list_number: TextStyle {
                fg: Some(palette.secondary),
                bold: true,
                ..Default::default()
            },
            quote_prefix: TextStyle {
                fg: Some(palette.primary),
                bold: true,
                ..Default::default()
            },
            caption: TextStyle {
                fg: Some(palette.text_muted),
                dim: true,
                italic: false,
                ..Default::default()
            },
        }
    }

    /// Dark theme - suitable for dark backgrounds
    pub fn dark() -> Self {
        let palette = ColorPalette::dark();
        Self {
            name: "dark".to_string(),
            palette: palette.clone(),

            text: TextStyle::default(), // Use terminal default
            heading: TextStyle {
                fg: Some(palette.primary),
                bold: true,
                ..Default::default()
            },
            emphasis: TextStyle {
                fg: Some(palette.text_secondary),
                italic: true,
                ..Default::default()
            },
            strong: TextStyle {
                bold: true,
                ..Default::default()
            },
            strikethrough: TextStyle {
                fg: Some(palette.text_secondary),
                strikethrough: true,
                ..Default::default()
            },
            code: TextStyle {
                fg: Some(palette.accent),
                bg: Some(palette.background_subtle),
                ..Default::default()
            },
            code_block: TextStyle {
                fg: Some(palette.text_secondary),
                ..Default::default()
            },
            link: TextStyle {
                fg: Some(palette.primary),
                underline: true,
                ..Default::default()
            },
            quote: TextStyle {
                fg: Some(palette.text_secondary),
                italic: true,
                ..Default::default()
            },
            rule: TextStyle {
                fg: Some(palette.border),
                ..Default::default()
            },
            list_bullet: TextStyle {
                fg: Some(palette.accent),
                bold: true,
                ..Default::default()
            },
            list_number: TextStyle {
                fg: Some(palette.secondary),
                bold: true,
                ..Default::default()
            },
            quote_prefix: TextStyle {
                fg: Some(palette.primary),
                bold: true,
                ..Default::default()
            },
            caption: TextStyle {
                fg: Some(palette.text_secondary),
                dim: true,
                ..Default::default()
            },
        }
    }

    /// No-color theme - attributes only, no colors
    pub fn none() -> Self {
        let palette = ColorPalette::none();
        Self {
            name: "none".to_string(),
            palette,

            text: TextStyle::default(),
            heading: TextStyle {
                bold: true,
                ..Default::default()
            },
            emphasis: TextStyle {
                italic: true,
                ..Default::default()
            },
            strong: TextStyle {
                bold: true,
                ..Default::default()
            },
            strikethrough: TextStyle {
                strikethrough: true,
                ..Default::default()
            },
            code: TextStyle {
                reverse: true,
                ..Default::default()
            },
            code_block: TextStyle::default(),
            link: TextStyle {
                underline: true,
                ..Default::default()
            },
            quote: TextStyle {
                italic: true,
                ..Default::default()
            },
            rule: TextStyle::default(),
            list_bullet: TextStyle {
                bold: true,
                ..Default::default()
            },
            list_number: TextStyle {
                bold: true,
                ..Default::default()
            },
            quote_prefix: TextStyle {
                bold: true,
                ..Default::default()
            },
            caption: TextStyle {
                dim: true,
                ..Default::default()
            },
        }
    }

    pub fn from_name(name: ColorThemeName) -> Self {
        match name {
            ColorThemeName::Light => Self::light(),
            ColorThemeName::Dark => Self::dark(),
            ColorThemeName::None => Self::none(),
        }
    }
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum ColorThemeName {
    Light,
    Dark,
    None,
}
