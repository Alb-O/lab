/// Update the filepath of a linked library file (LI block) and save changes to file
///
/// This function modifies the filepath field of a Library block in the blend file.
///
/// ### Example:
/// ```rust,no_run
/// use dot001_editor::BlendEditor;
/// fn main() -> Result<(), Box<dyn std::error::Error>> {
///     BlendEditor::update_libpath_and_save("file.blend", 42, "//libs/other.blend", false)?;
///     Ok(())
/// }
/// ```
/// # dot001-editor - EXPERIMENTAL FUNCTIONALITY
///
/// This crate provides editing capabilities for Blender .blend files.
///
/// ## WARNING: EXPERIMENTAL
///
/// This functionality directly modifies .blend file binary data. While designed
/// to be safe, there is inherent risk when modifying binary file formats.
/// Always work with backup copies of your files.
///
/// ### Current Capabilities:
/// - ID name modification for datablocks
/// - Input validation and safety checks
/// - File verification after modifications
///
/// ### Recommendations:
/// - Use only on backup copies
/// - Test modified files in Blender before production use
/// - Validate results after operations
pub mod commands;
use dot001_error::{Dot001Error, EditorErrorKind, Result};
use dot001_parser::BlendFile;
use std::io::{Read, Seek};
use std::path::Path;

/// Experimental blend file editor
///
/// Provides functionality to modify ID names in blend files.
/// Always work with backup copies and validate results.
pub struct BlendEditor;

impl BlendEditor {
    /// Update the filepath of a linked library file (LI block) and save changes to file
    ///
    /// This function modifies the filepath field of a Library block in the blend file.
    ///
    /// ### Example:
    /// ```rust,no_run
    /// use dot001_editor::BlendEditor;
    /// fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     BlendEditor::update_libpath_and_save("file.blend", 42, "//libs/other.blend", false)?;
    ///     Ok(())
    /// }
    /// ```
    pub fn update_libpath_and_save<P: AsRef<std::path::Path>>(
        file_path: P,
        block_index: usize,
        new_path: &str,
        no_validate: bool,
    ) -> Result<()> {
        crate::commands::libpath::LibPathCommand::update_libpath_and_save(
            file_path,
            block_index,
            new_path,
            no_validate,
        )
    }
    /// Rename an ID block and save changes to file
    ///
    /// This function modifies binary data in the blend file and writes changes to disk.
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
        crate::commands::rename::RenameCommand::rename_id_block_and_save(
            file_path,
            block_index,
            new_name,
        )
    }

    /// Rename an ID block (in-memory only, for testing)
    pub fn rename_id_block<R: Read + Seek>(
        blend_file: &mut BlendFile<R>,
        block_index: usize,
        new_name: &str,
    ) -> Result<()> {
        crate::commands::rename::RenameCommand::rename_id_block(blend_file, block_index, new_name)
    }
}

/// Maximum length for block names (including type prefix)
const MAX_BLOCK_NAME_LENGTH: usize = 64;

/// Validate that a new name meets safety requirements
pub(crate) fn validate_new_name(name: &str) -> Result<()> {
    // Check length
    if name.len() > MAX_BLOCK_NAME_LENGTH {
        return Err(Dot001Error::editor(
            format!(
                "Name too long (max {MAX_BLOCK_NAME_LENGTH} characters after type prefix): {name}"
            ),
            EditorErrorKind::NameTooLong,
        ));
    }

    // Check for valid ASCII printable characters only
    if !name.chars().all(|c| c.is_ascii() && !c.is_ascii_control()) {
        return Err(Dot001Error::editor(
            format!("Invalid characters in name (only ASCII printable allowed): {name}"),
            EditorErrorKind::InvalidCharacters,
        ));
    }

    // Don't allow empty names
    if name.trim().is_empty() {
        return Err(Dot001Error::editor(
            "Empty name".to_string(),
            EditorErrorKind::InvalidCharacters,
        ));
    }

    Ok(())
}
