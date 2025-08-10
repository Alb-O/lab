use std::io::{self, Write};
use std::path::{Path, PathBuf};

use ansi_term::Style;
use pulldown_cmark::{Event, HeadingLevel, Options, Parser, Tag, TagEnd};
use rasteroid::image_extended::InlineImage as _;
use rasteroid::term_misc::EnvIdentifiers;
use rasteroid::{InlineEncoder, inline_an_image};

use crate::str_width::str_width;
use crate::words::Words;

#[derive(Debug, Clone, Copy)]
pub struct Cells(pub usize);

#[derive(Debug, Clone)]
pub struct Config {
    pub width: Cells,
    pub tab_length: usize,
    pub hide_urls: bool,
    pub no_images: bool,
    pub syncat: bool,
    pub dev: bool,
    pub theme: ThemeName,
}

impl Config {
    pub fn validate(self) -> Self {
        // ensure sane bounds; enforce minimal width
        let width = Cells(self.width.0.max(20));
        Self { width, ..self }
    }
}

#[derive(Debug)]
enum Scope {
    Italic,
    Bold,
    Strikethrough,
    Link { dest_url: String, title: String },
    List(ListKind),
    ListItem,
    Code,
    CodeBlock(String),
    BlockQuote,
    Heading(HeadingLevel),
}

impl Scope {
    fn indent(&self) -> usize {
        match self {
            Scope::List(..) => 2,
            Scope::ListItem => 0, // handled via hanging indent and prefix
            Scope::BlockQuote => 2,
            Scope::CodeBlock(..) => 2,
            Scope::Heading(..) => 0,
            _ => 0,
        }
    }
}

#[derive(Debug, Clone)]
enum ListKind {
    Ordered { next: u64 },
    Unordered,
}

#[derive(Debug, Clone)]
pub struct GlyphTheme {
    pub hr: char,
    pub quote_prefix: &'static str,
    pub bullets: [char; 3],
}

impl GlyphTheme {
    pub fn default() -> Self {
        Self {
            hr: '─',
            quote_prefix: "┃",
            bullets: ['•', '–', '◦'],
        }
    }
    pub fn ascii() -> Self {
        Self {
            hr: '-',
            quote_prefix: ">",
            bullets: ['*', '-', 'o'],
        }
    }
    pub fn from_name(name: ThemeName) -> Self {
        match name {
            ThemeName::Unicode => Self::default(),
            ThemeName::Ascii => Self::ascii(),
        }
    }
    pub fn bullet_for_depth(&self, depth: usize) -> char {
        let idx = depth.min(self.bullets.len().saturating_sub(1));
        self.bullets[idx]
    }
}

#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub enum ThemeName {
    Unicode,
    Ascii,
}

pub struct Renderer {
    cfg: Config,
    theme: GlyphTheme,
}

impl Renderer {
    pub fn new(cfg: Config) -> Self {
        Self {
            cfg: cfg.clone(),
            theme: GlyphTheme::from_name(cfg.theme),
        }
    }

    pub fn render_markdown(&mut self, source: &str, file_path: Option<&Path>) -> io::Result<()> {
        if self.cfg.dev {
            for e in Parser::new_ext(source, Options::all()) {
                eprintln!("{e:?}");
            }
            return Ok(());
        }

        let mut scope: Vec<Scope> = vec![];
        // paragraph assembly
        struct Para {
            buffer: String,
            pending_prefix: Option<String>,
            hanging_extra: usize,
        }
        impl Para {
            fn new() -> Self {
                Self {
                    buffer: String::new(),
                    pending_prefix: None,
                    hanging_extra: 0,
                }
            }
            fn wrap_and_push(&mut self, scope: &Vec<Scope>, width: Cells, text: &str) {
                let base_indent = current_indent(scope);
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
                for word in Words::new(text) {
                    let current_avail = if self.pending_prefix.is_some() {
                        first_avail
                    } else {
                        width.0.saturating_sub(indent)
                    };
                    if str_width(&self.buffer) + str_width(&word) > current_avail {
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
                        self.buffer.push_str(word.trim());
                    } else {
                        self.buffer.push_str(&word);
                    }
                }
            }
            fn flush_paragraph(&mut self, scope: &Vec<Scope>, _width: Cells) {
                if !self.buffer.is_empty() {
                    let base_indent = current_indent(scope);
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
        }
        let mut para = Para::new();
        let mut code_buffer = String::new();

        fn current_indent(scope: &Vec<Scope>) -> usize {
            scope.iter().map(|s| s.indent()).sum()
        }
        fn flush_line(line: &str, indent: usize) {
            if line.is_empty() {
                return;
            }
            let pad = " ".repeat(indent);
            println!("{pad}{line}");
        }
        // list handling lives in `para`

        for event in Parser::new_ext(source, Options::all()) {
            match event {
                Event::Start(tag) => match tag {
                    Tag::Paragraph => {
                        // no-op; content will flow into buffer
                    }
                    Tag::Heading { level, .. } => {
                        // end any running line
                        para.flush_paragraph(&scope, self.cfg.width);
                        scope.push(Scope::Heading(level));
                    }
                    Tag::BlockQuote(..) => {
                        scope.push(Scope::BlockQuote);
                    }
                    Tag::CodeBlock(kind) => {
                        para.flush_paragraph(&scope, self.cfg.width);
                        let lang = match kind {
                            pulldown_cmark::CodeBlockKind::Indented => "".to_string(),
                            pulldown_cmark::CodeBlockKind::Fenced(l) => l.into_string(),
                        };
                        scope.push(Scope::CodeBlock(lang));
                    }
                    Tag::List(start) => {
                        let kind = match start {
                            Some(n) => ListKind::Ordered { next: n },
                            None => ListKind::Unordered,
                        };
                        scope.push(Scope::List(kind));
                    }
                    Tag::Item => {
                        // compute bullet/number prefix
                        let depth = scope.iter().filter(|s| matches!(s, Scope::List(_))).count();
                        let bullet = match scope.iter().rev().find_map(|s| match s {
                            Scope::List(ListKind::Ordered { next }) => Some(next.to_string() + "."),
                            Scope::List(ListKind::Unordered) => Some(
                                self.theme
                                    .bullet_for_depth(depth.saturating_sub(1))
                                    .to_string(),
                            ),
                            _ => None,
                        }) {
                            Some(s) => s,
                            None => String::from("-"),
                        };
                        para.pending_prefix = Some(format!("{bullet} "));
                        // hanging_extra remains until end of item
                        scope.push(Scope::ListItem);
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
                        if self.cfg.no_images {
                            // fallback textual representation
                            let text = format!("[Image: {title}]");
                            para.wrap_and_push(&scope, self.cfg.width, &text);
                            continue;
                        }
                        // finalize current paragraph before image
                        para.flush_paragraph(&scope, self.cfg.width);

                        // resolve path: absolute or relative to file_path
                        let raw = dest_url.into_string();
                        let path = Self::resolve_image_path(&raw, file_path);
                        match std::fs::read(&path) {
                            Ok(bytes) => {
                                match image::load_from_memory(&bytes) {
                                    Ok(img) => {
                                        let indent = current_indent(&scope) + para.hanging_extra;
                                        let available = self.cfg.width.0.saturating_sub(indent);
                                        let dim = format!("{available}c");
                                        let (resized_png, _offset, _w, _h) =
                                            match img.resize_plus(Some(&dim), None, false, false) {
                                                Ok(v) => v,
                                                Err(e) => {
                                                    eprintln!("Image resize failed: {e}");
                                                    (bytes, 0, 0, 0) // original
                                                }
                                            };
                                        let mut env = EnvIdentifiers::new();
                                        let encoder = InlineEncoder::auto_detect(
                                            false, false, false, false, &mut env,
                                        );
                                        let mut out = io::stdout();
                                        let left_offset = indent as u16;
                                        if let Err(e) = inline_an_image(
                                            &resized_png,
                                            &mut out,
                                            Some(left_offset),
                                            None,
                                            &encoder,
                                        ) {
                                            eprintln!("Image render failed: {e}");
                                        }
                                        let _ = out.flush();
                                        println!();
                                        if !title.is_empty() {
                                            para.wrap_and_push(&scope, self.cfg.width, &title);
                                            para.flush_paragraph(&scope, self.cfg.width);
                                        }
                                    }
                                    Err(e) => {
                                        eprintln!("Cannot decode image {}: {e}", path.display());
                                    }
                                }
                            }
                            Err(e) => {
                                eprintln!("Cannot open image {}: {e}", path.display());
                            }
                        }
                    }
                    _ => {}
                },
                Event::End(tag) => match tag {
                    TagEnd::Paragraph => {
                        para.flush_paragraph(&scope, self.cfg.width);
                    }
                    TagEnd::Heading(_) => {
                        // render heading buffer with simple styling
                        if !para.buffer.is_empty() {
                            let indent = current_indent(&scope);
                            let line = Style::new().bold().paint(&para.buffer).to_string();
                            flush_line(&line, indent);
                            para.buffer.clear();
                        }
                        println!();
                        scope.pop();
                    }
                    TagEnd::BlockQuote(..) => {
                        para.flush_paragraph(&scope, self.cfg.width);
                        scope.pop();
                    }
                    TagEnd::List(..) => {
                        para.flush_paragraph(&scope, self.cfg.width);
                        scope.pop();
                    }
                    TagEnd::Item => {
                        para.flush_paragraph(&scope, self.cfg.width);
                        scope.pop();
                        // increment ordered counter if needed
                        if let Some(Scope::List(ListKind::Ordered { next })) = scope.last_mut() {
                            *next += 1;
                        }
                        // reset list-item hanging indent
                        para.hanging_extra = 0;
                    }
                    TagEnd::CodeBlock => {
                        let indent = current_indent(&scope);
                        if self.cfg.syncat {
                            // try external highlighter if available
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
                                        flush_line(line, indent);
                                    }
                                }
                            }
                        } else {
                            for line in code_buffer.lines() {
                                flush_line(line, indent);
                            }
                        }
                        code_buffer.clear();
                        println!();
                        scope.pop();
                    }
                    TagEnd::Link => {
                        if let Some(Scope::Link { dest_url, title }) = scope.pop() {
                            if !title.is_empty() && !dest_url.is_empty() && !self.cfg.hide_urls {
                                let text = format!(" <{title}: {dest_url}>");
                                para.wrap_and_push(&scope, self.cfg.width, &text);
                            } else if !dest_url.is_empty() && !self.cfg.hide_urls {
                                let text = format!(" <{dest_url}>");
                                para.wrap_and_push(&scope, self.cfg.width, &text);
                            } else if !title.is_empty() {
                                let text = format!(" <{title}>");
                                para.wrap_and_push(&scope, self.cfg.width, &text);
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
                    } else if let Some(Scope::Code) = scope.last() {
                        para.wrap_and_push(&scope, self.cfg.width, &text);
                    } else {
                        para.wrap_and_push(&scope, self.cfg.width, &text);
                    }
                }
                Event::Code(text) => {
                    let styled = Style::new().reverse().paint(text.to_string()).to_string();
                    para.wrap_and_push(&scope, self.cfg.width, &styled);
                }
                Event::Rule => {
                    para.flush_paragraph(&scope, self.cfg.width);
                    println!("{}", self.theme.hr.to_string().repeat(self.cfg.width.0));
                    println!();
                }
                Event::SoftBreak => {
                    para.wrap_and_push(&scope, self.cfg.width, " ");
                }
                Event::HardBreak => {
                    para.flush_paragraph(&scope, self.cfg.width);
                }
                Event::TaskListMarker(checked) => {
                    let text = if checked { "[✓] " } else { "[ ] " };
                    para.wrap_and_push(&scope, self.cfg.width, text);
                }
                _ => {}
            }
        }

        // finalize
        if !para.buffer.is_empty() {
            para.flush_paragraph(&scope, self.cfg.width);
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
