use dot001_editor::BlendEditor;
use std::path::PathBuf;

pub fn cmd_libpath(
    file: PathBuf,
    block_index: usize,
    new_path: String,
    dry_run: bool,
    no_validate: bool,
) -> anyhow::Result<()> {
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
            Err(anyhow::anyhow!("Editor error: {}", e))
        }
    }
}
