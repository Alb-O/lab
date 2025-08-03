use crate::BlockExpander;
use dot001_parser::{BlendFile, Result};
use std::io::{Read, Seek};

/// Expander for Texture (TE) blocks
///
/// Textures in the legacy texture system can reference images and other data.
/// This expander handles the dependencies from textures to their image data.
pub struct TextureExpander;

impl<R: Read + Seek> BlockExpander<R> for TextureExpander {
    fn expand_block(
        &self,
        block_index: usize,
        blend_file: &mut BlendFile<R>,
    ) -> Result<Vec<usize>> {
        let mut dependencies = Vec::new();

        // Read the texture block data
        let texture_data = blend_file.read_block_data(block_index)?;
        let reader = blend_file.create_field_reader(&texture_data)?;

        // Check texture type to determine what kind of data it uses
        if let Ok(tex_type) = reader.read_field_u32("Tex", "type") {
            match tex_type {
                0 => {
                    // TEX_IMAGE = 0 - Image texture
                    if let Ok(ima_ptr) = reader.read_field_pointer("Tex", "ima") {
                        if ima_ptr != 0 {
                            if let Some(ima_index) = blend_file.find_block_by_address(ima_ptr) {
                                dependencies.push(ima_index);
                            }
                        }
                    }
                }
                14 => {
                    // TEX_VOXELDATA = 14 - Voxel data texture
                    if let Ok(vd_ptr) = reader.read_field_pointer("Tex", "vd") {
                        if vd_ptr != 0 {
                            if let Some(vd_index) = blend_file.find_block_by_address(vd_ptr) {
                                dependencies.push(vd_index);
                            }
                        }
                    }
                }
                15 => {
                    // TEX_POINTDENSITY = 15 - Point density texture
                    if let Ok(pd_ptr) = reader.read_field_pointer("Tex", "pd") {
                        if pd_ptr != 0 {
                            if let Some(pd_index) = blend_file.find_block_by_address(pd_ptr) {
                                dependencies.push(pd_index);
                            }
                        }
                    }
                }
                16 => {
                    // TEX_OCEAN = 16 - Ocean texture
                    if let Ok(ot_ptr) = reader.read_field_pointer("Tex", "ot") {
                        if ot_ptr != 0 {
                            if let Some(ot_index) = blend_file.find_block_by_address(ot_ptr) {
                                dependencies.push(ot_index);
                            }
                        }
                    }
                }
                _ => {
                    // Other texture types (procedural textures, etc.) typically don't
                    // have block dependencies, just parameters
                }
            }
        }

        // Check for node tree (in case this texture uses nodes)
        if let Ok(nodetree_ptr) = reader.read_field_pointer("Tex", "nodetree") {
            if nodetree_ptr != 0 {
                if let Some(nodetree_index) = blend_file.find_block_by_address(nodetree_ptr) {
                    dependencies.push(nodetree_index);
                }
            }
        }

        Ok(dependencies)
    }

    fn can_handle(&self, code: &[u8; 4]) -> bool {
        code == b"TE\0\0"
    }
}
