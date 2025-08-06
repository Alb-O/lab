// Utility functions for CLI

use dot001_error::Dot001Error;
use dot001_parser::{BlendFile, DecompressionPolicy, ParseOptions};
use dot001_tracer::NameResolver;
use log::warn;
use owo_colors::OwoColorize;
use regex::Regex;
use std::cell::OnceCell;
use std::fmt;
use std::path::PathBuf;

/// Command execution context containing common parameters
pub struct CommandContext<'a> {
    pub parse_options: &'a ParseOptions,
    pub no_auto_decompress: bool,
    pub output: &'a OutputHandler,
}

impl<'a> CommandContext<'a> {
    pub fn new(
        parse_options: &'a ParseOptions,
        no_auto_decompress: bool,
        output: &'a OutputHandler,
    ) -> Self {
        Self {
            parse_options,
            no_auto_decompress,
            output,
        }
    }

    /// Load a blend file using the context's parse options and decompression settings
    pub fn load_blend_file(
        &self,
        path: &PathBuf,
    ) -> Result<BlendFile<Box<dyn dot001_parser::ReadSeekSend>>, Dot001Error> {
        load_blend_file(path, self.parse_options, self.no_auto_decompress)
    }
}

/// Output handler that respects quiet mode
pub struct OutputHandler {
    quiet: bool,
}

impl OutputHandler {
    pub fn new(quiet: bool) -> Self {
        Self { quiet }
    }

    /// Print explanatory text (suppressed in quiet mode)
    pub fn print_info(&self, text: &str) {
        if !self.quiet {
            println!("{text}");
        }
    }

    /// Print formatted explanatory text (suppressed in quiet mode)
    pub fn print_info_fmt(&self, args: std::fmt::Arguments) {
        if !self.quiet {
            println!("{args}");
        }
    }

    /// Print raw results (always shown)
    pub fn print_result(&self, text: &str) {
        println!("{text}");
    }

    /// Print formatted raw results (always shown)
    pub fn print_result_fmt(&self, args: std::fmt::Arguments) {
        println!("{args}");
    }

    /// Print to stderr (always shown)
    pub fn print_error(&self, text: &str) {
        eprintln!("{text}");
    }
}

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
    ) -> Result<Self, dot001_error::Dot001Error> {
        let block = blend_file.get_block(index).ok_or_else(|| {
            dot001_error::Dot001Error::blend_file(
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
/// Basic template: "1220: GR | CollectionNew3"
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
            parts.push(format!("{index}: {code}"));
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

        parts.join(" | ")
    }
}

/// Compact template: "GR | CollectionNew3" (no index)
pub struct CompactFormatter;

impl BlockFormatter for CompactFormatter {
    fn format(&self, block: &BlockInfo, options: &DisplayOptions) -> String {
        let mut opts = options.clone();
        opts.show_index = false;
        BasicFormatter.format(block, &opts)
    }
}

/// Detailed template: "Block 1238: DNA1 | d[4] • size: 131,128 bytes • addr: 0x7ff6649f6a10"
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

// Helper functions for easier migration
impl BlockInfo {
    /// Create a BlockDisplay from this BlockInfo with default formatting
    pub fn display(&self) -> BlockDisplay {
        BlockDisplay::new(self.clone())
    }
}

pub fn create_parse_options(cli: &crate::Cli) -> ParseOptions {
    let mut policy = DecompressionPolicy {
        max_in_memory_bytes: cli.max_in_memory * 1024 * 1024,
        temp_dir: cli.temp_dir.clone(),
        ..Default::default()
    };
    if let Some(prefer_mmap) = cli.prefer_mmap {
        policy.prefer_mmap_temp = prefer_mmap;
    }
    ParseOptions {
        decompression_policy: policy,
    }
}

pub fn load_blend_file(
    file_path: &PathBuf,
    options: &ParseOptions,
    no_auto_decompress: bool,
) -> Result<dot001_parser::BlendFile<Box<dyn dot001_parser::ReadSeekSend>>, dot001_error::Dot001Error>
{
    use std::fs::File;
    use std::io::BufReader;
    if no_auto_decompress {
        let file = File::open(file_path)?;
        let reader = BufReader::new(file);
        let boxed_reader: Box<dyn dot001_parser::ReadSeekSend> = Box::new(reader);
        Ok(dot001_parser::BlendFile::new(boxed_reader)?)
    } else {
        let (blend_file, _mode) = dot001_parser::parse_from_path(file_path, Some(options))?;
        Ok(blend_file)
    }
}

/// Result of resolving a block identifier (name or index)
#[derive(Debug, Clone)]
pub enum BlockResolution {
    /// Single block found
    Single(usize),
    /// Multiple blocks found with the same name (ambiguous)
    Ambiguous(Vec<BlockMatch>),
    /// No blocks found
    NotFound,
}

/// Information about a matched block
#[derive(Debug, Clone)]
pub struct BlockMatch {
    pub index: usize,
    pub name: String,
    pub block_code: String,
}

/// Resolve a block identifier (name or index) to one or more block indices
///
/// This function accepts either:
/// - A numeric string (e.g., "5") representing a block index
/// - A name string (e.g., "Cube") representing a datablock name
///
/// Returns:
/// - `BlockResolution::Single(index)` if exactly one block is found
/// - `BlockResolution::Ambiguous(matches)` if multiple blocks have the same name
/// - `BlockResolution::NotFound` if no blocks match the identifier
pub fn resolve_block_identifier<R: std::io::Read + std::io::Seek>(
    identifier: &str,
    blend_file: &mut BlendFile<R>,
) -> Result<BlockResolution, Dot001Error> {
    let identifier = identifier.trim();

    // First, try to parse as a numeric index
    if let Ok(index) = identifier.parse::<usize>() {
        if index < blend_file.blocks_len() {
            return Ok(BlockResolution::Single(index));
        } else {
            return Ok(BlockResolution::NotFound);
        }
    }

    // If not numeric, search by name
    let mut matches = Vec::new();

    for block_index in 0..blend_file.blocks_len() {
        if let Some(name) = NameResolver::resolve_name(block_index, blend_file) {
            // Case-insensitive name matching
            if name.to_lowercase() == identifier.to_lowercase() {
                let block_code = if let Some(block) = blend_file.get_block(block_index) {
                    String::from_utf8_lossy(&block.header.code)
                        .trim_end_matches('\0')
                        .to_string()
                } else {
                    "????".to_string()
                };

                matches.push(BlockMatch {
                    index: block_index,
                    name,
                    block_code,
                });
            }
        }
    }

    match matches.len() {
        0 => Ok(BlockResolution::NotFound),
        1 => Ok(BlockResolution::Single(matches[0].index)),
        _ => Ok(BlockResolution::Ambiguous(matches)),
    }
}

/// Helper function to display ambiguous matches and suggest resolution
pub fn display_ambiguous_matches(identifier: &str, matches: &[BlockMatch]) {
    warn!("Multiple blocks found with name '{identifier}':");
    eprintln!();
    for (i, block_match) in matches.iter().enumerate() {
        let block_info = BlockInfo::with_name(
            block_match.index,
            block_match.block_code.clone(),
            block_match.name.clone(),
        );
        eprintln!("  {}: Block {}", i + 1, block_info.display());
    }
    eprintln!();
    eprintln!("Please re-run the command using a specific block index:");
    for block_match in matches {
        let colored_index = colorize_index(block_match.index);
        let block_info = BlockInfo::with_name(
            block_match.index,
            block_match.block_code.clone(),
            block_match.name.clone(),
        );
        let block_display = BlockDisplay::new(block_info).with_formatter(CompactFormatter);
        eprintln!("  --block-index {colored_index} (for {block_display})");
    }
}

/// Resolve a block identifier and handle common error cases
///
/// This is a convenience function that wraps `resolve_block_identifier` and handles
/// the common error cases that most commands need:
/// - Logs successful resolution at INFO level
/// - Shows ambiguous matches with suggestions
/// - Shows "not found" errors with helpful suggestions
///
/// Returns `Some(block_index)` on success, `None` on any error condition.
pub fn resolve_block_or_exit<R: std::io::Read + std::io::Seek>(
    identifier: &str,
    blend_file: &mut BlendFile<R>,
) -> Option<usize> {
    match resolve_block_identifier(identifier, blend_file) {
        Ok(resolution) => match resolution {
            BlockResolution::Single(index) => {
                log::info!("Resolved '{identifier}' to block index {index}");
                Some(index)
            }
            BlockResolution::Ambiguous(matches) => {
                display_ambiguous_matches(identifier, &matches);
                None
            }
            BlockResolution::NotFound => {
                log::error!("No block found with identifier '{identifier}'");
                eprintln!("Use 'blocks' command to list all available blocks.");
                None
            }
        },
        Err(e) => {
            log::error!("Failed to resolve block identifier '{identifier}': {e}");
            None
        }
    }
}

/// Resolve a block identifier with specific block type validation
///
/// This function resolves a block identifier and verifies that the resolved block
/// has the expected block type (e.g., "ME" for mesh blocks).
///
/// Returns `Some(block_index)` if a block of the correct type is found,
/// `None` on any error condition.
pub fn resolve_typed_block_or_exit<R: std::io::Read + std::io::Seek>(
    identifier: &str,
    expected_type: &str,
    blend_file: &mut BlendFile<R>,
) -> Option<usize> {
    match resolve_block_identifier(identifier, blend_file) {
        Ok(resolution) => match resolution {
            BlockResolution::Single(index) => {
                // Verify the block type
                if let Some(block) = blend_file.get_block(index) {
                    let code_str = String::from_utf8_lossy(&block.header.code);
                    let code = code_str.trim_end_matches('\0');
                    if code == expected_type {
                        log::info!(
                            "Resolved '{identifier}' to {expected_type} block at index {index}"
                        );
                        Some(index)
                    } else {
                        log::error!(
                            "Block {index} is not a {expected_type} block, it's a {code} block"
                        );
                        None
                    }
                } else {
                    log::error!("Block {index} not found");
                    None
                }
            }
            BlockResolution::Ambiguous(matches) => {
                // Filter matches to only show blocks of the expected type
                let typed_matches: Vec<_> = matches
                    .into_iter()
                    .filter(|m| m.block_code == expected_type)
                    .collect();

                if typed_matches.is_empty() {
                    log::error!("No {expected_type} blocks found with name '{identifier}'");
                    None
                } else if typed_matches.len() == 1 {
                    let index = typed_matches[0].index;
                    log::info!("Resolved '{identifier}' to {expected_type} block at index {index}");
                    Some(index)
                } else {
                    display_ambiguous_matches(identifier, &typed_matches);
                    None
                }
            }
            BlockResolution::NotFound => {
                log::error!("No {expected_type} block found with identifier '{identifier}'");
                eprintln!("Use 'blocks' command to list all available blocks.");
                None
            }
        },
        Err(e) => {
            log::error!("Failed to resolve {expected_type} block identifier '{identifier}': {e}");
            None
        }
    }
}
