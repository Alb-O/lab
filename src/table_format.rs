use crate::analysis::{AnalysisOptions, BlockStats, FileAnalysis};
use crate::format::{BHead, Header};
use crate::reader::BlendFile;
use comfy_table::modifiers::UTF8_ROUND_CORNERS;
use comfy_table::presets::UTF8_FULL;
use comfy_table::{Attribute, Cell, CellAlignment, Color, ContentArrangement, Table};
use std::collections::HashMap;

pub fn format_file_header(file_path: &str, header: &Header) -> String {
    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .apply_modifier(UTF8_ROUND_CORNERS)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_header(vec![
            Cell::new("File Information").add_attribute(Attribute::Bold),
        ]);

    table.add_row(vec![format!("File: {}", file_path)]);
    table.add_row(vec![format!("Pointer Size: {} bytes", header.pointer_size)]);
    table.add_row(vec![format!("Endianness: {:?}", header.endian)]);
    table.add_row(vec![format!(
        "Blender Version: {}",
        format_blender_version(header.file_version)
    )]);
    table.add_row(vec![format!(
        "Format Version: {}",
        header.file_format_version
    )]);

    table.to_string()
}

pub fn format_file_analysis(analysis: &FileAnalysis) -> String {
    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .apply_modifier(UTF8_ROUND_CORNERS)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_header(vec![
            Cell::new("File Analysis").add_attribute(Attribute::Bold),
        ]);

    table.add_row(vec![format!("Total Blocks: {}", analysis.total_blocks)]);
    table.add_row(vec![format!("Data Blocks: {}", analysis.data_blocks)]);
    table.add_row(vec![format!("Meta Blocks: {}", analysis.meta_blocks)]);
    table.add_row(vec![format!(
        "Total Size: {} bytes ({:.1} KB)",
        analysis.total_size,
        analysis.total_size as f64 / 1024.0
    )]);

    table.to_string()
}

pub fn format_warnings(warnings: &[String]) -> String {
    if warnings.is_empty() {
        return String::new();
    }

    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .apply_modifier(UTF8_ROUND_CORNERS)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_header(vec![
            Cell::new("Warnings")
                .add_attribute(Attribute::Bold)
                .fg(Color::Yellow),
        ]);

    for warning in warnings {
        table.add_row(vec![Cell::new(warning).fg(Color::Yellow)]);
    }

    table.to_string()
}

pub fn format_blocks_table(blocks: &[BHead], bf: &BlendFile, _options: &AnalysisOptions) -> String {
    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .apply_modifier(UTF8_ROUND_CORNERS)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_header(vec![
            Cell::new("Type").add_attribute(Attribute::Bold),
            Cell::new("Code").add_attribute(Attribute::Bold),
            Cell::new("Size").add_attribute(Attribute::Bold),
            Cell::new("Category").add_attribute(Attribute::Bold),
            Cell::new("Description").add_attribute(Attribute::Bold),
        ]);

    // Set column alignments
    table
        .column_mut(1)
        .expect("Column 1 exists")
        .set_cell_alignment(CellAlignment::Center); // Code
    table
        .column_mut(2)
        .expect("Column 2 exists")
        .set_cell_alignment(CellAlignment::Right); // Size
    table
        .column_mut(3)
        .expect("Column 3 exists")
        .set_cell_alignment(CellAlignment::Center); // Category

    for bh in blocks {
        let info = bh.block_info();
        let size_str = if bh.len >= 1024 {
            format!("{:.1} KB", bh.len as f64 / 1024.0)
        } else {
            format!("{} B", bh.len)
        };

        table.add_row(vec![
            Cell::new(info.name),
            Cell::new(bh.code_string()),
            Cell::new(size_str),
            Cell::new(bh.size_category()),
            Cell::new(info.description),
        ]);

        // Add SDNA info if this is a DNA1 block
        if bh.code == crate::format::codes::BLO_CODE_DNA1 {
            match bf.read_dna_block(bh) {
                Ok(sdna_info) => {
                    table.add_row(vec![
                        Cell::new("  +-- SDNA Info").fg(Color::Cyan),
                        Cell::new(""),
                        Cell::new(""),
                        Cell::new(""),
                        Cell::new(format!(
                            "Names: {}, Types: {}, Structs: {}",
                            sdna_info.names_len, sdna_info.types_len, sdna_info.structs_len
                        ))
                        .fg(Color::Cyan),
                    ]);
                }
                Err(e) => {
                    table.add_row(vec![
                        Cell::new("  +-- SDNA Error").fg(Color::Red),
                        Cell::new(""),
                        Cell::new(""),
                        Cell::new(""),
                        Cell::new(format!("Error: {e}")).fg(Color::Red),
                    ]);
                }
            }
        }
    }

    table.to_string()
}

pub fn format_block_type_summary(
    block_type_stats: &HashMap<String, BlockStats>,
    options: &AnalysisOptions,
) -> String {
    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .apply_modifier(UTF8_ROUND_CORNERS)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_header(vec![
            Cell::new("Block Type").add_attribute(Attribute::Bold),
            Cell::new("Count").add_attribute(Attribute::Bold),
            Cell::new("Total Size").add_attribute(Attribute::Bold),
            Cell::new("Avg Size").add_attribute(Attribute::Bold),
            Cell::new("Min Size").add_attribute(Attribute::Bold),
            Cell::new("Max Size").add_attribute(Attribute::Bold),
        ]);

    // Set numeric columns to right alignment
    table
        .column_mut(1)
        .expect("Column 1 exists")
        .set_cell_alignment(CellAlignment::Right);
    table
        .column_mut(2)
        .expect("Column 2 exists")
        .set_cell_alignment(CellAlignment::Right);
    table
        .column_mut(3)
        .expect("Column 3 exists")
        .set_cell_alignment(CellAlignment::Right);
    table
        .column_mut(4)
        .expect("Column 4 exists")
        .set_cell_alignment(CellAlignment::Right);
    table
        .column_mut(5)
        .expect("Column 5 exists")
        .set_cell_alignment(CellAlignment::Right);

    let mut types: Vec<_> = block_type_stats.iter().collect();
    types.sort_by(|a, b| b.1.count.cmp(&a.1.count));

    let mut system_blocks_count = 0;
    let mut system_blocks_size = 0;
    let system_blocks_avg;
    let mut system_blocks_min = usize::MAX;
    let mut system_blocks_max = 0;

    for (block_type, stats) in types.iter().take(10) {
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
            system_blocks_min = system_blocks_min.min(stats.min_size);
            system_blocks_max = system_blocks_max.max(stats.max_size);
        } else {
            table.add_row(vec![
                Cell::new(block_type),
                Cell::new(stats.count.to_string()),
                Cell::new(format_bytes(stats.total_size)),
                Cell::new(format_bytes(stats.avg_size)),
                Cell::new(format_bytes(stats.min_size)),
                Cell::new(format_bytes(stats.max_size)),
            ]);
        }
    }

    if !options.include_system_blocks && system_blocks_count > 0 {
        system_blocks_avg = system_blocks_size / system_blocks_count;
        table.add_row(vec![
            Cell::new("System").fg(Color::DarkGrey),
            Cell::new(system_blocks_count.to_string()).fg(Color::DarkGrey),
            Cell::new(format_bytes(system_blocks_size)).fg(Color::DarkGrey),
            Cell::new(format_bytes(system_blocks_avg)).fg(Color::DarkGrey),
            Cell::new(format_bytes(system_blocks_min)).fg(Color::DarkGrey),
            Cell::new(format_bytes(system_blocks_max)).fg(Color::DarkGrey),
        ]);
    }

    table.to_string()
}

fn format_bytes(bytes: usize) -> String {
    if bytes >= 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else if bytes >= 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{bytes} B")
    }
}

fn format_blender_version(version: u32) -> String {
    if version >= 1000 {
        // New format: 4-digit version like 4050 -> 4.05
        let major = version / 100;
        let minor = version % 100;
        if minor == 0 {
            format!("{major}.00")
        } else {
            format!("{major}.{minor:02}")
        }
    } else {
        // Legacy format: 3-digit version like 305 -> 3.05
        let major = version / 100;
        let minor = version % 100;
        if minor == 0 {
            format!("{major}.00")
        } else {
            format!("{major}.{minor:02}")
        }
    }
}

pub fn format_section_header(title: &str) -> String {
    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .apply_modifier(UTF8_ROUND_CORNERS)
        .set_content_arrangement(ContentArrangement::Dynamic);

    table.add_row(vec![
        Cell::new(title)
            .add_attribute(Attribute::Bold)
            .fg(Color::Blue),
    ]);

    table.to_string()
}
