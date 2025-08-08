//! Parallel dependency tracer implementation
//!
//! This module provides high-performance parallel dependency tracing using
//! layered BFS, DashMap for visited tracking, and deterministic merging.
//!
//! The implementation processes dependencies in parallel using rayon's work-stealing
//! scheduler while maintaining deterministic output through per-depth stable sorting.

use crate::ThreadSafeBlockExpander;
use crate::core::options::TracerOptions;
use crate::filter::{FilterEngine, FilterSpec};
use crate::utils::determinizer::Determinizer;
use dashmap::DashSet;
use dot001_events::error::{Error, Result, TracerErrorKind};
use dot001_events::{
    event::{Event, TracerEvent},
    prelude::*,
};
use dot001_parser::BlendFileBuf;
use log::{debug, trace};
use rayon::prelude::*;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

/// Parallel dependency tracer with concurrent BFS and deterministic output
pub struct ParallelDependencyTracer {
    pub(crate) thread_safe_expanders: HashMap<[u8; 4], Box<dyn ThreadSafeBlockExpander>>,
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

impl Default for ParallelDependencyTracer {
    fn default() -> Self {
        Self::new()
    }
}

impl ParallelDependencyTracer {
    /// Create a new parallel dependency tracer
    pub fn new() -> Self {
        ParallelDependencyTracer {
            thread_safe_expanders: HashMap::new(),
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

    /// Register a thread-safe block expander for parallel processing
    pub fn register_thread_safe_expander(
        &mut self,
        code: [u8; 4],
        expander: Box<dyn ThreadSafeBlockExpander>,
    ) {
        self.thread_safe_expanders.insert(code, expander);
    }

    /// Register all standard thread-safe block expanders for comprehensive dependency analysis
    pub fn with_default_expanders(mut self) -> Self {
        debug!("Registering thread-safe block expanders for parallel processing");

        // Import thread-safe expanders
        use crate::expanders::thread_safe::*;

        // Register all available thread-safe expanders
        self.register_thread_safe_expander(*b"OB\0\0", Box::new(ThreadSafeObjectExpander));
        self.register_thread_safe_expander(*b"SC\0\0", Box::new(ThreadSafeSceneExpander));
        self.register_thread_safe_expander(*b"ME\0\0", Box::new(ThreadSafeMeshExpander));
        self.register_thread_safe_expander(*b"MA\0\0", Box::new(ThreadSafeMaterialExpander));
        self.register_thread_safe_expander(*b"GR\0\0", Box::new(ThreadSafeGroupExpander));

        debug!(
            "Registered {} thread-safe block expanders",
            self.thread_safe_expanders.len()
        );
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

    /// Trace dependencies using parallel layered BFS
    ///
    /// This is the core parallel implementation that processes dependency
    /// traversal in parallel while maintaining deterministic output.
    pub fn trace_dependencies_parallel(
        &mut self,
        start_block_index: usize,
        blend_file: &BlendFileBuf,
    ) -> Result<Vec<usize>> {
        debug!("Starting parallel dependency trace from block {start_block_index}");

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
                "max_depth: {}, parallel: true, threads: {:?}",
                self.options.max_depth,
                self.thread_pool_size
                    .unwrap_or_else(rayon::current_num_threads)
            ),
        }));

        // Clear visited set
        self.visited.clear();
        let mut result = Vec::new();

        // Respect allowed set: if present and start not allowed, return empty
        if let Some(allowed) = &self.allowed {
            if !allowed.contains(&start_block_index) {
                return Ok(result);
            }
        }

        // Initialize with start block
        let mut current_layer = vec![start_block_index];
        self.visited.insert(start_block_index);

        // Process each depth layer
        for depth in 0..self.options.max_depth {
            if current_layer.is_empty() {
                break;
            }

            debug!(
                "Processing depth layer {depth} with {} blocks",
                current_layer.len()
            );

            // Process current layer in parallel
            let next_layer_chunks: Vec<Vec<usize>> = if self.thread_pool_size.is_some() {
                // Use custom thread pool if specified
                rayon::ThreadPoolBuilder::new()
                    .num_threads(self.thread_pool_size.unwrap())
                    .build()
                    .map_err(|e| {
                        Error::tracer(
                            format!("Failed to create thread pool: {e}"),
                            TracerErrorKind::BlockExpansionFailed,
                        )
                    })?
                    .install(|| self.process_layer_parallel(&current_layer, blend_file, depth))?
            } else {
                // Use default global thread pool
                self.process_layer_parallel(&current_layer, blend_file, depth)?
            };

            // Merge results from all threads deterministically
            let mut next_layer = Vec::new();
            for chunk in next_layer_chunks {
                next_layer.extend(chunk);
            }

            // Sort for deterministic output and remove duplicates
            next_layer.sort_unstable();
            next_layer.dedup();

            // Filter out already visited blocks and update visited set
            let mut filtered_next_layer = Vec::new();
            for block_index in next_layer {
                if self.visited.insert(block_index) {
                    // This block was not visited before
                    filtered_next_layer.push(block_index);
                    if block_index != start_block_index {
                        result.push(block_index);
                    }
                }
            }

            current_layer = filtered_next_layer;

            trace!(
                "Depth {depth} processed, next layer has {} blocks",
                current_layer.len()
            );
        }

        // Sort final result for deterministic output
        result.sort_unstable();

        debug!(
            "Parallel dependency trace completed, found {} total dependencies",
            result.len()
        );

        // Emit tracer finished event
        let duration_ms = start_time.elapsed().as_millis() as u64;
        let unique_dependencies = result.len(); // Already unique due to visited tracking

        emit_global_sync!(Event::Tracer(TracerEvent::Finished {
            total_blocks_traced: result.len(),
            unique_dependencies,
            duration_ms,
        }));

        Ok(result)
    }

    /// Process a single layer in parallel, returning per-thread dependency chunks
    fn process_layer_parallel(
        &self,
        layer: &[usize],
        blend_file: &BlendFileBuf,
        depth: usize,
    ) -> Result<Vec<Vec<usize>>> {
        // Process blocks in parallel chunks
        let chunk_results: Result<Vec<Vec<usize>>> = layer
            .par_iter()
            .map(|&block_index| self.expand_block_dependencies(block_index, blend_file, depth))
            .collect();

        chunk_results
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

            if let Some(expander) = self.thread_safe_expanders.get(&block.header.code) {
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
    fn test_parallel_tracer_creation() {
        let tracer = ParallelDependencyTracer::new();
        assert_eq!(tracer.visited_count(), 0);
    }

    #[test]
    fn test_thread_pool_size_configuration() {
        let tracer = ParallelDependencyTracer::new().with_thread_pool_size(4);
        assert_eq!(tracer.thread_pool_size, Some(4));
    }

    #[test]
    fn test_block_code_to_string() {
        assert_eq!(
            ParallelDependencyTracer::block_code_to_string(b"SC\0\0"),
            "SC"
        );
        assert_eq!(
            ParallelDependencyTracer::block_code_to_string(b"DATA"),
            "DATA"
        );
    }

    #[test]
    fn test_thread_safe_expander_registration() {
        let mut tracer = ParallelDependencyTracer::new();

        use crate::expanders::thread_safe::ThreadSafeObjectExpander;
        tracer.register_thread_safe_expander(*b"OB\0\0", Box::new(ThreadSafeObjectExpander));

        assert_eq!(tracer.thread_safe_expanders.len(), 1);
        assert!(tracer.thread_safe_expanders.contains_key(b"OB\0\0"));
    }

    #[test]
    fn test_with_default_expanders() {
        let tracer = ParallelDependencyTracer::new().with_default_expanders();

        // Should have registered the basic thread-safe expanders
        assert!(tracer.thread_safe_expanders.len() >= 5);
        assert!(tracer.thread_safe_expanders.contains_key(b"OB\0\0"));
        assert!(tracer.thread_safe_expanders.contains_key(b"SC\0\0"));
        assert!(tracer.thread_safe_expanders.contains_key(b"ME\0\0"));
        assert!(tracer.thread_safe_expanders.contains_key(b"MA\0\0"));
        assert!(tracer.thread_safe_expanders.contains_key(b"GR\0\0"));
    }
}
