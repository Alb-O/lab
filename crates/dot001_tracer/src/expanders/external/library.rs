/// Modernized Library expander using external_expander macro with fallback logic
use crate::external_expander;

external_expander! {
    LibraryExpander, b"LI\0\0", "Library" => {
        custom_external: |block_index, blend_file, dependencies, external_refs| {
            // Read the library block data
            if let Ok(library_data) = blend_file.read_block_data(block_index) {
                if let Ok(reader) = blend_file.create_field_reader(&library_data) {
                    // Libraries contain file paths to external .blend files in the "filepath" field
                    if let Ok(filepath) = reader.read_field_string("Library", "filepath") {
                        let path_str = filepath.trim_end_matches('\0').trim();
                        if !path_str.is_empty() {
                            let blend_path = crate::utils::bpath::BlendPath::new(path_str.as_bytes());
                            external_refs.push(blend_path.to_pathbuf_stripped());
                        }
                    }

                    // Also try the "name" field as fallback (older Blender versions might use this)
                    if external_refs.is_empty() {
                        if let Ok(name) = reader.read_field_string("Library", "name") {
                            let path_str = name.trim_end_matches('\0').trim();
                            if !path_str.is_empty() {
                                let blend_path = crate::utils::bpath::BlendPath::new(path_str.as_bytes());
                                external_refs.push(blend_path.to_pathbuf_stripped());
                            }
                        }
                    }
                }
            }
        }
    }
}
