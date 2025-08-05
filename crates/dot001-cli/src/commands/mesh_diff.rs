use crate::util::OutputHandler;
use dot001_error::Dot001Error;
use dot001_parser::ParseOptions;
use log::error;
use std::path::PathBuf;

pub fn cmd_mesh_diff(
    file1_path: PathBuf,
    file2_path: PathBuf,
    mesh_identifier: Option<&str>,
    verbose_provenance: bool,
    json: bool,
    options: &ParseOptions,
    no_auto_decompress: bool,
    output: &OutputHandler,
) -> Result<(), Dot001Error> {
    let mut blend_file1 = crate::util::load_blend_file(&file1_path, options, no_auto_decompress)?;
    let mut blend_file2 = crate::util::load_blend_file(&file2_path, options, no_auto_decompress)?;
    let differ = dot001_diff::BlendDiffer::new()
        .with_provenance_analysis(true)
        .with_provenance_config(|analyzer| analyzer.with_verbose(verbose_provenance));
    output.print_info("Enhanced Mesh Diff Analysis");
    output.print_info("==========================");
    output.print_info_fmt(format_args!("File 1: {}", file1_path.display()));
    output.print_info_fmt(format_args!("File 2: {}", file2_path.display()));
    output.print_info("");

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
                        Ok(json_str) => output.print_result(&json_str),
                        Err(e) => error!("Failed to serialize to JSON: {e}"),
                    }
                } else {
                    let me_name = dot001_tracer::NameResolver::get_display_name(
                        me_index,
                        &mut blend_file1,
                        "ME",
                    );
                    output.print_info_fmt(format_args!(
                        "Analysis for ME block {me_index} ({me_name}):"
                    ));
                    output.print_result_fmt(format_args!(
                        "  Classification: {:?}",
                        analysis.overall_classification
                    ));
                    output.print_result_fmt(format_args!(
                        "  Is True Edit: {}",
                        analysis.is_true_edit
                    ));
                    output.print_result_fmt(format_args!("  Summary: {}", analysis.summary));
                    output.print_result("");
                    if let Some(before) = &analysis.before_provenance {
                        output.print_result_fmt(format_args!(
                            "  Before: {} referenced DATA blocks",
                            before.referenced_data_blocks.len()
                        ));
                    }
                    if let Some(after) = &analysis.after_provenance {
                        output.print_result_fmt(format_args!(
                            "  After: {} referenced DATA blocks",
                            after.referenced_data_blocks.len()
                        ));
                    }
                    output.print_result("  DATA Block Correlations:");
                    for (i, correlation) in analysis.data_correlations.iter().enumerate() {
                        output.print_result_fmt(format_args!(
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
        output.print_info_fmt(format_args!(
            "Found {} ME blocks to analyze",
            me_blocks.len()
        ));
        output.print_info("");
        let mut analyses = Vec::new();
        for &me_index in &me_blocks {
            match differ.analyze_mesh_block(me_index, &mut blend_file1, &mut blend_file2) {
                Ok(analysis) => {
                    if !json {
                        let me_name = dot001_tracer::NameResolver::get_display_name(
                            me_index,
                            &mut blend_file1,
                            "ME",
                        );
                        output.print_result_fmt(format_args!(
                            "ME block {} ({}): {} ({})",
                            me_index,
                            me_name,
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
                Ok(json_str) => output.print_result(&json_str),
                Err(e) => error!("Failed to serialize to JSON: {e}"),
            }
        } else {
            output.print_result("");
            let true_edits = analyses.iter().filter(|a| a.is_true_edit).count();
            let layout_changes = analyses.len() - true_edits;
            output.print_result_fmt(format_args!(
                "Summary: {true_edits} true edits, {layout_changes} layout/noise changes"
            ));
        }
    }
    Ok(())
}
