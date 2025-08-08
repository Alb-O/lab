//! Thread-safe Sound block expander
//!
//! This expander handles Sound blocks (SO) which reference external sound files.
//! Sounds can be packed (embedded in the blend file) or reference external files.

use crate::{BlockExpander, ExpandResult, utils::bpath::BlendPath};
use dot001_events::error::Result;

pub struct SoundExpander;

impl BlockExpander for SoundExpander {
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
                    if let Some(sound_struct) = dna.structs.iter().find(|s| s.type_name == "bSound")
                    {
                        // Skip packed sounds - they don't have external file dependencies
                        let is_packed =
                            if let Some(packedfile_field) = sound_struct.find_field("packedfile") {
                                if packedfile_field.name.is_pointer {
                                    view.read_pointer(packedfile_field.offset).unwrap_or(0) != 0
                                } else {
                                    false
                                }
                            } else {
                                false
                            };

                        if !is_packed {
                            // The actual file path is stored in the "filepath" field
                            if let Ok(filepath) = view.read_field_string("bSound", "filepath") {
                                let path_str = filepath.trim_end_matches('\0').trim();
                                if !path_str.is_empty() {
                                    let blend_path = BlendPath::new(path_str.as_bytes());
                                    external_refs.push(blend_path.to_pathbuf_stripped());
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
        code == b"SO\0\0"
    }

    fn block_code(&self) -> [u8; 4] {
        *b"SO\0\0"
    }

    fn expander_name(&self) -> &'static str {
        "SoundExpander"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sound_expander_properties() {
        let expander = SoundExpander;
        assert_eq!(expander.block_code(), *b"SO\0\0");
        assert_eq!(expander.expander_name(), "SoundExpander");
        assert!(expander.can_handle(b"SO\0\0"));
        assert!(!expander.can_handle(b"OB\0\0"));
    }
}
