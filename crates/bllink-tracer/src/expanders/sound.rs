use crate::BlockExpander;
use bllink_parser::{BlendFile, Result};
use std::io::{Read, Seek};

/// Expander for Sound (SO) blocks.
/// Sound blocks reference audio files that are used in the sequencer or for audio objects.
pub struct SoundExpander;

impl<R: Read + Seek> BlockExpander<R> for SoundExpander {
    fn expand_block(
        &self,
        _block_index: usize,
        _blend_file: &mut BlendFile<R>,
    ) -> Result<Vec<usize>> {
        let dependencies = Vec::new();

        // Sound files reference external audio files
        // For now, we don't track external file dependencies
        // but this is where we would add external file tracking

        Ok(dependencies)
    }

    fn can_handle(&self, code: &[u8; 4]) -> bool {
        code == b"SO\0\0"
    }
}
