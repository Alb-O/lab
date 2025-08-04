use crate::BlockExpander;
use crate::ExpandResult;
use dot001_parser::{BlendFile, PointerTraversal, Result};
use std::io::{Read, Seek};

/// Expander for Object (OB) blocks
///
/// Objects contain references to their mesh data and materials.
/// Materials are stored in an array that we need to traverse.
pub struct ObjectExpander;

impl<R: Read + Seek> BlockExpander<R> for ObjectExpander {
    fn expand_block(
        &self,
        block_index: usize,
        blend_file: &mut BlendFile<R>,
    ) -> Result<ExpandResult> {
        let mut dependencies = Vec::new();

        // Add single pointer field dependencies (like 'data' field)
        if let Ok(single_targets) =
            PointerTraversal::read_pointer_fields(blend_file, block_index, "Object", &["data"])
        {
            dependencies.extend(single_targets);
        }

        // Add material dependencies using the pointer array helper
        if let Ok(mat_targets) =
            PointerTraversal::read_pointer_array(blend_file, block_index, "Object", "totcol", "mat")
        {
            dependencies.extend(mat_targets);
        }

        Ok(ExpandResult::new(dependencies))
    }

    fn can_handle(&self, code: &[u8; 4]) -> bool {
        code == b"OB\0\0"
    }
}
