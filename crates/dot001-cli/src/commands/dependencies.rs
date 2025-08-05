use crate::commands::{DependencyTracer, NameResolver};
use dot001_error::Dot001Error;
use dot001_parser::{BlendFile, ParseOptions};
use dot001_tracer::DependencyNode;
use std::path::PathBuf;
use text_trees::{FormatCharacters, StringTreeNode, TreeFormatting};

pub fn cmd_dependencies(
    file_path: PathBuf,
    block_index: usize,
    format: crate::OutputFormat,
    ascii: bool,
    options: &ParseOptions,
    no_auto_decompress: bool,
) -> Result<(), Dot001Error> {
    let mut blend_file = crate::util::load_blend_file(&file_path, options, no_auto_decompress)?;
    if block_index >= blend_file.blocks_len() {
        eprintln!(
            "Error: Block index {} is out of range (max: {})",
            block_index,
            blend_file.blocks_len() - 1
        );
        return Ok(());
    }
    let mut tracer = DependencyTracer::new().with_default_expanders();
    let Some(start_block) = blend_file.get_block(block_index) else {
        eprintln!("Error: Block index {block_index} is out of range");
        return Ok(());
    };
    let start_code = String::from_utf8_lossy(&start_block.header.code);
    match format {
        crate::OutputFormat::Flat => {
            println!(
                "Tracing dependencies for block {} ({}):",
                block_index,
                start_code.trim_end_matches('\0')
            );
            let deps = tracer.trace_dependencies(block_index, &mut blend_file)?;
            if deps.is_empty() {
                println!("  No dependencies found");
            } else {
                println!("  Found {} dependencies:", deps.len());
                for (i, &dep_index) in deps.iter().enumerate() {
                    if let Some(block) = blend_file.get_block(dep_index) {
                        let code_str = String::from_utf8_lossy(&block.header.code)
                            .trim_end_matches('\0')
                            .to_string();
                        let display_name =
                            NameResolver::get_display_name(dep_index, &mut blend_file, &code_str);
                        println!("    {}: Block {} ({})", i + 1, dep_index, display_name);
                    }
                }
            }
        }
        crate::OutputFormat::Tree => {
            println!(
                "Dependency tree for block {} ({}):",
                block_index,
                start_code.trim_end_matches('\0')
            );
            let tree = tracer.trace_dependency_tree(block_index, &mut blend_file)?;
            let tree_display = build_text_tree(&tree.root, &mut blend_file, true);
            let format_chars = if ascii {
                FormatCharacters::ascii()
            } else {
                FormatCharacters::box_chars()
            };
            let formatting = TreeFormatting::dir_tree(format_chars);
            match tree_display.to_string_with_format(&formatting) {
                Ok(output) => println!("{output}"),
                Err(e) => eprintln!("Error formatting tree: {e}"),
            }
            println!("Summary:");
            println!("  Total dependencies: {}", tree.total_dependencies);
            println!("  Maximum depth: {}", tree.max_depth);
        }
        crate::OutputFormat::Json => {
            let tree = tracer.trace_dependency_tree(block_index, &mut blend_file)?;
            match serde_json::to_string_pretty(&tree) {
                Ok(json) => println!("{json}"),
                Err(e) => eprintln!("Error serializing to JSON: {e}"),
            }
        }
    }
    Ok(())
}

pub fn build_text_tree<R: std::io::Read + std::io::Seek>(
    node: &DependencyNode,
    blend_file: &mut BlendFile<R>,
    show_names: bool,
) -> StringTreeNode {
    let display_code = if show_names {
        NameResolver::get_display_name(node.block_index, blend_file, &node.block_code)
    } else {
        node.block_code.clone()
    };
    let label = format!(
        "Block {} ({}) - size: {}, addr: 0x{:x}",
        node.block_index, display_code, node.block_size, node.block_address
    );
    if node.children.is_empty() {
        StringTreeNode::new(label)
    } else {
        let child_nodes: Vec<StringTreeNode> = node
            .children
            .iter()
            .map(|child| build_text_tree(child, blend_file, show_names))
            .collect();
        StringTreeNode::with_child_nodes(label, child_nodes.into_iter())
    }
}
