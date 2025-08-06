/// Modernized Image expander using external_expander macro with custom logic
use crate::external_expander;

external_expander! {
    ImageExpander, b"IM\0\0", "Image" => {
        custom_external: |block_index, blend_file, dependencies, external_refs| {
            // Read the image block data
            if let Ok(image_data) = blend_file.read_block_data(block_index) {
                if let Ok(reader) = blend_file.create_field_reader(&image_data) {
                    // Skip packed images - they don't have external file dependencies
                    let is_packed = if let Ok(packedfile_ptr) = reader.read_field_pointer("Image", "packedfile") {
                        packedfile_ptr != 0
                    } else {
                        false
                    };

                    if !is_packed {
                        // Check image source type to determine if we should process this image
                        // IMA_SRC_FILE = 1, IMA_SRC_SEQUENCE = 2, IMA_SRC_MOVIE = 3, IMA_SRC_TILED = 5
                        if let Ok(source) = reader.read_field_u32("Image", "source") {
                            if matches!(source, 1 | 2 | 3 | 5) {
                                // These are file-based sources that we need to track
                                // The actual file path is stored in the "filepath" field
                                if let Ok(filepath) = reader.read_field_string("Image", "filepath") {
                                    let path_str = filepath.trim_end_matches('\0').trim();
                                    if !path_str.is_empty() {
                                        let blend_path = crate::utils::bpath::BlendPath::new(path_str.as_bytes());
                                        external_refs.push(blend_path.to_pathbuf_stripped());
                                    }
                                }

                                // For image sequences, we might want to detect patterns like "image_####.png"
                                // and potentially expand to multiple files, but for now we just track the pattern
                                if source == 2 {
                                    // Image sequence - the filepath contains the pattern
                                    // We could expand this in the future to track all files in the sequence
                                }
                            } else {
                                // Generated images, viewer nodes, etc. - no external files
                            }
                        }
                    }
                }
            }
        }
    }
}
