use dot001_editor::BlendEditor;
use dot001_parser::{BlendError, Result};
use std::path::PathBuf;

pub fn cmd_libpath(
    file: PathBuf,
    block_index: usize,
    new_path: String,
    dry_run: bool,
    no_validate: bool,
) -> Result<()> {
    if dry_run {
        println!("[dry-run] Would update library path in block {block_index} to: {new_path}");
        return Ok(());
    }
    match BlendEditor::update_libpath_and_save(&file, block_index, &new_path, no_validate) {
        Ok(()) => {
            println!("Successfully updated library path in block {block_index} to: {new_path}");
            Ok(())
        }
        Err(e) => {
            use dot001_editor::EditorError;
            let mapped = match e {
                EditorError::Io(ioe) => BlendError::Io(ioe),
                EditorError::BlendFile(be) => {
                    BlendError::InvalidData(format!("Blend file error: {be}"))
                }
                EditorError::BlockNotFound(idx) => BlendError::InvalidBlockIndex(idx),
                EditorError::NoIdStructure => BlendError::InvalidField(
                    "Block has no ID structure (not a named datablock)".to_string(),
                ),
                EditorError::NameTooLong(s) => {
                    BlendError::InvalidData(format!("Name too long: {s}"))
                }
                EditorError::InvalidCharacters(s) => {
                    BlendError::InvalidData(format!("Invalid characters: {s}"))
                }
            };
            Err(mapped)
        }
    }
}
