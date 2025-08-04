use crate::expand_result::ExpandResult;
use crate::BlockExpander;
use dot001_parser::{BlendFile, Result};
use std::io::{Read, Seek};

/// Expander for NodeTree blocks (both NT and DATA blocks containing bNodeTree structures)
///
/// Node trees are used in materials, textures, lamps, etc. for procedural shading.
/// This expander handles the dependencies from node trees to their referenced data blocks,
/// particularly images used in Image Texture nodes.
pub struct NodeTreeExpander;

impl<R: Read + Seek> BlockExpander<R> for NodeTreeExpander {
    fn expand_block(
        &self,
        block_index: usize,
        blend_file: &mut BlendFile<R>,
    ) -> Result<ExpandResult> {
        // The critical insight from the Python implementation:
        // Node trees can be embedded in DATA blocks, not just NT blocks
        // We need to try multiple approaches to find the node data

        let expand = try_expand_as_nodetree(block_index, blend_file)?;
        Ok(expand)
    }

    fn can_handle(&self, code: &[u8; 4]) -> bool {
        // Only handle NT blocks - DATA blocks are too generic and can be collections, etc.
        code == b"NT\0\0"
    }
}

/// Try to expand a block as a node tree using different approaches
pub fn try_expand_as_nodetree<R: Read + Seek>(
    block_index: usize,
    blend_file: &mut BlendFile<R>,
) -> Result<ExpandResult> {
    let mut dependencies = Vec::new();
    let mut debug = None;

    // Read the block data
    let block_data = blend_file.read_block_data(block_index)?;
    let reader = blend_file.create_field_reader(&block_data)?;

    // Try different ways to access node data
    // The Python implementation uses block.get_pointer((b"nodes", b"first"))
    // This suggests a two-step process: first get "nodes", then get "first" from that

    // Approach 1: Try to read nodes ListBase directly
    // In Blender, nodes is a ListBase structure with first/last pointers
    let mut first_node_ptr = 0;

    // Try different struct names for the containing structure
    let struct_names = ["bNodeTree", "NodeTree"];
    let mut nodes_ptr = 0;
    let mut found_struct = None;

    for struct_name in &struct_names {
        // Try to read the nodes field as a pointer to ListBase
        if let Ok(ptr) = reader.read_field_pointer(struct_name, "nodes") {
            nodes_ptr = ptr;
            found_struct = Some(struct_name);
            break;
        }
    }

    // Now process the nodes pointer if we found one
    if nodes_ptr != 0 {
        // Now read the ListBase structure at that address
        // ListBase has 'first' and 'last' pointers
        if let Some(listbase_index) = blend_file.find_block_by_address(nodes_ptr) {
            let listbase_data = blend_file.read_block_data(listbase_index)?;
            let listbase_reader = blend_file.create_field_reader(&listbase_data)?;

            // Try to read the first pointer from the ListBase
            if let Ok(ptr) = listbase_reader.read_field_pointer("ListBase", "first") {
                first_node_ptr = ptr;
            }
        } else {
            // Maybe the nodes field is embedded, not a pointer
            // In that case, try direct access with the struct we found
            if let Some(struct_name) = found_struct {
                if let Ok(ptr) = reader.read_field_pointer(struct_name, "nodes.first") {
                    first_node_ptr = ptr;
                }
            }
        }
    }

    if first_node_ptr != 0 {
        if let Ok(node_deps) = traverse_nodes(first_node_ptr, blend_file) {
            dependencies.extend(node_deps);
        }
    } else {
        debug = Some(format!("Could not find nodes.first in block {block_index}"));
    }

    Ok(ExpandResult {
        dependencies,
        debug,
    })
}

/// Traverse a linked list of nodes starting from the first node pointer
fn traverse_nodes<R: Read + Seek>(
    first_node_ptr: u64,
    blend_file: &mut BlendFile<R>,
) -> Result<Vec<usize>> {
    let mut dependencies = Vec::new();

    if first_node_ptr == 0 {
        return Ok(dependencies);
    }

    let mut current_ptr = first_node_ptr;
    let mut node_count = 0;
    while current_ptr != 0 && node_count < 100 {
        // Safety limit
        if let Some(node_index) = blend_file.find_block_by_address(current_ptr) {
            node_count += 1;

            // Read this node
            let node_data = blend_file.read_block_data(node_index)?;
            let node_reader = blend_file.create_field_reader(&node_data)?;

            // Look for 'id' field in the node - this points to datablocks like images
            if let Ok(id_ptr) = node_reader.read_field_pointer("bNode", "id") {
                if id_ptr != 0 {
                    if let Some(id_index) = blend_file.find_block_by_address(id_ptr) {
                        dependencies.push(id_index);
                    }
                }
            }

            // Move to next node
            if let Ok(next_ptr) = node_reader.read_field_pointer("bNode", "next") {
                current_ptr = next_ptr;
            } else {
                break;
            }
        } else {
            break;
        }
    }

    Ok(dependencies)
}
