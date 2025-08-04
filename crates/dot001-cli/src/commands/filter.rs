use dot001_parser::ParseOptions;
use dot001_tracer::Result;
use std::path::PathBuf;

pub fn cmd_filter(
    file_path: PathBuf,
    filter_expressions: Vec<String>,
    format: crate::OutputFormat,
    verbose: bool,
    json: bool,
    options: &ParseOptions,
    no_auto_decompress: bool,
) -> Result<()> {
    let mut blend_file = crate::util::load_blend_file(&file_path, options, no_auto_decompress)?;
    let mut filter_triples: Vec<(String, String, String)> = Vec::new();
    for expr in &filter_expressions {
        match parse_filter_expression(expr) {
            Ok((modifier, key, value)) => {
                filter_triples.push((modifier, key, value));
            }
            Err(e) => {
                eprintln!("Error parsing filter expression '{expr}': {e}");
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
    if json {
        let filtered_blocks: Vec<serde_json::Value> = filtered_indices
            .iter()
            .map(|&i| {
                let block = &blend_file.blocks[i];
                let code_str = String::from_utf8_lossy(&block.header.code)
                    .trim_end_matches('\0')
                    .to_string();
                serde_json::json!({
                    "index": i,
                    "code": code_str,
                    "size": block.header.size,
                    "count": block.header.count,
                    "address": format!("{:#x}", block.header.old_address),
                    "file_offset": block.header_offset
                })
            })
            .collect();
        match serde_json::to_string_pretty(&filtered_blocks) {
            Ok(json_str) => println!("{json_str}"),
            Err(e) => {
                eprintln!("Error serializing to JSON: {e}");
                std::process::exit(1);
            }
        }
    } else {
        println!("Filtered blocks from {}:", file_path.display());
        println!(
            "Total blocks: {}, Filtered: {}",
            blend_file.blocks.len(),
            filtered_indices.len()
        );
        println!();
        let mut sorted_indices: Vec<_> = filtered_indices.into_iter().collect();
        sorted_indices.sort();
        for &i in &sorted_indices {
            let block = &blend_file.blocks[i];
            let code_str = String::from_utf8_lossy(&block.header.code)
                .trim_end_matches('\0')
                .to_string();
            if verbose {
                println!(
                    "Block {}: {} (size: {}, count: {}, addr: {:#x}, offset: {})",
                    i,
                    code_str,
                    block.header.size,
                    block.header.count,
                    block.header.old_address,
                    block.header_offset
                );
                if let Ok(data) = blend_file.read_block_data(i) {
                    if let Ok(reader) = blend_file.create_field_reader(&data) {
                        if let Ok(name) = reader.read_field_string("ID", "name") {
                            let trimmed = name.trim_end_matches('\0');
                            if !trimmed.is_empty() {
                                println!("  Name: {trimmed}");
                            }
                        }
                    }
                }
            } else {
                match format {
                    crate::OutputFormat::Flat => {
                        println!("{i}: {code_str}");
                    }
                    crate::OutputFormat::Tree => {
                        println!("├─ {i}: {code_str}");
                    }
                    crate::OutputFormat::Json => unreachable!(),
                }
            }
        }
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
