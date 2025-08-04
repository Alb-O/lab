use dot001_editor::BlendEditor;
use dot001_error::{CliErrorKind, Dot001Error};
use std::path::PathBuf;

pub fn cmd_libpath(
    file: PathBuf,
    block_index: usize,
    new_path: String,
    dry_run: bool,
    no_validate: bool,
) -> Result<(), Dot001Error> {
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
            eprintln!("Error updating library path: {e}");
            Err(Dot001Error::cli(
                format!("Editor error: {e}"),
                CliErrorKind::ExecutionFailed,
            ))
        }
    }
}
