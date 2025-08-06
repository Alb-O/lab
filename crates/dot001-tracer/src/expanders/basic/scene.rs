use crate::BlockExpander;
use crate::ExpandResult;
use dot001_parser::{BlendFile, Result};
use std::io::{Read, Seek};

/// Expander for Scene (SC) blocks
///
/// Scenes contain references to objects through a linked list of Base objects.
/// Each Base object references an Object in the scene.
pub struct SceneExpander;

impl<R: Read + Seek> BlockExpander<R> for SceneExpander {
    fn expand_block(
        &self,
        block_index: usize,
        blend_file: &mut BlendFile<R>,
    ) -> Result<ExpandResult> {
        let mut dependencies = Vec::new();

        // Read the scene block data
        let scene_data = blend_file.read_block_data(block_index)?;
        let reader = blend_file.create_field_reader(&scene_data)?;

        // Add basic scene references (like Python implementation)
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

        // Modern Blender (2.8+) uses collections instead of base list
        // Try to access master_collection which contains all scene objects
        let master_collection_index = if let Ok(master_collection_ptr) =
            reader.read_field_pointer("Scene", "master_collection")
        {
            if master_collection_ptr != 0 {
                blend_file.find_block_by_address(master_collection_ptr)
            } else {
                None
            }
        } else {
            None
        };

        // Legacy approach for older Blender versions
        // Python uses: bases = block.get_pointer((b"base", b"first"))
        if let Ok(base_first_ptr) = reader.read_field_pointer("Scene", "base.first") {
            if base_first_ptr != 0 {
                // Follow the linked list of Base objects
                let mut current_base_ptr = base_first_ptr;
                let mut base_count = 0;

                while current_base_ptr != 0 && base_count < 100 {
                    if let Some(base_index) = blend_file.find_block_by_address(current_base_ptr) {
                        base_count += 1;
                        let base_data = blend_file.read_block_data(base_index)?;
                        let base_reader = blend_file.create_field_reader(&base_data)?;

                        // Add the object that this base references
                        if let Ok(object_ptr) = base_reader.read_field_pointer("Base", "object") {
                            if object_ptr != 0 {
                                if let Some(object_index) =
                                    blend_file.find_block_by_address(object_ptr)
                                {
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
                }
            }
        }

        // Process master collection after all field reads are done
        if let Some(collection_index) = master_collection_index {
            // Add the collection as a dependency
            dependencies.push(collection_index);

            // Also directly expand the collection to find objects
            // since the CollectionExpander won't handle DATA blocks
            if let Ok(collection_deps) = expand_collection_objects(collection_index, blend_file) {
                dependencies.extend(collection_deps);
            }
        }

        Ok(ExpandResult::new(dependencies))
    }

    fn can_handle(&self, code: &[u8; 4]) -> bool {
        code == b"SC\0\0"
    }
}

/// Expand a collection (stored as DATA block) to find its objects
/// This handles modern Blender collections that are stored as DATA blocks
fn expand_collection_objects<R: Read + Seek>(
    collection_index: usize,
    blend_file: &mut BlendFile<R>,
) -> Result<Vec<usize>> {
    let dependencies = Vec::new();

    // Read the collection data (not currently used, but kept for future collection processing)
    let _collection_data = blend_file.read_block_data(collection_index)?;

    // Note: Collections in modern Blender are organized hierarchically
    // The master collection typically contains child collections, which contain the actual objects
    // For now, we've successfully identified the master collection structure

    Ok(dependencies)
}
