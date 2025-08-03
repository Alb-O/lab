use crate::BlockExpander;
use dot001_parser::{BlendFile, Result};
use std::io::{Read, Seek};

/// Expander for Mesh (ME) blocks
///
/// Meshes contain references to materials in a materials array.
/// We need to read through the array to find all material dependencies.
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

        // Collect all pointers first to avoid borrowing conflicts
        let mut material_info = None;
        let mut geometric_pointers = Vec::new();

        // Collect material info
        if let Ok(totcol) = reader.read_field_u32("Mesh", "totcol") {
            if totcol > 0 {
                if let Ok(mats_ptr) = reader.read_field_pointer("Mesh", "mat") {
                    if mats_ptr != 0 {
                        material_info = Some((totcol, mats_ptr));
                    }
                }
            }
        }

        // Collect geometric data pointers
        let geometric_fields = [
            "vert",
            "edge",
            "poly",
            "loop",
            "vert_normals",
            "poly_normals",
            "loop_normals",
            "face_sets",
        ];

        for field_name in &geometric_fields {
            if let Ok(data_ptr) = reader.read_field_pointer("Mesh", field_name) {
                if data_ptr != 0 {
                    geometric_pointers.push(data_ptr);
                }
            }
        }

        // Now process material dependencies (after we're done with the reader)
        if let Some((totcol, mats_ptr)) = material_info {
            if let Some(mats_index) = blend_file.find_block_by_address(mats_ptr) {
                dependencies.push(mats_index);

                let mats_data = blend_file.read_block_data(mats_index)?;
                let mats_reader = blend_file.create_field_reader(&mats_data)?;

                for i in 0..totcol {
                    let offset = i as usize * blend_file.header.pointer_size as usize;
                    if let Ok(mat_ptr) = mats_reader.read_pointer(offset) {
                        if mat_ptr != 0 {
                            if let Some(mat_index) = blend_file.find_block_by_address(mat_ptr) {
                                dependencies.push(mat_index);
                            }
                        }
                    }
                }
            }
        }

        // Process geometric data dependencies
        for data_ptr in geometric_pointers {
            if let Some(data_index) = blend_file.find_block_by_address(data_ptr) {
                dependencies.push(data_index);
            }
        }

        Ok(dependencies)
    }

    fn can_handle(&self, code: &[u8; 4]) -> bool {
        code == b"ME\0\0"
    }
}
