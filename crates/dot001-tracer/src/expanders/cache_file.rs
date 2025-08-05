use crate::BlockExpander;
use crate::ExpandResult;
use dot001_parser::{BlendFile, Result};
use std::io::{Read, Seek};
use std::path::PathBuf;

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
                // Convert Blender's path format (which might use '//' prefix for relative paths)
                let cleaned_path = path_str.strip_prefix("//").unwrap_or(path_str);
                external_refs.push(PathBuf::from(cleaned_path));
            }
        }

        Ok(ExpandResult::with_externals(dependencies, external_refs))
    }

    fn can_handle(&self, code: &[u8; 4]) -> bool {
        code == b"CF\0\0"
    }
}
