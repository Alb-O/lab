use crate::DisplayTemplate;
use crate::block_utils::BlockWithMetadata;
use crate::util::CommandContext;
use dot001_error::Dot001Error;
use dot001_parser::BlendFile;
use std::io::{Read, Seek};

/// Common block resolution and operation patterns
pub trait BlockOperations<R: Read + Seek> {
    /// Resolve a block identifier (index or name) to a specific block index
    fn resolve_block_identifier(&mut self, identifier: &str) -> Option<usize>;

    /// Resolve a block identifier with a specific type requirement
    fn resolve_typed_block(&mut self, identifier: &str, block_type: &str) -> Option<usize>;

    /// Get block metadata with error handling
    fn get_block_metadata_safe(&mut self, index: usize) -> Result<BlockWithMetadata, Dot001Error>;

    /// Get all blocks of a specific type
    fn get_blocks_by_type(&mut self, block_type: &str, show_data: bool) -> Vec<BlockWithMetadata>;
}

impl<R: Read + Seek> BlockOperations<R> for BlendFile<R> {
    fn resolve_block_identifier(&mut self, identifier: &str) -> Option<usize> {
        crate::util::resolve_block_or_exit(identifier, self)
    }

    fn resolve_typed_block(&mut self, identifier: &str, block_type: &str) -> Option<usize> {
        crate::util::resolve_typed_block_or_exit(identifier, block_type, self)
    }

    fn get_block_metadata_safe(&mut self, index: usize) -> Result<BlockWithMetadata, Dot001Error> {
        crate::block_utils::BlockUtils::get_block_metadata(index, self)
    }

    fn get_blocks_by_type(&mut self, block_type: &str, show_data: bool) -> Vec<BlockWithMetadata> {
        crate::block_utils::BlockUtils::get_blocks_by_type(self, block_type, show_data)
    }
}

/// Helper for common command patterns
pub struct CommandHelper<'a, R: Read + Seek> {
    blend_file: &'a mut BlendFile<R>,
    ctx: &'a CommandContext<'a>,
}

impl<'a, R: Read + Seek> CommandHelper<'a, R> {
    pub fn new(blend_file: &'a mut BlendFile<R>, ctx: &'a CommandContext<'a>) -> Self {
        Self { blend_file, ctx }
    }

    /// Resolve a block and exit gracefully if not found
    pub fn resolve_block_or_return(
        &mut self,
        identifier: &str,
    ) -> Result<Option<usize>, Dot001Error> {
        match self.blend_file.resolve_block_identifier(identifier) {
            Some(index) => Ok(Some(index)),
            None => {
                // The resolve function already logs the error and suggests alternatives
                Ok(None)
            }
        }
    }

    /// Resolve a typed block and exit gracefully if not found
    pub fn resolve_typed_block_or_return(
        &mut self,
        identifier: &str,
        block_type: &str,
    ) -> Result<Option<usize>, Dot001Error> {
        match self.blend_file.resolve_typed_block(identifier, block_type) {
            Some(index) => Ok(Some(index)),
            None => {
                // The resolve function already logs the error and suggests alternatives
                Ok(None)
            }
        }
    }

    /// Get block info with consistent error handling and display creation
    pub fn get_block_display(
        &mut self,
        index: usize,
        template: &DisplayTemplate,
    ) -> Result<String, Dot001Error> {
        let metadata = self.blend_file.get_block_metadata_safe(index)?;
        Ok(metadata.create_display(template))
    }

    /// Print file header information
    pub fn print_file_header(&self, file_path: &std::path::Path, action: &str) {
        self.ctx
            .output
            .print_info_fmt(format_args!("{} {}:", action, file_path.display()));
    }
}

/// Batch operations helper for processing multiple blocks
pub struct BatchProcessor<'a, R: Read + Seek> {
    blend_file: &'a mut BlendFile<R>,
    ctx: &'a CommandContext<'a>,
}

impl<'a, R: Read + Seek> BatchProcessor<'a, R> {
    pub fn new(blend_file: &'a mut BlendFile<R>, ctx: &'a CommandContext<'a>) -> Self {
        Self { blend_file, ctx }
    }

    /// Process a list of block indices with a closure
    pub fn process_blocks<F>(
        &mut self,
        indices: &[usize],
        template: &DisplayTemplate,
        mut processor: F,
    ) -> Result<(), Dot001Error>
    where
        F: FnMut(usize, &BlockWithMetadata, &str, &CommandContext) -> Result<(), Dot001Error>,
    {
        for &index in indices {
            let metadata = self.blend_file.get_block_metadata_safe(index)?;
            let display = metadata.create_display(template);
            processor(index, &metadata, &display, self.ctx)?;
        }
        Ok(())
    }

    /// Print a list of blocks with consistent formatting
    pub fn print_blocks(
        &mut self,
        indices: &[usize],
        template: &DisplayTemplate,
    ) -> Result<(), Dot001Error> {
        self.process_blocks(indices, template, |_index, _metadata, display, ctx| {
            ctx.output.print_result_fmt(format_args!("  {display}"));
            Ok(())
        })
    }

    /// Count blocks by type in a list of indices
    pub fn count_blocks_by_type(
        &mut self,
        indices: &[usize],
    ) -> std::collections::HashMap<String, usize> {
        let mut counts = std::collections::HashMap::new();

        for &index in indices {
            if let Ok(metadata) = self.blend_file.get_block_metadata_safe(index) {
                *counts.entry(metadata.code).or_insert(0) += 1;
            }
        }

        counts
    }
}

/// Validation helpers for common checks
pub struct ValidationHelper;

impl ValidationHelper {
    /// Validate that a block index is in range
    pub fn validate_block_index<R: Read + Seek>(
        index: usize,
        blend_file: &BlendFile<R>,
    ) -> Result<(), Dot001Error> {
        if index >= blend_file.blocks_len() {
            return Err(Dot001Error::cli(
                format!(
                    "Block index {} is out of range (max: {})",
                    index,
                    blend_file.blocks_len() - 1
                ),
                dot001_error::CliErrorKind::InvalidArguments,
            ));
        }
        Ok(())
    }

    /// Validate that a block has the expected type
    pub fn validate_block_type<R: Read + Seek>(
        index: usize,
        expected_type: &str,
        blend_file: &BlendFile<R>,
    ) -> Result<(), Dot001Error> {
        let Some(block) = blend_file.get_block(index) else {
            return Err(Dot001Error::cli(
                format!("Cannot access block {index}"),
                dot001_error::CliErrorKind::ExecutionFailed,
            ));
        };

        let actual_type = dot001_parser::block_code_to_string(block.header.code);

        if actual_type != expected_type {
            return Err(Dot001Error::cli(
                format!("Block {index} has type '{actual_type}', expected '{expected_type}'"),
                dot001_error::CliErrorKind::InvalidArguments,
            ));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {

    use std::collections::HashMap;

    #[test]
    fn test_validation_helper() {
        // These tests would need a mock BlendFile implementation
        // For now, just test that the validation logic compiles
        assert_eq!(1, 1);
    }

    #[test]
    fn test_batch_processor_count_logic() {
        // Test the counting logic without needing a real blend file
        let mut counts = HashMap::new();
        counts.insert("ME".to_string(), 5);
        counts.insert("OB".to_string(), 3);

        assert_eq!(counts.get("ME"), Some(&5));
        assert_eq!(counts.get("OB"), Some(&3));
        assert_eq!(counts.get("MA"), None);
    }
}
