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
/// use dot001_tracer::DependencyTracer;  
/// use dot001_parser::from_path;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// // Load a .blend file using the modern zero-copy parser
/// let blend_file = from_path("scene.blend")?;
///
/// // Create a parallel tracer with all available expanders (recommended)
/// let mut tracer = DependencyTracer::new()
///     .with_default_expanders(); // Registers all 13 expanders
///
/// // Or register only specific categories:
/// // .with_basic_expanders()      // Only objects, scenes, meshes, materials, etc.
/// // .with_external_expanders()   // Only images, sounds, libraries, etc.
/// // .with_structural_expanders() // Only collections, groups, node trees
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
// Re-exports and core functionality
pub mod core;
pub mod expand_result;
pub mod expander;
pub mod expanders;
pub mod filter;
pub mod macros;
pub mod utils;

// Include macros module to ensure macros are compiled and exported
// The actual macros are available at crate root due to #[macro_export]

// Re-export key types and traits
pub use core::{DependencyNode, DependencyTracer, DependencyTree, TracerOptions};
pub use expand_result::ExpandResult;
pub use expander::{BlockExpander, PointerTraversal};
pub use expanders::*;
pub use utils::{Determinizer, NameResolver, NameResolverTrait};

// Re-export bpath module for backward compatibility
pub use utils::bpath;

// Re-export from dependencies
pub use dot001_events::error::Result;
pub use dot001_parser::BlendFileBuf;
