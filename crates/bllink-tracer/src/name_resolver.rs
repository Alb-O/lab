use bllink_parser::BlendFile;
use std::io::{Read, Seek};

/// Utility for resolving user-defined names from Blender datablocks
///
/// Most Blender datablocks start with an ID structure that contains:
/// - name[66]: User-defined name (including 2-character type prefix)
/// - lib: Pointer to library (for linked data)
/// - etc.
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

        // Try to read the name field from various datablock structures
        // Most datablocks have an ID structure at the beginning with a name field
        let name_result = reader
            .read_field_string("ID", "name")
            .or_else(|_| reader.read_field_string("Object", "name"))
            .or_else(|_| reader.read_field_string("Mesh", "name"))
            .or_else(|_| reader.read_field_string("Material", "name"))
            .or_else(|_| reader.read_field_string("Image", "name"))
            .or_else(|_| reader.read_field_string("Collection", "name"))
            .or_else(|_| reader.read_field_string("bNodeTree", "name"))
            .or_else(|_| reader.read_field_string("NodeTree", "name"))
            .or_else(|_| reader.read_field_string("Scene", "name"))
            .or_else(|_| reader.read_field_string("Camera", "name"))
            .or_else(|_| reader.read_field_string("Lamp", "name"))
            .or_else(|_| reader.read_field_string("Light", "name"))
            .or_else(|_| reader.read_field_string("World", "name"))
            .or_else(|_| reader.read_field_string("Texture", "name"))
            .or_else(|_| reader.read_field_string("Sound", "name"))
            .or_else(|_| reader.read_field_string("Library", "name"))
            .or_else(|_| reader.read_field_string("CacheFile", "name"));

        match name_result {
            Ok(raw_name) => {
                let name = raw_name.trim_end_matches('\0').trim();
                if name.is_empty() {
                    return None;
                }

                // Remove type prefix if present (e.g., "MECube" -> "Cube", "OBCube" -> "Cube")
                // Blender names often start with a 2-character type code
                if name.len() > 2 {
                    let potential_name = &name[2..];
                    // Only strip prefix if it looks like a type code (2 uppercase letters)
                    let prefix = &name[0..2];
                    if prefix.chars().all(|c| c.is_ascii_uppercase()) {
                        Some(potential_name.to_string())
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
            Some(name) => format!("{block_code} ({name})"),
            None => block_code.to_string(),
        }
    }
}
