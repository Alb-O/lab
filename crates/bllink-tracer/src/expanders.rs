use crate::BlockExpander;
use bllink_parser::{BlendFile, Result};
use std::io::{Read, Seek};

/// Expander for Scene (SC) blocks
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

/// Expander for Object (OB) blocks
pub struct ObjectExpander;

impl<R: Read + Seek> BlockExpander<R> for ObjectExpander {
    fn expand_block(
        &self,
        block_index: usize,
        blend_file: &mut BlendFile<R>,
    ) -> Result<Vec<usize>> {
        let mut dependencies = Vec::new();

        // Read the object block data
        let object_data = blend_file.read_block_data(block_index)?;
        let reader = blend_file.create_field_reader(&object_data)?;

        // Add mesh data dependency
        if let Ok(data_ptr) = reader.read_field_pointer("Object", "data") {
            if data_ptr != 0 {
                if let Some(data_index) = blend_file.find_block_by_address(data_ptr) {
                    dependencies.push(data_index);
                }
            }
        }

        // Add material dependencies - read the materials array
        if let Ok(totcol) = reader.read_field_u32("Object", "totcol") {
            if totcol > 0 {
                if let Ok(mats_ptr) = reader.read_field_pointer("Object", "mat") {
                    if mats_ptr != 0 {
                        // Read the materials array block
                        if let Some(mats_index) = blend_file.find_block_by_address(mats_ptr) {
                            let mats_data = blend_file.read_block_data(mats_index)?;
                            let mats_reader = blend_file.create_field_reader(&mats_data)?;

                            // Read each material pointer in the array
                            for i in 0..totcol {
                                let offset = i as usize * blend_file.header.pointer_size as usize;
                                if let Ok(mat_ptr) = mats_reader.read_pointer(offset) {
                                    if mat_ptr != 0 {
                                        if let Some(mat_index) =
                                            blend_file.find_block_by_address(mat_ptr)
                                        {
                                            dependencies.push(mat_index);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(dependencies)
    }

    fn can_handle(&self, code: &[u8; 4]) -> bool {
        code == b"OB\0\0"
    }
}

/// Expander for Mesh (ME) blocks
pub struct MeshExpander;

impl<R: Read + Seek> BlockExpander<R> for MeshExpander {
    fn expand_block(
        &self,
        block_index: usize,
        blend_file: &mut BlendFile<R>,
    ) -> Result<Vec<usize>> {
        let mut dependencies = Vec::new();

        // Read the mesh block data
        let mesh_data = blend_file.read_block_data(block_index)?;
        let reader = blend_file.create_field_reader(&mesh_data)?;

        // Add material dependencies
        if let Ok(totcol) = reader.read_field_u32("Mesh", "totcol") {
            if totcol > 0 {
                if let Ok(mats_ptr) = reader.read_field_pointer("Mesh", "mat") {
                    if mats_ptr != 0 {
                        if let Some(mats_index) = blend_file.find_block_by_address(mats_ptr) {
                            let mats_data = blend_file.read_block_data(mats_index)?;
                            let mats_reader = blend_file.create_field_reader(&mats_data)?;

                            for i in 0..totcol {
                                let offset = i as usize * blend_file.header.pointer_size as usize;
                                if let Ok(mat_ptr) = mats_reader.read_pointer(offset) {
                                    if mat_ptr != 0 {
                                        if let Some(mat_index) =
                                            blend_file.find_block_by_address(mat_ptr)
                                        {
                                            dependencies.push(mat_index);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(dependencies)
    }

    fn can_handle(&self, code: &[u8; 4]) -> bool {
        code == b"ME\0\0"
    }
}
