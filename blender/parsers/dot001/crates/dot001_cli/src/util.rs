// Utility functions for CLI

use crate::block_display::{BlockDisplay, BlockInfo, CompactFormatter, colorize_index};
use dot001_events::error::Error;
use dot001_parser::NameResolver;
use dot001_parser::{BlendFile, DecompressionPolicy, ParseOptions};
use log::warn;
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
    pub fn load_blend_file(&self, path: &PathBuf) -> Result<BlendFile, Error> {
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
) -> Result<dot001_parser::BlendFile, dot001_events::error::Error> {
    if no_auto_decompress {
        // Direct zero-copy parse
        let blend_file = dot001_parser::from_path(file_path)?;
        Ok(blend_file)
    } else {
        // Use parse with decompression options
        let (blend_file, _mode) = dot001_parser::parse_from_path_buf(file_path, Some(options))?;
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
pub fn resolve_block_identifier(
    identifier: &str,
    blend_file: &mut BlendFile,
) -> Result<BlockResolution, Error> {
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
                    dot001_parser::block_code_to_string(block.header.code)
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
pub fn resolve_block_or_exit(identifier: &str, blend_file: &mut BlendFile) -> Option<usize> {
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
pub fn resolve_typed_block_or_exit(
    identifier: &str,
    expected_type: &str,
    blend_file: &mut BlendFile,
) -> Option<usize> {
    match resolve_block_identifier(identifier, blend_file) {
        Ok(resolution) => match resolution {
            BlockResolution::Single(index) => {
                // Verify the block type
                if let Some(block) = blend_file.get_block(index) {
                    let code = dot001_parser::block_code_to_string(block.header.code);
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
