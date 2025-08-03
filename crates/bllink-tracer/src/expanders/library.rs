use crate::BlockExpander;
use bllink_parser::{BlendFile, Result};
use std::io::{Read, Seek};

/// Expander for Library (LI) blocks
///
/// Libraries represent linked .blend files. They contain the file path
/// to the external .blend file that is being linked.
///
/// This is important for tracking dependencies between blend files.
pub struct LibraryExpander;

impl<R: Read + Seek> BlockExpander<R> for LibraryExpander {
    fn expand_block(
        &self,
        block_index: usize,
        blend_file: &mut BlendFile<R>,
    ) -> Result<Vec<usize>> {
        let dependencies = Vec::new();

        // Read the library block data
        let _library_data = blend_file.read_block_data(block_index)?;
        let _reader = blend_file.create_field_reader(&_library_data)?;

        // Libraries contain file paths to external .blend files in the "name" field
        // Since our current dependency system tracks block dependencies rather than
        // external file paths, we don't add anything to the dependencies vector.

        // TODO: In a full asset tracking system, we would want to:
        // 1. Read the "name" field to get the library file path
        // 2. Potentially track this as an external file dependency
        // 3. Maybe even recursively parse the linked blend file

        // For now, library blocks don't have internal block dependencies,
        // so we return an empty dependencies list.

        Ok(dependencies)
    }

    fn can_handle(&self, code: &[u8; 4]) -> bool {
        code == b"LI\0\0"
    }
}
