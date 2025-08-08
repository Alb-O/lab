use crate::DisplayTemplate;
use crate::block_display::{BlockInfo, create_display_for_template};
use dot001_events::error::Error;
use dot001_parser::{
    BlendFile, DataBlockVisibility, block_code_to_string, is_block_visible, is_data_block_code,
};

/// Block information with metadata commonly needed for display
#[derive(Debug, Clone)]
pub struct BlockWithMetadata {
    pub index: usize,
    pub info: BlockInfo,
    pub size: u64,
    pub address: u64,
    pub count: u32,
    pub code: String,
}

impl BlockWithMetadata {
    /// Create a display string using the specified template
    pub fn create_display(&self, template: &DisplayTemplate) -> String {
        create_display_for_template(
            self.info.clone(),
            template,
            Some(self.size),
            Some(self.address),
        )
        .to_string()
    }

    /// Check if this block is a DATA block
    pub fn is_data_block(&self) -> bool {
        is_data_block_code(&self.code)
    }

    /// Check if this block matches a specific code
    pub fn has_code(&self, code: &str) -> bool {
        self.code == code
    }
}

/// Utility functions for common block operations
pub struct BlockUtils;

impl BlockUtils {
    /// Extract block metadata from a blend file at a specific index
    pub fn get_block_metadata(
        index: usize,
        blend_file: &mut BlendFile,
    ) -> Result<BlockWithMetadata, Error> {
        // Get block info and extract data first to avoid borrow checker issues
        let (size, address, count, code) = {
            let block = blend_file
                .get_block(index)
                .ok_or_else(|| Error::io(format!("Block index {index} is out of range")))?;

            let code = block_code_to_string(block.header.code);

            (
                block.header.size as u64,
                block.header.old_address,
                block.header.count,
                code,
            )
        };

        let info = BlockInfo::from_blend_file(index, blend_file)
            .unwrap_or_else(|_| BlockInfo::new(index, code.clone()));

        Ok(BlockWithMetadata {
            index,
            info,
            size,
            address,
            count,
            code,
        })
    }

    /// Get all blocks from a blend file with optional DATA filtering
    pub fn get_all_blocks(blend_file: &mut BlendFile, show_data: bool) -> Vec<BlockWithMetadata> {
        (0..blend_file.blocks_len())
            .filter_map(|i| Self::get_block_metadata(i, blend_file).ok())
            .filter(|block| show_data || !block.is_data_block())
            .collect()
    }

    /// Get blocks of a specific type
    pub fn get_blocks_by_type(
        blend_file: &mut BlendFile,
        block_type: &str,
        show_data: bool,
    ) -> Vec<BlockWithMetadata> {
        Self::get_all_blocks(blend_file, show_data)
            .into_iter()
            .filter(|block| block.has_code(block_type))
            .collect()
    }

    /// Filter a list of block indices, removing DATA blocks unless show_data is true
    pub fn filter_data_blocks(indices: &mut Vec<usize>, blend_file: &BlendFile, show_data: bool) {
        let policy = DataBlockVisibility::from_flag(show_data);
        indices.retain(|&i| {
            if let Some(block) = blend_file.get_block(i) {
                let code_str = block_code_to_string(block.header.code);
                is_block_visible(&code_str, policy)
            } else {
                true // Keep if we can't read the block
            }
        });
    }

    /// Filter a HashSet of block indices, removing DATA blocks unless show_data is true
    pub fn filter_data_blocks_hashset(
        indices: &mut std::collections::HashSet<usize>,
        blend_file: &BlendFile,
        show_data: bool,
    ) {
        let policy = DataBlockVisibility::from_flag(show_data);
        indices.retain(|&i| {
            if let Some(block) = blend_file.get_block(i) {
                let code_str = block_code_to_string(block.header.code);
                is_block_visible(&code_str, policy)
            } else {
                true // Keep if we can't read the block
            }
        });
    }

    /// Extract basic block code from a block index
    pub fn get_block_code(index: usize, blend_file: &BlendFile) -> Option<String> {
        blend_file
            .get_block(index)
            .map(|block| block_code_to_string(block.header.code))
    }
}

/// Builder pattern for processing blocks with filters
pub struct BlockProcessor<'a> {
    blend_file: &'a mut BlendFile,
    show_data: bool,
    type_filters: Vec<String>,
    index_filters: Vec<usize>,
}

impl<'a> BlockProcessor<'a> {
    pub fn new(blend_file: &'a mut BlendFile) -> Self {
        Self {
            blend_file,
            show_data: false,
            type_filters: Vec::new(),
            index_filters: Vec::new(),
        }
    }

    /// Include DATA blocks in the results
    pub fn with_data_blocks(mut self, show_data: bool) -> Self {
        self.show_data = show_data;
        self
    }

    /// Filter to only include specific block types
    pub fn filter_by_type(mut self, block_type: &str) -> Self {
        self.type_filters.push(block_type.to_string());
        self
    }

    /// Filter to only include specific indices
    pub fn filter_by_indices(mut self, indices: &[usize]) -> Self {
        self.index_filters.extend(indices);
        self
    }

    /// Collect all blocks matching the current filters
    pub fn collect(self) -> Vec<BlockWithMetadata> {
        let all_indices: Vec<usize> = if self.index_filters.is_empty() {
            (0..self.blend_file.blocks_len()).collect()
        } else {
            self.index_filters
        };

        all_indices
            .into_iter()
            .filter_map(|i| BlockUtils::get_block_metadata(i, self.blend_file).ok())
            .filter(|block| self.show_data || !block.is_data_block())
            .filter(|block| {
                self.type_filters.is_empty() || self.type_filters.iter().any(|t| block.has_code(t))
            })
            .collect()
    }
}
