// Block display formatting and rendering logic

use dot001_error::Dot001Error;
use dot001_parser::BlendFile;
use dot001_tracer::NameResolver;
use owo_colors::OwoColorize;
use regex::Regex;
use std::cell::OnceCell;
use std::fmt;

// Colorization helpers
pub fn should_use_colors() -> bool {
    atty::is(atty::Stream::Stdout)
}

pub fn colorize_index(index: usize) -> String {
    if should_use_colors() {
        index.to_string().green().to_string()
    } else {
        index.to_string()
    }
}

pub fn colorize_code(code: &str) -> String {
    if should_use_colors() {
        code.blue().to_string()
    } else {
        code.to_string()
    }
}

pub fn colorize_name(name: &str) -> String {
    if should_use_colors() {
        name.yellow().to_string()
    } else {
        name.to_string()
    }
}

pub fn highlight_matches(text: &str, filter_expressions: &[(&str, &str, &str)]) -> String {
    if !should_use_colors() {
        return text.to_string();
    }

    let mut result = text.to_string();

    // Apply highlighting for name matches
    for (_, key, value) in filter_expressions {
        if *key == "name" && !value.is_empty() {
            // Try to use the value as a regex first, fall back to literal match
            let pattern = if let Ok(regex) = Regex::new(&format!("(?i){value}")) {
                regex
            } else {
                // If the value is not a valid regex, escape it for literal matching
                match Regex::new(&format!("(?i){}", regex::escape(value))) {
                    Ok(regex) => regex,
                    Err(_) => continue, // Skip this filter if we can't create a regex
                }
            };

            result = pattern
                .replace_all(&result, |caps: &regex::Captures| {
                    caps[0].to_string().red().to_string()
                })
                .to_string();
        }
    }

    result
}

/// Pure data structure representing a block - no display logic
#[derive(Debug, Clone)]
pub struct BlockInfo {
    pub index: usize,
    pub code: String,
    pub name: Option<String>,
}

impl BlockInfo {
    pub fn new(index: usize, code: String) -> Self {
        Self {
            index,
            code,
            name: None,
        }
    }

    pub fn with_name(index: usize, code: String, name: String) -> Self {
        Self {
            index,
            code,
            name: Some(name),
        }
    }

    pub fn from_blend_file<R: std::io::Read + std::io::Seek>(
        index: usize,
        blend_file: &mut BlendFile<R>,
    ) -> Result<Self, Dot001Error> {
        let block = blend_file.get_block(index).ok_or_else(|| {
            Dot001Error::blend_file(
                format!("Block index {index} out of range"),
                dot001_error::BlendFileErrorKind::InvalidBlockIndex,
            )
        })?;
        let code = String::from_utf8_lossy(&block.header.code)
            .trim_end_matches('\0')
            .to_string();
        let name = NameResolver::resolve_name(index, blend_file);

        Ok(if let Some(name) = name {
            Self::with_name(index, code, name)
        } else {
            Self::new(index, code)
        })
    }

    /// Create a BlockDisplay from this BlockInfo with default formatting
    pub fn display(&self) -> BlockDisplay {
        BlockDisplay::new(self.clone())
    }
}

/// Centralized display configuration
#[derive(Debug, Clone)]
pub struct DisplayOptions {
    pub show_index: bool,
    pub show_name: bool,
    pub use_colors: bool,
}

impl DisplayOptions {
    pub fn default() -> Self {
        Self {
            show_index: true,
            show_name: true,
            use_colors: should_use_colors(),
        }
    }
}

/// Trait for block formatting strategies
pub trait BlockFormatter: Send + Sync {
    fn format(&self, block: &BlockInfo, options: &DisplayOptions) -> String;
}

/// Template-based formatters with standardized display patterns
///
/// Basic template: "[1220] GR CollectionNew3"
pub struct BasicFormatter;

impl BlockFormatter for BasicFormatter {
    fn format(&self, block: &BlockInfo, options: &DisplayOptions) -> String {
        let code = if options.use_colors {
            colorize_code(&block.code)
        } else {
            block.code.clone()
        };

        let mut parts = Vec::new();

        if options.show_index {
            let index = if options.use_colors {
                colorize_index(block.index)
            } else {
                block.index.to_string()
            };
            parts.push(format!("[{index}] {code}"));
        } else {
            parts.push(code);
        }

        if options.show_name {
            if let Some(name) = &block.name {
                let name_str = if options.use_colors {
                    colorize_name(name)
                } else {
                    name.clone()
                };
                parts.push(name_str);
            }
        }

        parts.join(" ")
    }
}

/// Compact template: "GR CollectionNew3" (no index)
pub struct CompactFormatter;

impl BlockFormatter for CompactFormatter {
    fn format(&self, block: &BlockInfo, options: &DisplayOptions) -> String {
        let mut opts = options.clone();
        opts.show_index = false;
        BasicFormatter.format(block, &opts)
    }
}

/// Detailed template: "Block [1238] DNA1 d[4] • size: 131,128 bytes • addr: 0x7ff6649f6a10"
pub struct DetailedFormatter {
    pub size: Option<u64>,
    pub address: Option<u64>,
    pub offset: Option<u64>,
}

impl DetailedFormatter {
    pub fn new() -> Self {
        Self {
            size: None,
            address: None,
            offset: None,
        }
    }

    pub fn with_size(mut self, size: u64) -> Self {
        self.size = Some(size);
        self
    }

    pub fn with_address(mut self, address: u64) -> Self {
        self.address = Some(address);
        self
    }

    pub fn with_offset(mut self, offset: u64) -> Self {
        self.offset = Some(offset);
        self
    }
}

impl BlockFormatter for DetailedFormatter {
    fn format(&self, block: &BlockInfo, options: &DisplayOptions) -> String {
        let basic = BasicFormatter.format(block, options);
        let mut parts = vec![format!("Block {basic}")];

        if let Some(size) = self.size {
            parts.push(format!("size: {}", format_bytes(size)));
        }

        if let Some(addr) = self.address {
            parts.push(format!("addr: 0x{addr:x}"));
        }

        if let Some(offset) = self.offset {
            parts.push(format!("offset: 0x{offset:x}"));
        }

        parts.join(" • ")
    }
}

/// Helper function to format byte sizes in human-readable format
fn format_bytes(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{bytes} bytes")
    } else if bytes < 1024 * 1024 {
        format!("{:.1}K", bytes as f64 / 1024.0)
    } else if bytes < 1024 * 1024 * 1024 {
        format!("{:.1}M", bytes as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.1}G", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}

/// Highlighting wrapper formatter
pub struct HighlightFormatter {
    inner: Box<dyn BlockFormatter>,
    patterns: Vec<(String, String, String)>,
}

impl HighlightFormatter {
    pub fn new(inner: Box<dyn BlockFormatter>, patterns: Vec<(String, String, String)>) -> Self {
        Self { inner, patterns }
    }
}

impl BlockFormatter for HighlightFormatter {
    fn format(&self, block: &BlockInfo, options: &DisplayOptions) -> String {
        let base_format = self.inner.format(block, options);

        if options.use_colors && !self.patterns.is_empty() {
            let filter_slice_triples: Vec<(&str, &str, &str)> = self
                .patterns
                .iter()
                .map(|(m, k, v)| (m.as_str(), k.as_str(), v.as_str()))
                .collect();
            highlight_matches(&base_format, &filter_slice_triples)
        } else {
            base_format
        }
    }
}

/// Main display wrapper with lazy evaluation and caching
pub struct BlockDisplay {
    block: BlockInfo,
    formatter: Box<dyn BlockFormatter>,
    options: DisplayOptions,
    cached: OnceCell<String>,
}

impl BlockDisplay {
    pub fn new(block: BlockInfo) -> Self {
        Self {
            block,
            formatter: Box::new(BasicFormatter),
            options: DisplayOptions::default(),
            cached: OnceCell::new(),
        }
    }

    pub fn with_formatter<F: BlockFormatter + 'static>(mut self, formatter: F) -> Self {
        self.formatter = Box::new(formatter);
        self.cached = OnceCell::new(); // Clear cache
        self
    }

    pub fn with_highlighting(self, patterns: Vec<(String, String, String)>) -> Self {
        // Move current formatter into highlight wrapper
        let highlighted = HighlightFormatter::new(self.formatter, patterns);
        Self {
            block: self.block,
            formatter: Box::new(highlighted),
            options: self.options,
            cached: OnceCell::new(),
        }
    }

    fn render(&self) -> &String {
        self.cached
            .get_or_init(|| self.formatter.format(&self.block, &self.options))
    }
}

impl fmt::Display for BlockDisplay {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.render())
    }
}

impl std::ops::Deref for BlockDisplay {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.render()
    }
}
