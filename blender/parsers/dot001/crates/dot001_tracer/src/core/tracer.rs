//! Dependency tracer implementation
//!
//! This module provides dependency tracing using recursive tree building and
//! thread-safe block expansion. The implementation builds hierarchical dependency
//! trees and provides both tree and flattened list outputs with deterministic results.

use crate::BlockExpander;
use crate::core::options::TracerOptions;
use crate::core::tree::{DependencyNode, DependencyTree};
use crate::filter::{FilterEngine, FilterSpec};
use crate::utils::determinizer::Determinizer;
use dashmap::DashSet;
use dot001_events::error::Result;
use dot001_events::{
    event::{Event, TracerEvent},
    prelude::*,
};
use dot001_parser::BlendFileBuf;
use log::{debug, trace};
use rayon;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

/// Dependency tracer with recursive tree building and deterministic output
pub struct DependencyTracer {
    pub(crate) expanders: HashMap<[u8; 4], Box<dyn BlockExpander>>,
    /// Concurrent visited set using DashMap for thread-safe access
    visited: Arc<DashSet<usize>>,
    /// Optional filter of allowed blocks (indices) as a thread-safe set
    allowed: Option<Arc<HashSet<usize>>>,
    /// Determinizer for stable output generation
    determinizer: Option<Determinizer>,
    /// Tracer options (limits and behavior)
    options: TracerOptions,
    /// Thread pool size for parallel processing
    thread_pool_size: Option<usize>,
}

impl Default for DependencyTracer {
    fn default() -> Self {
        Self::new()
    }
}

impl DependencyTracer {
    /// Create a new parallel dependency tracer
    pub fn new() -> Self {
        DependencyTracer {
            expanders: HashMap::new(),
            visited: Arc::new(DashSet::new()),
            allowed: None,
            determinizer: None,
            options: TracerOptions::default(),
            thread_pool_size: None,
        }
    }

    /// Set tracer options (e.g., max_depth)
    pub fn with_options(mut self, options: TracerOptions) -> Self {
        self.options = options;
        self
    }

    /// Set the thread pool size for parallel processing
    pub fn with_thread_pool_size(mut self, size: usize) -> Self {
        self.thread_pool_size = Some(size);
        self
    }

    /// Enable deterministic output generation with address remapping
    pub fn with_deterministic_output(mut self, blend_file: &BlendFileBuf) -> Self {
        let mut determinizer = Determinizer::new();
        determinizer.build_address_map(blend_file);
        self.determinizer = Some(determinizer);
        self
    }

    /// Provide a pre-configured Determinizer for custom deterministic behavior
    pub fn with_determinizer(mut self, determinizer: Determinizer) -> Self {
        self.determinizer = Some(determinizer);
        self
    }

    /// Apply a FilterSpec using the FilterEngine and store the allowed set internally
    pub fn apply_filters(&mut self, spec: &FilterSpec, blend_file: &BlendFileBuf) -> Result<()> {
        let blocks_before = blend_file.blocks_len();
        let engine = FilterEngine::new();
        let allowed = engine.apply(spec, blend_file)?;
        let blocks_after = allowed.len();

        // Emit filter applied event
        emit_global_sync!(Event::Tracer(TracerEvent::FilterApplied {
            filter_name: format!("{spec:?}"),
            blocks_before,
            blocks_after,
        }));

        self.allowed = Some(Arc::new(allowed));
        debug!("Applied filter spec; allowed set size: {blocks_after}");
        Ok(())
    }

    /// Clear any previously applied filters
    pub fn clear_filters(&mut self) {
        self.allowed = None;
    }

    /// Register a block expander for parallel processing
    pub fn register_expander(&mut self, code: [u8; 4], expander: Box<dyn BlockExpander>) {
        self.expanders.insert(code, expander);
    }

    /// Register all standard block expanders for comprehensive dependency analysis
    pub fn with_default_expanders(mut self) -> Self {
        debug!("Registering all available block expanders for parallel processing");

        crate::expanders::register_all_expanders(&mut self);

        debug!(
            "Registered {} thread-safe block expanders (complete coverage)",
            self.expanders.len()
        );
        self
    }

    /// Register only basic data block expanders (objects, scenes, meshes, materials, etc.)
    pub fn with_basic_expanders(mut self) -> Self {
        debug!("Registering basic block expanders");

        crate::expanders::register_basic_expanders(&mut self);

        debug!("Registered {} basic block expanders", self.expanders.len());
        self
    }

    /// Register only external file expanders (images, sounds, libraries, etc.)
    pub fn with_external_expanders(mut self) -> Self {
        debug!("Registering external file expanders");

        crate::expanders::register_external_expanders(&mut self);

        debug!(
            "Registered {} external file expanders",
            self.expanders.len()
        );
        self
    }

    /// Register only structural expanders (collections, groups, node trees)
    pub fn with_structural_expanders(mut self) -> Self {
        debug!("Registering structural expanders");

        crate::expanders::register_structural_expanders(&mut self);

        debug!("Registered {} structural expanders", self.expanders.len());
        self
    }

    /// Helper to efficiently convert block code bytes to string
    fn block_code_to_string(code: &[u8; 4]) -> String {
        // Most block codes are ASCII, so we can avoid UTF-8 validation in common cases
        if code.iter().all(|&b| b.is_ascii() || b == 0) {
            // Fast path: convert ASCII directly
            let mut result = String::with_capacity(4);
            for &byte in code {
                if byte == 0 {
                    break;
                }
                result.push(byte as char);
            }
            result
        } else {
            // Fallback to UTF-8 conversion for non-ASCII codes
            String::from_utf8_lossy(code)
                .trim_end_matches('\0')
                .to_string()
        }
    }

    /// Trace dependencies and return as a flat list
    ///
    /// This method builds a hierarchical dependency tree and then flattens it
    /// to provide a simple list of dependencies, maintaining compatibility with
    /// existing code while using the more robust tree-building approach.
    pub fn trace_dependencies_parallel(
        &mut self,
        start_block_index: usize,
        blend_file: &BlendFileBuf,
    ) -> Result<Vec<usize>> {
        // Build the full tree
        let tree = self.trace_dependency_tree(start_block_index, blend_file)?;

        // Flatten it to a list
        let mut result = Vec::new();
        Self::flatten_tree(&tree.root, &mut result);

        // Sort for deterministic output
        result.sort_unstable();
        result.dedup();

        debug!(
            "Flattened dependency tree to {} unique dependencies",
            result.len()
        );

        Ok(result)
    }

    /// Trace dependencies and build a hierarchical tree structure
    ///
    /// This is the core tracing method that builds a proper `DependencyTree`
    /// with parent-child relationships preserved through recursive traversal.
    pub fn trace_dependency_tree(
        &mut self,
        start_block_index: usize,
        blend_file: &BlendFileBuf,
    ) -> Result<DependencyTree> {
        debug!("Starting parallel dependency tree trace from block {start_block_index}");

        let start_time = std::time::Instant::now();

        // Emit tracer started event
        let root_block_type = if let Some(block) = blend_file.get_block(start_block_index) {
            Self::block_code_to_string(&block.header.code)
        } else {
            "unknown".to_string()
        };

        emit_global_sync!(Event::Tracer(TracerEvent::Started {
            root_blocks: vec![format!("{}[{}]", root_block_type, start_block_index)],
            options: format!(
                "max_depth: {}, parallel: true, tree_mode: true, threads: {:?}",
                self.options.max_depth,
                self.thread_pool_size
                    .unwrap_or_else(rayon::current_num_threads)
            ),
        }));

        // Clear visited set
        self.visited.clear();

        // Respect allowed set: if present and start not allowed, return empty tree
        if let Some(allowed) = &self.allowed {
            if !allowed.contains(&start_block_index) {
                let root =
                    self.create_dependency_node(start_block_index, blend_file, Vec::new())?;
                return Ok(DependencyTree {
                    root,
                    total_dependencies: 0,
                    max_depth: 0,
                });
            }
        }

        // Build tree layer by layer, preserving relationships
        let root =
            self.build_tree_recursive(start_block_index, blend_file, 0, &mut HashSet::new())?;
        let total_dependencies = Self::count_dependencies(&root);
        let max_depth = Self::calculate_max_depth(&root);

        debug!(
            "Parallel dependency tree trace completed, found {total_dependencies} total dependencies with max depth {max_depth}"
        );

        // Emit tracer finished event
        let duration_ms = start_time.elapsed().as_millis() as u64;

        emit_global_sync!(Event::Tracer(TracerEvent::Finished {
            total_blocks_traced: total_dependencies,
            unique_dependencies: total_dependencies,
            duration_ms,
        }));

        Ok(DependencyTree {
            root,
            total_dependencies,
            max_depth,
        })
    }

    /// Recursively build tree nodes maintaining parent-child relationships
    fn build_tree_recursive(
        &self,
        block_index: usize,
        blend_file: &BlendFileBuf,
        current_depth: usize,
        visited: &mut HashSet<usize>,
    ) -> Result<DependencyNode> {
        // Avoid infinite recursion
        if visited.contains(&block_index) || current_depth >= self.options.max_depth {
            return self.create_dependency_node(block_index, blend_file, Vec::new());
        }

        visited.insert(block_index);

        // Get immediate dependencies for this block
        let immediate_deps =
            self.expand_block_dependencies(block_index, blend_file, current_depth)?;

        // Recursively build child nodes
        let mut children = Vec::new();
        for &child_index in &immediate_deps {
            if let Some(allowed) = &self.allowed {
                if !allowed.contains(&child_index) {
                    continue;
                }
            }

            let child_node =
                self.build_tree_recursive(child_index, blend_file, current_depth + 1, visited)?;
            children.push(child_node);
        }

        visited.remove(&block_index); // Allow this block to be visited via other paths

        self.create_dependency_node(block_index, blend_file, children)
    }

    /// Create a DependencyNode from block information
    fn create_dependency_node(
        &self,
        block_index: usize,
        blend_file: &BlendFileBuf,
        children: Vec<DependencyNode>,
    ) -> Result<DependencyNode> {
        if let Some(block) = blend_file.get_block(block_index) {
            Ok(DependencyNode {
                block_index,
                block_code: Self::block_code_to_string(&block.header.code),
                block_size: block.header.size,
                block_address: block.header.old_address,
                children,
            })
        } else {
            Ok(DependencyNode {
                block_index,
                block_code: "????".to_string(),
                block_size: 0,
                block_address: 0,
                children,
            })
        }
    }

    /// Count total dependencies in a tree
    fn count_dependencies(node: &DependencyNode) -> usize {
        let mut count = node.children.len();
        for child in &node.children {
            count += Self::count_dependencies(child);
        }
        count
    }

    /// Calculate maximum depth of a tree
    fn calculate_max_depth(node: &DependencyNode) -> usize {
        if node.children.is_empty() {
            0
        } else {
            1 + node
                .children
                .iter()
                .map(Self::calculate_max_depth)
                .max()
                .unwrap_or(0)
        }
    }

    /// Flatten a dependency tree into a list of block indices (excluding root)
    ///
    /// This provides a flat list of all dependencies, useful for compatibility
    /// with code that expects a simple Vec<usize> of dependencies.
    fn flatten_tree(node: &DependencyNode, result: &mut Vec<usize>) {
        for child in &node.children {
            result.push(child.block_index);
            Self::flatten_tree(child, result);
        }
    }

    /// Expand dependencies for a single block (thread-safe)
    fn expand_block_dependencies(
        &self,
        block_index: usize,
        blend_file: &BlendFileBuf,
        _depth: usize,
    ) -> Result<Vec<usize>> {
        let mut dependencies = Vec::new();

        if let Some(block) = blend_file.get_block(block_index) {
            // Skip if filtered out
            if let Some(allowed) = &self.allowed {
                if !allowed.contains(&block_index) {
                    return Ok(dependencies);
                }
            }

            if let Some(expander) = self.expanders.get(&block.header.code) {
                // Note: Event emission disabled in parallel context to avoid Tokio runtime issues
                // TODO: Fix event emission in parallel processing context

                // Use thread-safe expander with BlendFileBuf
                let deps = expander.expand_block_threadsafe(block_index, blend_file)?;

                trace!(
                    "Block {} expanded to {} dependencies",
                    block_index,
                    deps.dependencies.len()
                );

                for dep in deps.dependencies {
                    // Skip if filtered out
                    if let Some(allowed) = &self.allowed {
                        if !allowed.contains(&dep) {
                            continue;
                        }
                    }
                    dependencies.push(dep);
                }
            }
        }

        Ok(dependencies)
    }

    /// Get a reference to the internal determinizer, if configured
    pub fn determinizer(&self) -> Option<&Determinizer> {
        self.determinizer.as_ref()
    }

    /// Get visited blocks count (thread-safe)
    pub fn visited_count(&self) -> usize {
        self.visited.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tracer_creation() {
        let tracer = DependencyTracer::new();
        assert_eq!(tracer.visited_count(), 0);
    }

    #[test]
    fn test_thread_pool_size_configuration() {
        let tracer = DependencyTracer::new().with_thread_pool_size(4);
        assert_eq!(tracer.thread_pool_size, Some(4));
    }

    #[test]
    fn test_block_code_to_string() {
        assert_eq!(DependencyTracer::block_code_to_string(b"SC\0\0"), "SC");
        assert_eq!(DependencyTracer::block_code_to_string(b"DATA"), "DATA");
    }

    #[test]
    fn test_expander_registration() {
        let mut tracer = DependencyTracer::new();

        use crate::expanders::ObjectExpander;
        tracer.register_expander(*b"OB\0\0", Box::new(ObjectExpander));

        assert_eq!(tracer.expanders.len(), 1);
        assert!(tracer.expanders.contains_key(b"OB\0\0"));
    }

    #[test]
    fn test_with_default_expanders() {
        let tracer = DependencyTracer::new().with_default_expanders();

        // Should have registered the basic thread-safe expanders
        assert!(tracer.expanders.len() >= 5);
        assert!(tracer.expanders.contains_key(b"OB\0\0"));
        assert!(tracer.expanders.contains_key(b"SC\0\0"));
        assert!(tracer.expanders.contains_key(b"ME\0\0"));
        assert!(tracer.expanders.contains_key(b"MA\0\0"));
        assert!(tracer.expanders.contains_key(b"GR\0\0"));
    }
}
