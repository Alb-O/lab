//! High-performance indexing with AHashMap for fast lookups
//!
//! This module provides optimized index building for block lookup operations,
//! using AHashMap for better performance compared to std::HashMap.

use crate::BlendFileBlock;
use ahash::AHashMap;

/// Block type index using AHashMap for O(1) average lookups
pub type BlockIndex = AHashMap<[u8; 4], Vec<usize>>;

/// Address to block index mapping using AHashMap for O(1) average lookups  
pub type AddressIndex = AHashMap<u64, usize>;

/// Build a block type index from a slice of blocks
///
/// Maps block codes (like b"MESH", b"OB  ") to vectors of block indices.
/// Uses AHashMap for improved performance over standard HashMap.
pub fn build_block_index(blocks: &[BlendFileBlock]) -> BlockIndex {
    const INITIAL_BLOCK_INDEX_CAPACITY: usize = 32;
    let mut index = BlockIndex::with_capacity(INITIAL_BLOCK_INDEX_CAPACITY);

    for (i, block) in blocks.iter().enumerate() {
        index.entry(block.header.code).or_default().push(i);
    }

    // Shrink to fit to minimize memory usage after building
    index.shrink_to_fit();
    index
}

/// Build an address index from a slice of blocks
///
/// Maps old_address values to block indices for pointer resolution.
/// Uses AHashMap for improved performance over standard HashMap.
pub fn build_address_index(blocks: &[BlendFileBlock]) -> AddressIndex {
    let mut index = AddressIndex::with_capacity(blocks.len());

    for (i, block) in blocks.iter().enumerate() {
        index.insert(block.header.old_address, i);
    }

    index.shrink_to_fit();
    index
}

/// Build both indices in parallel when rayon feature is enabled
///
/// This function can build both indices concurrently for improved performance
/// on multi-core systems when processing large numbers of blocks.
#[cfg(feature = "rayon")]
pub fn build_indices_parallel(blocks: &[BlendFileBlock]) -> (BlockIndex, AddressIndex) {
    use rayon::prelude::*;

    // Use rayon's join to build both indices in parallel
    let (block_index, address_index) =
        rayon::join(|| build_block_index(blocks), || build_address_index(blocks));

    (block_index, address_index)
}

/// Build both indices sequentially (fallback or when rayon not available)
pub fn build_indices_sequential(blocks: &[BlendFileBlock]) -> (BlockIndex, AddressIndex) {
    let block_index = build_block_index(blocks);
    let address_index = build_address_index(blocks);
    (block_index, address_index)
}

/// Convenience function that chooses parallel or sequential based on feature flags
pub fn build_indices(blocks: &[BlendFileBlock]) -> (BlockIndex, AddressIndex) {
    #[cfg(feature = "rayon")]
    {
        build_indices_parallel(blocks)
    }
    #[cfg(not(feature = "rayon"))]
    {
        build_indices_sequential(blocks)
    }
}

/// Helper trait for fast block type lookups
pub trait BlockIndexExt {
    /// Get all block indices of a specific type
    fn blocks_by_type(&self, block_type: &[u8; 4]) -> Vec<usize>;

    /// Check if any blocks of the given type exist
    fn has_blocks_of_type(&self, block_type: &[u8; 4]) -> bool;

    /// Get count of blocks of a specific type
    fn count_blocks_of_type(&self, block_type: &[u8; 4]) -> usize;
}

impl BlockIndexExt for BlockIndex {
    fn blocks_by_type(&self, block_type: &[u8; 4]) -> Vec<usize> {
        self.get(block_type).cloned().unwrap_or_default()
    }

    fn has_blocks_of_type(&self, block_type: &[u8; 4]) -> bool {
        self.get(block_type).is_some_and(|v| !v.is_empty())
    }

    fn count_blocks_of_type(&self, block_type: &[u8; 4]) -> usize {
        self.get(block_type).map_or(0, |v| v.len())
    }
}

/// Helper trait for address index lookups
pub trait AddressIndexExt {
    /// Find block index by address
    fn find_block_by_address(&self, address: u64) -> Option<usize>;
}

impl AddressIndexExt for AddressIndex {
    fn find_block_by_address(&self, address: u64) -> Option<usize> {
        self.get(&address).copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{BlendFileBlock, BlockHeader};

    fn create_test_block(code: [u8; 4], address: u64) -> BlendFileBlock {
        BlendFileBlock {
            header: BlockHeader {
                code,
                size: 100,
                old_address: address,
                sdna_index: 1,
                count: 1,
            },
            data_offset: 0,
            header_offset: 0,
        }
    }

    #[test]
    fn test_block_index_building() {
        let blocks = vec![
            create_test_block(*b"MESH", 0x1000),
            create_test_block(*b"OB  ", 0x2000),
            create_test_block(*b"MESH", 0x3000),
        ];

        let index = build_block_index(&blocks);

        assert_eq!(index.blocks_by_type(b"MESH"), vec![0, 2]);
        assert_eq!(index.blocks_by_type(b"OB  "), vec![1]);
        assert_eq!(index.blocks_by_type(b"XXXX"), Vec::<usize>::new());

        assert!(index.has_blocks_of_type(b"MESH"));
        assert!(index.has_blocks_of_type(b"OB  "));
        assert!(!index.has_blocks_of_type(b"XXXX"));

        assert_eq!(index.count_blocks_of_type(b"MESH"), 2);
        assert_eq!(index.count_blocks_of_type(b"OB  "), 1);
        assert_eq!(index.count_blocks_of_type(b"XXXX"), 0);
    }

    #[test]
    fn test_address_index_building() {
        let blocks = vec![
            create_test_block(*b"MESH", 0x1000),
            create_test_block(*b"OB  ", 0x2000),
            create_test_block(*b"MESH", 0x3000),
        ];

        let index = build_address_index(&blocks);

        assert_eq!(index.find_block_by_address(0x1000), Some(0));
        assert_eq!(index.find_block_by_address(0x2000), Some(1));
        assert_eq!(index.find_block_by_address(0x3000), Some(2));
        assert_eq!(index.find_block_by_address(0x4000), None);
    }

    #[test]
    fn test_build_indices() {
        let blocks = vec![
            create_test_block(*b"MESH", 0x1000),
            create_test_block(*b"OB  ", 0x2000),
        ];

        let (block_index, address_index) = build_indices(&blocks);

        assert_eq!(block_index.blocks_by_type(b"MESH"), vec![0]);
        assert_eq!(address_index.find_block_by_address(0x1000), Some(0));
    }
}
