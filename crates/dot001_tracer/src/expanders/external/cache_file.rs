use crate::{BlockExpander, ExpandResult, utils::bpath::BlendPath};
use dot001_parser::{BlendFile, Result};
use std::io::{Read, Seek};

/// Expander for Cache File (CF) blocks.
/// Cache files are used for caching simulation data (fluid, smoke, cloth, etc.).
pub struct CacheFileExpander;

impl<R: Read + Seek> BlockExpander<R> for CacheFileExpander {
    fn expand_block(
        &self,
        block_index: usize,
        blend_file: &mut BlendFile<R>,
    ) -> Result<ExpandResult> {
        let dependencies = Vec::new();
        let mut external_refs = Vec::new();

        // Read the cache file block data
        let cache_data = blend_file.read_block_data(block_index)?;
        let reader = blend_file.create_field_reader(&cache_data)?;

        // Cache files reference external data files via the "filepath" field
        if let Ok(filepath) = reader.read_field_string("CacheFile", "filepath") {
            let path_str = filepath.trim_end_matches('\0').trim();
            if !path_str.is_empty() {
                let blend_path = BlendPath::new(path_str.as_bytes());
                external_refs.push(blend_path.to_pathbuf_stripped());
            }
        }

        Ok(ExpandResult::with_externals(dependencies, external_refs))
    }

    fn can_handle(&self, code: &[u8; 4]) -> bool {
        code == b"CF\0\0"
    }
}
