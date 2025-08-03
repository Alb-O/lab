use crate::BlockExpander;
use bllink_parser::{BlendFile, Result};
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
    ) -> Result<Vec<usize>> {
        let mut dependencies = Vec::new();

        // Read the scene block data
        let scene_data = blend_file.read_block_data(block_index)?;
        let reader = blend_file.create_field_reader(&scene_data)?;

        // Read the base list pointer (Scene.base.first)
        if let Ok(base_ptr) = reader.read_field_pointer("Scene", "base") {
            if base_ptr != 0 {
                // Find the block that contains this base
                if let Some(base_block_index) = blend_file.find_block_by_address(base_ptr) {
                    dependencies.push(base_block_index);

                    // Follow the linked list of Base objects
                    let mut current_base_ptr = base_ptr;

                    while current_base_ptr != 0 {
                        if let Some(base_index) = blend_file.find_block_by_address(current_base_ptr)
                        {
                            let base_data = blend_file.read_block_data(base_index)?;
                            let base_reader = blend_file.create_field_reader(&base_data)?;

                            // Add the object that this base references
                            if let Ok(object_ptr) = base_reader.read_field_pointer("Base", "object")
                            {
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
        }

        Ok(dependencies)
    }

    fn can_handle(&self, code: &[u8; 4]) -> bool {
        code == b"SC\0\0"
    }
}
