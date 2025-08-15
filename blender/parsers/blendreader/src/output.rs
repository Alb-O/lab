use crate::analysis::{AnalysisOptions, FileAnalysis};
use crate::format::{BHead, Header};
use crate::reader::BlendFile;
use crate::sdna::SdnaInfo;
use serde::{Deserialize, Serialize};
use serde_json;
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize)]
pub struct BlendFileData {
    pub path: String,
    pub header: Header,
    pub analysis: FileAnalysis,
    pub blocks: Vec<BlockData>,
    pub sdna_info: Option<SdnaInfo>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BlockData {
    pub name: String,
    pub description: String,
    pub is_data_block: bool,
    pub is_system_block: bool,
    pub typical_size_range: Option<(usize, usize)>,
    pub code: u32,
    pub code_string: String,
    pub sdn_anr: i64,
    pub old_ptr: u64,
    pub len: i64,
    pub nr: i64,
    pub is_valid_size: bool,
    pub size_category: String,
}

impl From<&BHead> for BlockData {
    fn from(bh: &BHead) -> Self {
        let info = bh.block_info();
        Self {
            name: info.name.to_string(),
            description: info.description.to_string(),
            is_data_block: info.is_data_block,
            is_system_block: info.is_system_block,
            typical_size_range: info.typical_size_range,
            code: bh.code,
            code_string: bh.code_string(),
            sdn_anr: bh.sdn_anr,
            old_ptr: bh.old_ptr,
            len: bh.len,
            nr: bh.nr,
            is_valid_size: bh.is_valid_size(),
            size_category: bh.size_category().to_string(),
        }
    }
}

pub fn serialize_blend_file(
    path: &str,
    blend_file: &BlendFile,
    options: &AnalysisOptions,
) -> Result<BlendFileData, crate::Error> {
    let analysis = FileAnalysis::analyze_with_options(blend_file, options);

    let mut blocks = Vec::new();
    let mut sdna_info = None;

    // Collect interesting blocks based on options
    let interesting_blocks = crate::analysis::get_interesting_blocks(blend_file, options);
    for bh in interesting_blocks {
        blocks.push(BlockData::from(&bh));

        // Extract SDNA info if this is a DNA1 block
        if bh.code == crate::format::codes::BLO_CODE_DNA1 {
            if let Ok(info) = blend_file.read_dna_block(&bh) {
                sdna_info = Some(info);
            }
        }
    }

    Ok(BlendFileData {
        path: path.to_string(),
        header: blend_file.header.clone(),
        analysis,
        blocks,
        sdna_info,
    })
}

pub fn serialize_to_json(data: &BlendFileData) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(data)
}

pub fn serialize_to_json_compact(data: &BlendFileData) -> Result<String, serde_json::Error> {
    serde_json::to_string(data)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MultiFileData {
    pub files: Vec<BlendFileData>,
    pub summary: MultiFileSummary,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MultiFileSummary {
    pub total_files: usize,
    pub total_blocks: usize,
    pub total_size: usize,
    pub blender_versions: HashMap<u32, usize>,
    pub most_common_blocks: HashMap<String, usize>,
}

pub fn serialize_multiple_files(files_data: Vec<BlendFileData>) -> MultiFileData {
    let mut summary = MultiFileSummary {
        total_files: files_data.len(),
        total_blocks: 0,
        total_size: 0,
        blender_versions: HashMap::new(),
        most_common_blocks: HashMap::new(),
    };

    for file_data in &files_data {
        summary.total_blocks += file_data.analysis.total_blocks;
        summary.total_size += file_data.analysis.total_size;

        // Count Blender versions
        *summary
            .blender_versions
            .entry(file_data.header.file_version)
            .or_insert(0) += 1;

        // Count block types
        for (block_type, stats) in &file_data.analysis.block_type_stats {
            *summary
                .most_common_blocks
                .entry(block_type.clone())
                .or_insert(0) += stats.count;
        }
    }

    MultiFileData {
        files: files_data,
        summary,
    }
}
