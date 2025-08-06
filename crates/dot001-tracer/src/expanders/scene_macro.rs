/// Scene expander implemented using the custom_expander macro
///
/// This demonstrates how the macro can be used for fully custom expansion logic
/// while still reducing the boilerplate of the BlockExpander trait implementation.
use crate::custom_expander;

custom_expander! {
    SceneExpanderMacro, b"SC\0\0" => |block_index, blend_file| {
        let mut dependencies = Vec::new();

        // Read the scene block data - use if let to avoid early returns
        if let Ok(scene_data) = blend_file.read_block_data(block_index) {
            if let Ok(reader) = blend_file.create_field_reader(&scene_data) {
                // Add basic scene references
                // Camera
                if let Ok(camera_ptr) = reader.read_field_pointer("Scene", "camera") {
                    if camera_ptr != 0 {
                        if let Some(camera_index) = blend_file.find_block_by_address(camera_ptr) {
                            dependencies.push(camera_index);
                        }
                    }
                }

                // World
                if let Ok(world_ptr) = reader.read_field_pointer("Scene", "world") {
                    if world_ptr != 0 {
                        if let Some(world_index) = blend_file.find_block_by_address(world_ptr) {
                            dependencies.push(world_index);
                        }
                    }
                }

                // Set (background scene)
                if let Ok(set_ptr) = reader.read_field_pointer("Scene", "set") {
                    if set_ptr != 0 {
                        if let Some(set_index) = blend_file.find_block_by_address(set_ptr) {
                            dependencies.push(set_index);
                        }
                    }
                }

                // Clip (movie clip)
                if let Ok(clip_ptr) = reader.read_field_pointer("Scene", "clip") {
                    if clip_ptr != 0 {
                        if let Some(clip_index) = blend_file.find_block_by_address(clip_ptr) {
                            dependencies.push(clip_index);
                        }
                    }
                }

                // Modern Blender (2.8+) master collection
                if let Ok(master_collection_ptr) = reader.read_field_pointer("Scene", "master_collection") {
                    if master_collection_ptr != 0 {
                        if let Some(collection_index) = blend_file.find_block_by_address(master_collection_ptr) {
                            dependencies.push(collection_index);
                        }
                    }
                }

                // Legacy approach for older Blender versions - base list traversal
                if let Ok(base_first_ptr) = reader.read_field_pointer("Scene", "base.first") {
                    if base_first_ptr != 0 {
                        // Follow the linked list of Base objects
                        let mut current_base_ptr = base_first_ptr;
                        let mut base_count = 0;

                        while current_base_ptr != 0 && base_count < 100 {
                            if let Some(base_index) = blend_file.find_block_by_address(current_base_ptr) {
                                base_count += 1;
                                if let Ok(base_data) = blend_file.read_block_data(base_index) {
                                    if let Ok(base_reader) = blend_file.create_field_reader(&base_data) {
                                        // Add the object that this base references
                                        if let Ok(object_ptr) = base_reader.read_field_pointer("Base", "object") {
                                            if object_ptr != 0 {
                                                if let Some(object_index) = blend_file.find_block_by_address(object_ptr) {
                                                    dependencies.push(object_index);
                                                }
                                            }
                                        }

                                        // Get the next base in the linked list
                                        if let Ok(next_ptr) = base_reader.read_field_pointer("Base", "next") {
                                            current_base_ptr = next_ptr;
                                        } else {
                                            break;
                                        }
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
                    }
                }
            }
        }

        dependencies
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scene_expander_macro_can_handle() {
        let expander = SceneExpanderMacro;
        assert!(expander.can_handle(b"SC\0\0"));
        assert!(!expander.can_handle(b"OB\0\0"));
    }
}
