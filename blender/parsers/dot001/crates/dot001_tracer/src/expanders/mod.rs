//! Thread-safe block expanders using zero-copy FieldView access
//!
//! This module contains thread-safe versions of all block expanders that work
//! with BlendFileBuf for parallel dependency tracing.
//!
//! ## Expander Categories
//!
//! ### Basic Data Block Expanders
//! These handle core Blender data structures with internal references:
//! - [`ObjectExpander`] - Objects with mesh/material references  
//! - [`SceneExpander`] - Scenes with camera/world/collection references
//! - [`MeshExpander`] - Mesh data blocks
//! - [`MaterialExpander`] - Materials with node tree references
//! - [`LampExpander`] - Lamps with node tree references
//! - [`TextureExpander`] - Textures with type-specific references
//!
//! ### External File Expanders
//! These handle blocks that reference external files:
//! - [`ImageExpander`] - Image files (.png, .jpg, etc.)
//! - [`SoundExpander`] - Audio files (.wav, .mp3, etc.)  
//! - [`LibraryExpander`] - External .blend files
//! - [`CacheFileExpander`] - Simulation cache files
//!
//! ### Structural Expanders  
//! These handle complex structural dependencies:
//! - [`CollectionExpander`] - Collections with object/child references
//! - [`GroupExpander`] - Legacy groups (pre-2.8)
//! - [`NodeTreeExpander`] - Node trees with linked node dependencies

// === BASIC DATA BLOCK EXPANDERS ===
pub mod lamp;
pub mod material;
pub mod mesh;
pub mod object;
pub mod scene;
pub mod texture;

// === EXTERNAL FILE EXPANDERS ===
pub mod cache_file;
pub mod image;
pub mod library;
pub mod sound;

// === STRUCTURAL EXPANDERS ===
pub mod collection;
pub mod group;
pub mod node_tree;

// === RE-EXPORTS ===

// Basic data block expanders
pub use lamp::LampExpander;
pub use material::MaterialExpander;
pub use mesh::MeshExpander;
pub use object::ObjectExpander;
pub use scene::SceneExpander;
pub use texture::TextureExpander;

// External file expanders
pub use cache_file::CacheFileExpander;
pub use image::ImageExpander;
pub use library::LibraryExpander;
pub use sound::SoundExpander;

// Structural expanders
pub use collection::CollectionExpander;
pub use group::GroupExpander;
pub use node_tree::NodeTreeExpander;

// === CONVENIENCE FUNCTIONS ===

/// Register all basic data block expanders to a tracer
pub fn register_basic_expanders(tracer: &mut crate::core::DependencyTracer) {
    tracer.register_expander(*b"LA\0\0", Box::new(LampExpander));
    tracer.register_expander(*b"MA\0\0", Box::new(MaterialExpander));
    tracer.register_expander(*b"ME\0\0", Box::new(MeshExpander));
    tracer.register_expander(*b"OB\0\0", Box::new(ObjectExpander));
    tracer.register_expander(*b"SC\0\0", Box::new(SceneExpander));
    tracer.register_expander(*b"TE\0\0", Box::new(TextureExpander));
}

/// Register all external file expanders to a tracer
pub fn register_external_expanders(tracer: &mut crate::core::DependencyTracer) {
    tracer.register_expander(*b"CF\0\0", Box::new(CacheFileExpander));
    tracer.register_expander(*b"IM\0\0", Box::new(ImageExpander));
    tracer.register_expander(*b"LI\0\0", Box::new(LibraryExpander));
    tracer.register_expander(*b"SO\0\0", Box::new(SoundExpander));
}

/// Register all structural expanders to a tracer
pub fn register_structural_expanders(tracer: &mut crate::core::DependencyTracer) {
    tracer.register_expander(*b"GR\0\0", Box::new(CollectionExpander));
    tracer.register_expander(*b"GR\0\0", Box::new(GroupExpander)); // Note: Both use GR code
    tracer.register_expander(*b"NT\0\0", Box::new(NodeTreeExpander));
}

/// Register all available expanders to a tracer
pub fn register_all_expanders(tracer: &mut crate::core::DependencyTracer) {
    register_basic_expanders(tracer);
    register_external_expanders(tracer);
    register_structural_expanders(tracer);
}
