use crate::DisplayTemplate;
use crate::block_display::{BlockInfo, create_display_for_template};
use crate::block_ops::{BatchProcessor, CommandHelper};
use crate::output_utils::{CommandSummary, OutputUtils, TreeFormatter};
use crate::util::CommandContext;
use dot001_events::error::Error;
use dot001_parser::{BlendFile, block_code_to_string, is_data_block_code};
use dot001_tracer::{DependencyNode, ParallelDependencyTracer};
use log::{debug, error, info};
use std::path::PathBuf;
use text_trees::StringTreeNode;

fn should_filter_block(block_index: usize, blend_file: &mut BlendFile, show_data: bool) -> bool {
    if show_data {
        return false; // Don't filter anything if show_data is true
    }

    if let Some(block) = blend_file.get_block(block_index) {
        let code = block_code_to_string(block.header.code);
        return is_data_block_code(&code);
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
) -> Result<(), Error> {
    info!("Loading blend file: {}", file_path.display());
    debug!("Target block identifier: '{block_identifier}', format: {format:?}, ascii: {ascii}");

    let mut blend_file = ctx.load_blend_file(&file_path)?;

    info!(
        "Blend file loaded successfully, total blocks: {}",
        blend_file.blocks_len()
    );

    // Resolve the block identifier to a specific block index
    let block_index = {
        let mut helper = CommandHelper::new(&mut blend_file, ctx);
        let Some(index) = helper.resolve_block_or_return(block_identifier)? else {
            return Ok(());
        };
        index
    };
    let mut tracer = ParallelDependencyTracer::new().with_default_expanders();
    debug!("Created dependency tracer with default expanders");

    let Some(start_block) = blend_file.get_block(block_index) else {
        error!("Block index {block_index} is out of range");
        return Ok(());
    };
    info!(
        "Starting dependency analysis for block {} ({})",
        block_index,
        block_code_to_string(start_block.header.code)
    );
    match format {
        crate::OutputFormat::Flat => {
            info!(
                "Tracing dependencies for block {} ({})",
                block_index,
                block_code_to_string(start_block.header.code)
            );
            let deps = tracer.trace_dependencies_parallel(block_index, &blend_file)?;

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
                let mut processor = BatchProcessor::new(&mut blend_file, ctx);
                processor.process_blocks(
                    &filtered_deps,
                    &template,
                    |index, _metadata, display, ctx| {
                        let position =
                            filtered_deps.iter().position(|&x| x == index).unwrap_or(0) + 1;
                        ctx.output
                            .print_result_fmt(format_args!("    {position}: {display}"));
                        Ok(())
                    },
                )?;
            }
        }
        crate::OutputFormat::Tree => {
            info!(
                "Building dependency tree for block {} ({})",
                block_index,
                block_code_to_string(start_block.header.code)
            );
            let deps = tracer.trace_dependencies_parallel(block_index, &blend_file)?;
            // Build a simple tree structure from deps for display purposes
            let root = DependencyNode {
                block_index,
                block_code: block_code_to_string(start_block.header.code),
                block_size: start_block.header.size,
                block_address: start_block.header.old_address,
                children: deps
                    .iter()
                    .map(|&i| DependencyNode {
                        block_index: i,
                        block_code: blend_file
                            .get_block(i)
                            .map(|b| block_code_to_string(b.header.code))
                            .unwrap_or_else(|| "????".to_string()),
                        block_size: blend_file.get_block(i).map(|b| b.header.size).unwrap_or(0),
                        block_address: blend_file
                            .get_block(i)
                            .map(|b| b.header.old_address)
                            .unwrap_or(0),
                        children: Vec::new(),
                    })
                    .collect(),
            };
            let tree = dot001_tracer::core::tree::DependencyTree {
                total_dependencies: deps.len(),
                max_depth: 1,
                root,
            };
            info!(
                "Dependency tree built: {} total nodes, max depth: {}",
                tree.total_dependencies + 1,
                tree.max_depth
            );
            let tree_display =
                build_text_tree(&tree.root, &mut blend_file, true, show_data, &template);
            let formatter = TreeFormatter::new(ascii);
            formatter.print_tree(&tree_display, ctx);

            CommandSummary::new("Summary")
                .add_count("Total dependencies", tree.total_dependencies)
                .add_count("Maximum depth", tree.max_depth)
                .print(ctx);
        }
        crate::OutputFormat::Json => {
            let deps = tracer.trace_dependencies_parallel(block_index, &blend_file)?;
            let json = serde_json::json!({
                "root": block_index,
                "dependencies": deps,
            });
            OutputUtils::try_print_json(&json, ctx, "dependencies", |data| {
                serde_json::to_string_pretty(data)
            });
        }
    }
    Ok(())
}

pub fn build_text_tree(
    node: &DependencyNode,
    blend_file: &mut BlendFile,
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
