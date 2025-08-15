//! Thread-safe Collection block expander
//!
//! This expander handles Collection blocks (GR and DATA) and traces dependencies to:
//! - Objects within the collection
//! - Child collections (Blender 2.8+)
//!
//! Collections (formerly Groups) can contain references to objects and can have
//! child collections. This expander handles both the old Group format and the
//! newer Collection format for cross-version compatibility.

use crate::{BlockExpander, custom_expander};
use dot001_events::error::Result;

custom_expander! {
    CollectionExpander, b"GR\0\0" => |block_index, blend_file| {
        let mut dependencies = Vec::new();

        // Get block data slice for zero-copy access
        let slice = blend_file.read_block_slice_for_field_view(block_index)?;
        let view = blend_file.create_field_view(&slice)?;
        let dna = blend_file.dna()?;

        // Try both "Collection" (Blender 2.8+) and "Group" (older versions)
        let (gobject_ptr, children_ptr) = {
            let mut gobject = 0u64;
            let mut children = 0u64;

            // Try Collection struct first
            if let Some(collection_struct) = dna.structs.iter().find(|s| s.type_name == "Collection") {
                if let Some(gobject_field) = collection_struct.find_field("gobject") {
                    if gobject_field.name.is_pointer {
                        gobject = view.read_pointer(gobject_field.offset).unwrap_or(0);
                    }
                }
                if let Some(children_field) = collection_struct.find_field("children") {
                    if children_field.name.is_pointer {
                        children = view.read_pointer(children_field.offset).unwrap_or(0);
                    }
                }
            }

            // Fallback to Group struct if Collection not found or fields empty
            if gobject == 0 {
                if let Some(group_struct) = dna.structs.iter().find(|s| s.type_name == "Group") {
                    if let Some(gobject_field) = group_struct.find_field("gobject") {
                        if gobject_field.name.is_pointer {
                            gobject = view.read_pointer(gobject_field.offset).unwrap_or(0);
                        }
                    }
                }
            }

            (gobject, children)
        };

        // Traverse objects in this collection (gobject.first)
        if gobject_ptr != 0 {
            traverse_object_list_threadsafe(gobject_ptr, blend_file, &mut dependencies)?;
        }

        // Traverse child collections (children.first) - Blender 2.8+
        if children_ptr != 0 {
            traverse_children_list_threadsafe(children_ptr, blend_file, &mut dependencies)?;
        }

        dependencies
    }
}

impl CollectionExpander {
    /// Override can_handle to also accept DATA blocks
    pub fn can_handle_extended(&self, code: &[u8; 4]) -> bool {
        // Handle both GR blocks (legacy/actual collections) and DATA blocks (modern collection containers)
        code == b"GR\0\0" || code == b"DATA"
    }
}

/// Traverse the linked list of CollectionObject/GroupObject items using thread-safe access
fn traverse_object_list_threadsafe(
    first_ptr: u64,
    blend_file: &dot001_parser::BlendFileBuf,
    dependencies: &mut Vec<usize>,
) -> Result<()> {
    let mut current_ptr = first_ptr;

    while current_ptr != 0 {
        if let Some(item_index) = blend_file.address_to_block_index(current_ptr) {
            // Get item data for zero-copy access
            let item_slice = blend_file.read_block_slice_for_field_view(item_index)?;
            let item_view = blend_file.create_field_view(&item_slice)?;
            let dna = blend_file.dna()?;

            let (object_ptr, next_ptr) = {
                let mut object = 0u64;
                let mut next = 0u64;

                // Try CollectionObject first (Blender 2.8+)
                if let Some(collection_obj_struct) = dna
                    .structs
                    .iter()
                    .find(|s| s.type_name == "CollectionObject")
                {
                    if let Some(ob_field) = collection_obj_struct.find_field("ob") {
                        if ob_field.name.is_pointer {
                            object = item_view.read_pointer(ob_field.offset).unwrap_or(0);
                        }
                    }
                    if let Some(next_field) = collection_obj_struct.find_field("next") {
                        if next_field.name.is_pointer {
                            next = item_view.read_pointer(next_field.offset).unwrap_or(0);
                        }
                    }
                }

                // Fallback to GroupObject (older versions)
                if object == 0 {
                    if let Some(group_obj_struct) =
                        dna.structs.iter().find(|s| s.type_name == "GroupObject")
                    {
                        if let Some(ob_field) = group_obj_struct.find_field("ob") {
                            if ob_field.name.is_pointer {
                                object = item_view.read_pointer(ob_field.offset).unwrap_or(0);
                            }
                        }
                        if next == 0 {
                            if let Some(next_field) = group_obj_struct.find_field("next") {
                                if next_field.name.is_pointer {
                                    next = item_view.read_pointer(next_field.offset).unwrap_or(0);
                                }
                            }
                        }
                    }
                }

                (object, next)
            };

            // Add the object to dependencies
            if object_ptr != 0 {
                if let Some(object_index) = blend_file.address_to_block_index(object_ptr) {
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

/// Traverse the linked list of CollectionChild items using thread-safe access
fn traverse_children_list_threadsafe(
    first_ptr: u64,
    blend_file: &dot001_parser::BlendFileBuf,
    dependencies: &mut Vec<usize>,
) -> Result<()> {
    let mut current_ptr = first_ptr;

    while current_ptr != 0 {
        if let Some(child_index) = blend_file.address_to_block_index(current_ptr) {
            // Get child data for zero-copy access
            let child_slice = blend_file.read_block_slice_for_field_view(child_index)?;
            let child_view = blend_file.create_field_view(&child_slice)?;
            let dna = blend_file.dna()?;

            let (collection_ptr, next_ptr) = {
                let mut collection = 0u64;
                let mut next = 0u64;

                if let Some(collection_child_struct) = dna
                    .structs
                    .iter()
                    .find(|s| s.type_name == "CollectionChild")
                {
                    if let Some(collection_field) = collection_child_struct.find_field("collection")
                    {
                        if collection_field.name.is_pointer {
                            collection = child_view
                                .read_pointer(collection_field.offset)
                                .unwrap_or(0);
                        }
                    }
                    if let Some(next_field) = collection_child_struct.find_field("next") {
                        if next_field.name.is_pointer {
                            next = child_view.read_pointer(next_field.offset).unwrap_or(0);
                        }
                    }
                }

                (collection, next)
            };

            // Process the collection reference
            if collection_ptr != 0 {
                if let Some(collection_index) = blend_file.address_to_block_index(collection_ptr) {
                    // Recursively expand the child collection
                    let expander = CollectionExpander;
                    let child_result =
                        expander.expand_block_threadsafe(collection_index, blend_file)?;
                    dependencies.extend(child_result.dependencies);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::BlockExpander;

    #[test]
    fn test_collection_expander_properties() {
        let expander = CollectionExpander;
        assert_eq!(expander.block_code(), *b"GR\0\0");
        assert_eq!(expander.expander_name(), "CollectionExpander");
        assert!(expander.can_handle(b"GR\0\0"));
        assert!(!expander.can_handle(b"OB\0\0"));

        // Test extended handler
        assert!(expander.can_handle_extended(b"GR\0\0"));
        assert!(expander.can_handle_extended(b"DATA"));
        assert!(!expander.can_handle_extended(b"OB\0\0"));
    }
}
