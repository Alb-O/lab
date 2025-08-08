//! Thread-safe Group/Collection block expander
//!
//! This expander handles Group blocks (GR) and traces dependencies to:
//! - gobject: Linked list of objects in the group/collection
//! - children: Linked list of child collections (Blender 2.8+)

use crate::thread_safe_custom_expander;

thread_safe_custom_expander! {
    ThreadSafeGroupExpander, b"GR\0\0" => |block_index, blend_file| {
        let mut dependencies = Vec::new();

        // Get block data slice
        if let Ok(slice) = blend_file.read_block_slice_for_field_view(block_index) {
            if let Ok(view) = blend_file.create_field_view(&slice) {
                if let Ok(dna) = blend_file.dna() {
                    // Try both "Collection" (Blender 2.8+) and "Group" (older versions)
                    let struct_names = ["Collection", "Group"];

                    for struct_name in &struct_names {
                        if let Some(struct_def) = dna.structs.iter().find(|s| &s.type_name == struct_name) {

                            // Read gobject pointer (linked list of objects)
                            if let Some(gobject_field) = struct_def.find_field("gobject") {
                                if gobject_field.name.is_pointer {
                                    if let Ok(gobject_ptr) = view.read_pointer(gobject_field.offset) {
                                        if gobject_ptr != 0 {
                                            // Traverse the linked list of objects
                                            traverse_object_list(gobject_ptr, blend_file, &mut dependencies);
                                        }
                                    }
                                } else {
                                    // In newer Blender versions, gobject is a ListBase structure
                                    // ListBase has 'first' and 'last' pointer fields
                                    if let Ok(first_ptr) = view.read_pointer(gobject_field.offset) {
                                        if first_ptr != 0 {
                                            traverse_object_list(first_ptr, blend_file, &mut dependencies);
                                        }
                                    }
                                }
                            }

                            // Read children pointer (linked list of child collections) - Blender 2.8+
                            if let Some(children_field) = struct_def.find_field("children") {
                                if children_field.name.is_pointer {
                                    if let Ok(children_ptr) = view.read_pointer(children_field.offset) {
                                        if children_ptr != 0 {
                                            // Traverse the linked list of child collections
                                            traverse_children_list(children_ptr, blend_file, &mut dependencies);
                                        }
                                    }
                                } else {
                                    // Children might also be a ListBase structure
                                    if let Ok(first_ptr) = view.read_pointer(children_field.offset) {
                                        if first_ptr != 0 {
                                            traverse_children_list(first_ptr, blend_file, &mut dependencies);
                                        }
                                    }
                                }
                            }

                            // If we found a matching struct, break
                            break;
                        }
                    }
                }
            }
        }

        dependencies
    }
}

/// Traverse the linked list of CollectionObject/GroupObject items
fn traverse_object_list(
    first_ptr: u64,
    blend_file: &dot001_parser::BlendFileBuf,
    dependencies: &mut Vec<usize>,
) {
    let mut current_ptr = first_ptr;

    while current_ptr != 0 {
        if let Some(item_index) = blend_file.address_to_block_index(current_ptr) {
            // Extract object pointer and next pointer
            if let Ok(slice) = blend_file.read_block_slice_for_field_view(item_index) {
                if let Ok(view) = blend_file.create_field_view(&slice) {
                    if let Ok(dna) = blend_file.dna() {
                        let struct_names = ["CollectionObject", "GroupObject"];
                        let mut object_ptr = 0;
                        let mut next_ptr = 0;

                        for struct_name in &struct_names {
                            if let Some(struct_def) =
                                dna.structs.iter().find(|s| &s.type_name == struct_name)
                            {
                                // Get object pointer
                                if let Some(ob_field) = struct_def.find_field("ob") {
                                    if ob_field.name.is_pointer {
                                        if let Ok(ptr) = view.read_pointer(ob_field.offset) {
                                            object_ptr = ptr;
                                        }
                                    }
                                }

                                // Get next pointer
                                if let Some(next_field) = struct_def.find_field("next") {
                                    if next_field.name.is_pointer {
                                        if let Ok(ptr) = view.read_pointer(next_field.offset) {
                                            next_ptr = ptr;
                                        }
                                    }
                                }

                                break;
                            }
                        }

                        // Add the object to dependencies
                        if object_ptr != 0 {
                            if let Some(object_index) =
                                blend_file.address_to_block_index(object_ptr)
                            {
                                dependencies.push(object_index);
                            }
                        }

                        // Move to next item
                        current_ptr = next_ptr;
                    }
                }
            } else {
                break;
            }
        } else {
            break;
        }
    }
}

/// Traverse the linked list of CollectionChild items (Blender 2.8+ only)
fn traverse_children_list(
    first_ptr: u64,
    blend_file: &dot001_parser::BlendFileBuf,
    dependencies: &mut Vec<usize>,
) {
    let mut current_ptr = first_ptr;

    while current_ptr != 0 {
        if let Some(child_index) = blend_file.address_to_block_index(current_ptr) {
            // Extract collection pointer and next pointer
            if let Ok(slice) = blend_file.read_block_slice_for_field_view(child_index) {
                if let Ok(view) = blend_file.create_field_view(&slice) {
                    if let Ok(dna) = blend_file.dna() {
                        if let Some(struct_def) = dna
                            .structs
                            .iter()
                            .find(|s| s.type_name == "CollectionChild")
                        {
                            let mut collection_ptr = 0;
                            let mut next_ptr = 0;

                            // Get collection pointer
                            if let Some(collection_field) = struct_def.find_field("collection") {
                                if collection_field.name.is_pointer {
                                    if let Ok(ptr) = view.read_pointer(collection_field.offset) {
                                        collection_ptr = ptr;
                                    }
                                }
                            }

                            // Get next pointer
                            if let Some(next_field) = struct_def.find_field("next") {
                                if next_field.name.is_pointer {
                                    if let Ok(ptr) = view.read_pointer(next_field.offset) {
                                        next_ptr = ptr;
                                    }
                                }
                            }

                            // Add the child collection to dependencies
                            if collection_ptr != 0 {
                                if let Some(collection_index) =
                                    blend_file.address_to_block_index(collection_ptr)
                                {
                                    dependencies.push(collection_index);
                                }
                            }

                            // Move to next child
                            current_ptr = next_ptr;
                        }
                    }
                }
            } else {
                break;
            }
        } else {
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ThreadSafeBlockExpander;

    #[test]
    fn test_group_expander_properties() {
        let expander = ThreadSafeGroupExpander;
        assert_eq!(expander.block_code(), *b"GR\0\0");
        assert_eq!(expander.expander_name(), "ThreadSafeGroupExpander");
        assert!(expander.can_handle(b"GR\0\0"));
        assert!(!expander.can_handle(b"OB\0\0"));
    }
}
