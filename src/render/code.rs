use super::{Renderer, Scope};
use crate::sink::Sink;

#[cfg(feature = "syntax-highlighting")]
use bat::{Input, PrettyPrinter};

impl<B: crate::media::ImageBackend, S: Sink> Renderer<B, S> {
    /// Render a code block with syntax highlighting using bat
    #[cfg(feature = "syntax-highlighting")]
    pub(super) fn render_highlighted_code_block(
        &mut self,
        code_buffer: &str,
        scope: &[Scope],
        indent: usize,
    ) {
        if let Some(Scope::CodeBlock(lang)) = scope.last() {
            let mut output_buffer = String::new();

            let mut printer = PrettyPrinter::new();
            printer
                .header(false)
                .grid(false)
                .line_numbers(false)
                .rule(false)
                .use_italics(true)
                .tab_width(Some(4));

            if !lang.is_empty() {
                printer.language(lang);
            }

            let theme_name = match self.cfg.code_theme {
                crate::code_theme::CodeThemeSetting::Named(name) => name.as_bat_name(),
                crate::code_theme::CodeThemeSetting::Auto => match self.color_theme.name.as_str() {
                    "dark" => crate::code_theme::CodeThemeName::OneHalfDark.as_bat_name(),
                    "light" => crate::code_theme::CodeThemeName::OneHalfLight.as_bat_name(),
                    _ => crate::code_theme::CodeThemeName::OneHalfDark.as_bat_name(),
                },
            };
            printer.theme(theme_name);

            let input = Input::from_bytes(code_buffer.as_bytes()).name("code-block");
            printer.input(input);

            match printer.print_with_writer(Some(&mut output_buffer)) {
                Ok(_) => {
                    for line in output_buffer.lines() {
                        let _ = self.sink.write_line(line, indent);
                    }
                }
                Err(_) => {
                    self.render_plain_code_block(code_buffer, indent);
                }
            }
        }
    }

    /// Render a plain code block without syntax highlighting
    pub(super) fn render_plain_code_block(&mut self, code_buffer: &str, indent: usize) {
        for line in code_buffer.lines() {
            self.render_plain_code_line(line, indent);
        }
    }

    /// Render a single line of code with the default code block styling
    pub(super) fn render_plain_code_line(&mut self, line: &str, indent: usize) {
        let styled_line = self.color_theme.code_block.apply(line);
        let _ = self.sink.write_line(&styled_line, indent);
    }
}
