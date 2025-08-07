//! Thread-safe block expanders using zero-copy FieldView access
//!
//! This module contains thread-safe versions of all block expanders that work
//! with BlendFileBuf for parallel dependency tracing.

pub mod material;
pub mod mesh;
pub mod object;
pub mod scene;

// Re-export all thread-safe expanders
pub use material::ThreadSafeMaterialExpander;
pub use mesh::ThreadSafeMeshExpander;
pub use object::ThreadSafeObjectExpander;
pub use scene::ThreadSafeSceneExpander;
