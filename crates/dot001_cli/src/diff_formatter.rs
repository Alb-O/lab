//! # Diff Formatter - EXPERIMENTAL IMPLEMENTATION
//!
//! This module provides formatting for blend file diff results with hierarchical tree display.
//! The current implementation uses conservative dependency-based hierarchy to avoid false positives.
//!
//! ## Current Limitations:
//! - Hierarchical relationships are only shown for confirmed dependencies
//! - Spatial relationships between blocks are not established due to lack of solid proof
//! - Many modified blocks appear as flat top-level items when no dependency relationships exist
//! - More sophisticated analysis is needed to establish proper parent-child relationships
//!
//! This formatter prioritizes accuracy over visual hierarchy, showing only relationships
//! that can be proven through dependency tracing.

use crate::DisplayTemplate;
use crate::block_display::{BlockInfo, create_display_for_template};
use crate::util::CommandContext;
use dot001_diff::{BlendDiff, BlockChangeType, BlockDiff};
use dot001_parser::BlendFile;
use log::info;

/// Formatter for diff results with multiple output formats
pub struct DiffFormatter;

impl DiffFormatter {
    /// Display diff results in flat format
    pub fn display_flat(diff: &BlendDiff, only_modified: bool, ctx: &CommandContext) {
        if only_modified {
            info!("Showing only modified blocks");
            ctx.output.print_info("Modified blocks:");
            for block_diff in &diff.block_diffs {
                if block_diff.change_type == BlockChangeType::Modified {
                    ctx.output.print_result_fmt(format_args!(
                        "  Block {}: {} (size: {} -> {})",
                        block_diff.block_index,
                        block_diff.block_code,
                        block_diff.size_before.unwrap_or(0),
                        block_diff.size_after.unwrap_or(0)
                    ));
                }
            }
        } else {
            info!("Showing all differences");
            ctx.output.print_info("All differences:");
            for block_diff in &diff.block_diffs {
                match block_diff.change_type {
                    BlockChangeType::Modified => {
                        ctx.output.print_result_fmt(format_args!(
                            "  M Block {}: {} (size: {} -> {})",
                            block_diff.block_index,
                            block_diff.block_code,
                            block_diff.size_before.unwrap_or(0),
                            block_diff.size_after.unwrap_or(0)
                        ));
                    }
                    BlockChangeType::Added => {
                        ctx.output.print_result_fmt(format_args!(
                            "  + Block {}: {} (size: {})",
                            block_diff.block_index,
                            block_diff.block_code,
                            block_diff.size_after.unwrap_or(0)
                        ));
                    }
                    BlockChangeType::Removed => {
                        ctx.output.print_result_fmt(format_args!(
                            "  - Block {}: {} (size: {})",
                            block_diff.block_index,
                            block_diff.block_code,
                            block_diff.size_before.unwrap_or(0)
                        ));
                    }
                    BlockChangeType::Unchanged => {
                        // Skip unchanged blocks unless explicitly requested
                    }
                }
            }
        }
    }

    /// Display diff results in hierarchical tree format
    pub fn display_tree(
        diff: &BlendDiff,
        blend_file: &mut BlendFile,
        _only_modified: bool,
        template: DisplayTemplate,
        ascii: bool,
        ctx: &CommandContext,
    ) -> dot001_tracer::Result<()> {
        info!("Building hierarchical diff tree");
        ctx.output.print_info("Hierarchical diff tree:");

        // Get all modified block indices
        let modified_blocks: Vec<&BlockDiff> = diff
            .block_diffs
            .iter()
            .filter(|d| d.change_type == BlockChangeType::Modified)
            .collect();

        if modified_blocks.is_empty() {
            ctx.output.print_result("  No modifications found");
            return Ok(());
        }

        // Try to build evidence-based hierarchical relationships
        let hierarchy =
            Self::build_evidence_based_hierarchy(&modified_blocks, blend_file, &template)?;

        // Display the hierarchy
        Self::display_hierarchy(&hierarchy, ascii);

        Ok(())
    }

    /// Build evidence-based hierarchy using dependency tracing and spatial analysis
    fn build_evidence_based_hierarchy(
        modified_blocks: &[&BlockDiff],
        blend_file: &mut BlendFile,
        template: &DisplayTemplate,
    ) -> dot001_tracer::Result<Vec<HierarchyNode>> {
        let mut tracer = dot001_tracer::DependencyTracer::new().with_default_expanders();

        let mut hierarchy = Vec::new();
        let mut processed_indices = std::collections::HashSet::new();

        // Find blocks that could be parents (ME, OB, etc.)
        for block_diff in modified_blocks {
            if processed_indices.contains(&block_diff.block_index) {
                continue;
            }

            match block_diff.block_code.as_str() {
                "ME" | "OB" | "MA" | "TE" | "IM" | "NT" => {
                    // This could be a parent block - check its dependencies
                    let deps =
                        tracer.trace_dependencies_parallel(block_diff.block_index, &blend_file)?;

                    let block_info = BlockInfo::from_blend_file(block_diff.block_index, blend_file)
                        .unwrap_or_else(|_| {
                            BlockInfo::new(block_diff.block_index, block_diff.block_code.clone())
                        });

                    let (size, address) = blend_file
                        .get_block(block_diff.block_index)
                        .map(|block| (block.header.size as u64, block.header.old_address))
                        .unwrap_or((0, 0));

                    let display = create_display_for_template(
                        block_info,
                        template,
                        Some(size),
                        Some(address),
                    );
                    let display_name = display.to_string();

                    let mut node = HierarchyNode {
                        block_diff: (*block_diff).clone(),
                        display_name,
                        children: Vec::new(),
                    };

                    // Look for children in our modified set that are also dependencies
                    for &dep_index in &deps {
                        if let Some(child_diff) =
                            modified_blocks.iter().find(|d| d.block_index == dep_index)
                        {
                            let child_block_info =
                                BlockInfo::from_blend_file(child_diff.block_index, blend_file)
                                    .unwrap_or_else(|_| {
                                        BlockInfo::new(
                                            child_diff.block_index,
                                            child_diff.block_code.clone(),
                                        )
                                    });

                            let (child_size, child_address) = blend_file
                                .get_block(child_diff.block_index)
                                .map(|block| (block.header.size as u64, block.header.old_address))
                                .unwrap_or((0, 0));

                            let child_display = create_display_for_template(
                                child_block_info,
                                template,
                                Some(child_size),
                                Some(child_address),
                            );
                            let child_display_name = child_display.to_string();

                            node.children.push(HierarchyNode {
                                block_diff: (*child_diff).clone(),
                                display_name: child_display_name,
                                children: Vec::new(),
                            });
                            processed_indices.insert(child_diff.block_index);
                        }
                    }

                    // For ME blocks, we only show confirmed dependency relationships
                    // Previously we attempted spatial analysis but this created false positives
                    // The user requested "solid proof" of parent-child relationships

                    hierarchy.push(node);
                    processed_indices.insert(block_diff.block_index);
                }
                _ => {}
            }
        }

        // Add remaining blocks as top-level items
        for block_diff in modified_blocks {
            if !processed_indices.contains(&block_diff.block_index) {
                let block_info = BlockInfo::from_blend_file(block_diff.block_index, blend_file)
                    .unwrap_or_else(|_| {
                        BlockInfo::new(block_diff.block_index, block_diff.block_code.clone())
                    });

                let (size, address) = blend_file
                    .get_block(block_diff.block_index)
                    .map(|block| (block.header.size as u64, block.header.old_address))
                    .unwrap_or((0, 0));

                let display =
                    create_display_for_template(block_info, template, Some(size), Some(address));
                let display_name = display.to_string();

                hierarchy.push(HierarchyNode {
                    block_diff: (*block_diff).clone(),
                    display_name,
                    children: Vec::new(),
                });
            }
        }

        Ok(hierarchy)
    }

    fn display_hierarchy(hierarchy: &[HierarchyNode], ascii: bool) {
        for (i, node) in hierarchy.iter().enumerate() {
            let is_last = i == hierarchy.len() - 1;
            Self::display_hierarchy_node(node, "", is_last, ascii);
        }
    }

    fn display_hierarchy_node(node: &HierarchyNode, prefix: &str, is_last: bool, ascii: bool) {
        let current_prefix = if ascii {
            if is_last { "└── " } else { "├── " }
        } else if is_last {
            "└── "
        } else {
            "├── "
        };

        // For hierarchical display, we'll use println! to maintain formatting
        // This is acceptable since hierarchical display has complex indentation
        match (node.block_diff.size_before, node.block_diff.size_after) {
            (Some(before), Some(after)) if before != after => {
                println!(
                    "{}{}Block {} ({}) (size: {before} → {after})",
                    prefix, current_prefix, node.block_diff.block_index, node.display_name
                );
            }
            _ => {
                println!(
                    "{}{}Block {} ({})",
                    prefix, current_prefix, node.block_diff.block_index, node.display_name
                );
            }
        }

        let child_prefix = if ascii {
            if is_last { "    " } else { "│   " }
        } else if is_last {
            "    "
        } else {
            "│   "
        };

        for (j, child) in node.children.iter().enumerate() {
            let child_is_last = j == node.children.len() - 1;
            let mut new_prefix = String::with_capacity(prefix.len() + child_prefix.len());
            new_prefix.push_str(prefix);
            new_prefix.push_str(child_prefix);
            Self::display_hierarchy_node(child, &new_prefix, child_is_last, ascii);
        }
    }

    // Default expanders are registered by with_default_expanders() in the new tracer.
}

#[derive(Debug, Clone)]
struct HierarchyNode {
    block_diff: BlockDiff,
    display_name: String,
    children: Vec<HierarchyNode>,
}
