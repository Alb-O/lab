use crate::DisplayTemplate;
use crate::util::CommandContext;
use dot001_error::Dot001Error;
use log::error;
use std::path::PathBuf;

pub fn cmd_diff(
    file1_path: PathBuf,
    file2_path: PathBuf,
    only_modified: bool,
    format: crate::OutputFormat,
    template: DisplayTemplate,
    ascii: bool,
    ctx: &CommandContext,
) -> Result<(), Dot001Error> {
    let mut blend_file1 = ctx.load_blend_file(&file1_path)?;
    let mut blend_file2 = ctx.load_blend_file(&file2_path)?;
    let differ = dot001_diff::BlendDiffer::new();
    let diff_result = differ
        .diff(&mut blend_file1, &mut blend_file2)
        .map_err(|e| std::io::Error::other(format!("Diff error: {e}")))?;
    ctx.output.print_info_fmt(format_args!(
        "Comparing {} vs {}",
        file1_path.display(),
        file2_path.display()
    ));
    ctx.output.print_info("Summary:");
    ctx.output.print_result_fmt(format_args!(
        "  Total blocks: {}",
        diff_result.summary.total_blocks
    ));
    ctx.output.print_result_fmt(format_args!(
        "  Modified: {}",
        diff_result.summary.modified_blocks
    ));
    ctx.output.print_result_fmt(format_args!(
        "  Added: {}",
        diff_result.summary.added_blocks
    ));
    ctx.output.print_result_fmt(format_args!(
        "  Removed: {}",
        diff_result.summary.removed_blocks
    ));
    ctx.output.print_result_fmt(format_args!(
        "  Unchanged: {}",
        diff_result.summary.unchanged_blocks
    ));
    ctx.output.print_result("");
    match format {
        crate::OutputFormat::Tree => {
            crate::diff_formatter::DiffFormatter::display_tree(
                &diff_result,
                &mut blend_file1,
                only_modified,
                template,
                ascii,
            )?;
        }
        crate::OutputFormat::Json => match serde_json::to_string_pretty(&diff_result) {
            Ok(json) => ctx.output.print_result(&json),
            Err(e) => error!("Failed to serialize diff result to JSON: {e}"),
        },
        crate::OutputFormat::Flat => {
            crate::diff_formatter::DiffFormatter::display_flat(&diff_result, only_modified);
        }
    }
    Ok(())
}
