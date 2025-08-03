// bllink-tracer/src/lib.rs

//! # bllink-tracer
//!
//! Dependency tracing engine for Blender .blend files.
//!
//! This crate provides the core dependency tracing functionality with support for
//! sophisticated traversal patterns including linked lists and array dereferencing.
//!
//! ## Key Features
//!
//! - **Dynamic data access**: Block expanders can read additional data on-demand
//! - **Material array dereferencing**: Properly handles objects with multiple materials
//! - **Cross-version compatibility**: Works with Blender 2.79 through 5.0+
//! - **Extensible architecture**: Easy to add new block expanders
//!
//! ## Example
//!
//! ```rust,no_run
//! use bllink_tracer::{BlendFile, DependencyTracer, ObjectExpander};
//! use std::fs::File;
//! use std::io::BufReader;
//!
//! let file = File::open("scene.blend")?;
//! let mut reader = BufReader::new(file);
//! let mut blend_file = BlendFile::new(&mut reader)?;
//!
//! let mut tracer = DependencyTracer::new();
//! tracer.register_expander(*b"OB\0\0", Box::new(ObjectExpander));
//!
//! let deps = tracer.trace_dependencies(object_block_index, &mut blend_file)?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

pub mod expanders;

pub use bllink_parser::{BlendFile, Result};
pub use expanders::{
    CacheFileExpander, CollectionExpander, ImageExpander, LampExpander, LibraryExpander,
    MaterialExpander, MeshExpander, NodeTreeExpander, ObjectExpander, SceneExpander, SoundExpander,
    TextureExpander,
};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
use std::io::{Read, Seek};
use std::marker::PhantomData;

/// Represents a block in the dependency tree
#[derive(Debug, Clone, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyTree {
    /// Root node representing the starting block
    pub root: DependencyNode,
    /// Total number of dependencies found
    pub total_dependencies: usize,
    /// Maximum depth of the tree
    pub max_depth: usize,
}

pub trait BlockExpander<R: Read + Seek> {
    fn expand_block(&self, block_index: usize, blend_file: &mut BlendFile<R>)
        -> Result<Vec<usize>>;

    fn can_handle(&self, code: &[u8; 4]) -> bool;
}

pub struct DependencyTracer<'a, R: Read + Seek> {
    expanders: HashMap<[u8; 4], Box<dyn BlockExpander<R> + 'a>>,
    visited: HashSet<usize>,
    visiting: HashSet<usize>,
    _phantom: PhantomData<&'a R>,
}

impl<'a, R: Read + Seek> Default for DependencyTracer<'a, R> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a, R: Read + Seek> DependencyTracer<'a, R> {
    pub fn new() -> Self {
        DependencyTracer {
            expanders: HashMap::new(),
            visited: HashSet::new(),
            visiting: HashSet::new(),
            _phantom: PhantomData,
        }
    }

    pub fn register_expander(&mut self, code: [u8; 4], expander: Box<dyn BlockExpander<R> + 'a>) {
        self.expanders.insert(code, expander);
    }

    pub fn trace_dependencies(
        &mut self,
        start_block_index: usize,
        blend_file: &mut BlendFile<R>,
    ) -> Result<Vec<usize>> {
        self.visited.clear();
        self.visiting.clear();
        let mut result = Vec::new();
        let mut queue = VecDeque::new();
        queue.push_back(start_block_index);

        while let Some(block_index) = queue.pop_front() {
            if self.visited.contains(&block_index) {
                continue;
            }
            if !self.visiting.insert(block_index) {
                continue;
            }

            if let Some(block) = blend_file.blocks.get(block_index) {
                if let Some(expander) = self.expanders.get(&block.header.code) {
                    let deps = expander.expand_block(block_index, blend_file)?;
                    for dep in deps {
                        if !self.visited.contains(&dep) {
                            queue.push_back(dep);
                        }
                    }
                }
            }

            self.visiting.remove(&block_index);
            self.visited.insert(block_index);
            if block_index != start_block_index {
                result.push(block_index);
            }
        }
        Ok(result)
    }

    /// Trace dependencies and build a hierarchical tree
    pub fn trace_dependency_tree(
        &mut self,
        start_block_index: usize,
        blend_file: &mut BlendFile<R>,
    ) -> Result<DependencyTree> {
        self.visited.clear();
        self.visiting.clear();

        let root = self.build_dependency_node(start_block_index, blend_file, 0)?;
        let total_dependencies = Self::count_nodes(&root) - 1; // Exclude root
        let max_depth = Self::calculate_max_depth(&root, 0);

        Ok(DependencyTree {
            root,
            total_dependencies,
            max_depth,
        })
    }

    fn build_dependency_node(
        &mut self,
        block_index: usize,
        blend_file: &mut BlendFile<R>,
        depth: usize,
    ) -> Result<DependencyNode> {
        // Prevent infinite recursion with circular dependencies
        if self.visited.contains(&block_index) {
            // Return a placeholder node for already visited blocks
            if let Some(block) = blend_file.blocks.get(block_index) {
                let block_code = String::from_utf8_lossy(&block.header.code)
                    .trim_end_matches('\0')
                    .to_string();
                return Ok(DependencyNode {
                    block_index,
                    block_code,
                    block_size: block.header.size,
                    block_address: block.header.old_address,
                    children: Vec::new(),
                });
            }
        }

        self.visited.insert(block_index);

        // Extract block info before mutable operations
        let (block_code, block_size, block_address, expander_code) = {
            let block = blend_file.blocks.get(block_index).ok_or_else(|| {
                bllink_parser::BlendError::Io(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    format!("Invalid block index: {block_index}"),
                ))
            })?;

            let block_code = String::from_utf8_lossy(&block.header.code)
                .trim_end_matches('\0')
                .to_string();
            (
                block_code,
                block.header.size,
                block.header.old_address,
                block.header.code,
            )
        };

        let mut children = Vec::new();

        // Get direct dependencies using the appropriate expander
        if let Some(expander) = self.expanders.get(&expander_code) {
            let deps = expander.expand_block(block_index, blend_file)?;

            for dep_index in deps {
                // Prevent excessive depth to avoid stack overflow
                if depth < 10 {
                    let child_node =
                        self.build_dependency_node(dep_index, blend_file, depth + 1)?;
                    children.push(child_node);
                }
            }
        }

        Ok(DependencyNode {
            block_index,
            block_code,
            block_size,
            block_address,
            children,
        })
    }

    fn count_nodes(node: &DependencyNode) -> usize {
        1 + node
            .children
            .iter()
            .map(|child| Self::count_nodes(child))
            .sum::<usize>()
    }

    fn calculate_max_depth(node: &DependencyNode, current_depth: usize) -> usize {
        if node.children.is_empty() {
            current_depth
        } else {
            node.children
                .iter()
                .map(|child| Self::calculate_max_depth(child, current_depth + 1))
                .max()
                .unwrap_or(current_depth)
        }
    }
}
