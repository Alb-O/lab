use crate::format::{BHead, codes};
use crate::reader::BlendFile;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Configuration options for file analysis and display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisOptions {
    /// Whether to include system blocks (DNA1, GLOB, etc.) in output
    pub include_system_blocks: bool,
    /// Maximum number of blocks to show in detailed view
    pub max_blocks_to_show: usize,
    /// Whether to show warnings about invalid block sizes
    pub show_invalid_blocks: bool,
    /// Whether to show analysis warnings
    pub show_warnings: bool,
}

impl Default for AnalysisOptions {
    fn default() -> Self {
        Self {
            include_system_blocks: false,
            max_blocks_to_show: 15,
            show_invalid_blocks: true,
            show_warnings: true,
        }
    }
}

/// Statistics for a particular block type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockStats {
    /// Number of blocks of this type
    pub count: usize,
    /// Total size of all blocks of this type in bytes
    pub total_size: usize,
    /// Average size per block in bytes
    pub avg_size: usize,
    /// Smallest block size in bytes
    pub min_size: usize,
    /// Largest block size in bytes
    pub max_size: usize,
}

impl BlockStats {
    fn new() -> Self {
        Self {
            count: 0,
            total_size: 0,
            avg_size: 0,
            min_size: usize::MAX,
            max_size: 0,
        }
    }

    fn add_block(&mut self, size: usize) {
        self.count += 1;
        self.total_size += size;
        self.min_size = self.min_size.min(size);
        self.max_size = self.max_size.max(size);
        self.avg_size = self.total_size / self.count;
    }
}

/// Complete analysis results for a .blend file
#[derive(Debug, Serialize, Deserialize)]
pub struct FileAnalysis {
    /// Total number of blocks in the file
    pub total_blocks: usize,
    /// Number of data blocks (Objects, Meshes, etc.)
    pub data_blocks: usize,
    /// Number of metadata blocks (DNA1, GLOB, etc.)
    pub meta_blocks: usize,
    /// Total file size in bytes
    pub total_size: usize,
    /// Statistics grouped by block type
    pub block_type_stats: HashMap<String, BlockStats>,
    /// List of blocks with invalid/suspicious sizes
    pub invalid_blocks: Vec<String>,
    /// Analysis warnings and issues found
    pub warnings: Vec<String>,
}

impl FileAnalysis {
    /// Analyze a .blend file with default options
    pub fn analyze(bf: &BlendFile) -> Self {
        Self::analyze_with_options(bf, &AnalysisOptions::default())
    }

    /// Analyze a .blend file with custom options
    pub fn analyze_with_options(bf: &BlendFile, _options: &AnalysisOptions) -> Self {
        let mut analysis = FileAnalysis {
            total_blocks: 0,
            data_blocks: 0,
            meta_blocks: 0,
            total_size: 0,
            block_type_stats: HashMap::new(),
            invalid_blocks: Vec::new(),
            warnings: Vec::new(),
        };

        for bh in bf.blocks() {
            analysis.analyze_block(&bh);
        }

        analysis.validate();
        analysis
    }

    fn analyze_block(&mut self, bh: &BHead) {
        self.total_blocks += 1;
        let size = bh.len as usize;
        self.total_size += size;

        let block_info = bh.block_info();
        let type_name = block_info.name.to_string();

        if block_info.is_data_block {
            self.data_blocks += 1;
        } else {
            self.meta_blocks += 1;
        }

        let stats = self
            .block_type_stats
            .entry(type_name.clone())
            .or_insert_with(BlockStats::new);
        stats.add_block(size);

        if !bh.is_valid_size() {
            self.invalid_blocks.push(format!(
                "{} ({}): size {} bytes",
                type_name,
                bh.code_string(),
                size
            ));
        }

        if bh.len < 0 {
            self.warnings
                .push(format!("{}: negative size {}", type_name, bh.len));
        }

        if bh.code == codes::BLO_CODE_ENDB && bh.len != 0 {
            self.warnings
                .push("ENDB block should have zero length".to_string());
        }
    }

    fn validate(&mut self) {
        if !self.block_type_stats.contains_key("DNA1") {
            self.warnings
                .push("Missing DNA1 block - file may be corrupted or very old".to_string());
        }

        if !self.block_type_stats.contains_key("ENDB") {
            self.warnings
                .push("Missing ENDB block - file may be truncated".to_string());
        }

        if self.data_blocks == 0 {
            self.warnings
                .push("No data blocks found - file appears empty".to_string());
        }
    }
}

/// Determine if a block should be shown based on analysis options
pub fn should_show_block(bh: &BHead, options: &AnalysisOptions) -> bool {
    if !options.include_system_blocks && bh.is_system_block() {
        return false;
    }
    true
}

/// Get the most interesting blocks from a file for display
///
/// This function selects blocks to show based on the analysis options,
/// prioritizing user data blocks and optionally including system blocks.
pub fn get_interesting_blocks(bf: &BlendFile, options: &AnalysisOptions) -> Vec<BHead> {
    let mut blocks = Vec::new();
    let mut seen_count = 0;

    // If including system blocks, prioritize DNA1 for SDNA info
    if options.include_system_blocks {
        for bh in bf.blocks() {
            if seen_count >= options.max_blocks_to_show {
                break;
            }

            if bh.code == codes::BLO_CODE_DNA1 {
                blocks.push(bh);
                seen_count += 1;
                break;
            }
        }
    }

    // Now get user data blocks and any remaining blocks
    for bh in bf.blocks() {
        if seen_count >= options.max_blocks_to_show {
            break;
        }

        if !should_show_block(&bh, options) {
            continue;
        }

        // Skip DNA1 if we already added it
        if bh.code == codes::BLO_CODE_DNA1 && blocks.iter().any(|b| b.code == codes::BLO_CODE_DNA1)
        {
            continue;
        }

        // Show user data blocks or system blocks if requested
        if !bh.is_system_block() || options.include_system_blocks {
            blocks.push(bh);
            seen_count += 1;
        }
    }

    blocks
}
