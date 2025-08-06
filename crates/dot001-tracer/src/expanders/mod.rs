pub use crate::expand_result::ExpandResult;

/// Block expanders for different Blender data types
///
/// This module contains expanders that extract dependencies from specific
/// types of Blender blocks (Scene, Object, Mesh, Collection/Group, Material, etc.).
pub mod cache_file;
pub mod collection;
pub mod data_block;
pub mod image;
pub mod lamp;
pub mod library;
pub mod material;
pub mod material_macro;
pub mod mesh;
pub mod node_tree;
pub mod object;
pub mod object_macro;
pub mod scene;
pub mod scene_macro;
pub mod sound;
pub mod texture;

pub use cache_file::CacheFileExpander;
pub use collection::CollectionExpander;
pub use data_block::DataBlockExpander;
pub use image::ImageExpander;
pub use lamp::LampExpander;
pub use library::LibraryExpander;
pub use material::MaterialExpander;
pub use material_macro::MaterialExpanderMacro;
pub use mesh::MeshExpander;
pub use node_tree::NodeTreeExpander;
pub use object::ObjectExpander;
pub use object_macro::ObjectExpanderMacro;
pub use scene::SceneExpander;
pub use scene_macro::SceneExpanderMacro;
pub use sound::SoundExpander;
pub use texture::TextureExpander;
