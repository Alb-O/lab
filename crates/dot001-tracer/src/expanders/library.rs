use crate::{BlockExpander, ExpandResult, bpath::BlendPath};
use dot001_parser::{BlendFile, Result};
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
    ) -> Result<ExpandResult> {
        let dependencies = Vec::new();
        let mut external_refs = Vec::new();

        // Read the library block data
        let library_data = blend_file.read_block_data(block_index)?;
        let reader = blend_file.create_field_reader(&library_data)?;

        // Libraries contain file paths to external .blend files in the "filepath" field
        if let Ok(filepath) = reader.read_field_string("Library", "filepath") {
            let path_str = filepath.trim_end_matches('\0').trim();
            if !path_str.is_empty() {
                let blend_path = BlendPath::new(path_str.as_bytes());
                external_refs.push(blend_path.to_pathbuf_stripped());
            }
        }

        // Also try the "name" field as fallback (older Blender versions might use this)
        if external_refs.is_empty() {
            if let Ok(name) = reader.read_field_string("Library", "name") {
                let path_str = name.trim_end_matches('\0').trim();
                if !path_str.is_empty() {
                    let blend_path = BlendPath::new(path_str.as_bytes());
                    external_refs.push(blend_path.to_pathbuf_stripped());
                }
            }
        }

        Ok(ExpandResult::with_externals(dependencies, external_refs))
    }

    fn can_handle(&self, code: &[u8; 4]) -> bool {
        code == b"LI\0\0"
    }
}
