pub mod blocks;
pub mod dependencies;
pub mod diff;
pub mod filter;
pub mod info;
pub mod mesh_diff;
pub mod rename;

// Re-export command functions for main.rs
pub use blocks::cmd_blocks;
pub use dependencies::cmd_dependencies;
pub use diff::cmd_diff;
pub use filter::cmd_filter;
pub use info::cmd_info;
pub use mesh_diff::cmd_mesh_diff;
pub use rename::cmd_rename;

// Re-export expanders and NameResolver for dependencies.rs
pub use dot001_tracer::{
    CacheFileExpander, CollectionExpander, DataBlockExpander, DependencyTracer, ImageExpander,
    LampExpander, LibraryExpander, MaterialExpander, MeshExpander, NameResolver, NodeTreeExpander,
    ObjectExpander, SceneExpander, SoundExpander, TextureExpander,
};
