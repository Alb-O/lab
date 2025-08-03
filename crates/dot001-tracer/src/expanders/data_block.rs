use crate::BlockExpander;
use dot001_parser::{BlendFile, Result};
use std::io::{Read, Seek};

/// Smart expander for DATA blocks that can contain different types of structures
///
/// DATA blocks in Blender can contain various types of data structures:
/// - Collections (bNodeTree structures for scene organization)
/// - NodeTrees (bNodeTree structures for shader/geometry nodes)
/// - Other data structures
///
/// This expander detects the type and routes to the appropriate handler.
pub struct DataBlockExpander;

impl<R: Read + Seek> BlockExpander<R> for DataBlockExpander {
    fn expand_block(
        &self,
        block_index: usize,
        blend_file: &mut BlendFile<R>,
    ) -> Result<Vec<usize>> {
        let mut dependencies = Vec::new();

        // Read the data block
        let data = blend_file.read_block_data(block_index)?;
        let reader = blend_file.create_field_reader(&data)?;

        // Try to detect what type of data structure this is

        // Check for NodeTree first (more specific than Collection)
        let has_nodes = reader.read_field_pointer("bNodeTree", "nodes").is_ok();
        let has_nodetree = reader.read_field_pointer("NodeTree", "nodes").is_ok();

        if has_nodes || has_nodetree {
            // This looks like a NodeTree - use nodetree expansion logic
            if let Ok(nodetree_deps) = expand_as_nodetree(block_index, blend_file) {
                dependencies.extend(nodetree_deps);
            }
            return Ok(dependencies);
        }

        // Check if it's a Collection (has gobject or children fields)
        let has_gobject = reader.read_field_pointer("Collection", "gobject").is_ok();
        let has_children = reader.read_field_pointer("Collection", "children").is_ok();

        if has_gobject || has_children {
            // This looks like a Collection - use collection expansion logic
            if let Ok(collection_deps) = expand_as_collection(block_index, blend_file) {
                dependencies.extend(collection_deps);
            }
            return Ok(dependencies);
        }

        // Unknown DATA block type - no expansion
        Ok(dependencies)
    }

    fn can_handle(&self, code: &[u8; 4]) -> bool {
        code == b"DATA"
    }
}

/// Expand DATA block as a Collection
fn expand_as_collection<R: Read + Seek>(
    block_index: usize,
    blend_file: &mut BlendFile<R>,
) -> Result<Vec<usize>> {
    // Reuse the CollectionExpander logic
    use super::collection::CollectionExpander;
    let expander = CollectionExpander;
    expander.expand_block(block_index, blend_file)
}

/// Expand DATA block as a NodeTree
fn expand_as_nodetree<R: Read + Seek>(
    block_index: usize,
    blend_file: &mut BlendFile<R>,
) -> Result<Vec<usize>> {
    // Reuse the NodeTreeExpander logic
    use super::node_tree::try_expand_as_nodetree;
    try_expand_as_nodetree(block_index, blend_file)
}
