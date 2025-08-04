//! # dot001-editor - EXPERIMENTAL FUNCTIONALITY
//!
//! This crate provides editing capabilities for Blender .blend files.
//!
//! ## WARNING: EXPERIMENTAL
//!
//! This functionality directly modifies .blend file binary data. While designed
//! to be safe, there is inherent risk when modifying binary file formats.
//! Always work with backup copies of your files.
//!
//! ### Current Capabilities:
//! - ID name modification for datablocks
//! - Input validation and safety checks
//! - File verification after modifications
//!
//! ### Recommendations:
//! - Use only on backup copies
//! - Test modified files in Blender before production use
//! - Validate results after operations

use dot001_parser::BlendFile;
use dot001_tracer::NameResolver;
use std::fs::OpenOptions;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum EditorError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Blend file error: {0}")]
    BlendFile(#[from] dot001_parser::BlendError),
    #[error("Block not found: {0}")]
    BlockNotFound(usize),
    #[error("Block has no ID structure (not a named datablock)")]
    NoIdStructure,
    #[error("Name too long (max 64 characters after type prefix): {0}")]
    NameTooLong(String),
    #[error("Invalid characters in name (only ASCII printable allowed): {0}")]
    InvalidCharacters(String),
}

pub type Result<T> = std::result::Result<T, EditorError>;

/// Experimental blend file editor
///
/// Provides functionality to modify ID names in blend files.
/// Always work with backup copies and validate results.
pub struct BlendEditor;

impl BlendEditor {
    /// Rename an ID block and save changes to file
    ///
    /// This function modifies binary data in the blend file and writes changes to disk.
    ///
    /// ### Parameters:
    /// - `file_path`: Path to the blend file to modify
    /// - `block_index`: Index of the block to rename
    /// - `new_name`: New name (will be prefixed with block type code)
    ///
    /// ### Requirements:
    /// - Block must be a named datablock (have ID structure)
    /// - Name must be 64 characters or less (after type prefix)
    /// - Name must contain only ASCII printable characters
    /// - File must be writable
    ///
    /// ### Example:
    /// ```rust,no_run
    /// use dot001_editor::BlendEditor;
    /// fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     // Rename collection "Lighting" to "Lights"
    ///     BlendEditor::rename_id_block_and_save("file.blend", 5015, "Lights")?;
    ///     Ok(())
    /// }
    /// ```
    pub fn rename_id_block_and_save<P: AsRef<Path>>(
        file_path: P,
        block_index: usize,
        new_name: &str,
    ) -> Result<()> {
        // Validate input
        Self::validate_new_name(new_name)?;

        // First, read the file to get block information
        let file = std::fs::File::open(&file_path)?;
        let mut reader = std::io::BufReader::new(file);
        let mut blend_file = BlendFile::new(&mut reader)?;

        // Check if block exists
        if block_index >= blend_file.blocks.len() {
            return Err(EditorError::BlockNotFound(block_index));
        }

        // Get the block code to determine the type prefix
        let block_code = {
            let block = &blend_file.blocks[block_index];
            String::from_utf8_lossy(&block.header.code)
                .trim_end_matches('\0')
                .to_string()
        };

        // Verify this is a named datablock by trying to read current name
        let _current_name = NameResolver::resolve_name(block_index, &mut blend_file)
            .ok_or(EditorError::NoIdStructure)?;

        // Get block offset information
        let block = &blend_file.blocks[block_index];
        let block_data_offset = block.data_offset;

        // Read the current block data
        let mut block_data = blend_file.read_block_data(block_index)?;

        // Create a field reader to locate the name field
        let reader = blend_file.create_field_reader(&block_data)?;

        // Get the offset of the name field in the ID structure
        let name_offset = reader
            .get_field_offset("ID", "name")
            .map_err(|_| EditorError::NoIdStructure)?;

        // Prepare the new name with type prefix
        let prefixed_name = format!("{block_code}{new_name}");
        let mut name_bytes = [0u8; 66]; // ID name field is 66 bytes

        // Copy the new name, ensuring null termination
        let name_bytes_to_copy = std::cmp::min(prefixed_name.len(), 65);
        name_bytes[..name_bytes_to_copy]
            .copy_from_slice(&prefixed_name.as_bytes()[..name_bytes_to_copy]);

        // Modify the block data in memory
        let start_offset = name_offset;
        let end_offset = start_offset + 66;

        if end_offset > block_data.len() {
            return Err(EditorError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Name field extends beyond block data",
            )));
        }

        // Replace the name in the block data
        block_data[start_offset..end_offset].copy_from_slice(&name_bytes);

        // Write the modified block data back to the file
        let mut file = OpenOptions::new().read(true).write(true).open(&file_path)?;

        // Seek to the block's data position in the file
        file.seek(SeekFrom::Start(block_data_offset))?;

        // Write the modified block data
        file.write_all(&block_data)?;
        file.flush()?;

        Ok(())
    }

    /// Rename an ID block (in-memory only, for testing)
    ///
    /// This function modifies binary data in memory but does not persist changes.
    /// Use rename_id_block_and_save() to actually modify files.
    pub fn rename_id_block<R: Read + Seek>(
        blend_file: &mut BlendFile<R>,
        block_index: usize,
        new_name: &str,
    ) -> Result<()> {
        // Validate input
        Self::validate_new_name(new_name)?;

        // Check if block exists
        if block_index >= blend_file.blocks.len() {
            return Err(EditorError::BlockNotFound(block_index));
        }

        // Get the block code to determine the type prefix
        let block_code = {
            let block = &blend_file.blocks[block_index];
            String::from_utf8_lossy(&block.header.code)
                .trim_end_matches('\0')
                .to_string()
        };

        // Verify this is a named datablock by trying to read current name
        let _current_name = NameResolver::resolve_name(block_index, blend_file)
            .ok_or(EditorError::NoIdStructure)?;

        // Read the block data
        let mut block_data = blend_file.read_block_data(block_index)?;

        // Create a field reader to locate the name field
        let reader = blend_file.create_field_reader(&block_data)?;

        // Get the offset of the name field in the ID structure
        let name_offset = reader
            .get_field_offset("ID", "name")
            .map_err(|_| EditorError::NoIdStructure)?;

        // Prepare the new name with type prefix
        let prefixed_name = format!("{block_code}{new_name}");
        let mut name_bytes = [0u8; 66]; // ID name field is 66 bytes

        // Copy the new name, ensuring null termination
        let name_bytes_to_copy = std::cmp::min(prefixed_name.len(), 65);
        name_bytes[..name_bytes_to_copy]
            .copy_from_slice(&prefixed_name.as_bytes()[..name_bytes_to_copy]);

        // Modify the binary data
        let start_offset = name_offset;
        let end_offset = start_offset + 66;

        if end_offset > block_data.len() {
            return Err(EditorError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Name field extends beyond block data",
            )));
        }

        // Replace the name in the block data
        block_data[start_offset..end_offset].copy_from_slice(&name_bytes);

        // Note: Changes are only in memory, not persisted to file

        Ok(())
    }

    /// Validate that a new name meets safety requirements
    fn validate_new_name(name: &str) -> Result<()> {
        // Check length (64 chars max after 2-char type prefix)
        if name.len() > 64 {
            return Err(EditorError::NameTooLong(name.to_string()));
        }

        // Check for valid ASCII printable characters only
        if !name.chars().all(|c| c.is_ascii() && !c.is_ascii_control()) {
            return Err(EditorError::InvalidCharacters(name.to_string()));
        }

        // Don't allow empty names
        if name.trim().is_empty() {
            return Err(EditorError::InvalidCharacters("Empty name".to_string()));
        }

        Ok(())
    }
}
