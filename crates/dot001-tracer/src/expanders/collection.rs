use crate::BlockExpander;
use crate::ExpandResult;
use dot001_parser::{BlendFile, Result};
use std::io::{Read, Seek};

/// Expander for Collection (GR) blocks
///
/// Collections (formerly Groups) contain references to objects and can have
/// child collections (in Blender 2.8+). This expander handles both the old
/// Group format and the newer Collection format for cross-version compatibility.
pub struct CollectionExpander;

impl<R: Read + Seek> BlockExpander<R> for CollectionExpander {
    fn expand_block(
        &self,
        block_index: usize,
        blend_file: &mut BlendFile<R>,
    ) -> Result<ExpandResult> {
        let mut dependencies = Vec::new();

        // Read the collection block data and extract pointers
        // Try both "Collection" (Blender 2.8+) and "Group" (older versions)
        let (gobject_ptr, children_ptr) = {
            let collection_data = blend_file.read_block_data(block_index)?;
            let reader = blend_file.create_field_reader(&collection_data)?;

            // For DATA blocks, check if this is actually a collection or something else (like a nodetree)
            let block = blend_file.get_block(block_index).unwrap();
            if block.header.code == *b"DATA" {
                // Try to detect if this is a collection by looking for collection fields
                let has_gobject = reader.read_field_pointer("Collection", "gobject").is_ok();
                let has_children = reader.read_field_pointer("Collection", "children").is_ok();

                if !has_gobject && !has_children {
                    // This DATA block doesn't look like a collection, skip it
                    return Ok(ExpandResult::new(dependencies));
                }
            }

            // Try "Collection" first (Blender 2.8+)
            let gobject = reader
                .read_field_pointer("Collection", "gobject")
                .or_else(|_| reader.read_field_pointer("Group", "gobject"))
                .unwrap_or(0);

            // Children field only exists in Blender 2.8+
            let children = reader
                .read_field_pointer("Collection", "children")
                .unwrap_or(0);

            (gobject, children)
        };

        // Traverse objects in this collection (gobject.first)
        if gobject_ptr != 0 {
            traverse_object_list(gobject_ptr, blend_file, &mut dependencies)?;
        }

        // Traverse child collections (children.first) - Blender 2.8+
        if children_ptr != 0 {
            traverse_children_list(children_ptr, blend_file, &mut dependencies)?;
        }

        Ok(ExpandResult::new(dependencies))
    }

    fn can_handle(&self, code: &[u8; 4]) -> bool {
        // Handle both GR blocks (legacy/actual collections) and DATA blocks (modern collection containers)
        code == b"GR\0\0" || code == b"DATA"
    }
}

/// Traverse the linked list of CollectionObject/GroupObject items
///
/// This function handles both the newer CollectionObject (Blender 2.8+) and
/// the older GroupObject structures for backward compatibility.
fn traverse_object_list<R: Read + Seek>(
    first_ptr: u64,
    blend_file: &mut BlendFile<R>,
    dependencies: &mut Vec<usize>,
) -> Result<()> {
    let mut current_ptr = first_ptr;

    while current_ptr != 0 {
        if let Some(item_index) = blend_file.find_block_by_address(current_ptr) {
            // Extract object pointer and next pointer in a block
            let (object_ptr, next_ptr) = {
                let item_data = blend_file.read_block_data(item_index)?;
                let item_reader = blend_file.create_field_reader(&item_data)?;

                // Try both "CollectionObject" (Blender 2.8+) and "GroupObject" (older versions)
                let object = item_reader
                    .read_field_pointer("CollectionObject", "ob")
                    .or_else(|_| item_reader.read_field_pointer("GroupObject", "ob"))
                    .unwrap_or(0);

                let next = item_reader
                    .read_field_pointer("CollectionObject", "next")
                    .or_else(|_| item_reader.read_field_pointer("GroupObject", "next"))
                    .unwrap_or(0);

                (object, next)
            };

            // Add the object to dependencies
            if object_ptr != 0 {
                if let Some(object_index) = blend_file.find_block_by_address(object_ptr) {
                    dependencies.push(object_index);
                }
            }

            // Move to next item
            current_ptr = next_ptr;
        } else {
            break;
        }
    }

    Ok(())
}

/// Traverse the linked list of CollectionChild items and recursively expand child collections
///
/// This function is only used in Blender 2.8+ where Collections can have child Collections.
/// It recursively expands child collections to build the complete dependency tree.
fn traverse_children_list<R: Read + Seek>(
    first_ptr: u64,
    blend_file: &mut BlendFile<R>,
    dependencies: &mut Vec<usize>,
) -> Result<()> {
    let mut current_ptr = first_ptr;

    while current_ptr != 0 {
        if let Some(child_index) = blend_file.find_block_by_address(current_ptr) {
            // Extract the collection pointer and next pointer in a block
            let (collection_ptr, next_ptr) = {
                let child_data = blend_file.read_block_data(child_index)?;
                let child_reader = blend_file.create_field_reader(&child_data)?;

                let collection = child_reader
                    .read_field_pointer("CollectionChild", "collection")
                    .unwrap_or(0);
                let next = child_reader
                    .read_field_pointer("CollectionChild", "next")
                    .unwrap_or(0);

                (collection, next)
            };

            // Process the collection reference
            if collection_ptr != 0 {
                if let Some(collection_index) = blend_file.find_block_by_address(collection_ptr) {
                    // Recursively expand the child collection
                    let expander = CollectionExpander;
                    let child_deps = expander.expand_block(collection_index, blend_file)?;
                    dependencies.extend(child_deps.dependencies);
                }
            }

            // Move to next child
            current_ptr = next_ptr;
        } else {
            break;
        }
    }

    Ok(())
}
