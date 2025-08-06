/// Trait-based diff engine architecture
///
/// This module provides a trait-based approach for implementing different
/// diffing strategies, allowing for pluggable comparison algorithms.
use crate::{BlendDiff, BlockChangeType, BlockDiff, DiffSummary, Result, policies::PolicyRegistry};
use dot001_parser::BlendFile;
use std::collections::HashMap;
use std::io::{Read, Seek};

/// Trait for diffing engines that compare blend files
pub trait DiffEngine {
    type Reader1: Read + Seek;
    type Reader2: Read + Seek;

    /// Compare two blend files and produce a diff
    fn diff(
        &self,
        file1: &mut BlendFile<Self::Reader1>,
        file2: &mut BlendFile<Self::Reader2>,
    ) -> Result<BlendDiff>;

    /// Get a summary of changes without full block analysis
    fn summary(
        &self,
        file1: &mut BlendFile<Self::Reader1>,
        file2: &mut BlendFile<Self::Reader2>,
    ) -> Result<DiffSummary>;
}

/// Policy-based diff engine that uses pluggable comparison policies
pub struct PolicyDiffEngine<R1: Read + Seek, R2: Read + Seek> {
    policy_registry: PolicyRegistry<R1, R2>,
    include_unchanged: bool,
}

impl<R1: Read + Seek, R2: Read + Seek> PolicyDiffEngine<R1, R2> {
    pub fn new(policy_registry: PolicyRegistry<R1, R2>) -> Self {
        Self {
            policy_registry,
            include_unchanged: false,
        }
    }

    pub fn with_default_policies() -> Self {
        Self::new(PolicyRegistry::with_default_policies())
    }

    pub fn include_unchanged(mut self, include: bool) -> Self {
        self.include_unchanged = include;
        self
    }

    /// Create block mappings between two files based on address or position
    fn create_block_mappings(
        &self,
        file1: &BlendFile<R1>,
        file2: &BlendFile<R2>,
    ) -> (HashMap<usize, usize>, Vec<usize>, Vec<usize>) {
        let mut mappings = HashMap::new();
        let mut only_in_first = Vec::new();
        let mut only_in_second = Vec::new();

        // Simple mapping by position - in reality this would be more sophisticated
        let len1 = file1.blocks_len();
        let len2 = file2.blocks_len();
        let min_len = len1.min(len2);

        // Map blocks by position for now
        for i in 0..min_len {
            if let (Some(block1), Some(block2)) = (file1.get_block(i), file2.get_block(i)) {
                // Only map if block codes match
                if block1.header.code == block2.header.code {
                    mappings.insert(i, i);
                }
            }
        }

        // Collect blocks that exist only in one file
        for i in min_len..len1 {
            only_in_first.push(i);
        }
        for i in min_len..len2 {
            only_in_second.push(i);
        }

        (mappings, only_in_first, only_in_second)
    }
}

impl<R1: Read + Seek, R2: Read + Seek> DiffEngine for PolicyDiffEngine<R1, R2> {
    type Reader1 = R1;
    type Reader2 = R2;

    fn diff(
        &self,
        file1: &mut BlendFile<Self::Reader1>,
        file2: &mut BlendFile<Self::Reader2>,
    ) -> Result<BlendDiff> {
        let (mappings, only_in_first, only_in_second) = self.create_block_mappings(file1, file2);

        let mut block_diffs = Vec::new();
        let mut modified_count = 0;
        let mut unchanged_count = 0;

        // Compare mapped blocks
        for (index1, index2) in mappings {
            // Extract block info first to avoid borrowing conflicts
            let (block_code, block_code_str, size1, size2) = {
                if let (Some(block1), Some(block2)) =
                    (file1.get_block(index1), file2.get_block(index2))
                {
                    let code = block1.header.code;
                    let code_str = String::from_utf8_lossy(&code)
                        .trim_end_matches('\0')
                        .to_string();
                    (code, code_str, block1.header.size, block2.header.size)
                } else {
                    continue;
                }
            };

            let change_type =
                self.policy_registry
                    .compare_blocks(&block_code, index1, file1, index2, file2)?;

            match change_type {
                BlockChangeType::Modified => modified_count += 1,
                BlockChangeType::Unchanged => unchanged_count += 1,
                _ => {} // Should not happen for mapped blocks
            }

            if change_type != BlockChangeType::Unchanged || self.include_unchanged {
                block_diffs.push(BlockDiff {
                    block_index: index1,
                    block_code: block_code_str,
                    block_name: None, // Could use NameResolver here
                    change_type,
                    size_before: Some(size1),
                    size_after: Some(size2),
                });
            }
        }

        // Get lengths before consuming the vectors
        let added_count = only_in_second.len();
        let removed_count = only_in_first.len();

        // Handle blocks only in first file (removed)
        for index in &only_in_first {
            if let Some(block) = file1.get_block(*index) {
                let block_code_str = dot001_parser::block_code_to_string(block.header.code);

                block_diffs.push(BlockDiff {
                    block_index: *index,
                    block_code: block_code_str,
                    block_name: None,
                    change_type: BlockChangeType::Removed,
                    size_before: Some(block.header.size),
                    size_after: None,
                });
            }
        }

        // Handle blocks only in second file (added)
        for index in &only_in_second {
            if let Some(block) = file2.get_block(*index) {
                let block_code_str = dot001_parser::block_code_to_string(block.header.code);

                block_diffs.push(BlockDiff {
                    block_index: *index,
                    block_code: block_code_str,
                    block_name: None,
                    change_type: BlockChangeType::Added,
                    size_before: None,
                    size_after: Some(block.header.size),
                });
            }
        }

        let summary = DiffSummary {
            total_blocks: file1.blocks_len().max(file2.blocks_len()),
            modified_blocks: modified_count,
            added_blocks: added_count,
            removed_blocks: removed_count,
            unchanged_blocks: unchanged_count,
        };

        Ok(BlendDiff {
            block_diffs,
            summary,
        })
    }

    fn summary(
        &self,
        file1: &mut BlendFile<Self::Reader1>,
        file2: &mut BlendFile<Self::Reader2>,
    ) -> Result<DiffSummary> {
        let (mappings, only_in_first, only_in_second) = self.create_block_mappings(file1, file2);

        let mut modified_count = 0;
        let mut unchanged_count = 0;

        // Get lengths before consuming the vectors
        let added_count = only_in_second.len();
        let removed_count = only_in_first.len();

        // Quick comparison of mapped blocks
        for (index1, index2) in mappings {
            if let (Some(block1), Some(_block2)) =
                (file1.get_block(index1), file2.get_block(index2))
            {
                let block_code = block1.header.code;

                let change_type = self.policy_registry.compare_blocks(
                    &block_code,
                    index1,
                    file1,
                    index2,
                    file2,
                )?;

                match change_type {
                    BlockChangeType::Modified => modified_count += 1,
                    BlockChangeType::Unchanged => unchanged_count += 1,
                    _ => {}
                }
            }
        }

        Ok(DiffSummary {
            total_blocks: file1.blocks_len().max(file2.blocks_len()),
            modified_blocks: modified_count,
            added_blocks: added_count,
            removed_blocks: removed_count,
            unchanged_blocks: unchanged_count,
        })
    }
}
