use crate::types::Cells;

pub trait IndentedScope {
    fn indent(&self) -> usize;
}

pub struct Paragraph {
    buffer: String,
    pending_prefix: Option<String>,
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
            hanging_extra: 0,
        }
    }

    pub fn set_prefix(&mut self, prefix: String) {
        self.pending_prefix = Some(prefix);
    }

    fn current_indent<S: IndentedScope>(scope: &[S]) -> usize {
        scope.iter().map(|s| s.indent()).sum()
    }

    pub fn wrap_and_push<S: IndentedScope, F: Fn(&str, usize)>(
        &mut self,
        scope: &[S],
        width: Cells,
        text: &str,
        flush_line: &F,
        str_width: &dyn Fn(&str) -> usize,
    ) {
        let base_indent = Self::current_indent(scope);
        let mut indent = base_indent + self.hanging_extra;
        let mut first_avail = width.0.saturating_sub(indent);
        let prefix_width = self
            .pending_prefix
            .as_ref()
            .map(|s| s.chars().count())
            .unwrap_or(0);
        if self.pending_prefix.is_some() {
            first_avail = first_avail.saturating_sub(prefix_width);
        }
        for word in text.split_inclusive(char::is_whitespace) {
            let current_avail = if self.pending_prefix.is_some() {
                first_avail
            } else {
                width.0.saturating_sub(indent)
            };
            if str_width(&self.buffer) + str_width(word) > current_avail {
                let mut line = self.buffer.clone();
                if let Some(prefix) = self.pending_prefix.take() {
                    line = format!("{prefix}{line}");
                    self.hanging_extra = prefix_width;
                    indent = base_indent + self.hanging_extra;
                }
                flush_line(&line, indent);
                self.buffer.clear();
            }
            if self.buffer.is_empty() {
                self.buffer.push_str(word.trim_start());
            } else {
                self.buffer.push_str(word);
            }
        }
    }

    pub fn flush_paragraph<S: IndentedScope, F: Fn(&str, usize)>(
        &mut self,
        scope: &[S],
        _width: Cells,
        flush_line: &F,
    ) {
        if !self.buffer.is_empty() {
            let base_indent = Self::current_indent(scope);
            let mut indent = base_indent + self.hanging_extra;
            let mut line = self.buffer.clone();
            if let Some(prefix) = self.pending_prefix.take() {
                line = format!("{prefix}{line}");
                let prefix_width = prefix.chars().count();
                self.hanging_extra = prefix_width;
                indent = base_indent + self.hanging_extra;
            }
            flush_line(&line, indent);
            self.buffer.clear();
        }
        println!();
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
