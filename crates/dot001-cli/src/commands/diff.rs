use crate::DisplayTemplate;
use crate::output_utils::{CommandSummary, OutputUtils};
use crate::util::CommandContext;
use dot001_diff::DiffEngine;
use dot001_error::Dot001Error;
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
    
    // Use modern policy-based diff engine instead of legacy hardcoded logic
    let differ = dot001_diff::PolicyDiffEngine::with_default_policies();
    let diff_result = differ
        .diff(&mut blend_file1, &mut blend_file2)
        .map_err(|e| std::io::Error::other(format!("Diff error: {e}")))?;
    ctx.output.print_info_fmt(format_args!(
        "Comparing {} vs {}",
        file1_path.display(),
        file2_path.display()
    ));

    CommandSummary::new("Summary")
        .add_count("Total blocks", diff_result.summary.total_blocks)
        .add_count("Modified", diff_result.summary.modified_blocks)
        .add_count("Added", diff_result.summary.added_blocks)
        .add_count("Removed", diff_result.summary.removed_blocks)
        .add_count("Unchanged", diff_result.summary.unchanged_blocks)
        .print(ctx);
    match format {
        crate::OutputFormat::Tree => {
            crate::diff_formatter::DiffFormatter::display_tree(
                &diff_result,
                &mut blend_file1,
                only_modified,
                template,
                ascii,
                ctx,
            )?;
        }
        crate::OutputFormat::Json => {
            OutputUtils::try_print_json(&diff_result, ctx, "diff result", |data| {
                serde_json::to_string_pretty(data)
            });
        }
        crate::OutputFormat::Flat => {
            crate::diff_formatter::DiffFormatter::display_flat(&diff_result, only_modified, ctx);
        }
    }
    Ok(())
}
