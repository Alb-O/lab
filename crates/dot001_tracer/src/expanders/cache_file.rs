//! Thread-safe Cache File block expander
//!
//! This expander handles Cache File blocks (CF) which reference external cache data files.
//! Cache files are used for caching simulation data (fluid, smoke, cloth, etc.).

use crate::{BlockExpander, ExpandResult, utils::bpath::BlendPath};
use dot001_events::error::Result;

pub struct CacheFileExpander;

impl BlockExpander for CacheFileExpander {
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
                // Cache files reference external data files via the "filepath" field
                if let Ok(filepath) = view.read_field_string("CacheFile", "filepath") {
                    let path_str = filepath.trim_end_matches('\0').trim();
                    if !path_str.is_empty() {
                        let blend_path = BlendPath::new(path_str.as_bytes());
                        external_refs.push(blend_path.to_pathbuf_stripped());
                    }
                }
            }
        }

        Ok(ExpandResult::with_externals(dependencies, external_refs))
    }

    fn can_handle(&self, code: &[u8; 4]) -> bool {
        code == b"CF\0\0"
    }

    fn block_code(&self) -> [u8; 4] {
        *b"CF\0\0"
    }

    fn expander_name(&self) -> &'static str {
        " CacheFileExpander"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_file_expander_properties() {
        let expander = CacheFileExpander;
        assert_eq!(expander.block_code(), *b"CF\0\0");
        assert_eq!(expander.expander_name(), " CacheFileExpander");
        assert!(expander.can_handle(b"CF\0\0"));
        assert!(!expander.can_handle(b"OB\0\0"));
    }
}
