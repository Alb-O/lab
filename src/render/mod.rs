use std::io::{self, Write};
use std::path::{Path, PathBuf};

use ansi_term::Style;
use pulldown_cmark::{Event, HeadingLevel, Options, Parser, Tag, TagEnd};

use crate::config::Config;
use crate::media::{ImageBackend, RasteroidBackend};
use crate::str_width::str_width;
use crate::theme::Theme;
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
    Link { dest_url: String, title: String },
    List(ListKind),
    ListItem,
    Code,
    CodeBlock(String),
    BlockQuote,
    Heading(HeadingLevel),
}

impl IndentedScope for Scope {
    fn indent(&self) -> usize {
        match self {
            Scope::List(..) => 2,
            Scope::ListItem => 0,
            Scope::BlockQuote => 2,
            Scope::CodeBlock(..) => 2,
            Scope::Heading(..) => 0,
            _ => 0,
        }
    }
}

pub struct Renderer<B: ImageBackend = RasteroidBackend> {
    cfg: Config,
    theme: Theme,
    images: B,
}

impl<B: ImageBackend + Default> Renderer<B> {
    pub fn new(cfg: Config) -> Self {
        let theme = Theme::from_name(cfg.theme);
        Self {
            cfg,
            theme,
            images: B::default(),
        }
    }
}

impl<B: ImageBackend> Renderer<B> {
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

        let flush_line = |line: &str, indent: usize| {
            if line.is_empty() {
                return;
            }
            let pad = " ".repeat(indent);
            println!("{pad}{line}");
        };

        for event in Parser::new_ext(source, Options::all()) {
            match event {
                Event::Start(tag) => match tag {
                    Tag::Paragraph => {}
                    Tag::Heading { level, .. } => {
                        para.flush_paragraph(&scope, self.cfg.width, &flush_line);
                        scope.push(Scope::Heading(level));
                    }
                    Tag::BlockQuote(..) => {
                        scope.push(Scope::BlockQuote);
                    }
                    Tag::CodeBlock(kind) => {
                        para.flush_paragraph(&scope, self.cfg.width, &flush_line);
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
                        para.set_prefix(format!("{bullet} "));
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
                            let text = format!("[Image: {title}]");
                            para.wrap_and_push(
                                &scope,
                                self.cfg.width,
                                &text,
                                &flush_line,
                                &str_width,
                            );
                            continue;
                        }
                        para.flush_paragraph(&scope, self.cfg.width, &flush_line);

                        let raw = dest_url.into_string();
                        let path = Self::resolve_image_path(&raw, file_path);
                        match std::fs::read(&path) {
                            Ok(bytes) => match image::load_from_memory(&bytes) {
                                Ok(img) => {
                                    let base_indent: usize = scope.iter().map(|s| s.indent()).sum();
                                    let indent = base_indent + para.hanging_extra;
                                    let available = self.cfg.width.0.saturating_sub(indent);
                                    let resized_png = self
                                        .images
                                        .resize_for_width(&img, available)
                                        .unwrap_or(bytes);
                                    let _ = self.images.render_inline(&resized_png, indent as u16);
                                    println!();
                                    if !title.is_empty() {
                                        para.wrap_and_push(
                                            &scope,
                                            self.cfg.width,
                                            &title,
                                            &flush_line,
                                            &str_width,
                                        );
                                        para.flush_paragraph(&scope, self.cfg.width, &flush_line);
                                    }
                                }
                                Err(e) => {
                                    eprintln!("Cannot decode image {}: {e}", path.display());
                                }
                            },
                            Err(e) => {
                                eprintln!("Cannot open image {}: {e}", path.display());
                            }
                        }
                    }
                    _ => {}
                },
                Event::End(tag) => match tag {
                    TagEnd::Paragraph => {
                        para.flush_paragraph(&scope, self.cfg.width, &flush_line);
                    }
                    TagEnd::Heading(_) => {
                        if !para.is_empty() {
                            let base_indent: usize = scope.iter().map(|s| s.indent()).sum();
                            let line = Style::new()
                                .bold()
                                .paint(para.as_str().to_string())
                                .to_string();
                            flush_line(&line, base_indent);
                            para.clear();
                        }
                        println!();
                        scope.pop();
                    }
                    TagEnd::BlockQuote(..) => {
                        para.flush_paragraph(&scope, self.cfg.width, &flush_line);
                        scope.pop();
                    }
                    TagEnd::List(..) => {
                        para.flush_paragraph(&scope, self.cfg.width, &flush_line);
                        scope.pop();
                    }
                    TagEnd::Item => {
                        para.flush_paragraph(&scope, self.cfg.width, &flush_line);
                        scope.pop();
                        if let Some(Scope::List(ListKind::Ordered { next })) = scope.last_mut() {
                            *next += 1;
                        }
                        para.hanging_extra = 0;
                    }
                    TagEnd::CodeBlock => {
                        let indent: usize = scope.iter().map(|s| s.indent()).sum();
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
                                para.wrap_and_push(
                                    &scope,
                                    self.cfg.width,
                                    &text,
                                    &flush_line,
                                    &str_width,
                                );
                            } else if !dest_url.is_empty() && !self.cfg.hide_urls {
                                let text = format!(" <{dest_url}>");
                                para.wrap_and_push(
                                    &scope,
                                    self.cfg.width,
                                    &text,
                                    &flush_line,
                                    &str_width,
                                );
                            } else if !title.is_empty() {
                                let text = format!(" <{title}>");
                                para.wrap_and_push(
                                    &scope,
                                    self.cfg.width,
                                    &text,
                                    &flush_line,
                                    &str_width,
                                );
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
                    } else {
                        para.wrap_and_push(&scope, self.cfg.width, &text, &flush_line, &str_width);
                    }
                }
                Event::Code(text) => {
                    let styled = Style::new().reverse().paint(text.to_string()).to_string();
                    para.wrap_and_push(&scope, self.cfg.width, &styled, &flush_line, &str_width);
                }
                Event::Rule => {
                    para.flush_paragraph(&scope, self.cfg.width, &flush_line);
                    println!("{}", self.theme.hr.to_string().repeat(self.cfg.width.0));
                    println!();
                }
                Event::SoftBreak => {
                    para.wrap_and_push(&scope, self.cfg.width, " ", &flush_line, &str_width);
                }
                Event::HardBreak => {
                    para.flush_paragraph(&scope, self.cfg.width, &flush_line);
                }
                Event::TaskListMarker(checked) => {
                    let text = if checked { "[✓] " } else { "[ ] " };
                    para.wrap_and_push(&scope, self.cfg.width, text, &flush_line, &str_width);
                }
                _ => {}
            }
        }

        if !para.is_empty() {
            para.flush_paragraph(&scope, self.cfg.width, &flush_line);
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
