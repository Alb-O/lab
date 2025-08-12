use crate::format::{BHead, codes};
use crate::reader::BlendFile;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct AnalysisOptions {
    pub include_system_blocks: bool,
    pub max_blocks_to_show: usize,
    pub show_invalid_blocks: bool,
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

#[derive(Debug, Clone)]
pub struct BlockStats {
    pub count: usize,
    pub total_size: usize,
    pub avg_size: usize,
    pub min_size: usize,
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

#[derive(Debug)]
pub struct FileAnalysis {
    pub total_blocks: usize,
    pub data_blocks: usize,
    pub meta_blocks: usize,
    pub total_size: usize,
    pub block_type_stats: HashMap<String, BlockStats>,
    pub invalid_blocks: Vec<String>,
    pub warnings: Vec<String>,
}

impl FileAnalysis {
    pub fn analyze(bf: &BlendFile) -> Self {
        Self::analyze_with_options(bf, &AnalysisOptions::default())
    }

    pub fn analyze_with_options(bf: &BlendFile, options: &AnalysisOptions) -> Self {
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

    pub fn print_summary(&self) {
        self.print_summary_with_options(&AnalysisOptions::default())
    }

    pub fn print_summary_with_options(&self, options: &AnalysisOptions) {
        println!("=== Block Analysis Summary ===");
        println!("Total blocks: {}", self.total_blocks);
        println!(
            "Data blocks: {} | Meta blocks: {}",
            self.data_blocks, self.meta_blocks
        );
        println!(
            "Total size: {} bytes ({:.2} KB)",
            self.total_size,
            self.total_size as f64 / 1024.0
        );

        if options.show_warnings && !self.warnings.is_empty() {
            println!("\n=== Warnings ===");
            for warning in &self.warnings {
                println!("⚠️  {warning}");
            }
        }

        if options.show_invalid_blocks && !self.invalid_blocks.is_empty() {
            println!("\n=== Invalid Block Sizes ===");
            for invalid in &self.invalid_blocks {
                println!("❌ {invalid}");
            }
        }
    }

    pub fn print_detailed(&self) {
        self.print_detailed_with_options(&AnalysisOptions::default())
    }

    pub fn print_detailed_with_options(&self, options: &AnalysisOptions) {
        self.print_summary_with_options(options);

        println!("\n=== Block Type Statistics ===");
        let mut types: Vec<_> = self.block_type_stats.iter().collect();
        types.sort_by(|a, b| b.1.total_size.cmp(&a.1.total_size));

        let mut system_blocks_count = 0;
        let mut system_blocks_size = 0;

        for (block_type, stats) in types {
            // Check if this is a system block type
            let is_system = matches!(
                block_type.as_str(),
                "DATA"
                    | "GLOB"
                    | "DNA1"
                    | "REND"
                    | "USER"
                    | "ENDB"
                    | "WindowManager"
                    | "Screen"
                    | "TEST"
            );

            if !options.include_system_blocks && is_system {
                system_blocks_count += stats.count;
                system_blocks_size += stats.total_size;
                continue;
            }

            println!(
                "{:15} | Count: {:4} | Total: {:8} bytes | Avg: {:6} | Min: {:6} | Max: {:8}",
                block_type,
                stats.count,
                stats.total_size,
                stats.avg_size,
                stats.min_size,
                stats.max_size
            );
        }

        if !options.include_system_blocks && system_blocks_count > 0 {
            println!(
                "System (filtered)| Count: {system_blocks_count:4} | Total: {system_blocks_size:8} bytes (use --include-system to show details)"
            );
        }
    }
}

pub fn detailed_block_info(bh: &BHead) -> String {
    detailed_block_info_with_options(bh, &AnalysisOptions::default())
}

pub fn detailed_block_info_with_options(bh: &BHead, _options: &AnalysisOptions) -> String {
    let info = bh.block_info();
    let block_type = if info.is_system_block {
        "System"
    } else {
        "User Data"
    };

    format!(
        "Block: {} ({}) - {}\n  Size: {} bytes ({}) | Type: {}\n  SDN: {} | OldPtr: 0x{:X} | Nr: {}\n  Valid: {}",
        info.name,
        bh.code_string(),
        info.description,
        bh.len,
        bh.size_category(),
        block_type,
        bh.sdn_anr,
        bh.old_ptr,
        bh.nr,
        if bh.is_valid_size() { "✓" } else { "❌" }
    )
}

pub fn should_show_block(bh: &BHead, options: &AnalysisOptions) -> bool {
    if !options.include_system_blocks && bh.is_system_block() {
        return false;
    }
    true
}

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

        // Show user data blocks (actual blend file content)
        if !bh.is_system_block() {
            blocks.push(bh);
            seen_count += 1;
        }
        // Show system blocks if requested
        else if options.include_system_blocks {
            blocks.push(bh);
            seen_count += 1;
        }
    }

    blocks
}
