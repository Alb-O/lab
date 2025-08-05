use crate::commands::NameResolver;
use dot001_error::Dot001Error;
use dot001_parser::{BlendFile, ParseOptions};
use log::error;
use owo_colors::OwoColorize;
use regex::Regex;
use std::path::PathBuf;
use text_trees::{FormatCharacters, StringTreeNode, TreeFormatting};

pub fn cmd_filter(
    file_path: PathBuf,
    filter_expressions: Vec<String>,
    format: crate::OutputFormat,
    verbose_details: bool,
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
                    let colored_index = colorize_index(i);
                    let colored_code = colorize_code(&code_str);

                    if verbose_details {
                        println!(
                            "Block {colored_index}: {colored_code} (size: {size}, count: {count}, addr: {old_address:#x}, offset: {block_offset})"
                        );
                        if let Some(name) = &name {
                            if !name.is_empty() {
                                let highlighted_name =
                                    highlight_matches(name, &filter_slice_triples);
                                println!("  Name: {highlighted_name}");
                            }
                        }
                    } else if let Some(name) = &name {
                        if !name.is_empty() {
                            let highlighted_name = highlight_matches(name, &filter_slice_triples);
                            println!("{colored_index}: {colored_code} ({highlighted_name})");
                        } else {
                            println!("{colored_index}: {colored_code}");
                        }
                    } else {
                        println!("{colored_index}: {colored_code}");
                    }
                }
            }
            crate::OutputFormat::Tree => {
                // Build a tree using text_trees
                let indices_vec: Vec<usize> = filtered_indices.iter().copied().collect();
                let tree = build_filter_tree(&indices_vec, &mut blend_file, &filter_slice_triples);
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
        filter_expressions: &[(&str, &str, &str)],
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
                let colored_index = colorize_index(i);
                let colored_code = colorize_code(&code_str);

                let label = if let Some(name) = name {
                    if !name.is_empty() {
                        let highlighted_name = highlight_matches(&name, filter_expressions);
                        format!("{colored_index}: {colored_code} ({highlighted_name})")
                    } else {
                        format!("{colored_index}: {colored_code}")
                    }
                } else {
                    format!("{colored_index}: {colored_code}")
                };
                Some(StringTreeNode::new(label))
            })
            .collect();
        StringTreeNode::with_child_nodes("Filtered Blocks".to_string(), children.into_iter())
    }
    Ok(())
}

fn should_use_colors() -> bool {
    atty::is(atty::Stream::Stdout)
}

fn colorize_index(index: usize) -> String {
    if should_use_colors() {
        index.to_string().green().to_string()
    } else {
        index.to_string()
    }
}

fn colorize_code(code: &str) -> String {
    if should_use_colors() {
        code.blue().to_string()
    } else {
        code.to_string()
    }
}

fn highlight_matches(text: &str, filter_expressions: &[(&str, &str, &str)]) -> String {
    if !should_use_colors() {
        return text.to_string();
    }

    let mut result = text.to_string();

    // Apply highlighting for name matches
    for (_, key, value) in filter_expressions {
        if *key == "name" && !value.is_empty() {
            // Try to use the value as a regex first, fall back to literal match
            let pattern = if let Ok(regex) = Regex::new(&format!("(?i){value}")) {
                regex
            } else {
                // If the value is not a valid regex, escape it for literal matching
                match Regex::new(&format!("(?i){}", regex::escape(value))) {
                    Ok(regex) => regex,
                    Err(_) => continue, // Skip this filter if we can't create a regex
                }
            };

            result = pattern
                .replace_all(&result, |caps: &regex::Captures| {
                    caps[0].to_string().red().to_string()
                })
                .to_string();
        }
    }

    result
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
