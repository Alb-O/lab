//! # dot001-diff - EXPERIMENTAL AND INCOMPLETE
//!
//! This crate provides basic diffing capabilities for Blender .blend files.
//!
//! ## Current Status: INCOMPLETE
//!
//! This implementation is a proof-of-concept that demonstrates content-aware diffing
//! for mesh blocks and size-based filtering for DATA blocks. However, it requires
//! significant foundational work before being production-ready:
//!
//! ### Known Limitations:
//! - Hierarchical relationships between blocks are not fully established
//! - Dependency tracing between modified blocks needs deeper analysis
//! - Many block types use simple binary comparison rather than semantic diffing
//! - Complex data structures within blocks are not properly analyzed
//! - Memory layout changes vs actual content changes need better differentiation
//!
//! ### Areas Needing Work:
//! - Enhanced block-type-specific content analysis
//! - Better understanding of Blender's internal data relationships
//! - More sophisticated heuristics for detecting real vs artificial changes
//! - Integration with DNA system for semantic field-level diffing
//! - Performance optimization for large files
//!
//! This crate serves as a foundation for future development but should not be
//! considered complete or production-ready.

use dot001_parser::BlendFile;
use serde::{Deserialize, Serialize};
use std::io::{Read, Seek};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DiffError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Blend file error: {0}")]
    BlendFile(#[from] dot001_parser::BlendError),
    #[error("Block not found: {0}")]
    BlockNotFound(usize),
}

pub type Result<T> = std::result::Result<T, DiffError>;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum BlockChangeType {
    /// Content of the block has been modified
    Modified,
    /// Block exists in both files with identical content
    Unchanged,
    /// Block exists only in the first file
    Removed,
    /// Block exists only in the second file
    Added,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockDiff {
    pub block_index: usize,
    pub block_code: String,
    pub block_name: Option<String>,
    pub change_type: BlockChangeType,
    pub size_before: Option<u32>,
    pub size_after: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BlendDiff {
    pub block_diffs: Vec<BlockDiff>,
    pub summary: DiffSummary,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DiffSummary {
    pub total_blocks: usize,
    pub modified_blocks: usize,
    pub added_blocks: usize,
    pub removed_blocks: usize,
    pub unchanged_blocks: usize,
}

/// Main diffing engine for Blender .blend files
///
/// ## EXPERIMENTAL STATUS
/// This diffing engine is a proof-of-concept implementation that demonstrates
/// content-aware comparison for mesh blocks and size-based filtering for DATA blocks.
/// It successfully reduces false positives but requires significant additional work
/// to properly handle the full complexity of Blender's data structures.
///
/// The current implementation should be considered incomplete and experimental.
pub struct BlendDiffer {
    /// Whether to perform content-aware comparison for specific block types
    pub content_aware: bool,
}

impl BlendDiffer {
    pub fn new() -> Self {
        Self {
            content_aware: true,
        }
    }

    /// Compare two blend files and return a diff
    pub fn diff<R1, R2>(
        &self,
        file1: &mut BlendFile<R1>,
        file2: &mut BlendFile<R2>,
    ) -> Result<BlendDiff>
    where
        R1: Read + Seek,
        R2: Read + Seek,
    {
        let mut block_diffs = Vec::new();

        // Get block count from both files
        let max_blocks = file1.blocks.len().max(file2.blocks.len());

        // Collect block information first to avoid borrowing conflicts
        let block_info: Vec<_> = (0..max_blocks)
            .map(|i| {
                let block1_info = file1.blocks.get(i).map(|b| (b.header.code, b.header.size));
                let block2_info = file2.blocks.get(i).map(|b| (b.header.code, b.header.size));
                (i, block1_info, block2_info)
            })
            .collect();

        for (i, block1_info, block2_info) in block_info {
            let diff = match (block1_info, block2_info) {
                (Some((code1, size1)), Some((_code2, _size2))) => {
                    let block_code = String::from_utf8_lossy(&code1)
                        .trim_end_matches('\0')
                        .to_string();
                    let block_name = self.get_block_name(i, file1);

                    let change_type = if self.content_aware {
                        self.content_aware_compare_by_index(
                            i,
                            file1,
                            file2,
                            &block_code,
                            size1,
                            _size2,
                        )?
                    } else {
                        // Simple binary comparison
                        if self.binary_compare_blocks_by_index(i, file1, file2)? {
                            BlockChangeType::Unchanged
                        } else {
                            BlockChangeType::Modified
                        }
                    };

                    BlockDiff {
                        block_index: i,
                        block_code,
                        block_name,
                        change_type,
                        size_before: Some(size1),
                        size_after: Some(_size2),
                    }
                }
                (Some((code1, size1)), None) => BlockDiff {
                    block_index: i,
                    block_code: String::from_utf8_lossy(&code1)
                        .trim_end_matches('\0')
                        .to_string(),
                    block_name: self.get_block_name(i, file1),
                    change_type: BlockChangeType::Removed,
                    size_before: Some(size1),
                    size_after: None,
                },
                (None, Some((code2, size2))) => BlockDiff {
                    block_index: i,
                    block_code: String::from_utf8_lossy(&code2)
                        .trim_end_matches('\0')
                        .to_string(),
                    block_name: self.get_block_name(i, file2),
                    change_type: BlockChangeType::Added,
                    size_before: None,
                    size_after: Some(size2),
                },
                (None, None) => unreachable!(),
            };

            block_diffs.push(diff);
        }

        let summary = self.calculate_summary(&block_diffs);

        Ok(BlendDiff {
            block_diffs,
            summary,
        })
    }

    fn content_aware_compare_by_index<R1, R2>(
        &self,
        index: usize,
        file1: &mut BlendFile<R1>,
        file2: &mut BlendFile<R2>,
        block_code: &str,
        size1: u32,
        size2: u32,
    ) -> Result<BlockChangeType>
    where
        R1: Read + Seek,
        R2: Read + Seek,
    {
        match block_code {
            "ME" => {
                // For mesh blocks, use content-aware comparison
                self.compare_mesh_blocks(index, file1, file2)
            }
            "DATA" => {
                // For DATA blocks, use size-based filtering to reduce false positives
                self.compare_data_blocks(index, file1, file2, size1, size2)
            }
            "OB" | "GR" | "NT" | "CA" => {
                // For object/group/nodetree/camera blocks, use relaxed comparison
                self.compare_structural_blocks(index, file1, file2, size1, size2)
            }
            _ => {
                // For other block types, fall back to binary comparison
                if self.binary_compare_blocks_by_index(index, file1, file2)? {
                    Ok(BlockChangeType::Unchanged)
                } else {
                    Ok(BlockChangeType::Modified)
                }
            }
        }
    }

    fn compare_mesh_blocks<R1, R2>(
        &self,
        index: usize,
        file1: &mut BlendFile<R1>,
        file2: &mut BlendFile<R2>,
    ) -> Result<BlockChangeType>
    where
        R1: Read + Seek,
        R2: Read + Seek,
    {
        // Read mesh data from both files
        let mesh1 = self.extract_mesh_content(index, file1)?;
        let mesh2 = self.extract_mesh_content(index, file2)?;

        // Optional debug output for specific mesh blocks
        // if index == 10687 {
        //     println!("DEBUG: Mesh block {} comparison:", index);
        //     println!("  Before: vertices={}, edges={}, polys={}, loops={}",
        //         mesh1.totvert, mesh1.totedge, mesh1.totpoly, mesh1.totloop);
        //     println!("  After:  vertices={}, edges={}, polys={}, loops={}",
        //         mesh2.totvert, mesh2.totedge, mesh2.totpoly, mesh2.totloop);
        // }

        if mesh1 == mesh2 {
            Ok(BlockChangeType::Unchanged)
        } else {
            Ok(BlockChangeType::Modified)
        }
    }

    fn compare_data_blocks<R1, R2>(
        &self,
        _index: usize,
        _file1: &mut BlendFile<R1>,
        _file2: &mut BlendFile<R2>,
        size1: u32,
        size2: u32,
    ) -> Result<BlockChangeType>
    where
        R1: Read + Seek,
        R2: Read + Seek,
    {
        // For DATA blocks, prioritize size changes as the primary indicator of real content changes
        if size1 != size2 {
            // Size change strongly indicates actual data modification
            Ok(BlockChangeType::Modified)
        } else {
            // Same size - most likely just pointer/metadata updates
            // Use very conservative approach to minimize false positives
            // since same-size DATA block changes are usually memory layout artifacts
            Ok(BlockChangeType::Unchanged)
        }
    }

    fn compare_structural_blocks<R1, R2>(
        &self,
        _index: usize,
        _file1: &mut BlendFile<R1>,
        _file2: &mut BlendFile<R2>,
        size1: u32,
        size2: u32,
    ) -> Result<BlockChangeType>
    where
        R1: Read + Seek,
        R2: Read + Seek,
    {
        // For structural blocks (OB, GR, NT, CA), size changes are significant
        if size1 != size2 {
            Ok(BlockChangeType::Modified)
        } else {
            // Same size - these are likely just pointer/metadata updates
            // Use a much more conservative approach to minimize false positives
            // Most same-size changes in structural blocks are just memory layout changes
            Ok(BlockChangeType::Unchanged)
        }
    }

    fn extract_mesh_content<R: Read + Seek>(
        &self,
        index: usize,
        file: &mut BlendFile<R>,
    ) -> Result<MeshContent> {
        let data = file.read_block_data(index)?;
        let reader = file.create_field_reader(&data)?;

        // Extract key mesh content fields, ignoring pointers and metadata
        let mut content = MeshContent::default();

        // Try to read basic mesh properties
        if let Ok(totvert) = reader.read_field_u32("Mesh", "totvert") {
            content.totvert = totvert;
        }

        if let Ok(totedge) = reader.read_field_u32("Mesh", "totedge") {
            content.totedge = totedge;
        }

        if let Ok(totpoly) = reader.read_field_u32("Mesh", "totpoly") {
            content.totpoly = totpoly;
        }

        if let Ok(totloop) = reader.read_field_u32("Mesh", "totloop") {
            content.totloop = totloop;
        }

        // For more robust comparison, we could extract vertex coordinates,
        // edge connectivity, polygon definitions, etc. This would require
        // following pointers to the actual vertex/edge/poly arrays and
        // reading their content while ignoring memory addresses.

        // For now, the vertex/edge/polygon counts provide a good content signature

        Ok(content)
    }

    fn binary_compare_blocks_by_index<R1, R2>(
        &self,
        index: usize,
        file1: &mut BlendFile<R1>,
        file2: &mut BlendFile<R2>,
    ) -> Result<bool>
    where
        R1: Read + Seek,
        R2: Read + Seek,
    {
        let data1 = file1.read_block_data(index)?;
        let data2 = file2.read_block_data(index)?;

        Ok(data1 == data2)
    }

    fn get_block_name<R: Read + Seek>(
        &self,
        _index: usize,
        _file: &mut BlendFile<R>,
    ) -> Option<String> {
        // Use the existing NameResolver if available
        // For now, return None - we'll integrate with the name resolver later
        None
    }

    fn calculate_summary(&self, diffs: &[BlockDiff]) -> DiffSummary {
        let mut summary = DiffSummary {
            total_blocks: diffs.len(),
            modified_blocks: 0,
            added_blocks: 0,
            removed_blocks: 0,
            unchanged_blocks: 0,
        };

        for diff in diffs {
            match diff.change_type {
                BlockChangeType::Modified => summary.modified_blocks += 1,
                BlockChangeType::Added => summary.added_blocks += 1,
                BlockChangeType::Removed => summary.removed_blocks += 1,
                BlockChangeType::Unchanged => summary.unchanged_blocks += 1,
            }
        }

        summary
    }
}

impl Default for BlendDiffer {
    fn default() -> Self {
        Self::new()
    }
}

/// Content signature for mesh blocks
#[derive(Debug, Clone, PartialEq, Default)]
struct MeshContent {
    totvert: u32, // Number of vertices
    totedge: u32, // Number of edges
    totpoly: u32, // Number of polygons
    totloop: u32, // Number of loops
                  // TODO: Add vertex coordinates, edge indices, face data, etc.
}
