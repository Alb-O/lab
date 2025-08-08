//! Thread-safe block expander trait and implementations
//!
//! This module provides the new thread-safe BlockExpander architecture that works
//! with BlendFileBuf for zero-copy, immutable access patterns required for parallel
//! dependency tracing.

use crate::ExpandResult;
use dot001_events::error::Result;
use dot001_parser::BlendFileBuf;

/// Thread-safe trait for expanding block dependencies
///
/// This replaces the legacy BlockExpander<R> with a version that:
/// - Works with BlendFileBuf for zero-copy access
/// - Uses only immutable references for thread safety
/// - Provides FieldView-based field access for performance
/// - Can be safely shared across threads (Send + Sync)
pub trait BlockExpander: Send + Sync {
    /// Expand dependencies for a block using zero-copy access
    ///
    /// This method receives only immutable references and must not mutate
    /// any shared state. All data access should go through FieldView for
    /// zero-copy performance.
    fn expand_block_threadsafe(
        &self,
        block_index: usize,
        blend_file: &BlendFileBuf,
    ) -> Result<ExpandResult>;

    /// Check if this expander can handle the given block code
    fn can_handle(&self, code: &[u8; 4]) -> bool;

    /// Get the primary block code this expander handles
    fn block_code(&self) -> [u8; 4];

    /// Get a human-readable name for this expander
    fn expander_name(&self) -> &'static str;
}

/// Thread-safe pointer traversal utilities
///
/// This provides zero-copy versions of the pointer traversal functions
/// that work with FieldView instead of FieldReader.
pub struct PointerTraversal;

impl PointerTraversal {
    /// Read single pointer fields using FieldView
    ///
    /// This is the thread-safe, zero-copy equivalent of PointerTraversal::read_pointer_fields
    pub fn read_pointer_fields_threadsafe(
        blend_file: &BlendFileBuf,
        block_index: usize,
        struct_name: &str,
        field_names: &[&str],
    ) -> Result<Vec<usize>> {
        let mut targets = Vec::new();

        // Get block data slice
        let slice = blend_file.read_block_slice_for_field_view(block_index)?;

        // Create field view for zero-copy access
        let view = blend_file.create_field_view(&slice)?;

        // Find the struct in the DNA
        let dna = blend_file.dna()?;
        let struct_def = dna
            .structs
            .iter()
            .find(|s| s.type_name == struct_name)
            .ok_or_else(|| {
                dot001_events::error::Error::tracer(
                    format!("Struct '{struct_name}' not found in DNA"),
                    dot001_events::error::TracerErrorKind::BlockExpansionFailed,
                )
            })?;

        // Read each pointer field
        for field_name in field_names {
            if let Some(field) = struct_def.find_field(field_name) {
                if field.name.is_pointer {
                    // Read pointer value
                    match view.read_pointer(field.offset) {
                        Ok(ptr_value) if ptr_value != 0 => {
                            // Convert pointer to block index
                            if let Some(target_index) = blend_file.address_to_block_index(ptr_value)
                            {
                                targets.push(target_index);
                            }
                        }
                        _ => {} // Null pointer or read error, skip
                    }
                }
            }
        }

        Ok(targets)
    }

    /// Read pointer array fields using FieldView
    ///
    /// This is the thread-safe, zero-copy equivalent of PointerTraversal::read_pointer_array
    pub fn read_pointer_array_threadsafe(
        blend_file: &BlendFileBuf,
        block_index: usize,
        struct_name: &str,
        count_field: &str,
        array_field: &str,
    ) -> Result<Vec<usize>> {
        let mut targets = Vec::new();

        // Get block data slice
        let slice = blend_file.read_block_slice_for_field_view(block_index)?;

        // Create field view for zero-copy access
        let view = blend_file.create_field_view(&slice)?;

        // Find the struct in the DNA
        let dna = blend_file.dna()?;
        let struct_def = dna
            .structs
            .iter()
            .find(|s| s.type_name == struct_name)
            .ok_or_else(|| {
                dot001_events::error::Error::tracer(
                    format!("Struct '{struct_name}' not found in DNA"),
                    dot001_events::error::TracerErrorKind::BlockExpansionFailed,
                )
            })?;

        // Read the count field
        let count = if let Some(count_field_def) = struct_def.find_field(count_field) {
            // Determine count field size and read appropriately
            match count_field_def.size {
                1 => view.read_u8(count_field_def.offset)? as usize,
                2 => view.read_u16(count_field_def.offset)? as usize,
                4 => view.read_u32(count_field_def.offset)? as usize,
                8 => view.read_u64(count_field_def.offset)? as usize,
                _ => return Ok(targets), // Unsupported count field size
            }
        } else {
            return Ok(targets); // Count field not found
        };

        // Read the array pointer
        if let Some(array_field_def) = struct_def.find_field(array_field) {
            if array_field_def.name.is_pointer {
                match view.read_pointer(array_field_def.offset) {
                    Ok(array_ptr) if array_ptr != 0 => {
                        // Find the block containing the array
                        if let Some(array_block_index) =
                            blend_file.address_to_block_index(array_ptr)
                        {
                            // Read the array block
                            let array_slice =
                                blend_file.read_block_slice_for_field_view(array_block_index)?;
                            let array_view = blend_file.create_field_view(&array_slice)?;

                            // Read each pointer in the array
                            let pointer_size = blend_file.header().pointer_size as usize;
                            for i in 0..count {
                                let offset = i * pointer_size;
                                if let Ok(ptr) = array_view.read_pointer(offset) {
                                    if ptr != 0 {
                                        if let Some(target_index) =
                                            blend_file.address_to_block_index(ptr)
                                        {
                                            targets.push(target_index);
                                        }
                                    }
                                }
                            }
                        }
                    }
                    _ => {} // Null pointer or read error, skip
                }
            }
        }

        Ok(targets)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestExpander;

    impl BlockExpander for TestExpander {
        fn expand_block_threadsafe(
            &self,
            _block_index: usize,
            _blend_file: &BlendFileBuf,
        ) -> Result<ExpandResult> {
            Ok(ExpandResult::new(vec![]))
        }

        fn can_handle(&self, code: &[u8; 4]) -> bool {
            code == b"TE\0\0"
        }

        fn block_code(&self) -> [u8; 4] {
            *b"TE\0\0"
        }

        fn expander_name(&self) -> &'static str {
            "TestExpander"
        }
    }

    #[test]
    fn test_expander_trait() {
        let expander = TestExpander;
        assert!(expander.can_handle(b"TE\0\0"));
        assert!(!expander.can_handle(b"OB\0\0"));
        assert_eq!(expander.block_code(), *b"TE\0\0");
        assert_eq!(expander.expander_name(), "TestExpander");
    }
}
