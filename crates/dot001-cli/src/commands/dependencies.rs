use crate::commands::DependencyTracer;
use crate::util::{BlockDisplay, BlockInfo, CommandContext, DisplayOptions, SimpleFormatter};
use dot001_error::Dot001Error;
use dot001_parser::BlendFile;
use dot001_tracer::DependencyNode;
use log::{debug, error, info};
use std::path::PathBuf;
use text_trees::{FormatCharacters, StringTreeNode, TreeFormatting};

pub fn cmd_dependencies(
    file_path: PathBuf,
    block_identifier: &str,
    format: crate::OutputFormat,
    ascii: bool,
    ctx: &CommandContext,
) -> Result<(), Dot001Error> {
    info!("Loading blend file: {}", file_path.display());
    debug!("Target block identifier: '{block_identifier}', format: {format:?}, ascii: {ascii}");

    let mut blend_file = ctx.load_blend_file(&file_path)?;

    info!(
        "Blend file loaded successfully, total blocks: {}",
        blend_file.blocks_len()
    );

    // Resolve the block identifier to a specific block index
    let Some(block_index) = crate::util::resolve_block_or_exit(block_identifier, &mut blend_file)
    else {
        return Ok(());
    };
    let mut tracer = DependencyTracer::new().with_default_expanders();
    debug!("Created dependency tracer with default expanders");

    let Some(start_block) = blend_file.get_block(block_index) else {
        error!("Block index {block_index} is out of range");
        return Ok(());
    };
    let start_code = String::from_utf8_lossy(&start_block.header.code);
    info!(
        "Starting dependency analysis for block {} ({})",
        block_index,
        start_code.trim_end_matches('\0')
    );
    match format {
        crate::OutputFormat::Flat => {
            info!(
                "Tracing dependencies for block {} ({})",
                block_index,
                start_code.trim_end_matches('\0')
            );
            let deps = tracer.trace_dependencies(block_index, &mut blend_file)?;
            info!(
                "Dependency tracing completed, found {} dependencies",
                deps.len()
            );
            if deps.is_empty() {
                ctx.output.print_result("  No dependencies found");
            } else {
                ctx.output
                    .print_info_fmt(format_args!("  Found {} dependencies:", deps.len()));
                for (i, &dep_index) in deps.iter().enumerate() {
                    if let Some(_block) = blend_file.get_block(dep_index) {
                        // We don't need the code_str anymore since BlockInfo handles it
                        let block_info = BlockInfo::from_blend_file(dep_index, &mut blend_file)
                            .unwrap_or_else(|_| BlockInfo::new(dep_index, "????".to_string()));
                        ctx.output.print_result_fmt(format_args!(
                            "    {}: Block {}",
                            i + 1,
                            block_info.display()
                        ));
                    }
                }
            }
        }
        crate::OutputFormat::Tree => {
            info!(
                "Building dependency tree for block {} ({})",
                block_index,
                start_code.trim_end_matches('\0')
            );
            let tree = tracer.trace_dependency_tree(block_index, &mut blend_file)?;
            info!(
                "Dependency tree built: {} total nodes, max depth: {}",
                tree.total_dependencies + 1,
                tree.max_depth
            );
            let tree_display = build_text_tree(&tree.root, &mut blend_file, true);
            let format_chars = if ascii {
                FormatCharacters::ascii()
            } else {
                FormatCharacters::box_chars()
            };
            let formatting = TreeFormatting::dir_tree(format_chars);
            match tree_display.to_string_with_format(&formatting) {
                Ok(tree_output) => ctx.output.print_result(&tree_output),
                Err(e) => error!("Failed to format dependency tree: {e}"),
            }
            ctx.output.print_info("Summary:");
            ctx.output.print_result_fmt(format_args!(
                "  Total dependencies: {}",
                tree.total_dependencies
            ));
            ctx.output
                .print_result_fmt(format_args!("  Maximum depth: {}", tree.max_depth));
        }
        crate::OutputFormat::Json => {
            let tree = tracer.trace_dependency_tree(block_index, &mut blend_file)?;
            match serde_json::to_string_pretty(&tree) {
                Ok(json) => ctx.output.print_result(&json),
                Err(e) => error!("Failed to serialize dependency tree to JSON: {e}"),
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
    let block_info = BlockInfo::from_blend_file(node.block_index, blend_file)
        .unwrap_or_else(|_| BlockInfo::new(node.block_index, node.block_code.clone()));

    let display_options = if show_names {
        DisplayOptions::default()
    } else {
        DisplayOptions::default().with_show_name(false)
    };

    let display_code = BlockDisplay::new(block_info.clone())
        .with_formatter(SimpleFormatter)
        .with_options(display_options);
    let label = format!(
        "Block {} ({}) - size: {}, addr: 0x{:x}",
        block_info.index, display_code, node.block_size, node.block_address
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
