use dot001_error::Dot001Error;
use dot001_parser::ParseOptions;
use log::error;
use std::path::PathBuf;

pub fn cmd_diff(
    file1_path: PathBuf,
    file2_path: PathBuf,
    only_modified: bool,
    format: crate::OutputFormat,
    ascii: bool,
    options: &ParseOptions,
    no_auto_decompress: bool,
) -> Result<(), Dot001Error> {
    let mut blend_file1 = crate::util::load_blend_file(&file1_path, options, no_auto_decompress)?;
    let mut blend_file2 = crate::util::load_blend_file(&file2_path, options, no_auto_decompress)?;
    let differ = dot001_diff::BlendDiffer::new();
    let diff_result = differ
        .diff(&mut blend_file1, &mut blend_file2)
        .map_err(|e| std::io::Error::other(format!("Diff error: {e}")))?;
    println!(
        "Comparing {} vs {}",
        file1_path.display(),
        file2_path.display()
    );
    println!("Summary:");
    println!("  Total blocks: {}", diff_result.summary.total_blocks);
    println!("  Modified: {}", diff_result.summary.modified_blocks);
    println!("  Added: {}", diff_result.summary.added_blocks);
    println!("  Removed: {}", diff_result.summary.removed_blocks);
    println!("  Unchanged: {}", diff_result.summary.unchanged_blocks);
    println!();
    match format {
        crate::OutputFormat::Tree => {
            crate::diff_formatter::DiffFormatter::display_tree(
                &diff_result,
                &mut blend_file1,
                only_modified,
                ascii,
                true,
            )?;
        }
        crate::OutputFormat::Json => match serde_json::to_string_pretty(&diff_result) {
            Ok(json) => println!("{json}"),
            Err(e) => error!("Failed to serialize diff result to JSON: {e}"),
        },
        crate::OutputFormat::Flat => {
            crate::diff_formatter::DiffFormatter::display_flat(&diff_result, only_modified);
        }
    }
    Ok(())
}
