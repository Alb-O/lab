//! Thread-safe Library block expander
//!
//! This expander handles Library blocks (LI) which reference external .blend files.

use crate::{BlockExpander, ExpandResult, utils::bpath::BlendPath};
use dot001_events::error::Result;

pub struct LibraryExpander;

impl BlockExpander for LibraryExpander {
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
                // Libraries contain file paths to external .blend files in the "filepath" field
                if let Ok(filepath) = view.read_field_string("Library", "filepath") {
                    let path_str = filepath.trim_end_matches('\0').trim();
                    if !path_str.is_empty() {
                        let blend_path = BlendPath::new(path_str.as_bytes());
                        external_refs.push(blend_path.to_pathbuf_stripped());
                    }
                }

                // Also try the "name" field as fallback (older Blender versions might use this)
                if external_refs.is_empty() {
                    if let Ok(name) = view.read_field_string("Library", "name") {
                        let path_str = name.trim_end_matches('\0').trim();
                        if !path_str.is_empty() {
                            let blend_path = BlendPath::new(path_str.as_bytes());
                            external_refs.push(blend_path.to_pathbuf_stripped());
                        }
                    }
                }
            }
        }

        Ok(ExpandResult::with_externals(dependencies, external_refs))
    }

    fn can_handle(&self, code: &[u8; 4]) -> bool {
        code == b"LI\0\0"
    }

    fn block_code(&self) -> [u8; 4] {
        *b"LI\0\0"
    }

    fn expander_name(&self) -> &'static str {
        "LibraryExpander"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_library_expander_properties() {
        let expander = LibraryExpander;
        assert_eq!(expander.block_code(), *b"LI\0\0");
        assert_eq!(expander.expander_name(), "LibraryExpander");
        assert!(expander.can_handle(b"LI\0\0"));
        assert!(!expander.can_handle(b"OB\0\0"));
    }
}
