use crate::DisplayTemplate;
use crate::block_display::{BlockInfo, create_display_for_template};
use crate::commands::DependencyTracer;
use crate::util::CommandContext;
use dot001_error::Dot001Error;
use dot001_parser::BlendFile;
use dot001_tracer::DependencyNode;
use log::{debug, error, info};
use std::path::PathBuf;
use text_trees::{FormatCharacters, StringTreeNode, TreeFormatting};

fn should_filter_block<R: std::io::Read + std::io::Seek>(
    block_index: usize,
    blend_file: &mut BlendFile<R>,
    show_data: bool,
) -> bool {
    if show_data {
        return false; // Don't filter anything if show_data is true
    }

    if let Some(block) = blend_file.get_block(block_index) {
        let code_str = String::from_utf8_lossy(&block.header.code);
        let code = code_str.trim_end_matches('\0');
        return code == "DATA";
    }
    false
}

pub fn cmd_dependencies(
    file_path: PathBuf,
    block_identifier: &str,
    format: crate::OutputFormat,
    ascii: bool,
    show_data: bool,
    template: DisplayTemplate,
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

            // Filter out DATA blocks if show_data is false
            let filtered_deps: Vec<usize> = deps
                .iter()
                .filter(|&&dep_index| !should_filter_block(dep_index, &mut blend_file, show_data))
                .copied()
                .collect();

            info!(
                "Dependency tracing completed, found {} dependencies ({} after filtering)",
                deps.len(),
                filtered_deps.len()
            );

            if filtered_deps.is_empty() {
                ctx.output.print_result("  No dependencies found");
            } else {
                ctx.output.print_info_fmt(format_args!(
                    "  Found {} dependencies:",
                    filtered_deps.len()
                ));
                for (i, &dep_index) in filtered_deps.iter().enumerate() {
                    if let Some(block) = blend_file.get_block(dep_index) {
                        let size = block.header.size;
                        let address = block.header.old_address;

                        let block_info = BlockInfo::from_blend_file(dep_index, &mut blend_file)
                            .unwrap_or_else(|_| BlockInfo::new(dep_index, "????".to_string()));

                        let display = create_display_for_template(
                            block_info,
                            &template,
                            Some(size as u64),
                            Some(address),
                        );

                        ctx.output
                            .print_result_fmt(format_args!("    {}: {}", i + 1, display));
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
            let tree_display =
                build_text_tree(&tree.root, &mut blend_file, true, show_data, &template);
            let format_chars = if ascii {
                FormatCharacters::ascii()
            } else {
                FormatCharacters::box_chars()
            };
            let formatting = TreeFormatting::dir_tree(format_chars);
            match tree_display.to_string_with_format(&formatting) {
                Ok(tree_output) => ctx.output.print_result(tree_output.trim_end()),
                Err(e) => error!("Failed to format dependency tree: {e}"),
            }
            ctx.output.print_info("Summary:");
            ctx.output.print_info_fmt(format_args!(
                "  Total dependencies: {}",
                tree.total_dependencies
            ));
            ctx.output
                .print_info_fmt(format_args!("  Maximum depth: {}", tree.max_depth));
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
    show_data: bool,
    template: &DisplayTemplate,
) -> StringTreeNode {
    let mut block_info = BlockInfo::from_blend_file(node.block_index, blend_file)
        .unwrap_or_else(|_| BlockInfo::new(node.block_index, node.block_code.clone()));

    if !show_names {
        block_info.name = None;
    }

    let display = create_display_for_template(
        block_info,
        template,
        Some(node.block_size as u64),
        Some(node.block_address),
    );

    let label = display.to_string();

    if node.children.is_empty() {
        StringTreeNode::new(label)
    } else {
        // Filter out DATA block children if show_data is false
        let filtered_children: Vec<&DependencyNode> = node
            .children
            .iter()
            .filter(|child| !should_filter_block(child.block_index, blend_file, show_data))
            .collect();

        let child_nodes: Vec<StringTreeNode> = filtered_children
            .iter()
            .map(|child| build_text_tree(child, blend_file, show_names, show_data, template))
            .collect();
        StringTreeNode::with_child_nodes(label, child_nodes.into_iter())
    }
}
