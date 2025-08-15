//! Thread-safe Texture block expander
//!
//! This expander handles Texture blocks (TE) and traces dependencies to:
//! - nodetree: Node tree for shader nodes
//! - Custom type-specific dependencies (images, voxel data, etc.)

use crate::hybrid_expander;

hybrid_expander! {
    TextureExpander, b"TE\0\0", "Tex" => {
        single_fields: ["nodetree"],
        array_fields: [],
        custom: |block_index, blend_file, dependencies| {
            // Get block data slice for zero-copy access
            if let Ok(slice) = blend_file.read_block_slice_for_field_view(block_index) {
                if let Ok(view) = blend_file.create_field_view(&slice) {
                    if let Ok(dna) = blend_file.dna() {
                        if let Some(tex_struct) = dna.structs.iter().find(|s| s.type_name == "Tex") {
                            // Check texture type to determine what kind of data it uses
                            if let Some(type_field) = tex_struct.find_field("type") {
                                if let Ok(tex_type) = view.read_u32(type_field.offset) {
                                    match tex_type {
                                        0 => {
                                            // TEX_IMAGE = 0 - Image texture
                                            if let Some(ima_field) = tex_struct.find_field("ima") {
                                                if ima_field.name.is_pointer {
                                                    if let Ok(ima_ptr) = view.read_pointer(ima_field.offset) {
                                                        if ima_ptr != 0 {
                                                            if let Some(ima_index) = blend_file.address_to_block_index(ima_ptr) {
                                                                dependencies.push(ima_index);
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                        14 => {
                                            // TEX_VOXELDATA = 14 - Voxel data texture
                                            if let Some(vd_field) = tex_struct.find_field("vd") {
                                                if vd_field.name.is_pointer {
                                                    if let Ok(vd_ptr) = view.read_pointer(vd_field.offset) {
                                                        if vd_ptr != 0 {
                                                            if let Some(vd_index) = blend_file.address_to_block_index(vd_ptr) {
                                                                dependencies.push(vd_index);
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                        15 => {
                                            // TEX_POINTDENSITY = 15 - Point density texture
                                            if let Some(pd_field) = tex_struct.find_field("pd") {
                                                if pd_field.name.is_pointer {
                                                    if let Ok(pd_ptr) = view.read_pointer(pd_field.offset) {
                                                        if pd_ptr != 0 {
                                                            if let Some(pd_index) = blend_file.address_to_block_index(pd_ptr) {
                                                                dependencies.push(pd_index);
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                        16 => {
                                            // TEX_OCEAN = 16 - Ocean texture
                                            if let Some(ot_field) = tex_struct.find_field("ot") {
                                                if ot_field.name.is_pointer {
                                                    if let Ok(ot_ptr) = view.read_pointer(ot_field.offset) {
                                                        if ot_ptr != 0 {
                                                            if let Some(ot_index) = blend_file.address_to_block_index(ot_ptr) {
                                                                dependencies.push(ot_index);
                                                            }
                                                        }
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
                            }
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::BlockExpander;

    #[test]
    fn test_texture_expander_properties() {
        let expander = TextureExpander;
        assert_eq!(expander.block_code(), *b"TE\0\0");
        assert_eq!(expander.expander_name(), "TextureExpander");
        assert!(expander.can_handle(b"TE\0\0"));
        assert!(!expander.can_handle(b"OB\0\0"));
    }
}
