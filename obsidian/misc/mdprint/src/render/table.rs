use super::{Renderer, Scope};
use crate::sink::Sink;
use crate::wrap::IndentedScope;
use comfy_table::{ContentArrangement, Table, presets};
use pulldown_cmark::Alignment;

#[derive(Debug, Default, Clone)]
pub(super) struct TableState {
    pub(super) headers: Vec<String>,
    pub(super) rows: Vec<Vec<String>>,
    pub(super) cur_row: Vec<String>,
    pub(super) cur_cell: String,
    pub(super) in_head: bool,
    pub(super) alignments: Vec<Alignment>,
}

impl TableState {
    pub(super) fn new() -> Self {
        Self::default()
    }
    pub(super) fn start_row(&mut self) {
        self.cur_row.clear();
    }
    pub(super) fn start_cell(&mut self) {
        self.cur_cell.clear();
    }
    pub(super) fn push_text(&mut self, s: &str) {
        self.cur_cell.push_str(s);
    }
    pub(super) fn finish_cell(&mut self) {
        self.cur_row.push(self.cur_cell.clone());
        self.cur_cell.clear();
    }
    pub(super) fn finish_row(&mut self) {
        if self.in_head && self.headers.is_empty() {
            self.headers = self.cur_row.clone();
        } else {
            self.rows.push(self.cur_row.clone());
        }
        self.cur_row.clear();
    }
}

impl<B: crate::media::ImageBackend, S: Sink> Renderer<B, S> {
    pub(super) fn render_table(
        &mut self,
        state: TableState,
        scope: &[Scope],
    ) -> std::io::Result<()> {
        let base_indent: usize = scope.iter().map(|s| s.indent().0).sum();
        let indent = base_indent;
        let available = self.cfg.width.0.saturating_sub(indent);

        let mut table = Table::new();
        if self.glyph_theme.hr == '─' {
            table.load_preset(presets::UTF8_FULL);
            use comfy_table::TableComponent as TC;
            table
                .set_style(TC::VerticalLines, '│')
                .set_style(TC::HorizontalLines, '─');
        } else {
            table.load_preset(presets::ASCII_FULL);
        }
        table
            .set_content_arrangement(ContentArrangement::Dynamic)
            .set_width(available as u16);

        if !state.headers.is_empty() {
            table.set_header(state.headers.clone());
        }
        for row in state.rows {
            table.add_row(row);
        }

        use comfy_table::CellAlignment as CtAlign;
        for (i, a) in state.alignments.iter().enumerate() {
            if let Some(col) = table.column_mut(i) {
                let ca = match a {
                    Alignment::Left => CtAlign::Left,
                    Alignment::Right => CtAlign::Right,
                    Alignment::Center => CtAlign::Center,
                    Alignment::None => CtAlign::Left,
                };
                col.set_cell_alignment(ca);
            }
        }

        for line in table.lines() {
            let _ = self.sink.write_line(&line, indent);
        }
        Ok(())
    }
}
