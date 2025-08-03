use crate::BlockExpander;
use bllink_parser::{BlendFile, Result};
use std::io::{Read, Seek};

/// Expander for Lamp/Light (LA) blocks.
/// Lamps can have shader node trees that reference external files (e.g., IES files).
pub struct LampExpander;

impl<R: Read + Seek> BlockExpander<R> for LampExpander {
    fn expand_block(
        &self,
        block_index: usize,
        blend_file: &mut BlendFile<R>,
    ) -> Result<Vec<usize>> {
        let mut dependencies = Vec::new();

        // Read the lamp block data
        let lamp_data = blend_file.read_block_data(block_index)?;
        let reader = blend_file.create_field_reader(&lamp_data)?;

        // Check if this lamp has a node tree
        if let Ok(nodetree_ptr) = reader.read_field_pointer("Lamp", "nodetree") {
            if nodetree_ptr != 0 {
                if let Some(nodetree_index) = blend_file.find_block_by_address(nodetree_ptr) {
                    dependencies.push(nodetree_index);

                    // Note: In the Python reference, they also walk through individual nodes
                    // to find storage blocks with filepath fields (e.g., NodeShaderTexIES)
                    // This would require more complex node tree traversal
                }
            }
        }

        Ok(dependencies)
    }

    fn can_handle(&self, code: &[u8; 4]) -> bool {
        code == b"LA\0\0"
    }
}
