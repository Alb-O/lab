use std::io::{self, Write};
use std::path::{Path, PathBuf};

use comfy_table::{ContentArrangement, Table, presets};
use pulldown_cmark::{Alignment, Event, HeadingLevel, Options, Parser, Tag, TagEnd};

use crate::config::Config;
use crate::media::{ImageBackend, RasteroidBackend};
use crate::sink::Sink;
use crate::spacing::{BlankLines, Block, DefaultSpacingPolicy, SpacingPolicy};
use crate::str_width::str_width;
use crate::style::ColorTheme;
use crate::theme::GlyphTheme;
use crate::types::Indent;
use crate::wrap::{IndentedScope, Paragraph};

#[derive(Debug)]
enum ListKind {
    Ordered { next: u64 },
    Unordered,
}

#[derive(Debug)]
enum Scope {
    Italic,
    Bold,
    Strikethrough,
    Link {
        dest_url: String,
        title: String,
    },
    List(ListKind),
    ListItem,
    Code,
    CodeBlock(String),
    BlockQuote,
    Heading(HeadingLevel),
    ImageCollect {
        url: String,
        title: String,
        alt: String,
    },
    DefinitionList,
    DefinitionTitle,
    DefinitionDesc,
    // Collects Markdown table content until the table ends
    TableCollect(TableState),
}

impl IndentedScope for Scope {
    fn indent(&self) -> Indent {
        match self {
            // Indent per list nesting level; items use hanging indent for bullets
            Scope::List(..) => Indent(2),
            Scope::ListItem => Indent(0),
            Scope::BlockQuote => Indent(2),
            Scope::CodeBlock(..) => Indent(2),
            Scope::Heading(..) => Indent(0),
            Scope::DefinitionList => Indent(0),
            Scope::DefinitionTitle => Indent(0),
            Scope::DefinitionDesc => Indent(2),
            _ => Indent(0),
        }
    }
}

pub struct Renderer<B: ImageBackend = RasteroidBackend, S: Sink = crate::sink::StdoutSink> {
    cfg: Config,
    glyph_theme: GlyphTheme,
    color_theme: ColorTheme,
    images: B,
    sink: S,
    spacing: DefaultSpacingPolicy,
    last_block: Option<Block>,
}

impl<B: ImageBackend + Default, S: Sink + Default> Renderer<B, S> {
    pub fn new(cfg: Config) -> Self {
        let glyph_theme = GlyphTheme::from_name(cfg.theme);
        let color_theme = ColorTheme::from_name(cfg.color_theme);
        Self {
            cfg,
            glyph_theme,
            color_theme,
            images: B::default(),
            sink: S::default(),
            spacing: DefaultSpacingPolicy,
            last_block: None,
        }
    }

    pub fn with_sink(cfg: Config, sink: S) -> Self {
        let glyph_theme = GlyphTheme::from_name(cfg.theme);
        let color_theme = ColorTheme::from_name(cfg.color_theme);
        Self {
            cfg,
            glyph_theme,
            color_theme,
            images: B::default(),
            sink,
            spacing: DefaultSpacingPolicy,
            last_block: None,
        }
    }
}

impl<B: ImageBackend, S: Sink> Renderer<B, S> {
    fn ensure_spacing_before(&mut self, next: Block, scope: &[Scope]) {
        let in_list = scope.iter().any(|s| matches!(s, Scope::List(_)));
        let BlankLines(n) = self.spacing.between(self.last_block, next, in_list);
        for _ in 0..n {
            let _ = self.sink.write_blank_line();
        }
    }

    fn wrap_caption_lines(&self, text: &str, max_cells: usize) -> Vec<String> {
        // Simple greedy wrapper aware of cell widths
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
    fn apply_text_styling(&self, text: &str, scope: &[Scope]) -> String {
        let mut styled_text = text.to_string();

        // Apply styles based on active scopes, in order
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

    pub fn render_markdown(&mut self, source: &str, file_path: Option<&Path>) -> io::Result<()> {
        if self.cfg.dev {
            for e in Parser::new_ext(source, Options::all()) {
                eprintln!("{e:?}");
            }
            return Ok(());
        }

        let mut scope: Vec<Scope> = vec![];
        let mut para = Paragraph::new();
        let mut code_buffer = String::new();

        for event in Parser::new_ext(source, Options::all()) {
            match event {
                Event::Start(tag) => match tag {
                    Tag::Table(alignments) => {
                        // Finish any running paragraph, then prepare table collection
                        para.flush_paragraph(&scope, self.cfg.width, &mut self.sink);
                        self.ensure_spacing_before(Block::Table, &scope);
                        let mut st = TableState::new();
                        st.alignments = alignments.into_iter().collect();
                        scope.push(Scope::TableCollect(st));
                    }
                    Tag::TableHead => {
                        if let Some(Scope::TableCollect(state)) = scope.last_mut() {
                            state.in_head = true;
                        }
                    }
                    Tag::TableRow => {
                        if let Some(Scope::TableCollect(state)) = scope.last_mut() {
                            state.start_row();
                        }
                    }
                    Tag::TableCell => {
                        if let Some(Scope::TableCollect(state)) = scope.last_mut() {
                            state.start_cell();
                        }
                    }
                    Tag::Paragraph => {
                        self.ensure_spacing_before(Block::Paragraph, &scope);
                    }
                    Tag::Heading { level, .. } => {
                        para.flush_paragraph(&scope, self.cfg.width, &mut self.sink);
                        self.ensure_spacing_before(Block::Heading, &scope);
                        scope.push(Scope::Heading(level));
                    }
                    Tag::BlockQuote(..) => {
                        // Only add spacing before top-level blockquotes, not nested ones
                        let already_in_quote = scope.iter().any(|s| matches!(s, Scope::BlockQuote));
                        if !already_in_quote {
                            self.ensure_spacing_before(Block::BlockQuote, &scope);
                        }
                        scope.push(Scope::BlockQuote);
                        let quote_depth = scope
                            .iter()
                            .filter(|s| matches!(s, Scope::BlockQuote))
                            .count();
                        let quote_prefix = self.glyph_theme.quote_prefix.repeat(quote_depth);
                        let styled_prefix = self.color_theme.quote_prefix.apply(&quote_prefix);
                        para.set_line_prefix(format!("{styled_prefix} "));
                    }
                    Tag::CodeBlock(kind) => {
                        para.flush_paragraph(&scope, self.cfg.width, &mut self.sink);
                        self.ensure_spacing_before(Block::CodeBlock, &scope);
                        let lang = match kind {
                            pulldown_cmark::CodeBlockKind::Indented => "".to_string(),
                            pulldown_cmark::CodeBlockKind::Fenced(l) => l.into_string(),
                        };
                        scope.push(Scope::CodeBlock(lang));
                    }
                    Tag::List(start) => {
                        // Ensure any running paragraph text is finished before a nested list begins
                        para.flush_paragraph(&scope, self.cfg.width, &mut self.sink);
                        // Don't add spacing when starting a nested list directly under a list item
                        let inside_item = scope.iter().any(|s| matches!(s, Scope::ListItem));
                        if !inside_item {
                            self.ensure_spacing_before(Block::List, &scope);
                        }
                        let kind = match start {
                            Some(n) => ListKind::Ordered { next: n },
                            None => ListKind::Unordered,
                        };
                        scope.push(Scope::List(kind));
                        // Mark that we are now in a list to suppress extra spacing for first item
                        self.last_block = Some(Block::List);
                    }
                    Tag::Item => {
                        let depth = scope.iter().filter(|s| matches!(s, Scope::List(_))).count();
                        let (bullet_text, bullet_styled) =
                            match scope.iter().rev().find_map(|s| match s {
                                Scope::List(ListKind::Ordered { next }) => {
                                    let text = format!("{next}.");
                                    Some((text.clone(), self.color_theme.list_number.apply(&text)))
                                }
                                Scope::List(ListKind::Unordered) => {
                                    let text = self
                                        .glyph_theme
                                        .bullet_for_depth(depth.saturating_sub(1))
                                        .to_string();
                                    Some((text.clone(), self.color_theme.list_bullet.apply(&text)))
                                }
                                _ => None,
                            }) {
                                Some((text, styled)) => (text, styled),
                                None => ("-".to_string(), self.color_theme.list_bullet.apply("-")),
                            };

                        // Use consistent prefix formatting; base indent handles positioning
                        const MIN_PREFIX_CELLS: usize = 2;
                        let bullet_cells = str_width(&bullet_text).max(1);
                        let padding_needed = MIN_PREFIX_CELLS.saturating_sub(bullet_cells);
                        let prefix = format!("{bullet_styled}{} ", " ".repeat(padding_needed));

                        para.set_prefix(prefix);

                        scope.push(Scope::ListItem);
                    }
                    Tag::DefinitionList => {
                        // Similar to lists: suppress spacing when nested under an item
                        let inside_item = scope.iter().any(|s| matches!(s, Scope::ListItem));
                        if !inside_item {
                            self.ensure_spacing_before(Block::List, &scope);
                        }
                        scope.push(Scope::DefinitionList);
                        self.last_block = Some(Block::List);
                    }
                    Tag::DefinitionListTitle => {
                        para.flush_paragraph(&scope, self.cfg.width, &mut self.sink);
                        scope.push(Scope::DefinitionTitle);
                    }
                    Tag::DefinitionListDefinition => {
                        para.flush_paragraph(&scope, self.cfg.width, &mut self.sink);
                        scope.push(Scope::DefinitionDesc);
                    }
                    Tag::Emphasis => scope.push(Scope::Italic),
                    Tag::Strong => scope.push(Scope::Bold),
                    Tag::Strikethrough => scope.push(Scope::Strikethrough),
                    Tag::Link {
                        dest_url, title, ..
                    } => scope.push(Scope::Link {
                        dest_url: dest_url.into_string(),
                        title: title.into_string(),
                    }),
                    Tag::Image {
                        dest_url, title, ..
                    } => {
                        // Defer rendering until we collect alt text content between Start and End
                        scope.push(Scope::ImageCollect {
                            url: dest_url.into_string(),
                            title: title.into_string(),
                            alt: String::new(),
                        });
                    }
                    _ => {}
                },
                Event::End(tag) => match tag {
                    TagEnd::TableCell => {
                        if let Some(Scope::TableCollect(state)) = scope.last_mut() {
                            state.finish_cell();
                        }
                    }
                    TagEnd::TableRow => {
                        if let Some(Scope::TableCollect(state)) = scope.last_mut() {
                            state.finish_row();
                        }
                    }
                    TagEnd::TableHead => {
                        if let Some(Scope::TableCollect(state)) = scope.last_mut() {
                            state.in_head = false;
                        }
                    }
                    TagEnd::Table => {
                        // Render the collected table using comfy-table
                        if let Some(Scope::TableCollect(state)) = scope.pop() {
                            self.render_table(state, &scope)?;
                            self.last_block = Some(Block::Table);
                        }
                    }
                    TagEnd::Paragraph => {
                        para.flush_paragraph(&scope, self.cfg.width, &mut self.sink);
                        self.last_block = Some(Block::Paragraph);
                    }
                    TagEnd::Heading(_) => {
                        if !para.is_empty() {
                            let base_indent: usize = scope.iter().map(|s| s.indent().0).sum();
                            let line = self.color_theme.heading.apply(para.as_str());
                            let _ = self.sink.write_line(&line, base_indent);
                            para.clear();
                        }
                        scope.pop();
                        self.last_block = Some(Block::Heading);
                    }
                    TagEnd::BlockQuote(..) => {
                        para.flush_paragraph(&scope, self.cfg.width, &mut self.sink);
                        scope.pop();
                        self.last_block = Some(Block::BlockQuote);
                        let quote_depth = scope
                            .iter()
                            .filter(|s| matches!(s, Scope::BlockQuote))
                            .count();
                        if quote_depth > 0 {
                            let quote_prefix = self.glyph_theme.quote_prefix.repeat(quote_depth);
                            let styled_prefix = self.color_theme.quote_prefix.apply(&quote_prefix);
                            para.set_line_prefix(format!("{styled_prefix} "));
                        } else {
                            para.clear_line_prefix();
                        }
                    }
                    TagEnd::List(..) => {
                        para.flush_paragraph(&scope, self.cfg.width, &mut self.sink);
                        scope.pop();
                        self.last_block = Some(Block::List);
                    }
                    TagEnd::DefinitionList => {
                        para.flush_paragraph(&scope, self.cfg.width, &mut self.sink);
                        let _ = scope.pop();
                        self.last_block = Some(Block::List);
                    }
                    TagEnd::DefinitionListTitle => {
                        para.flush_paragraph(&scope, self.cfg.width, &mut self.sink);
                        let _ = scope.pop();
                    }
                    TagEnd::DefinitionListDefinition => {
                        para.flush_paragraph(&scope, self.cfg.width, &mut self.sink);
                        let _ = scope.pop();
                    }
                    TagEnd::Item => {
                        para.flush_paragraph(&scope, self.cfg.width, &mut self.sink);
                        scope.pop();
                        self.last_block = Some(Block::ListItem);
                        if let Some(Scope::List(ListKind::Ordered { next })) = scope.last_mut() {
                            *next += 1;
                        }
                        para.hanging_extra = 0;
                    }
                    TagEnd::CodeBlock => {
                        let indent: usize = scope.iter().map(|s| s.indent().0).sum();
                        if self.cfg.syncat {
                            let mut child = std::process::Command::new("syncat")
                                .arg("-l")
                                .arg("")
                                .arg("-w")
                                .arg(self.cfg.width.0.to_string())
                                .stdin(std::process::Stdio::piped())
                                .stdout(std::process::Stdio::piped())
                                .spawn();
                            if let Ok(ch) = child.as_mut() {
                                if let Some(mut stdin) = ch.stdin.take() {
                                    let _ = write!(stdin, "{code_buffer}");
                                }
                            }
                            if let Ok(ch) = child {
                                if let Ok(output) = ch.wait_with_output() {
                                    let text = String::from_utf8_lossy(&output.stdout);
                                    for line in text.lines() {
                                        let _ = self.sink.write_line(line, indent);
                                    }
                                }
                            }
                        } else {
                            for line in code_buffer.lines() {
                                let styled_line = self.color_theme.code_block.apply(line);
                                let _ = self.sink.write_line(&styled_line, indent);
                            }
                        }
                        code_buffer.clear();
                        scope.pop();
                        self.last_block = Some(Block::Heading);
                    }
                    TagEnd::Link => {
                        if let Some(Scope::Link { dest_url, title }) = scope.pop() {
                            let text_opt =
                                if !title.is_empty() && !dest_url.is_empty() && !self.cfg.hide_urls
                                {
                                    Some(format!(" <{title}: {dest_url}>"))
                                } else if !dest_url.is_empty() && !self.cfg.hide_urls {
                                    Some(format!(" <{dest_url}>"))
                                } else if !title.is_empty() {
                                    Some(format!(" <{title}>"))
                                } else {
                                    None
                                };

                            if let Some(text) = text_opt {
                                // If we are inside a table, append link trail to the current cell
                                let mut pushed_into_table = false;
                                if let Some(table_state) =
                                    scope.iter_mut().rev().find_map(|s| match s {
                                        Scope::TableCollect(st) => Some(st),
                                        _ => None,
                                    })
                                {
                                    table_state.push_text(&text);
                                    pushed_into_table = true;
                                }
                                if !pushed_into_table {
                                    para.wrap_and_push(
                                        &scope,
                                        self.cfg.width,
                                        &text,
                                        &mut self.sink,
                                        &str_width,
                                    );
                                }
                            }
                        }
                    }
                    TagEnd::Image => {
                        if let Some(Scope::ImageCollect {
                            url,
                            title: _title_attr,
                            alt,
                        }) = scope.pop()
                        {
                            if self.cfg.no_images {
                                if !alt.is_empty() {
                                    let styled = self.color_theme.link.apply(&alt);
                                    para.wrap_and_push(
                                        &scope,
                                        self.cfg.width,
                                        &styled,
                                        &mut self.sink,
                                        &str_width,
                                    );
                                }
                                if !self.cfg.hide_urls && !url.is_empty() {
                                    let t = if !alt.is_empty() {
                                        format!(" <{url}>")
                                    } else {
                                        format!("<{url}>")
                                    };
                                    para.wrap_and_push(
                                        &scope,
                                        self.cfg.width,
                                        &t,
                                        &mut self.sink,
                                        &str_width,
                                    );
                                }
                            } else {
                                // Render image and caption
                                para.flush_paragraph(&scope, self.cfg.width, &mut self.sink);
                                self.ensure_spacing_before(Block::Image, &scope);
                                let path = Self::resolve_image_path(&url, file_path);
                                match std::fs::read(&path) {
                                    Ok(bytes) => match image::load_from_memory(&bytes) {
                                        Ok(img) => {
                                            let base_indent: usize =
                                                scope.iter().map(|s| s.indent().0).sum();
                                            let indent = base_indent + para.hanging_extra;
                                            let available = self.cfg.width.0.saturating_sub(indent);
                                            let (resized_png, used_cells) = self
                                                .images
                                                .resize_for_width(&img, available)
                                                .unwrap_or((bytes, available));
                                            let _ = self
                                                .images
                                                .render_inline(&resized_png, indent as u16);
                                            self.last_block = Some(Block::Image);

                                            let caption = alt.trim();
                                            if !caption.is_empty() {
                                                self.ensure_spacing_before(Block::Caption, &scope);
                                                for line in
                                                    self.wrap_caption_lines(caption, used_cells)
                                                {
                                                    let lw = str_width(&line);
                                                    let extra_pad =
                                                        used_cells.saturating_sub(lw) / 2;
                                                    let column = indent + extra_pad;
                                                    let styled =
                                                        self.color_theme.caption.apply(&line);
                                                    let _ = self
                                                        .sink
                                                        .write_line_absolute(&styled, column);
                                                }
                                                self.last_block = Some(Block::Caption);
                                            }
                                        }
                                        Err(e) => {
                                            eprintln!(
                                                "Cannot decode image {}: {e}",
                                                path.display()
                                            );
                                            if !alt.is_empty() {
                                                let styled = self.color_theme.link.apply(&alt);
                                                para.wrap_and_push(
                                                    &scope,
                                                    self.cfg.width,
                                                    &styled,
                                                    &mut self.sink,
                                                    &str_width,
                                                );
                                            }
                                            if !self.cfg.hide_urls && !url.is_empty() {
                                                let t = if !alt.is_empty() {
                                                    format!(" <{url}>")
                                                } else {
                                                    format!("<{url}>")
                                                };
                                                para.wrap_and_push(
                                                    &scope,
                                                    self.cfg.width,
                                                    &t,
                                                    &mut self.sink,
                                                    &str_width,
                                                );
                                            }
                                        }
                                    },
                                    Err(e) => {
                                        eprintln!("Cannot open image {}: {e}", path.display());
                                        if !alt.is_empty() {
                                            let styled = self.color_theme.link.apply(&alt);
                                            para.wrap_and_push(
                                                &scope,
                                                self.cfg.width,
                                                &styled,
                                                &mut self.sink,
                                                &str_width,
                                            );
                                        }
                                        if !self.cfg.hide_urls && !url.is_empty() {
                                            let t = if !alt.is_empty() {
                                                format!(" <{url}>")
                                            } else {
                                                format!("<{url}>")
                                            };
                                            para.wrap_and_push(
                                                &scope,
                                                self.cfg.width,
                                                &t,
                                                &mut self.sink,
                                                &str_width,
                                            );
                                        }
                                    }
                                }
                            }
                        }
                    }
                    _ => {
                        let _ = scope.pop();
                    }
                },
                Event::Text(text) => {
                    if let Some(Scope::CodeBlock(..)) = scope.last() {
                        code_buffer.push_str(&text);
                    } else if let Some(Scope::ImageCollect { alt, .. }) = scope.last_mut() {
                        alt.push_str(&text);
                    } else if let Some(idx) = scope
                        .iter()
                        .rposition(|s| matches!(s, Scope::TableCollect(_)))
                    {
                        let styled_text = self.apply_text_styling(&text, &scope);
                        // Convert literal "\n" into actual newlines for table cells
                        let styled_text = styled_text.replace("\\n", "\n");
                        if let Scope::TableCollect(state) = scope.get_mut(idx).unwrap() {
                            state.push_text(&styled_text);
                        }
                    } else {
                        let styled_text = self.apply_text_styling(&text, &scope);
                        para.wrap_and_push(
                            &scope,
                            self.cfg.width,
                            &styled_text,
                            &mut self.sink,
                            &str_width,
                        );
                    }
                }
                Event::Code(text) => {
                    if let Some(Scope::ImageCollect { alt, .. }) = scope.last_mut() {
                        alt.push_str(&text);
                    } else if let Some(idx) = scope
                        .iter()
                        .rposition(|s| matches!(s, Scope::TableCollect(_)))
                    {
                        let styled = self.color_theme.code.apply(&text);
                        if let Scope::TableCollect(state) = scope.get_mut(idx).unwrap() {
                            state.push_text(&styled);
                        }
                    } else {
                        let styled = self.color_theme.code.apply(&text);
                        para.wrap_and_push(
                            &scope,
                            self.cfg.width,
                            &styled,
                            &mut self.sink,
                            &str_width,
                        );
                    }
                }
                Event::Rule => {
                    para.flush_paragraph(&scope, self.cfg.width, &mut self.sink);
                    self.ensure_spacing_before(Block::Rule, &scope);
                    let hr_line = self.glyph_theme.hr.to_string().repeat(self.cfg.width.0);
                    let styled_hr = self.color_theme.rule.apply(&hr_line);
                    let _ = self.sink.write_line(&styled_hr, 0);
                    self.last_block = Some(Block::Rule);
                }
                Event::SoftBreak => {
                    if let Some(idx) = scope
                        .iter()
                        .rposition(|s| matches!(s, Scope::TableCollect(_)))
                    {
                        if let Scope::TableCollect(state) = scope.get_mut(idx).unwrap() {
                            state.push_text("\n");
                        }
                    } else {
                        para.wrap_and_push(&scope, self.cfg.width, " ", &mut self.sink, &str_width);
                    }
                }
                Event::HardBreak => {
                    if let Some(idx) = scope
                        .iter()
                        .rposition(|s| matches!(s, Scope::TableCollect(_)))
                    {
                        if let Scope::TableCollect(state) = scope.get_mut(idx).unwrap() {
                            state.push_text("\n");
                        }
                    } else {
                        para.flush_paragraph(&scope, self.cfg.width, &mut self.sink);
                    }
                }
                Event::TaskListMarker(checked) => {
                    let text = if checked { "[✓] " } else { "[ ] " };
                    para.wrap_and_push(&scope, self.cfg.width, text, &mut self.sink, &str_width);
                }
                _ => {}
            }
        }

        if !para.is_empty() {
            para.flush_paragraph(&scope, self.cfg.width, &mut self.sink);
        }
        Ok(())
    }

    fn resolve_image_path(raw: &str, file_path: Option<&Path>) -> PathBuf {
        let path = Path::new(raw);
        if path.is_absolute() {
            return path.to_path_buf();
        }
        if path.exists() {
            return path.to_path_buf();
        }
        if let Some(p) = file_path.and_then(|f| f.parent()) {
            let cand = p.join(path);
            if cand.exists() {
                return cand;
            }
        }
        path.to_path_buf()
    }
}

#[derive(Debug, Default, Clone)]
struct TableState {
    headers: Vec<String>,
    rows: Vec<Vec<String>>,
    cur_row: Vec<String>,
    cur_cell: String,
    in_head: bool,
    alignments: Vec<Alignment>,
}

impl TableState {
    fn new() -> Self {
        Self::default()
    }
    fn start_row(&mut self) {
        self.cur_row.clear();
    }
    fn start_cell(&mut self) {
        self.cur_cell.clear();
    }
    fn push_text(&mut self, s: &str) {
        self.cur_cell.push_str(s);
    }
    fn finish_cell(&mut self) {
        self.cur_row.push(self.cur_cell.clone());
        self.cur_cell.clear();
    }
    fn finish_row(&mut self) {
        if self.in_head && self.headers.is_empty() {
            self.headers = self.cur_row.clone();
        } else {
            self.rows.push(self.cur_row.clone());
        }
        self.cur_row.clear();
    }
}

impl<B: ImageBackend, S: Sink> Renderer<B, S> {
    fn render_table(&mut self, state: TableState, scope: &[Scope]) -> io::Result<()> {
        // Compute indent and available width using strong types
        let base_indent: usize = scope.iter().map(|s| s.indent().0).sum();
        let indent = base_indent;
        let available = self.cfg.width.0.saturating_sub(indent);

        let mut table = Table::new();
        // Choose preset according to glyph theme
        // Default GlyphTheme uses a UTF8 hr char; ASCII theme uses '-'
        if self.glyph_theme.hr == '─' {
            table.load_preset(presets::UTF8_FULL);
            // Prefer solid lines for inner separators
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

        // Map Markdown alignments to comfy-table column alignment
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
