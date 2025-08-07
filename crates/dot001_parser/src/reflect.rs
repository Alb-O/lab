use crate::{BlendFile, Result};
use dot001_events::error::{BlendFileErrorKind, Error};
use std::io::{Read, Seek};

/// Utilities for reflective pointer traversal in BlendFile data structures.
/// This module consolidates pointer traversal logic that was previously duplicated
/// between FilterEngine and various BlockExpanders.
pub struct PointerTraversal;

impl PointerTraversal {
    /// Find all pointer field targets in a block using DNA reflection.
    /// This provides a generic way to discover pointer dependencies without
    /// hard-coding specific struct layouts.
    pub fn find_pointer_targets<R: Read + Seek>(
        blend_file: &mut BlendFile<R>,
        block_index: usize,
    ) -> Result<Vec<usize>> {
        let mut targets = Vec::new();

        // Get block info
        let block = blend_file.get_block(block_index).ok_or(Error::blend_file(
            "Block index out of range",
            BlendFileErrorKind::InvalidBlockIndex,
        ))?;
        let code = block.header.code;

        // Get type name from block code
        let code_string = String::from_utf8_lossy(&code);
        let type_name = code_string.trim_end_matches('\0');

        // Find struct definition in DNA and collect pointer field info
        let pointer_fields = {
            let dna = blend_file.dna()?;
            let mut fields = Vec::new();

            if let Some(struct_def) = dna.structs.iter().find(|s| s.type_name == type_name) {
                for field in &struct_def.fields {
                    if field.name.is_pointer {
                        fields.push((struct_def.type_name.clone(), field.name.name_only.clone()));
                    }
                }
            }
            fields
        };

        // Now read the block data and find pointer targets
        if !pointer_fields.is_empty() {
            let block_data = blend_file.read_block_data(block_index)?;
            let reader = blend_file.create_field_reader(&block_data)?;

            for (struct_name, field_name) in pointer_fields {
                if let Ok(ptr_value) = reader.read_field_pointer(&struct_name, &field_name) {
                    if ptr_value != 0 {
                        if let Some(target_index) = blend_file.find_block_by_address(ptr_value) {
                            targets.push(target_index);
                        }
                    }
                }
            }
        }

        Ok(targets)
    }

    /// Helper function for reading pointer arrays (like materials arrays in Object/Mesh).
    /// This consolidates the common pattern of reading an array count and pointer,
    /// then traversing the array to find all pointer targets.
    pub fn read_pointer_array<R: Read + Seek>(
        blend_file: &mut BlendFile<R>,
        block_index: usize,
        struct_name: &str,
        count_field: &str,
        array_ptr_field: &str,
    ) -> Result<Vec<usize>> {
        let mut targets = Vec::new();

        let block_data = blend_file.read_block_data(block_index)?;
        let reader = blend_file.create_field_reader(&block_data)?;

        if let Ok(count) = reader.read_field_u32(struct_name, count_field) {
            if count > 0 {
                if let Ok(array_ptr) = reader.read_field_pointer(struct_name, array_ptr_field) {
                    if array_ptr != 0 {
                        if let Some(array_index) = blend_file.find_block_by_address(array_ptr) {
                            // Add the array block itself as a dependency
                            targets.push(array_index);

                            // Read through the array to find individual pointers
                            let array_data = blend_file.read_block_data(array_index)?;
                            let array_reader = blend_file.create_field_reader(&array_data)?;
                            let pointer_size = blend_file.header().pointer_size as usize;

                            for i in 0..count {
                                let offset = i as usize * pointer_size;
                                if let Ok(ptr_value) = array_reader.read_pointer(offset) {
                                    if ptr_value != 0 {
                                        if let Some(target_index) =
                                            blend_file.find_block_by_address(ptr_value)
                                        {
                                            targets.push(target_index);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(targets)
    }

    /// Helper for reading multiple single pointer fields from a struct.
    /// Takes a list of field names and returns all valid pointer targets.
    pub fn read_pointer_fields<R: Read + Seek>(
        blend_file: &mut BlendFile<R>,
        block_index: usize,
        struct_name: &str,
        field_names: &[&str],
    ) -> Result<Vec<usize>> {
        let mut targets = Vec::new();

        let block_data = blend_file.read_block_data(block_index)?;
        let reader = blend_file.create_field_reader(&block_data)?;

        for field_name in field_names {
            if let Ok(ptr_value) = reader.read_field_pointer(struct_name, field_name) {
                if ptr_value != 0 {
                    if let Some(target_index) = blend_file.find_block_by_address(ptr_value) {
                        targets.push(target_index);
                    }
                }
            }
        }

        Ok(targets)
    }
}
