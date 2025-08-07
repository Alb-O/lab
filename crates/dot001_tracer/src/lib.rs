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
/// Re-exports and core functionality
pub mod core;
pub mod expand_result;
pub mod expanders;
pub mod filter;
pub mod utils;

// Include macros module to ensure macros are compiled and exported
// The actual macros are available at crate root due to #[macro_export]

// Re-export key types and traits
pub use core::{DependencyNode, DependencyTracer, DependencyTree, TracerOptions};
pub use expand_result::ExpandResult;
pub use expanders::*;
pub use utils::{Determinizer, NameResolver, NameResolverTrait};

// Re-export bpath module for backward compatibility
pub use utils::bpath;

// Re-export from dependencies
pub use dot001_error::Result;
pub use dot001_parser::BlendFile;
use std::io::{Read, Seek};

/// Core trait for expanding block dependencies
pub trait BlockExpander<R: Read + Seek> {
    fn expand_block(
        &self,
        block_index: usize,
        blend_file: &mut BlendFile<R>,
    ) -> Result<ExpandResult>;

    fn can_handle(&self, code: &[u8; 4]) -> bool;
}
