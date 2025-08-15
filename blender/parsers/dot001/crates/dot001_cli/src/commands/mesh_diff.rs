use crate::DisplayTemplate;
use crate::block_display::{BlockInfo, create_display_for_template};
use crate::block_ops::CommandHelper;
use crate::output_utils::{CommandSummary, OutputUtils};
use crate::util::CommandContext;
use dot001_events::error::Error;
use log::error;
use std::path::PathBuf;

pub fn cmd_mesh_diff(
    file1_path: PathBuf,
    file2_path: PathBuf,
    mesh_identifier: Option<&str>,
    verbose_provenance: bool,
    template: DisplayTemplate,
    json: bool,
    ctx: &CommandContext,
) -> Result<(), Error> {
    let mut blend_file1 = ctx.load_blend_file(&file1_path)?;
    let mut blend_file2 = ctx.load_blend_file(&file2_path)?;
    let differ = dot001_diff::BlendDiffer::new()
        .with_provenance_analysis(true)
        .with_provenance_config(|analyzer| analyzer.with_verbose(verbose_provenance));
    CommandSummary::new("Enhanced Mesh Diff Analysis")
        .add_item("File 1", file1_path.display().to_string())
        .add_item("File 2", file2_path.display().to_string())
        .print(ctx);

    if let Some(mesh_id) = mesh_identifier {
        // Resolve the mesh identifier to a specific ME block index
        let me_index = {
            let mut helper = CommandHelper::new(&mut blend_file1, ctx);
            let Some(index) = helper.resolve_typed_block_or_return(mesh_id, "ME")? else {
                return Ok(());
            };
            index
        };
        match differ.analyze_mesh_block(me_index, &mut blend_file1, &mut blend_file2) {
            Ok(analysis) => {
                if json {
                    OutputUtils::try_print_json(&analysis, ctx, "mesh analysis", |data| {
                        serde_json::to_string_pretty(data)
                    });
                } else {
                    let block_info = BlockInfo::from_blend_file(me_index, &mut blend_file1)
                        .unwrap_or_else(|_| BlockInfo::new(me_index, "ME".to_string()));

                    let (size, address) = blend_file1
                        .get_block(me_index)
                        .map(|block| (block.header.size as u64, block.header.old_address))
                        .unwrap_or((0, 0));

                    let me_display = create_display_for_template(
                        block_info.clone(),
                        &template,
                        Some(size),
                        Some(address),
                    );
                    ctx.output.print_info_fmt(format_args!(
                        "Analysis for ME block {} ({}):",
                        block_info.index, me_display
                    ));

                    let mut analysis_summary = CommandSummary::new("Analysis Results")
                        .add_item(
                            "Classification",
                            format!("{:?}", analysis.overall_classification),
                        )
                        .add_item("Is True Edit", analysis.is_true_edit.to_string())
                        .add_item("Summary", analysis.summary.clone());

                    if let Some(before) = &analysis.before_provenance {
                        analysis_summary = analysis_summary.add_count(
                            "Before: Referenced DATA blocks",
                            before.referenced_data_blocks.len(),
                        );
                    }
                    if let Some(after) = &analysis.after_provenance {
                        analysis_summary = analysis_summary.add_count(
                            "After: Referenced DATA blocks",
                            after.referenced_data_blocks.len(),
                        );
                    }

                    analysis_summary.print(ctx);
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
                    let code = dot001_parser::block_code_to_string(block.header.code);
                    if code == "ME" { Some(i) } else { None }
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

                        let (size, address) = blend_file1
                            .get_block(me_index)
                            .map(|block| (block.header.size as u64, block.header.old_address))
                            .unwrap_or((0, 0));

                        let me_display = create_display_for_template(
                            block_info.clone(),
                            &template,
                            Some(size),
                            Some(address),
                        );
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
