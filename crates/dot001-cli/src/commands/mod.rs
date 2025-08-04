pub mod blocks;
#[cfg(feature = "trace")]
pub mod dependencies;
#[cfg(feature = "diff")]
pub mod diff;
#[cfg(feature = "trace")]
pub mod filter;
pub mod info;
#[cfg(feature = "editor")]
pub mod libpath;
#[cfg(feature = "diff")]
pub mod mesh_diff;
#[cfg(feature = "editor")]
pub mod rename;

// Re-export command functions for main.rs
pub use blocks::cmd_blocks;
#[cfg(feature = "trace")]
pub use dependencies::cmd_dependencies;
#[cfg(feature = "diff")]
pub use diff::cmd_diff;
#[cfg(feature = "trace")]
pub use filter::cmd_filter;
pub use info::cmd_info;
#[cfg(feature = "editor")]
pub use libpath::cmd_libpath;
#[cfg(feature = "diff")]
pub use mesh_diff::cmd_mesh_diff;
#[cfg(feature = "editor")]
pub use rename::cmd_rename;

// Re-export expanders and NameResolver for dependencies.rs
#[cfg(feature = "trace")]
pub use dot001_tracer::{
    CacheFileExpander, CollectionExpander, DataBlockExpander, DependencyTracer, ImageExpander,
    LampExpander, LibraryExpander, MaterialExpander, MeshExpander, NameResolver, NodeTreeExpander,
    ObjectExpander, SceneExpander, SoundExpander, TextureExpander,
};
