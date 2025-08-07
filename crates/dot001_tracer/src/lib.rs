/// # dot001_tracer
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
/// ```rust,no_run
/// use dot001_tracer::ParallelDependencyTracer;  
/// use dot001_parser::BlendFileBuf;
/// use std::fs::File;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// // Load a .blend file using the modern zero-copy parser
/// let file = File::open("scene.blend")?;
/// let blend_file = BlendFileBuf::from_file(file)?;
///
/// // Create a parallel tracer with thread-safe expanders
/// let mut tracer = ParallelDependencyTracer::new()
///     .with_default_expanders();
///
/// // Trace dependencies in parallel for high performance  
/// // object_block_index would come from your application logic
/// let object_block_index = 5; // Example block index
/// let deps = tracer.trace_dependencies_parallel(object_block_index, &blend_file)?;
///
/// println!("Found {} dependencies", deps.len());
/// # Ok(())
/// # }
/// ```
///
/// ## Legacy Usage (Deprecated)
///
/// The old API is still available but deprecated:
/// Re-exports and core functionality
pub mod core;
pub mod expand_result;
pub mod expanders;
pub mod filter;
pub mod thread_safe_expander;
pub mod thread_safe_macros;
pub mod utils;

// Include macros module to ensure macros are compiled and exported
// The actual macros are available at crate root due to #[macro_export]

// Re-export key types and traits - NEW ARCHITECTURE
pub use core::{DependencyNode, DependencyTree, ParallelDependencyTracer, TracerOptions};
pub use expand_result::ExpandResult;
pub use expanders::*;
pub use thread_safe_expander::{ThreadSafeBlockExpander, ThreadSafePointerTraversal};
pub use utils::{Determinizer, NameResolver, NameResolverTrait};

// REMOVED: DependencyTracer - use ParallelDependencyTracer instead

// Re-export bpath module for backward compatibility
pub use utils::bpath;

// Re-export from dependencies
pub use dot001_events::error::Result;
pub use dot001_parser::BlendFileBuf;

// REMOVED: Legacy BlockExpander trait
// This trait has been completely removed in favor of ThreadSafeBlockExpander.
// Use thread_safe_simple_expander!, thread_safe_custom_expander!, etc. instead.
