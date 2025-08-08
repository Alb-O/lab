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
/// # dot001_editor - EXPERIMENTAL FUNCTIONALITY
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
use dot001_events::error::{EditorErrorKind, Error, Result};
use dot001_events::{
    event::{EditorEvent, Event},
    prelude::*,
};
use dot001_parser::BlendFile;
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
        // Emit started event
        emit_global_sync!(Event::Editor(EditorEvent::Started {
            operation: "update_libpath".to_string(),
            target_file: file_path.as_ref().to_path_buf(),
            block_count: Some(1),
        }));

        let result = crate::commands::libpath::LibPathCommand::update_libpath_and_save(
            file_path,
            block_index,
            new_path,
            no_validate,
        );

        // Emit result event
        match &result {
            Ok(()) => {
                emit_global_sync!(Event::Editor(EditorEvent::Finished {
                    operation: "update_libpath".to_string(),
                    blocks_modified: 1,
                    duration_ms: 0, // TODO: Add timing
                    success: true,
                }));
            }
            Err(e) => {
                let events_error = dot001_events::error::Error::editor(
                    e.user_message(),
                    dot001_events::error::EditorErrorKind::InvalidName,
                );
                emit_global_sync!(Event::Editor(EditorEvent::Error {
                    error: events_error,
                }));
            }
        }

        result
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
        // Emit started event
        emit_global_sync!(Event::Editor(EditorEvent::Started {
            operation: "rename_id_block".to_string(),
            target_file: file_path.as_ref().to_path_buf(),
            block_count: Some(1),
        }));

        let result = crate::commands::rename::RenameCommand::rename_id_block_and_save(
            file_path,
            block_index,
            new_name,
        );

        // Emit result event
        match &result {
            Ok(()) => {
                emit_global_sync!(Event::Editor(EditorEvent::Finished {
                    operation: "rename_id_block".to_string(),
                    blocks_modified: 1,
                    duration_ms: 0, // TODO: Add timing
                    success: true,
                }));
            }
            Err(e) => {
                let events_error = dot001_events::error::Error::editor(
                    e.user_message(),
                    dot001_events::error::EditorErrorKind::InvalidName,
                );
                emit_global_sync!(Event::Editor(EditorEvent::Error {
                    error: events_error,
                }));
            }
        }

        result
    }

    /// Rename an ID block (in-memory only, for testing)
    pub fn rename_id_block(
        blend_file: &mut BlendFile,
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
    // Emit validation started event
    emit_global_sync!(
        Event::Editor(EditorEvent::ValidationPerformed {
            validator: "name_validation".to_string(),
            passed: true, // Will be set to false if validation fails
            message: Some(format!("Validating name: {name}")),
        }),
        Severity::Debug
    );

    // Check length
    if name.len() > MAX_BLOCK_NAME_LENGTH {
        emit_global_sync!(
            Event::Editor(EditorEvent::ValidationPerformed {
                validator: "name_length_check".to_string(),
                passed: false,
                message: Some(format!("Name too long: {name}")),
            }),
            Severity::Debug
        );
        return Err(Error::editor(
            format!(
                "Name too long (max {MAX_BLOCK_NAME_LENGTH} characters after type prefix): {name}"
            ),
            EditorErrorKind::NameTooLong,
        ));
    }

    // Check for valid ASCII printable characters only
    if !name.chars().all(|c| c.is_ascii() && !c.is_ascii_control()) {
        emit_global_sync!(
            Event::Editor(EditorEvent::ValidationPerformed {
                validator: "name_character_check".to_string(),
                passed: false,
                message: Some(format!("Invalid characters in name: {name}")),
            }),
            Severity::Debug
        );
        return Err(Error::editor(
            format!("Invalid characters in name (only ASCII printable allowed): {name}"),
            EditorErrorKind::InvalidCharacters,
        ));
    }

    // Don't allow empty names
    if name.trim().is_empty() {
        emit_global_sync!(
            Event::Editor(EditorEvent::ValidationPerformed {
                validator: "name_empty_check".to_string(),
                passed: false,
                message: Some("Name cannot be empty".to_string()),
            }),
            Severity::Debug
        );
        return Err(Error::editor(
            "Empty name".to_string(),
            EditorErrorKind::InvalidCharacters,
        ));
    }

    // Emit validation success
    emit_global_sync!(
        Event::Editor(EditorEvent::ValidationPerformed {
            validator: "name_validation".to_string(),
            passed: true,
            message: Some(format!("Name validation passed: {name}")),
        }),
        Severity::Debug
    );

    Ok(())
}
