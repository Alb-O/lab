/// Modernized Material expander using hybrid_expander macro
use crate::hybrid_expander;

hybrid_expander! {
    MaterialExpander, b"MA\0\0", "Material" => {
        single_fields: ["nodetree"],
        array_fields: [],
        custom: |block_index, blend_file, dependencies| {
            // Legacy material system - check for texture slots (mtex array)
            if let Ok(material_data) = blend_file.read_block_data(block_index) {
                if let Ok(reader) = blend_file.create_field_reader(&material_data) {
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

                    // Process the mtex pointers to find texture dependencies
                    for mtex_ptr in mtex_pointers {
                        if let Some(mtex_index) = blend_file.find_block_by_address(mtex_ptr) {
                            // Read the MTex block to get the texture reference
                            if let Ok(mtex_data) = blend_file.read_block_data(mtex_index) {
                                if let Ok(mtex_reader) = blend_file.create_field_reader(&mtex_data) {
                                    if let Ok(tex_ptr) = mtex_reader.read_field_pointer("MTex", "tex") {
                                        if tex_ptr != 0 {
                                            if let Some(tex_index) = blend_file.find_block_by_address(tex_ptr) {
                                                dependencies.push(tex_index);
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
}
