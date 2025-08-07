/// Block expanders for different Blender data types
///
/// This module contains expanders that extract dependencies from specific
/// types of Blender blocks (Scene, Object, Mesh, Collection/Group, Material, etc.).
///
/// The expanders are organized by category:
/// - `basic`: Internal Blender structures (objects, meshes, materials, etc.)
/// - `external`: Blocks that reference external files (sounds, images, libraries, etc.)
/// - `macros`: Macro generators for creating expanders
/// - `thread_safe`: Thread-safe versions using zero-copy FieldView access
pub mod basic;
pub mod external;
pub mod macros;
pub mod thread_safe;

// Re-export all expanders at the expanders level
pub use basic::*;
pub use external::*;
// Re-export thread-safe expanders with prefixed names to avoid conflicts
pub use thread_safe::{
    ThreadSafeMaterialExpander, ThreadSafeMeshExpander, ThreadSafeObjectExpander,
    ThreadSafeSceneExpander,
};
