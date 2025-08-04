use crate::BlockExpander;
use crate::ExpandResult;
use dot001_parser::{BlendFile, Result};
use std::io::{Read, Seek};
use std::path::PathBuf;

/// Expander for Sound (SO) blocks.
/// Sound blocks reference audio files that are used in the sequencer or for audio objects.
pub struct SoundExpander;

impl<R: Read + Seek> BlockExpander<R> for SoundExpander {
    fn expand_block(
        &self,
        block_index: usize,
        blend_file: &mut BlendFile<R>,
    ) -> Result<ExpandResult> {
        let dependencies = Vec::new();
        let mut external_refs = Vec::new();

        // Read the sound block data
        let sound_data = blend_file.read_block_data(block_index)?;
        let reader = blend_file.create_field_reader(&sound_data)?;

        // Sound files reference external audio files in the "filepath" field
        if let Ok(filepath) = reader.read_field_string("bSound", "filepath") {
            let path_str = filepath.trim_end_matches('\0').trim();
            if !path_str.is_empty() {
                // Convert Blender's path format (which might use '//' prefix for relative paths)
                let cleaned_path = if path_str.starts_with("//") {
                    // Relative path in Blender format - convert to standard relative path
                    &path_str[2..]
                } else {
                    path_str
                };
                external_refs.push(PathBuf::from(cleaned_path));
            }
        }

        // Check if the sound is packed (has internal data)
        if let Ok(packedfile_ptr) = reader.read_field_pointer("bSound", "packedfile") {
            if packedfile_ptr != 0 {
                // Sound is packed, clear external references since it doesn't depend on external files
                external_refs.clear();
            }
        }

        Ok(ExpandResult::with_externals(dependencies, external_refs))
    }

    fn can_handle(&self, code: &[u8; 4]) -> bool {
        code == b"SO\0\0"
    }
}
