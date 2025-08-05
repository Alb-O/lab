use crate::commands::NameResolver;
use dot001_error::Dot001Error;
use dot001_parser::{BlendFile, ParseOptions};
use log::error;
use std::path::PathBuf;
use text_trees::{FormatCharacters, StringTreeNode, TreeFormatting};

pub fn cmd_filter(
    file_path: PathBuf,
    filter_expressions: Vec<String>,
    format: crate::OutputFormat,
    verbose: bool,
    json: bool,
    options: &ParseOptions,
    no_auto_decompress: bool,
) -> Result<(), Dot001Error> {
    let mut blend_file = crate::util::load_blend_file(&file_path, options, no_auto_decompress)?;
    let mut filter_triples: Vec<(String, String, String)> = Vec::new();
    for expr in &filter_expressions {
        match parse_filter_expression(expr) {
            Ok((modifier, key, value)) => {
                filter_triples.push((modifier, key, value));
            }
            Err(e) => {
                error!("Failed to parse filter expression '{expr}': {e}");
                std::process::exit(1);
            }
        }
    }
    let filter_slice_triples: Vec<(&str, &str, &str)> = filter_triples
        .iter()
        .map(|(m, k, v)| (m.as_str(), k.as_str(), v.as_str()))
        .collect();
    let filter_spec = dot001_tracer::filter::build_filter_spec(&filter_slice_triples)?;
    let filter_engine = dot001_tracer::filter::FilterEngine::new();
    let filtered_indices = filter_engine.apply(&filter_spec, &mut blend_file)?;
    if json || matches!(format, crate::OutputFormat::Json) {
        let mut filtered_blocks: Vec<serde_json::Value> = Vec::new();
        for &i in &filtered_indices {
            // Copy block fields by value to avoid borrow conflicts
            let (code_str, size, count, old_address, block_offset) = {
                let Some(block) = blend_file.get_block(i) else {
                    continue; // Skip invalid block indices
                };
                (
                    String::from_utf8_lossy(&block.header.code)
                        .trim_end_matches('\0')
                        .to_string(),
                    block.header.size,
                    block.header.count,
                    block.header.old_address,
                    block.header_offset,
                )
            };
            let name = NameResolver::resolve_name(i, &mut blend_file);
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
        match serde_json::to_string_pretty(&filtered_blocks) {
            Ok(json_str) => println!("{json_str}"),
            Err(e) => {
                error!("Failed to serialize filter results to JSON: {e}");
                std::process::exit(1);
            }
        }
        return Ok(());
    } else {
        println!("Filtered blocks from {}:", file_path.display());
        println!(
            "Total blocks: {}, Filtered: {}",
            blend_file.blocks_len(),
            filtered_indices.len()
        );
        println!();
        match format {
            crate::OutputFormat::Flat => {
                let mut sorted_indices: Vec<_> = filtered_indices.into_iter().collect();
                sorted_indices.sort();
                for &i in &sorted_indices {
                    let (code_str, size, count, old_address, block_offset) = {
                        let Some(block) = blend_file.get_block(i) else {
                            continue; // Skip invalid block indices
                        };
                        (
                            String::from_utf8_lossy(&block.header.code)
                                .trim_end_matches('\0')
                                .to_string(),
                            block.header.size,
                            block.header.count,
                            block.header.old_address,
                            block.header_offset,
                        )
                    };
                    let name = NameResolver::resolve_name(i, &mut blend_file);
                    if verbose {
                        println!(
                            "Block {i}: {code_str} (size: {size}, count: {count}, addr: {old_address:#x}, offset: {block_offset})"
                        );
                        if let Some(name) = &name {
                            if !name.is_empty() {
                                println!("  Name: {name}");
                            }
                        }
                    } else if let Some(name) = &name {
                        if !name.is_empty() {
                            println!("{i}: {code_str} ({name})");
                        } else {
                            println!("{i}: {code_str}");
                        }
                    } else {
                        println!("{i}: {code_str}");
                    }
                }
            }
            crate::OutputFormat::Tree => {
                // Build a tree using text_trees
                let indices_vec: Vec<usize> = filtered_indices.iter().copied().collect();
                let tree = build_filter_tree(&indices_vec, &mut blend_file);
                let format_chars = FormatCharacters::box_chars();
                let formatting = TreeFormatting::dir_tree(format_chars);
                match tree.to_string_with_format(&formatting) {
                    Ok(output) => println!("{output}"),
                    Err(e) => error!("Failed to format filter results tree: {e}"),
                }
            }
            crate::OutputFormat::Json => return Ok(()),
        }
    }
    /// Build a simple flat tree for filtered blocks (no hierarchy, just a list)
    fn build_filter_tree<R: std::io::Read + std::io::Seek>(
        indices: &[usize],
        blend_file: &mut BlendFile<R>,
    ) -> StringTreeNode {
        let mut sorted_indices: Vec<_> = indices.to_vec();
        sorted_indices.sort();
        let children: Vec<StringTreeNode> = sorted_indices
            .iter()
            .filter_map(|&i| {
                let (code_str, _size, _count, _old_address, _block_offset) = {
                    let block = blend_file.get_block(i)?;
                    Some((
                        String::from_utf8_lossy(&block.header.code)
                            .trim_end_matches('\0')
                            .to_string(),
                        block.header.size,
                        block.header.count,
                        block.header.old_address,
                        block.header_offset,
                    ))
                }?;
                let name = NameResolver::resolve_name(i, blend_file);
                let label = if let Some(name) = name {
                    if !name.is_empty() {
                        format!("{i}: {code_str} ({name})")
                    } else {
                        format!("{i}: {code_str}")
                    }
                } else {
                    format!("{i}: {code_str}")
                };
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
    if parts.len() != 2 {
        return Err("Filter expression must contain '=' to separate key and value".into());
    }
    let key = parts[0].to_string();
    let value = parts[1].to_string();
    if key.is_empty() {
        return Err("Filter key cannot be empty".into());
    }
    let modifier = format!("{include_sign}{recursion}");
    Ok((modifier, key, value))
}
