//! Thread-safe Image block expander
//!
//! This expander handles Image blocks (IM) which reference external image files.
//! Images can be packed (embedded in the blend file) or reference external files.

use crate::{BlockExpander, ExpandResult, utils::bpath::BlendPath};
use dot001_events::error::Result;

pub struct ImageExpander;

impl BlockExpander for ImageExpander {
    fn expand_block_threadsafe(
        &self,
        block_index: usize,
        blend_file: &dot001_parser::BlendFileBuf,
    ) -> Result<ExpandResult> {
        let dependencies = Vec::new();
        let mut external_refs = Vec::new();

        // Get block data slice for zero-copy access
        if let Ok(slice) = blend_file.read_block_slice_for_field_view(block_index) {
            if let Ok(view) = blend_file.create_field_view(&slice) {
                if let Ok(dna) = blend_file.dna() {
                    if let Some(image_struct) = dna.structs.iter().find(|s| s.type_name == "Image")
                    {
                        // Skip packed images - they don't have external file dependencies
                        let is_packed =
                            if let Some(packedfile_field) = image_struct.find_field("packedfile") {
                                if packedfile_field.name.is_pointer {
                                    view.read_pointer(packedfile_field.offset).unwrap_or(0) != 0
                                } else {
                                    false
                                }
                            } else {
                                false
                            };

                        if !is_packed {
                            // Check image source type to determine if we should process this image
                            // IMA_SRC_FILE = 1, IMA_SRC_SEQUENCE = 2, IMA_SRC_MOVIE = 3, IMA_SRC_TILED = 5
                            if let Some(source_field) = image_struct.find_field("source") {
                                if let Ok(source) = view.read_u32(source_field.offset) {
                                    if matches!(source, 1 | 2 | 3 | 5) {
                                        // These are file-based sources that we need to track
                                        // The actual file path is stored in the "filepath" field
                                        if let Ok(filepath) =
                                            view.read_field_string("Image", "filepath")
                                        {
                                            let path_str = filepath.trim_end_matches('\0').trim();
                                            if !path_str.is_empty() {
                                                let blend_path =
                                                    BlendPath::new(path_str.as_bytes());
                                                external_refs
                                                    .push(blend_path.to_pathbuf_stripped());
                                            }
                                        }

                                        // For image sequences, we might want to detect patterns like "image_####.png"
                                        // and potentially expand to multiple files, but for now we just track the pattern
                                        if source == 2 {
                                            // Image sequence - the filepath contains the pattern
                                            // We could expand this in the future to track all files in the sequence
                                        }
                                    }
                                    // Other source types (generated images, viewer nodes, etc.) - no external files
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(ExpandResult::with_externals(dependencies, external_refs))
    }

    fn can_handle(&self, code: &[u8; 4]) -> bool {
        code == b"IM\0\0"
    }

    fn block_code(&self) -> [u8; 4] {
        *b"IM\0\0"
    }

    fn expander_name(&self) -> &'static str {
        "ImageExpander"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_image_expander_properties() {
        let expander = ImageExpander;
        assert_eq!(expander.block_code(), *b"IM\0\0");
        assert_eq!(expander.expander_name(), "ImageExpander");
        assert!(expander.can_handle(b"IM\0\0"));
        assert!(!expander.can_handle(b"OB\0\0"));
    }
}
