/// # dot001-tracer
///
/// Dependency tracing engine for Blender .blend files.
///
/// This crate provides the core dependency tracing functionality with support for
/// sophisticated traversal patterns including linked lists and array dereferencing.
///
/// ## Key Features
///
/// - **Dynamic data access**: Block expanders can read additional data on-demand
/// - **Material array dereferencing**: Properly handles objects with multiple materials
/// - **Cross-version compatibility**: Works with Blender 2.79 through 5.0+
/// - **Extensible architecture**: Easy to add new block expanders
///
/// ## Example
///
// Example usage (not a real test):
// use dot001_tracer::{BlendFile, DependencyTracer, ObjectExpander};
// use std::fs::File;
// use std::io::BufReader;
//
// let file = File::open("scene.blend")?;
// let mut reader = BufReader::new(file);
// let mut blend_file = BlendFile::new(&mut reader)?;
//
// let mut tracer = DependencyTracer::new();
// tracer.register_expander(*b"OB\0\0", Box::new(ObjectExpander));
//
// let deps = tracer.trace_dependencies(object_block_index, &mut blend_file)?;
// Ok::<(), Box<dyn std::error::Error>>(())
pub mod bpath;
// dot001-tracer/src/lib.rs

pub mod expand_result;
pub mod expanders;
pub use expand_result::ExpandResult;
pub mod name_resolver;

pub use dot001_parser::BlendFile;
pub use dot001_parser::Result;

/// New unified result type - preferred for new code
pub use dot001_error::Result as UnifiedResult;
use dot001_error::{Dot001Error, TracerErrorKind};
pub use expanders::{
    CacheFileExpander, CollectionExpander, DataBlockExpander, ImageExpander, LampExpander,
    LibraryExpander, MaterialExpander, MeshExpander, NodeTreeExpander, ObjectExpander,
    SceneExpander, SoundExpander, TextureExpander,
};
pub use name_resolver::NameResolver;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
use std::io::{Read, Seek};
use std::marker::PhantomData;

// Re-export filter module only once
pub mod filter;
use crate::filter::{FilterEngine, FilterSpec};

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
    fn expand_block(
        &self,
        block_index: usize,
        blend_file: &mut BlendFile<R>,
    ) -> Result<ExpandResult>;

    fn can_handle(&self, code: &[u8; 4]) -> bool;
}

/// Options to control traversal limits and behavior.
#[derive(Debug, Clone, Copy)]
pub struct TracerOptions {
    pub max_depth: usize,
}

impl Default for TracerOptions {
    fn default() -> Self {
        Self { max_depth: 10 }
    }
}

pub struct DependencyTracer<'a, R: Read + Seek> {
    expanders: HashMap<[u8; 4], Box<dyn BlockExpander<R> + 'a>>,
    visited: HashSet<usize>,
    visiting: HashSet<usize>,
    /// Optional filter of allowed blocks (indices). If Some, traversal will only enqueue blocks in this set.
    allowed: Option<HashSet<usize>>,
    /// Optional address remapping (old_address -> remapped_id) for deterministic outputs.
    address_map: Option<HashMap<u64, u64>>,
    /// Tracer options (limits and behavior).
    options: TracerOptions,
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
            allowed: None,
            address_map: None,
            options: TracerOptions::default(),
            _phantom: PhantomData,
        }
    }

    /// Set tracer options (e.g., max_depth).
    pub fn with_options(mut self, options: TracerOptions) -> Self {
        self.options = options;
        self
    }

    /// Provide an address map to remap old addresses to deterministic IDs during output.
    pub fn with_address_map(mut self, map: HashMap<u64, u64>) -> Self {
        self.address_map = Some(map);
        self
    }

    /// Apply a FilterSpec using the FilterEngine and store the allowed set internally.
    pub fn apply_filters(
        &mut self,
        spec: &FilterSpec,
        blend_file: &mut BlendFile<R>,
    ) -> Result<()> {
        let engine = FilterEngine::new();
        let allowed = engine.apply(spec, blend_file)?;
        self.allowed = Some(allowed);
        Ok(())
    }

    /// Clear any previously applied filters.
    pub fn clear_filters(&mut self) {
        self.allowed = None;
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

        // Respect allowed set: if present and start not allowed, return empty.
        if let Some(allowed) = &self.allowed {
            if !allowed.contains(&start_block_index) {
                return Ok(result);
            }
        }
        queue.push_back((start_block_index, 0usize));

        while let Some((block_index, depth)) = queue.pop_front() {
            if self.visited.contains(&block_index) {
                continue;
            }
            if !self.visiting.insert(block_index) {
                continue;
            }

            if let Some(block) = blend_file.get_block(block_index) {
                // Skip if filtered out
                if let Some(allowed) = &self.allowed {
                    if !allowed.contains(&block_index) {
                        self.visiting.remove(&block_index);
                        self.visited.insert(block_index);
                        continue;
                    }
                }

                if let Some(expander) = self.expanders.get(&block.header.code) {
                    let deps = expander.expand_block(block_index, blend_file)?;
                    for dep in deps.dependencies {
                        // Skip if filtered out
                        if let Some(allowed) = &self.allowed {
                            if !allowed.contains(&dep) {
                                continue;
                            }
                        }
                        if !self.visited.contains(&dep) {
                            // Depth limit
                            if depth < self.options.max_depth {
                                queue.push_back((dep, depth + 1));
                            }
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
            if let Some(block) = blend_file.get_block(block_index) {
                let block_code = String::from_utf8_lossy(&block.header.code)
                    .trim_end_matches('\0')
                    .to_string();
                return Ok(DependencyNode {
                    block_index,
                    block_code,
                    block_size: block.header.size,
                    block_address: self.remap_address(block.header.old_address),
                    children: Vec::new(),
                });
            }
        }

        // Respect filter: skip building this node if not allowed
        if let Some(allowed) = &self.allowed {
            if !allowed.contains(&block_index) {
                return Ok(DependencyNode {
                    block_index,
                    block_code: String::from("FILTERED"),
                    block_size: 0,
                    block_address: 0,
                    children: Vec::new(),
                });
            }
        }

        self.visited.insert(block_index);

        // Extract block info before mutable operations
        let (block_code, block_size, block_address, expander_code) = {
            let block = blend_file.get_block(block_index).ok_or_else(|| {
                Dot001Error::tracer(
                    format!("Invalid block index: {block_index}"),
                    TracerErrorKind::BlockExpansionFailed,
                )
            })?;

            let block_code = String::from_utf8_lossy(&block.header.code)
                .trim_end_matches('\0')
                .to_string();
            (
                block_code,
                block.header.size,
                self.remap_address(block.header.old_address),
                block.header.code,
            )
        };

        let mut children = Vec::new();

        // Get direct dependencies using the appropriate expander
        if let Some(expander) = self.expanders.get(&expander_code) {
            let deps = expander.expand_block(block_index, blend_file)?;
            for dep_index in deps.dependencies {
                // Respect filter
                if let Some(allowed) = &self.allowed {
                    if !allowed.contains(&dep_index) {
                        continue;
                    }
                }
                // Prevent excessive depth to avoid stack overflow
                if depth < self.options.max_depth {
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

    /// Remap an address using the optional address_map if present.
    fn remap_address(&self, addr: u64) -> u64 {
        if let Some(map) = &self.address_map {
            if let Some(mapped) = map.get(&addr) {
                return *mapped;
            }
        }
        addr
    }
}

/// Helper functions for creating unified errors with tracer context
impl<'a, R: Read + Seek> DependencyTracer<'a, R> {
    /// Create a unified tracer error for dependency resolution failures
    pub fn dependency_error<M: Into<String>>(message: M) -> Dot001Error {
        Dot001Error::tracer(message.into(), TracerErrorKind::DependencyResolutionFailed)
    }

    /// Create a unified tracer error for name resolution failures
    pub fn name_resolution_error<M: Into<String>>(message: M) -> Dot001Error {
        Dot001Error::tracer(message.into(), TracerErrorKind::NameResolutionFailed)
    }

    /// Create a unified tracer error for block expansion failures
    pub fn block_expansion_error<M: Into<String>>(message: M) -> Dot001Error {
        Dot001Error::tracer(message.into(), TracerErrorKind::BlockExpansionFailed)
    }

    /// Create a unified tracer error for circular dependency detection
    pub fn circular_dependency_error<M: Into<String>>(message: M) -> Dot001Error {
        Dot001Error::tracer(message.into(), TracerErrorKind::CircularDependency)
    }
}

/// Convert unified errors to tracer context
pub fn to_tracer_error(unified_err: Dot001Error) -> Dot001Error {
    match unified_err {
        Dot001Error::BlendFile {
            message,
            file_path,
            block_index,
            ..
        } => Dot001Error::tracer(message, TracerErrorKind::DependencyResolutionFailed)
            .with_file_path(file_path.unwrap_or_default())
            .with_block_index(block_index.unwrap_or(0)),
        other => other,
    }
}
