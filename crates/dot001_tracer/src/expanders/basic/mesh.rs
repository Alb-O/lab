/// Mesh expander with robust material handling
use crate::BlockExpander;
use dot001_parser::{BlendFile, PointerTraversal, Result};
use crate::ExpandResult;
use std::io::{Read, Seek};

pub struct MeshExpander;

impl<R: Read + Seek> BlockExpander<R> for MeshExpander {
    fn expand_block(
        &self,
        block_index: usize,
        blend_file: &mut BlendFile<R>,
    ) -> Result<ExpandResult> {
        let mut dependencies = Vec::new();

        // Add single pointer field dependencies
        let single_fields = [
            "vert", "edge", "poly", "loop",
            "vert_normals", "poly_normals", "loop_normals", "face_sets"
        ];
        
        for field in single_fields {
            if let Ok(single_targets) = PointerTraversal::read_pointer_fields(
                blend_file,
                block_index,
                "Mesh",
                &[field]
            ) {
                dependencies.extend(single_targets);
            }
        }

        // Try to read material array with error handling for version differences
        if let Ok(array_targets) = PointerTraversal::read_pointer_array(
            blend_file,
            block_index,
            "Mesh",
            "totcol",
            "mat"
        ) {
            dependencies.extend(array_targets);
        }
        // If material array reading fails, continue without materials
        // This handles cases where different Blender versions have different struct layouts

        Ok(ExpandResult::new(dependencies))
    }

    fn can_handle(&self, code: &[u8; 4]) -> bool {
        code == b"ME\0\0"
    }
}
