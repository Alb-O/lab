use dot001_editor::BlendEditor;
use dot001_error::{CliErrorKind, Dot001Error};
use dot001_parser::ParseOptions;
use log::error;
use std::path::PathBuf;

pub fn cmd_libpath(
    file_path: PathBuf,
    block_identifier: &str,
    new_path: String,
    dry_run: bool,
    no_validate: bool,
    options: &ParseOptions,
    no_auto_decompress: bool,
) -> Result<(), Dot001Error> {
    let mut blend_file = crate::util::load_blend_file(&file_path, options, no_auto_decompress)?;

    // Resolve the block identifier to a specific block index
    let Some(block_index) = crate::util::resolve_block_or_exit(block_identifier, &mut blend_file)
    else {
        return Ok(());
    };

    if dry_run {
        println!("[dry-run] Would update library path in block {block_index} to: {new_path}");
        return Ok(());
    }
    match BlendEditor::update_libpath_and_save(&file_path, block_index, &new_path, no_validate) {
        Ok(()) => {
            println!("Successfully updated library path in block {block_index} to: {new_path}");
            Ok(())
        }
        Err(e) => {
            error!("Failed to update library path: {e}");
            Err(Dot001Error::cli(
                format!("Editor error: {e}"),
                CliErrorKind::ExecutionFailed,
            ))
        }
    }
}
