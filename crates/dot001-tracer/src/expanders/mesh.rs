use crate::BlockExpander;
use crate::ExpandResult;
use dot001_parser::{BlendFile, PointerTraversal, Result};
use std::io::{Read, Seek};

/// Expander for Mesh (ME) blocks
///
/// Meshes contain references to materials in a materials array.
/// We need to read through the array to find all material dependencies.
pub struct MeshExpander;

impl<R: Read + Seek> BlockExpander<R> for MeshExpander {
    fn expand_block(
        &self,
        block_index: usize,
        blend_file: &mut BlendFile<R>,
    ) -> Result<ExpandResult> {
        let mut dependencies = Vec::new();

        // Add material dependencies using the pointer array helper
        if let Ok(mat_targets) =
            PointerTraversal::read_pointer_array(blend_file, block_index, "Mesh", "totcol", "mat")
        {
            dependencies.extend(mat_targets);
        }

        // Add geometric data dependencies using the pointer fields helper
        let geometric_fields = [
            "vert",
            "edge",
            "poly",
            "loop",
            "vert_normals",
            "poly_normals",
            "loop_normals",
            "face_sets",
        ];

        if let Ok(geo_targets) = PointerTraversal::read_pointer_fields(
            blend_file,
            block_index,
            "Mesh",
            &geometric_fields,
        ) {
            dependencies.extend(geo_targets);
        }

        Ok(ExpandResult::new(dependencies))
    }

    fn can_handle(&self, code: &[u8; 4]) -> bool {
        code == b"ME\0\0"
    }
}
