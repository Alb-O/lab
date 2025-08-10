use crate::sink::Sink;
use crate::str_width::str_width;
use crate::types::{Cells, Indent};

pub trait IndentedScope {
    fn indent(&self) -> Indent;
}

pub struct Paragraph {
    buffer: String,
    pending_prefix: Option<String>,
    line_prefix: String,
    pub hanging_extra: usize,
}

impl Default for Paragraph {
    fn default() -> Self {
        Self::new()
    }
}

impl Paragraph {
    pub fn new() -> Self {
        Self {
            buffer: String::new(),
            pending_prefix: None,
            line_prefix: String::new(),
            hanging_extra: 0,
        }
    }

    pub fn set_prefix(&mut self, prefix: String) {
        self.pending_prefix = Some(prefix);
    }

    pub fn set_line_prefix(&mut self, prefix: String) {
        self.line_prefix = prefix;
    }

    pub fn clear_line_prefix(&mut self) {
        self.line_prefix.clear();
    }

    fn current_indent<S: IndentedScope>(scope: &[S]) -> usize {
        scope.iter().map(|s| s.indent().0).sum()
    }

    pub fn wrap_and_push<S: IndentedScope, K: Sink>(
        &mut self,
        scope: &[S],
        width: Cells,
        text: &str,
        sink: &mut K,
        str_width: &dyn Fn(&str) -> usize,
    ) {
        let base_indent = Self::current_indent(scope);
        let mut indent = base_indent + self.hanging_extra;

        let line_prefix_width = str_width(&self.line_prefix);
        let mut first_avail = width.0.saturating_sub(indent + line_prefix_width);

        let prefix_width = self
            .pending_prefix
            .as_ref()
            .map(|s| str_width(s))
            .unwrap_or(0);
        if self.pending_prefix.is_some() {
            first_avail = first_avail.saturating_sub(prefix_width);
        }
        for word in text.split_inclusive(char::is_whitespace) {
            let current_avail = if self.pending_prefix.is_some() {
                first_avail
            } else {
                width.0.saturating_sub(indent + line_prefix_width)
            };
            if str_width(&self.buffer) + str_width(word) > current_avail {
                let mut line = self.buffer.clone();
                let mut write_indent = indent;
                if let Some(prefix) = self.pending_prefix.take() {
                    // Write the first line with the base indent only; apply hanging indent
                    // for subsequent wrapped lines.
                    line = format!("{prefix}{line}");
                    self.hanging_extra = prefix_width;
                    // Do NOT add hanging to this first prefixed line's indent
                    write_indent = base_indent;
                    // Update indent for any following lines in this paragraph
                    indent = base_indent + self.hanging_extra;
                }
                if !self.line_prefix.is_empty() {
                    line = format!("{}{line}", self.line_prefix);
                }
                let _ = sink.write_line(&line, write_indent);
                self.buffer.clear();
            }
            if self.buffer.is_empty() {
                self.buffer.push_str(word.trim_start());
            } else {
                self.buffer.push_str(word);
            }
        }
    }

    pub fn flush_paragraph<S: IndentedScope, K: Sink>(
        &mut self,
        scope: &[S],
        _width: Cells,
        sink: &mut K,
    ) {
        if !self.buffer.is_empty() {
            let base_indent = Self::current_indent(scope);
            let mut indent = base_indent + self.hanging_extra;
            let mut line = self.buffer.clone();
            let mut write_indent = indent;
            if let Some(prefix) = self.pending_prefix.take() {
                // First written line uses base indent; future lines hang.
                line = format!("{prefix}{line}");
                let prefix_width = str_width(&prefix);
                self.hanging_extra = prefix_width;
                write_indent = base_indent;
                indent = base_indent + self.hanging_extra;
            }
            if !self.line_prefix.is_empty() {
                line = format!("{}{line}", self.line_prefix);
            }
            let _ = sink.write_line(&line, write_indent);
            self.buffer.clear();
        }
    }

    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }
    pub fn as_str(&self) -> &str {
        &self.buffer
    }
    pub fn clear(&mut self) {
        self.buffer.clear();
    }
}
