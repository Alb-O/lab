/// Modernized Sound expander using custom_expander macro
use crate::{bpath::BlendPath, custom_expander};

custom_expander! {
    SoundExpander, b"SO\0\0" => |block_index, blend_file| {
        let dependencies = Vec::new();
        let mut external_refs = Vec::new();

        // Read the sound block data for external file handling
        if let Ok(sound_data) = blend_file.read_block_data(block_index) {
            if let Ok(reader) = blend_file.create_field_reader(&sound_data) {
                // Sound files reference external audio files in the "filepath" field
                if let Ok(filepath) = reader.read_field_string("bSound", "filepath") {
                    let path_str = filepath.trim_end_matches('\0').trim();
                    if !path_str.is_empty() {
                        let blend_path = BlendPath::new(path_str.as_bytes());
                        external_refs.push(blend_path.to_pathbuf_stripped());
                    }
                }

                // Check if the sound is packed (has internal data)
                if let Ok(packedfile_ptr) = reader.read_field_pointer("bSound", "packedfile") {
                    if packedfile_ptr != 0 {
                        // Sound is packed, clear external references since it doesn't depend on external files
                        external_refs.clear();
                    }
                }
            }
        }

        // Note: The macro generates ExpandResult::new(dependencies) by default
        // We need to manually create the result with externals
        // This expander will need special handling or we should create a new macro type for externals
        dependencies
    }
}
