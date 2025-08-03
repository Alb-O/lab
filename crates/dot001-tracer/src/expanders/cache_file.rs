use crate::BlockExpander;
use dot001_parser::{BlendFile, Result};
use std::io::{Read, Seek};

/// Expander for Cache File (CF) blocks.
/// Cache files are used for caching simulation data (fluid, smoke, cloth, etc.).
pub struct CacheFileExpander;

impl<R: Read + Seek> BlockExpander<R> for CacheFileExpander {
    fn expand_block(
        &self,
        _block_index: usize,
        _blend_file: &mut BlendFile<R>,
    ) -> Result<Vec<usize>> {
        let dependencies = Vec::new();

        // Cache files typically reference external data files on disk
        // They don't usually have block dependencies within the blend file
        // For now, we don't track external file dependencies

        Ok(dependencies)
    }

    fn can_handle(&self, code: &[u8; 4]) -> bool {
        code == b"CF\0\0"
    }
}
