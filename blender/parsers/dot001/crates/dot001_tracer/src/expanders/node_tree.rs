//! Thread-safe NodeTree block expander
//!
//! This expander handles NodeTree blocks (NT and DATA) and traces dependencies to:
//! - Images used in Image Texture nodes
//! - Other data-blocks referenced by nodes
//!
//! Node trees are used in materials, textures, lamps, etc. for procedural shading.
//! This expander handles the dependencies from node trees to their referenced data-blocks,
//! particularly images used in Image Texture nodes.

use crate::custom_expander;
use dot001_events::error::Result;

custom_expander! {
    NodeTreeExpander, b"NT\0\0" => |block_index, blend_file| {
        let mut dependencies = Vec::new();

        // Try to expand this block as a node tree
        if let Ok(node_deps) = try_expand_as_nodetree_threadsafe(block_index, blend_file) {
            dependencies.extend(node_deps);
        }

        dependencies
    }
}

/// Try to expand a block as a node tree using thread-safe zero-copy access
fn try_expand_as_nodetree_threadsafe(
    block_index: usize,
    blend_file: &dot001_parser::BlendFileBuf,
) -> Result<Vec<usize>> {
    let mut dependencies = Vec::new();

    // Get block data slice for zero-copy access
    let slice = blend_file.read_block_slice_for_field_view(block_index)?;
    let view = blend_file.create_field_view(&slice)?;
    let dna = blend_file.dna()?;

    // Try different struct names for the containing structure
    let struct_names = ["bNodeTree", "NodeTree"];
    let mut first_node_ptr = 0u64;

    for struct_name in &struct_names {
        if let Some(struct_def) = dna.structs.iter().find(|s| s.type_name == *struct_name) {
            // Try to read the nodes field as a pointer to ListBase
            if let Some(nodes_field) = struct_def.find_field("nodes") {
                if nodes_field.name.is_pointer {
                    // nodes is a pointer to a ListBase
                    if let Ok(nodes_ptr) = view.read_pointer(nodes_field.offset) {
                        if nodes_ptr != 0 {
                            // Read the ListBase at that address
                            if let Some(listbase_index) =
                                blend_file.address_to_block_index(nodes_ptr)
                            {
                                let listbase_slice =
                                    blend_file.read_block_slice_for_field_view(listbase_index)?;
                                let listbase_view =
                                    blend_file.create_field_view(&listbase_slice)?;

                                // Read the first pointer from the ListBase
                                if let Some(listbase_struct) =
                                    dna.structs.iter().find(|s| s.type_name == "ListBase")
                                {
                                    if let Some(first_field) = listbase_struct.find_field("first") {
                                        if first_field.name.is_pointer {
                                            first_node_ptr = listbase_view
                                                .read_pointer(first_field.offset)
                                                .unwrap_or(0);
                                        }
                                    }
                                }
                            }
                        }
                    }
                } else {
                    // nodes is embedded, try direct access with dot notation
                    // This is tricky with FieldView - we need to calculate the offset
                    // For now, let's try a different approach: look for a nodes.first pattern

                    // In an embedded ListBase, first would be at the nodes field offset
                    let nodes_offset = nodes_field.offset;
                    // ListBase typically has first as the first field (offset 0)
                    first_node_ptr = view.read_pointer(nodes_offset).unwrap_or(0);
                }

                if first_node_ptr != 0 {
                    break;
                }
            }
        }
    }

    // If we found a first node pointer, traverse the node list
    if first_node_ptr != 0 {
        if let Ok(node_deps) = traverse_nodes_threadsafe(first_node_ptr, blend_file) {
            dependencies.extend(node_deps);
        }
    }

    Ok(dependencies)
}

/// Traverse a linked list of nodes starting from the first node pointer using thread-safe access
fn traverse_nodes_threadsafe(
    first_node_ptr: u64,
    blend_file: &dot001_parser::BlendFileBuf,
) -> Result<Vec<usize>> {
    let mut dependencies = Vec::new();

    if first_node_ptr == 0 {
        return Ok(dependencies);
    }

    let mut current_ptr = first_node_ptr;
    let mut node_count = 0;
    let dna = blend_file.dna()?;

    // Find the bNode struct definition
    let node_struct = dna
        .structs
        .iter()
        .find(|s| s.type_name == "bNode")
        .ok_or_else(|| {
            dot001_events::error::Error::tracer(
                "bNode struct not found in DNA".to_string(),
                dot001_events::error::TracerErrorKind::BlockExpansionFailed,
            )
        })?;

    while current_ptr != 0 && node_count < 100 {
        // Safety limit
        if let Some(node_index) = blend_file.address_to_block_index(current_ptr) {
            node_count += 1;

            // Get node data for zero-copy access
            let node_slice = blend_file.read_block_slice_for_field_view(node_index)?;
            let node_view = blend_file.create_field_view(&node_slice)?;

            // Look for 'id' field in the node - this points to datablocks like images
            if let Some(id_field) = node_struct.find_field("id") {
                if id_field.name.is_pointer {
                    if let Ok(id_ptr) = node_view.read_pointer(id_field.offset) {
                        if id_ptr != 0 {
                            if let Some(id_index) = blend_file.address_to_block_index(id_ptr) {
                                dependencies.push(id_index);
                            }
                        }
                    }
                }
            }

            // Move to next node
            if let Some(next_field) = node_struct.find_field("next") {
                if next_field.name.is_pointer {
                    current_ptr = node_view.read_pointer(next_field.offset).unwrap_or(0);
                } else {
                    break;
                }
            } else {
                break;
            }
        } else {
            break;
        }
    }

    Ok(dependencies)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::BlockExpander;

    #[test]
    fn test_node_tree_expander_properties() {
        let expander = NodeTreeExpander;
        assert_eq!(expander.block_code(), *b"NT\0\0");
        assert_eq!(expander.expander_name(), "NodeTreeExpander");
        assert!(expander.can_handle(b"NT\0\0"));
        assert!(!expander.can_handle(b"OB\0\0"));
    }
}
