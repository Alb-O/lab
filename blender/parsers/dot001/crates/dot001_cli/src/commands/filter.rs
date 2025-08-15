use crate::DisplayTemplate;
use crate::block_display::{BlockInfo, create_display_for_template, highlight_matches};
use crate::block_utils::BlockUtils;
use crate::commands::NameResolver;
use crate::output_utils::{CommandSummary, OutputUtils, TreeFormatter};
use crate::util::CommandContext;
use dot001_events::error::Error;
use dot001_parser::BlendFile;
use std::path::PathBuf;
use text_trees::StringTreeNode;

pub fn cmd_filter(
    file_path: PathBuf,
    filter_expressions: Vec<String>,
    format: crate::OutputFormat,
    template: DisplayTemplate,
    show_data: bool,
    json: bool,
    ctx: &CommandContext,
) -> Result<(), Error> {
    let mut blend_file = ctx.load_blend_file(&file_path)?;
    let mut filter_triples: Vec<(String, String, String)> = Vec::new();
    for expr in &filter_expressions {
        match parse_filter_expression(expr) {
            Ok((modifier, key, value)) => {
                filter_triples.push((modifier, key, value));
            }
            Err(e) => {
                return Err(crate::invalid_arguments_error(format!(
                    "Failed to parse filter expression '{expr}': {e}"
                )));
            }
        }
    }
    let filter_slice_triples: Vec<(&str, &str, &str)> = filter_triples
        .iter()
        .map(|(m, k, v)| (m.as_str(), k.as_str(), v.as_str()))
        .collect();
    let filter_spec = dot001_tracer::filter::build_filter_spec(&filter_slice_triples)?;
    let filter_engine = dot001_tracer::filter::FilterEngine::new();
    let mut filtered_indices = filter_engine.apply(&filter_spec, &blend_file)?;

    // Filter out DATA blocks by default unless show_data is true
    BlockUtils::filter_data_blocks_hashset(&mut filtered_indices, &blend_file, show_data);
    if json || matches!(format, crate::OutputFormat::Json) {
        let mut filtered_blocks: Vec<serde_json::Value> = Vec::new();
        for &i in &filtered_indices {
            // Copy block fields by value to avoid borrow conflicts
            let (code_str, size, count, old_address, block_offset) = {
                let Some(block) = blend_file.get_block(i) else {
                    continue; // Skip invalid block indices
                };
                (
                    dot001_parser::block_code_to_string(block.header.code),
                    block.header.size,
                    block.header.count,
                    block.header.old_address,
                    block.header_offset,
                )
            };
            let name = NameResolver::resolve_name(i, &blend_file);
            filtered_blocks.push(serde_json::json!({
                "index": i,
                "code": code_str,
                "size": size,
                "count": count,
                "address": format!("{:#x}", old_address),
                "file_offset": block_offset,
                "name": name
            }));
        }
        OutputUtils::try_print_json(&filtered_blocks, ctx, "filter results", |data| {
            serde_json::to_string_pretty(data)
        });
        return Ok(());
    } else {
        ctx.output.print_info_fmt(format_args!(
            "Filtered blocks from {}:",
            file_path.display()
        ));

        CommandSummary::new("Filter Results")
            .add_count("Total blocks", blend_file.blocks_len())
            .add_count("Filtered", filtered_indices.len())
            .print(ctx);
        match format {
            crate::OutputFormat::Flat => {
                let mut sorted_indices: Vec<_> = filtered_indices.into_iter().collect();
                sorted_indices.sort();
                for &i in &sorted_indices {
                    let (_code_str, size, count, old_address, _block_offset) = {
                        let Some(block) = blend_file.get_block(i) else {
                            continue; // Skip invalid block indices
                        };
                        (
                            dot001_parser::block_code_to_string(block.header.code),
                            block.header.size,
                            block.header.count,
                            block.header.old_address,
                            block.header_offset,
                        )
                    };
                    let block_info = BlockInfo::from_blend_file(i, &mut blend_file)
                        .unwrap_or_else(|_| BlockInfo::new(i, "????".to_string()));

                    let display = create_display_for_template(
                        block_info.clone(),
                        &template,
                        Some(size as u64),
                        Some(old_address),
                    );
                    ctx.output
                        .print_result_fmt(format_args!("  {display} • count: {count}"));

                    // Show highlighted name if available and we have name filters
                    if let Some(name) = &block_info.name {
                        if !name.is_empty()
                            && filter_slice_triples.iter().any(|(_, k, _)| *k == "name")
                        {
                            let highlighted_name = highlight_matches(name, &filter_slice_triples);
                            ctx.output
                                .print_result_fmt(format_args!("    Name: {highlighted_name}"));
                        }
                    }
                }
            }
            crate::OutputFormat::Tree => {
                // Build a tree using text_trees
                let indices_vec: Vec<usize> = filtered_indices.iter().copied().collect();
                let tree = build_filter_tree(
                    &indices_vec,
                    &mut blend_file,
                    &filter_slice_triples,
                    &template,
                );
                let formatter = TreeFormatter::new(false); // Use Unicode characters
                formatter.print_tree(&tree, ctx);
            }
            crate::OutputFormat::Json => return Ok(()),
        }
    }
    /// Build a simple flat tree for filtered blocks (no hierarchy, just a list)
    fn build_filter_tree(
        indices: &[usize],
        blend_file: &mut BlendFile,
        _filter_expressions: &[(&str, &str, &str)],
        template: &DisplayTemplate,
    ) -> StringTreeNode {
        let mut sorted_indices: Vec<_> = indices.to_vec();
        sorted_indices.sort();
        let children: Vec<StringTreeNode> = sorted_indices
            .iter()
            .filter_map(|&i| {
                let (_code_str, size, _count, old_address, _block_offset) = {
                    let block = blend_file.get_block(i)?;
                    Some((
                        dot001_parser::block_code_to_string(block.header.code),
                        block.header.size,
                        block.header.count,
                        block.header.old_address,
                        block.header_offset,
                    ))
                }?;
                let block_info = match BlockInfo::from_blend_file(i, blend_file) {
                    Ok(info) => info,
                    Err(_) => return None,
                };

                let display = create_display_for_template(
                    block_info,
                    template,
                    Some(size as u64),
                    Some(old_address),
                );
                let label = display.to_string();
                Some(StringTreeNode::new(label))
            })
            .collect();
        StringTreeNode::with_child_nodes("Filtered Blocks".to_string(), children.into_iter())
    }
    Ok(())
}

pub fn parse_filter_expression(
    expr: &str,
) -> std::result::Result<(String, String, String), Box<dyn std::error::Error>> {
    if expr.is_empty() {
        return Err("Empty filter expression".into());
    }
    let mut chars = expr.chars();
    let first_char = chars.next().unwrap();
    let (include_sign, rest) = if first_char == '+' || first_char == '-' {
        (first_char, chars.as_str())
    } else {
        ('+', expr)
    };
    let mut recursion = String::new();
    let mut key_value = rest;
    for (i, ch) in rest.char_indices() {
        if ch.is_ascii_digit() || ch == '*' {
            recursion.push(ch);
        } else {
            key_value = &rest[i..];
            break;
        }
    }
    let parts: Vec<&str> = key_value.splitn(2, '=').collect();
    let (key, value) = if parts.len() == 2 {
        (parts[0].to_string(), parts[1].to_string())
    } else {
        // If no '=' is found, default to name matching
        ("name".to_string(), key_value.to_string())
    };
    if key.is_empty() {
        return Err("Filter key cannot be empty".into());
    }
    let modifier = format!("{include_sign}{recursion}");
    Ok((modifier, key, value))
}
