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

use bllink_diff::{BlendDiff, BlockChangeType, BlockDiff};
use bllink_tracer::{BlendFile, DependencyTracer, NameResolver};
use bllink_tracer::{
    CacheFileExpander, CollectionExpander, DataBlockExpander, ImageExpander, LampExpander,
    LibraryExpander, MaterialExpander, MeshExpander, NodeTreeExpander, ObjectExpander,
    SceneExpander, SoundExpander, TextureExpander,
};
use std::io::{Read, Seek};

/// Formatter for diff results with multiple output formats
pub struct DiffFormatter;

impl DiffFormatter {
    /// Display diff results in flat format
    pub fn display_flat(diff: &BlendDiff, only_modified: bool) {
        if only_modified {
            println!("Modified blocks:");
            for block_diff in &diff.block_diffs {
                if block_diff.change_type == BlockChangeType::Modified {
                    println!(
                        "  Block {}: {} (size: {} -> {})",
                        block_diff.block_index,
                        block_diff.block_code,
                        block_diff.size_before.unwrap_or(0),
                        block_diff.size_after.unwrap_or(0)
                    );
                }
            }
        } else {
            println!("All differences:");
            for block_diff in &diff.block_diffs {
                match block_diff.change_type {
                    BlockChangeType::Modified => {
                        println!(
                            "  M Block {}: {} (size: {} -> {})",
                            block_diff.block_index,
                            block_diff.block_code,
                            block_diff.size_before.unwrap_or(0),
                            block_diff.size_after.unwrap_or(0)
                        );
                    }
                    BlockChangeType::Added => {
                        println!(
                            "  + Block {}: {} (size: {})",
                            block_diff.block_index,
                            block_diff.block_code,
                            block_diff.size_after.unwrap_or(0)
                        );
                    }
                    BlockChangeType::Removed => {
                        println!(
                            "  - Block {}: {} (size: {})",
                            block_diff.block_index,
                            block_diff.block_code,
                            block_diff.size_before.unwrap_or(0)
                        );
                    }
                    BlockChangeType::Unchanged => {
                        // Skip unchanged blocks unless explicitly requested
                    }
                }
            }
        }
    }

    /// Display diff results in hierarchical tree format
    pub fn display_tree<R: Read + Seek>(
        diff: &BlendDiff,
        blend_file: &mut BlendFile<R>,
        _only_modified: bool,
        ascii: bool,
        show_names: bool,
    ) -> bllink_tracer::Result<()> {
        println!("Hierarchical diff tree:");

        // Get all modified block indices
        let modified_blocks: Vec<&BlockDiff> = diff
            .block_diffs
            .iter()
            .filter(|d| d.change_type == BlockChangeType::Modified)
            .collect();

        if modified_blocks.is_empty() {
            println!("  No modifications found");
            return Ok(());
        }

        // Try to build evidence-based hierarchical relationships
        let hierarchy =
            Self::build_evidence_based_hierarchy(&modified_blocks, blend_file, show_names)?;

        // Display the hierarchy
        Self::display_hierarchy(&hierarchy, ascii);

        Ok(())
    }

    /// Build evidence-based hierarchy using dependency tracing and spatial analysis
    fn build_evidence_based_hierarchy<R: Read + Seek>(
        modified_blocks: &[&BlockDiff],
        blend_file: &mut BlendFile<R>,
        show_names: bool,
    ) -> bllink_tracer::Result<Vec<HierarchyNode>> {
        let mut tracer = DependencyTracer::new();
        Self::register_tracer_expanders(&mut tracer);

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
                    let deps = tracer.trace_dependencies(block_diff.block_index, blend_file)?;

                    let display_name = if show_names {
                        NameResolver::get_display_name(
                            block_diff.block_index,
                            blend_file,
                            &block_diff.block_code,
                        )
                    } else {
                        block_diff.block_code.clone()
                    };

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
                            let child_display_name = if show_names {
                                NameResolver::get_display_name(
                                    child_diff.block_index,
                                    blend_file,
                                    &child_diff.block_code,
                                )
                            } else {
                                child_diff.block_code.clone()
                            };

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
                let display_name = if show_names {
                    NameResolver::get_display_name(
                        block_diff.block_index,
                        blend_file,
                        &block_diff.block_code,
                    )
                } else {
                    block_diff.block_code.clone()
                };

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
            if is_last {
                "└── "
            } else {
                "├── "
            }
        } else if is_last {
            "└── "
        } else {
            "├── "
        };

        let size_info = match (node.block_diff.size_before, node.block_diff.size_after) {
            (Some(before), Some(after)) if before != after => {
                format!(" (size: {before} → {after})")
            }
            _ => "".to_string(),
        };

        println!(
            "{}{}Block {} ({}){}",
            prefix, current_prefix, node.block_diff.block_index, node.display_name, size_info
        );

        let child_prefix = if ascii {
            if is_last {
                "    "
            } else {
                "│   "
            }
        } else if is_last {
            "    "
        } else {
            "│   "
        };

        for (j, child) in node.children.iter().enumerate() {
            let child_is_last = j == node.children.len() - 1;
            Self::display_hierarchy_node(
                child,
                &format!("{prefix}{child_prefix}"),
                child_is_last,
                ascii,
            );
        }
    }

    fn register_tracer_expanders<R: Read + Seek>(tracer: &mut bllink_tracer::DependencyTracer<R>) {
        tracer.register_expander(*b"SC\0\0", Box::new(SceneExpander));
        tracer.register_expander(*b"OB\0\0", Box::new(ObjectExpander));
        tracer.register_expander(*b"ME\0\0", Box::new(MeshExpander));
        tracer.register_expander(*b"GR\0\0", Box::new(CollectionExpander));
        tracer.register_expander(*b"MA\0\0", Box::new(MaterialExpander));
        tracer.register_expander(*b"TE\0\0", Box::new(TextureExpander));
        tracer.register_expander(*b"IM\0\0", Box::new(ImageExpander));
        tracer.register_expander(*b"LI\0\0", Box::new(LibraryExpander));
        tracer.register_expander(*b"CF\0\0", Box::new(CacheFileExpander));
        tracer.register_expander(*b"SO\0\0", Box::new(SoundExpander));
        tracer.register_expander(*b"LA\0\0", Box::new(LampExpander));
        tracer.register_expander(*b"NT\0\0", Box::new(NodeTreeExpander));
        tracer.register_expander(*b"DATA", Box::new(DataBlockExpander));
    }
}

#[derive(Debug, Clone)]
struct HierarchyNode {
    block_diff: BlockDiff,
    display_name: String,
    children: Vec<HierarchyNode>,
}
