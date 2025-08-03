use crate::BlockExpander;
use dot001_parser::{BlendFile, Result};
use std::io::{Read, Seek};

/// Expander for Object (OB) blocks
///
/// Objects contain references to their mesh data and materials.
/// Materials are stored in an array that we need to traverse.
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
