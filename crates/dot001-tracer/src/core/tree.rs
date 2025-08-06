/// Hierarchical dependency tree structures

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Represents a block in the dependency tree
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct DependencyNode {
    /// Block index in the blend file
    pub block_index: usize,
    /// Block type code (e.g., "OB", "ME", "MA")
    pub block_code: String,
    /// Block size in bytes
    pub block_size: u32,
    /// Block address
    pub block_address: u64,
    /// Child dependencies
    pub children: Vec<DependencyNode>,
}

/// Hierarchical dependency tree
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct DependencyTree {
    /// Root node representing the starting block
    pub root: DependencyNode,
    /// Total number of dependencies found
    pub total_dependencies: usize,
    /// Maximum depth of the tree
    pub max_depth: usize,
}
