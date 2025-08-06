/// Policy-based architecture for diff operations
///
/// This module provides traits and policies for modular, extensible diffing
/// of different block types and content analysis strategies.
use crate::{BlockChangeType, Result};
use dot001_parser::BlendFile;
use std::io::{Read, Seek};

/// Policy for determining how to compare blocks of a specific type
pub trait BlockDiffPolicy<R1: Read + Seek, R2: Read + Seek> {
    /// Check if this policy can handle the given block code
    fn can_handle(&self, block_code: &[u8; 4]) -> bool;

    /// Compare two blocks and determine the change type
    fn compare_blocks(
        &self,
        block_index1: usize,
        file1: &mut BlendFile<R1>,
        block_index2: usize,
        file2: &mut BlendFile<R2>,
    ) -> Result<BlockChangeType>;

    /// Get additional metadata for the diff result
    fn get_metadata(
        &self,
        _block_index1: Option<usize>,
        _file1: Option<&mut BlendFile<R1>>,
        _block_index2: Option<usize>,
        _file2: Option<&mut BlendFile<R2>>,
    ) -> Result<Option<String>> {
        Ok(None)
    }
}

/// Simple binary comparison policy for generic blocks
pub struct BinaryDiffPolicy;

impl<R1: Read + Seek, R2: Read + Seek> BlockDiffPolicy<R1, R2> for BinaryDiffPolicy {
    fn can_handle(&self, _block_code: &[u8; 4]) -> bool {
        true // Default fallback for all block types
    }

    fn compare_blocks(
        &self,
        block_index1: usize,
        file1: &mut BlendFile<R1>,
        block_index2: usize,
        file2: &mut BlendFile<R2>,
    ) -> Result<BlockChangeType> {
        let data1 = file1.read_block_data(block_index1)?;
        let data2 = file2.read_block_data(block_index2)?;

        if data1 == data2 {
            Ok(BlockChangeType::Unchanged)
        } else {
            Ok(BlockChangeType::Modified)
        }
    }
}

/// Size-based comparison policy for DATA blocks
pub struct SizeBasedDiffPolicy {
    /// Minimum size difference threshold to consider as changed
    pub size_threshold: u32,
}

impl SizeBasedDiffPolicy {
    pub fn new(size_threshold: u32) -> Self {
        Self { size_threshold }
    }
}

impl<R1: Read + Seek, R2: Read + Seek> BlockDiffPolicy<R1, R2> for SizeBasedDiffPolicy {
    fn can_handle(&self, block_code: &[u8; 4]) -> bool {
        block_code == b"DATA"
    }

    fn compare_blocks(
        &self,
        block_index1: usize,
        file1: &mut BlendFile<R1>,
        block_index2: usize,
        file2: &mut BlendFile<R2>,
    ) -> Result<BlockChangeType> {
        let size1 = file1
            .get_block(block_index1)
            .map(|b| b.header.size)
            .unwrap_or(0);
        let size2 = file2
            .get_block(block_index2)
            .map(|b| b.header.size)
            .unwrap_or(0);

        if size1.abs_diff(size2) < self.size_threshold {
            Ok(BlockChangeType::Unchanged)
        } else {
            Ok(BlockChangeType::Modified)
        }
    }

    fn get_metadata(
        &self,
        block_index1: Option<usize>,
        file1: Option<&mut BlendFile<R1>>,
        block_index2: Option<usize>,
        file2: Option<&mut BlendFile<R2>>,
    ) -> Result<Option<String>> {
        let size1 = if let (Some(idx), Some(file)) = (block_index1, file1) {
            file.get_block(idx).map(|b| b.header.size).unwrap_or(0)
        } else {
            0
        };

        let size2 = if let (Some(idx), Some(file)) = (block_index2, file2) {
            file.get_block(idx).map(|b| b.header.size).unwrap_or(0)
        } else {
            0
        };

        Ok(Some(format!("size_change: {size1} -> {size2}")))
    }
}

/// Content-aware comparison policy for mesh blocks
pub struct MeshContentDiffPolicy;

impl<R1: Read + Seek, R2: Read + Seek> BlockDiffPolicy<R1, R2> for MeshContentDiffPolicy {
    fn can_handle(&self, block_code: &[u8; 4]) -> bool {
        block_code == b"ME\0\0"
    }

    fn compare_blocks(
        &self,
        block_index1: usize,
        file1: &mut BlendFile<R1>,
        block_index2: usize,
        file2: &mut BlendFile<R2>,
    ) -> Result<BlockChangeType> {
        // Simplified mesh comparison - in reality this would analyze mesh structure
        let hash1 = file1.block_content_hash(block_index1)?;
        let hash2 = file2.block_content_hash(block_index2)?;

        if hash1 == hash2 {
            Ok(BlockChangeType::Unchanged)
        } else {
            Ok(BlockChangeType::Modified)
        }
    }

    fn get_metadata(
        &self,
        _block_index1: Option<usize>,
        _file1: Option<&mut BlendFile<R1>>,
        _block_index2: Option<usize>,
        _file2: Option<&mut BlendFile<R2>>,
    ) -> Result<Option<String>> {
        Ok(Some("mesh_content_analysis".to_string()))
    }
}

/// Registry for managing multiple diff policies
pub struct PolicyRegistry<R1: Read + Seek, R2: Read + Seek> {
    policies: Vec<Box<dyn BlockDiffPolicy<R1, R2>>>,
}

impl<R1: Read + Seek, R2: Read + Seek> Default for PolicyRegistry<R1, R2> {
    fn default() -> Self {
        Self::new()
    }
}

impl<R1: Read + Seek, R2: Read + Seek> PolicyRegistry<R1, R2> {
    pub fn new() -> Self {
        Self {
            policies: Vec::new(),
        }
    }

    /// Add a policy to the registry
    pub fn register_policy(&mut self, policy: Box<dyn BlockDiffPolicy<R1, R2>>) {
        self.policies.push(policy);
    }

    /// Create a registry with default policies
    pub fn with_default_policies() -> Self {
        let mut registry = Self::new();

        // Register policies in order of specificity (most specific first)
        registry.register_policy(Box::new(MeshContentDiffPolicy));
        registry.register_policy(Box::new(SizeBasedDiffPolicy::new(1024))); // 1KB threshold
        registry.register_policy(Box::new(BinaryDiffPolicy)); // Fallback

        registry
    }

    /// Find the first policy that can handle the given block code
    pub fn find_policy(&self, block_code: &[u8; 4]) -> Option<&dyn BlockDiffPolicy<R1, R2>> {
        self.policies
            .iter()
            .find(|policy| policy.can_handle(block_code))
            .map(|p| p.as_ref())
    }

    /// Apply the appropriate policy to compare two blocks
    pub fn compare_blocks(
        &self,
        block_code: &[u8; 4],
        block_index1: usize,
        file1: &mut BlendFile<R1>,
        block_index2: usize,
        file2: &mut BlendFile<R2>,
    ) -> Result<BlockChangeType> {
        if let Some(policy) = self.find_policy(block_code) {
            policy.compare_blocks(block_index1, file1, block_index2, file2)
        } else {
            // Fallback to binary comparison
            let binary_policy = BinaryDiffPolicy;
            binary_policy.compare_blocks(block_index1, file1, block_index2, file2)
        }
    }
}
