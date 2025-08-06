use crate::util::{BlockDisplay, BlockInfo, CommandContext, SimpleFormatter, colorize_name};
use dot001_error::Dot001Error;
use log::{error, info};
use std::path::PathBuf;

pub fn cmd_rename(
    file_path: PathBuf,
    block_identifier: &str,
    new_name: String,
    dry_run: bool,
    ctx: &CommandContext,
) -> Result<(), Dot001Error> {
    use dot001_editor::BlendEditor;
    let mut blend_file = ctx.load_blend_file(&file_path)?;

    // Resolve the block identifier to a specific block index
    let Some(block_index) = crate::util::resolve_block_or_exit(block_identifier, &mut blend_file)
    else {
        return Ok(());
    };
    let block_code = {
        let Some(block) = blend_file.get_block(block_index) else {
            error!("Block index {block_index} is out of range");
            return Ok(());
        };
        String::from_utf8_lossy(&block.header.code)
            .trim_end_matches('\0')
            .to_string()
    };
    #[cfg(feature = "trace")]
    let current_name_opt =
        { dot001_tracer::NameResolver::resolve_name(block_index, &mut blend_file) };

    #[cfg(not(feature = "trace"))]
    let current_name_opt = Some(format!("Block{}", block_index));

    match current_name_opt {
        Some(current_name) => {
            let block_info =
                BlockInfo::with_name(block_index, block_code.clone(), current_name.clone());
            let block_display = BlockDisplay::new(block_info).with_formatter(SimpleFormatter);
            let _colored_current_name = colorize_name(&current_name);
            let colored_new_name = colorize_name(&new_name);
            if dry_run {
                info!("Would rename {block_display} block to '{colored_new_name}'");
            } else {
                info!("Renaming {block_display} block to '{colored_new_name}'");
                match BlendEditor::rename_id_block_and_save(&file_path, block_index, &new_name) {
                    Ok(()) => {
                        #[cfg(feature = "trace")]
                        {
                            let mut updated_blend_file = ctx.load_blend_file(&file_path)?;
                            match dot001_tracer::NameResolver::resolve_name(
                                block_index,
                                &mut updated_blend_file,
                            ) {
                                Some(updated_name) => {
                                    if updated_name == new_name {
                                        let colored_updated_name = colorize_name(&updated_name);
                                        ctx.output.print_result_fmt(format_args!(
                                            "Success: Block renamed to '{colored_updated_name}'"
                                        ));
                                    } else {
                                        let colored_updated_name = colorize_name(&updated_name);
                                        let colored_expected_name = colorize_name(&new_name);
                                        ctx.output.print_error(&format!(
                                            "Warning: Name is '{colored_updated_name}', expected '{colored_expected_name}'"
                                        ));
                                    }
                                }
                                None => {
                                    ctx.output
                                        .print_error("Warning: Could not verify name change");
                                }
                            }
                        }
                        #[cfg(not(feature = "trace"))]
                        {
                            ctx.output.print_result(
                                "Success: Block renamed (verification unavailable without trace feature)"
                            );
                        }
                    }
                    Err(e) => {
                        error!("Failed to rename block: {e}");
                    }
                }
            }
        }
        None => {
            error!("Block {block_index} is not a named datablock");
        }
    }
    Ok(())
}
