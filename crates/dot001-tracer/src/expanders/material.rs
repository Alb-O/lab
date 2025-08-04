use crate::BlockExpander;
use crate::ExpandResult;
use dot001_parser::{BlendFile, Result};
use std::io::{Read, Seek};

/// Expander for Material (MA) blocks
///
/// Materials can reference textures, images, and other materials through their node trees.
/// This expander handles both the legacy material system and the newer node-based materials.
pub struct MaterialExpander;

impl<R: Read + Seek> BlockExpander<R> for MaterialExpander {
    fn expand_block(
        &self,
        block_index: usize,
        blend_file: &mut BlendFile<R>,
    ) -> Result<ExpandResult> {
        let mut dependencies = Vec::new();

        // Read the material block data
        let material_data = blend_file.read_block_data(block_index)?;
        let reader = blend_file.create_field_reader(&material_data)?;

        // Check for node tree (Shader Editor nodes) - collect pointer first
        let nodetree_ptr = reader
            .read_field_pointer("Material", "nodetree")
            .unwrap_or(0);

        // Legacy material system - check for texture slots
        // Materials can have multiple texture slots (mtex array)
        // Extract all mtex pointers first to avoid borrowing conflicts
        let mut mtex_pointers = Vec::new();
        for i in 0..18 {
            // MAX_MTEX is typically 18 in Blender
            if let Ok(mtex_ptr) = reader.read_field_pointer("Material", &format!("mtex[{i}]")) {
                if mtex_ptr != 0 {
                    mtex_pointers.push(mtex_ptr);
                }
            }
        }

        // Now process the mtex pointers
        for mtex_ptr in mtex_pointers {
            if let Some(mtex_index) = blend_file.find_block_by_address(mtex_ptr) {
                // Read the MTex block to get the texture reference
                let mtex_data = blend_file.read_block_data(mtex_index)?;
                let mtex_reader = blend_file.create_field_reader(&mtex_data)?;

                if let Ok(tex_ptr) = mtex_reader.read_field_pointer("MTex", "tex") {
                    if tex_ptr != 0 {
                        if let Some(tex_index) = blend_file.find_block_by_address(tex_ptr) {
                            dependencies.push(tex_index);
                        }
                    }
                }
            }
        }

        // Process nodetree if found
        if nodetree_ptr != 0 {
            if let Some(nodetree_index) = blend_file.find_block_by_address(nodetree_ptr) {
                dependencies.push(nodetree_index);
            }
        }

        Ok(ExpandResult::new(dependencies))
    }

    fn can_handle(&self, code: &[u8; 4]) -> bool {
        code == b"MA\0\0"
    }
}
