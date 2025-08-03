use crate::BlockExpander;
use dot001_parser::{BlendFile, Result};
use std::io::{Read, Seek};

/// Expander for Image (IM) blocks
///
/// Images can reference external files and may be part of image sequences.
/// This expander handles the file path dependencies for images.
///
/// Note: Images with packed data (packedfile != null) don't have external dependencies.
pub struct ImageExpander;

impl<R: Read + Seek> BlockExpander<R> for ImageExpander {
    fn expand_block(
        &self,
        block_index: usize,
        blend_file: &mut BlendFile<R>,
    ) -> Result<Vec<usize>> {
        let dependencies = Vec::new();

        // Read the image block data
        let image_data = blend_file.read_block_data(block_index)?;
        let reader = blend_file.create_field_reader(&image_data)?;

        // Skip packed images - they don't have external file dependencies
        if let Ok(packedfile_ptr) = reader.read_field_pointer("Image", "packedfile") {
            if packedfile_ptr != 0 {
                // Image is packed, no external file dependency
                return Ok(dependencies);
            }
        }

        // Check image source type to determine if we should process this image
        // IMA_SRC_FILE = 1, IMA_SRC_SEQUENCE = 2, IMA_SRC_MOVIE = 3, IMA_SRC_TILED = 5
        if let Ok(source) = reader.read_field_u32("Image", "source") {
            if matches!(source, 1 | 2 | 3 | 5) {
                // These are file-based sources that we need to track
                // The actual file path is stored in the "name" field
                // Note: In a full implementation, we would add the file path as an external asset,
                // but since our current system tracks block dependencies rather than file paths,
                // we don't add anything to dependencies here.

                // TODO: In the future, we might want to extend the dependency system
                // to also track external file references, not just block references.
            } else {
                // Generated images, viewer nodes, etc. - no external files
            }
        }

        Ok(dependencies)
    }

    fn can_handle(&self, code: &[u8; 4]) -> bool {
        code == b"IM\0\0"
    }
}
