/// Minimal name resolution utility for Blender datablocks
///
/// This module provides basic name extraction from Blender datablocks without
/// requiring the full dependency tracing capabilities of the tracer crate.
/// It's designed for "read-only list" workflows that only need display names.
use crate::BlendFile;
use std::io::{Read, Seek};

/// Minimal name resolver for extracting user-defined names from datablocks
pub struct NameResolver;

impl NameResolver {
    /// Extract the user-defined name from a datablock
    ///
    /// Returns the clean name without type prefixes (e.g., "Cube" instead of "MECube")
    /// Returns None if the name cannot be read or is empty
    pub fn resolve_name<R: Read + Seek>(
        block_index: usize,
        blend_file: &mut BlendFile<R>,
    ) -> Option<String> {
        // Read the block data
        let data = match blend_file.read_block_data(block_index) {
            Ok(data) => data,
            Err(_) => return None,
        };

        let reader = match blend_file.create_field_reader(&data) {
            Ok(reader) => reader,
            Err(_) => return None,
        };

        // Most datablocks start with an `ID` struct, which contains the name.
        // We can read this directly. If it fails, it's not a named block.
        let name_result = reader.read_field_string("ID", "name");

        match name_result {
            Ok(raw_name) => {
                let name = raw_name.trim_end_matches('\0').trim();
                if name.is_empty() {
                    return None;
                }

                // Remove type prefix if present (e.g., "MECube" -> "Cube", "OBCube" -> "Cube")
                // Blender names often start with a 2-character type code
                if name.len() > 2 {
                    let prefix = &name[0..2];
                    // Only strip prefix if it looks like a type code (2 uppercase letters)
                    if prefix.chars().all(|c| c.is_ascii_uppercase()) {
                        Some(name[2..].to_string())
                    } else {
                        Some(name.to_string())
                    }
                } else {
                    Some(name.to_string())
                }
            }
            Err(_) => None,
        }
    }

    /// Get a display name for a block, combining type and user name if available
    ///
    /// Examples:
    /// - "Object (Cube)" if name is available
    /// - "Object" if name is not available
    pub fn get_display_name<R: Read + Seek>(
        block_index: usize,
        blend_file: &mut BlendFile<R>,
        block_code: &str,
    ) -> String {
        match Self::resolve_name(block_index, blend_file) {
            Some(name) => {
                let mut display_name = String::with_capacity(block_code.len() + name.len() + 3);
                display_name.push_str(block_code);
                display_name.push_str(" (");
                display_name.push_str(&name);
                display_name.push(')');
                display_name
            }
            None => block_code.to_string(),
        }
    }
}
