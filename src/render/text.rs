use super::{Renderer, Scope};

impl<B: crate::media::ImageBackend, S: crate::sink::Sink> Renderer<B, S> {
    pub(super) fn wrap_caption_lines(&self, text: &str, max_cells: usize) -> Vec<String> {
        let mut lines = Vec::new();
        let mut buf = String::new();
        for part in text.split_inclusive(char::is_whitespace) {
            let cur = crate::str_width::str_width(&buf);
            let pw = crate::str_width::str_width(part);
            if cur + pw > max_cells && !buf.is_empty() {
                lines.push(buf.trim_end().to_string());
                buf.clear();
            }
            if buf.is_empty() {
                buf.push_str(part.trim_start());
            } else {
                buf.push_str(part);
            }
        }
        if !buf.is_empty() {
            lines.push(buf.trim_end().to_string());
        }
        if lines.is_empty() {
            lines.push(text.trim().to_string());
        }
        lines
    }

    /// Apply styling to text based on the current scope stack
    pub(super) fn apply_text_styling(&self, text: &str, scope: &[Scope]) -> String {
        let mut styled_text = text.to_string();
        for s in scope {
            styled_text = match s {
                Scope::Italic => self.color_theme.emphasis.apply(&styled_text),
                Scope::Bold => self.color_theme.strong.apply(&styled_text),
                Scope::Link { .. } => self.color_theme.link.apply(&styled_text),
                _ => styled_text,
            };
        }
        styled_text
    }
}
