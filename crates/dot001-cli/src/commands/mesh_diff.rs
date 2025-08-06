use crate::util::{BlockDisplay, BlockInfo, CommandContext, SimpleFormatter};
use dot001_error::Dot001Error;
use log::error;
use std::path::PathBuf;

pub fn cmd_mesh_diff(
    file1_path: PathBuf,
    file2_path: PathBuf,
    mesh_identifier: Option<&str>,
    verbose_provenance: bool,
    json: bool,
    ctx: &CommandContext,
) -> Result<(), Dot001Error> {
    let mut blend_file1 = ctx.load_blend_file(&file1_path)?;
    let mut blend_file2 = ctx.load_blend_file(&file2_path)?;
    let differ = dot001_diff::BlendDiffer::new()
        .with_provenance_analysis(true)
        .with_provenance_config(|analyzer| analyzer.with_verbose(verbose_provenance));
    ctx.output.print_info("Enhanced Mesh Diff Analysis");
    ctx.output.print_info("==========================");
    ctx.output
        .print_info_fmt(format_args!("File 1: {}", file1_path.display()));
    ctx.output
        .print_info_fmt(format_args!("File 2: {}", file2_path.display()));
    ctx.output.print_info("");

    if let Some(mesh_id) = mesh_identifier {
        // Resolve the mesh identifier to a specific ME block index
        let Some(me_index) =
            crate::util::resolve_typed_block_or_exit(mesh_id, "ME", &mut blend_file1)
        else {
            return Ok(());
        };
        match differ.analyze_mesh_block(me_index, &mut blend_file1, &mut blend_file2) {
            Ok(analysis) => {
                if json {
                    match serde_json::to_string_pretty(&analysis) {
                        Ok(json_str) => ctx.output.print_result(&json_str),
                        Err(e) => error!("Failed to serialize to JSON: {e}"),
                    }
                } else {
                    let block_info = BlockInfo::from_blend_file(me_index, &mut blend_file1)
                        .unwrap_or_else(|_| BlockInfo::new(me_index, "ME".to_string()));
                    let me_display =
                        BlockDisplay::new(block_info.clone()).with_formatter(SimpleFormatter);
                    ctx.output.print_info_fmt(format_args!(
                        "Analysis for ME block {} ({}):",
                        block_info.index, me_display
                    ));
                    ctx.output.print_result_fmt(format_args!(
                        "  Classification: {:?}",
                        analysis.overall_classification
                    ));
                    ctx.output.print_result_fmt(format_args!(
                        "  Is True Edit: {}",
                        analysis.is_true_edit
                    ));
                    ctx.output
                        .print_result_fmt(format_args!("  Summary: {}", analysis.summary));
                    ctx.output.print_result("");
                    if let Some(before) = &analysis.before_provenance {
                        ctx.output.print_result_fmt(format_args!(
                            "  Before: {} referenced DATA blocks",
                            before.referenced_data_blocks.len()
                        ));
                    }
                    if let Some(after) = &analysis.after_provenance {
                        ctx.output.print_result_fmt(format_args!(
                            "  After: {} referenced DATA blocks",
                            after.referenced_data_blocks.len()
                        ));
                    }
                    ctx.output.print_result("  DATA Block Correlations:");
                    for (i, correlation) in analysis.data_correlations.iter().enumerate() {
                        ctx.output.print_result_fmt(format_args!(
                            "    {}: {:?} (confidence: {:.2}) - {}",
                            i + 1,
                            correlation.change_class,
                            correlation.confidence,
                            correlation.rationale
                        ));
                    }
                }
            }
            Err(e) => {
                error!("Failed to analyze ME block {me_index}: {e}");
            }
        }
    } else {
        let me_blocks: Vec<usize> = (0..blend_file1.blocks_len())
            .filter_map(|i| {
                blend_file1.get_block(i).and_then(|block| {
                    let code = String::from_utf8_lossy(&block.header.code);
                    if code.trim_end_matches('\0') == "ME" {
                        Some(i)
                    } else {
                        None
                    }
                })
            })
            .collect();
        ctx.output.print_info_fmt(format_args!(
            "Found {} ME blocks to analyze",
            me_blocks.len()
        ));
        ctx.output.print_info("");
        let mut analyses = Vec::new();
        for &me_index in &me_blocks {
            match differ.analyze_mesh_block(me_index, &mut blend_file1, &mut blend_file2) {
                Ok(analysis) => {
                    if !json {
                        let block_info = BlockInfo::from_blend_file(me_index, &mut blend_file1)
                            .unwrap_or_else(|_| BlockInfo::new(me_index, "ME".to_string()));
                        let me_display =
                            BlockDisplay::new(block_info.clone()).with_formatter(SimpleFormatter);
                        ctx.output.print_result_fmt(format_args!(
                            "ME block {} ({}): {} ({})",
                            block_info.index,
                            me_display,
                            if analysis.is_true_edit {
                                "TRUE EDIT"
                            } else {
                                "Layout/Noise"
                            },
                            analysis.summary
                        ));
                    }
                    analyses.push(analysis);
                }
                Err(e) => {
                    error!("Failed to analyze ME block {me_index}: {e}");
                }
            }
        }
        if json {
            match serde_json::to_string_pretty(&analyses) {
                Ok(json_str) => ctx.output.print_result(&json_str),
                Err(e) => error!("Failed to serialize to JSON: {e}"),
            }
        } else {
            ctx.output.print_result("");
            let true_edits = analyses.iter().filter(|a| a.is_true_edit).count();
            let layout_changes = analyses.len() - true_edits;
            ctx.output.print_result_fmt(format_args!(
                "Summary: {true_edits} true edits, {layout_changes} layout/noise changes"
            ));
        }
    }
    Ok(())
}
