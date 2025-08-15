use crate::block_ops::CommandHelper;
use crate::output_utils::CommandSummary;
use crate::util::CommandContext;
use dot001_editor::BlendEditor;
use dot001_events::error::{CliErrorKind, Error};
use log::error;
use std::path::PathBuf;

pub fn cmd_libpath(
    file_path: PathBuf,
    block_identifier: &str,
    new_path: String,
    dry_run: bool,
    no_validate: bool,
    ctx: &CommandContext,
) -> Result<(), Error> {
    let mut blend_file = ctx.load_blend_file(&file_path)?;

    // Resolve the block identifier to a specific block index
    let block_index = {
        let mut helper = CommandHelper::new(&mut blend_file, ctx);
        let Some(index) = helper.resolve_block_or_return(block_identifier)? else {
            return Ok(());
        };
        index
    };

    if dry_run {
        CommandSummary::new("Dry Run")
            .add_item("Block", block_index.to_string())
            .add_item("New Path", new_path.clone())
            .add_item("Action", "Would update library path".to_string())
            .print(ctx);
        return Ok(());
    }
    match BlendEditor::update_libpath_and_save(&file_path, block_index, &new_path, no_validate) {
        Ok(()) => {
            CommandSummary::new("Success")
                .add_item("Block", block_index.to_string())
                .add_item("New Path", new_path)
                .add_item("Action", "Updated library path".to_string())
                .print(ctx);
            Ok(())
        }
        Err(e) => {
            error!("Failed to update library path: {e}");
            Err(Error::cli(
                format!("Editor error: {e}"),
                CliErrorKind::ExecutionFailed,
            ))
        }
    }
}
