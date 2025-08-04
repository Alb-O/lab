// dot001-cli/src/main.rs

use dot001_diff::BlendDiffer;
use dot001_parser::{parse_from_path, DecompressionPolicy, ParseOptions};
use dot001_tracer::{BlendFile, Result};

mod diff_formatter;
use clap::{Parser, Subcommand, ValueEnum};
use dot001_tracer::{
    CacheFileExpander, CollectionExpander, DataBlockExpander, DependencyTracer, ImageExpander,
    LampExpander, LibraryExpander, MaterialExpander, MeshExpander, NameResolver, NodeTreeExpander,
    ObjectExpander, SceneExpander, SoundExpander, TextureExpander,
};
use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;
use text_trees::{FormatCharacters, StringTreeNode, TreeFormatting};

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Maximum size to decompress into memory (in MB)
    #[arg(long, global = true, default_value = "256")]
    max_in_memory: usize,

    /// Custom temp directory for large compressed files
    #[arg(long, global = true)]
    temp_dir: Option<PathBuf>,

    /// Prefer memory-mapped temp files
    #[arg(long, global = true, action = clap::ArgAction::Set)]
    prefer_mmap: Option<bool>,

    /// Disable automatic decompression of compressed files
    #[arg(long, global = true)]
    no_auto_decompress: bool,
}

#[derive(Clone, ValueEnum)]
enum OutputFormat {
    /// Simple flat list of dependencies
    Flat,
    /// Hierarchical tree structure
    Tree,
    /// JSON output
    Json,
}

#[derive(Subcommand)]
enum Commands {
    /// Show basic info about the blend file
    Info { file: PathBuf },
    /// List all blocks in the file
    Blocks { file: PathBuf },
    /// Trace dependencies from a specific block
    Dependencies {
        file: PathBuf,
        #[arg(short, long)]
        block_index: usize,
        #[arg(short, long, value_enum, default_value_t = OutputFormat::Flat)]
        format: OutputFormat,
        #[arg(
            long,
            help = "Use ASCII characters instead of Unicode box characters for tree output"
        )]
        ascii: bool,
    },
    /// Compare two blend files and show differences [EXPERIMENTAL - INCOMPLETE IMPLEMENTATION]
    ///
    /// WARNING: This diff functionality is a proof-of-concept implementation that requires
    /// significant foundational work. It demonstrates content-aware mesh diffing and size-based
    /// DATA block filtering, but has known limitations in establishing proper hierarchical
    /// relationships between blocks. The current implementation should be considered incomplete
    /// and experimental. Many block types use simple binary comparison rather than semantic
    /// analysis, and complex data relationships within Blender files are not fully analyzed.
    Diff {
        file1: PathBuf,
        file2: PathBuf,
        #[arg(long, help = "Show only modified blocks, not all differences")]
        only_modified: bool,
        #[arg(short, long, value_enum, default_value_t = OutputFormat::Flat)]
        format: OutputFormat,
        #[arg(
            long,
            help = "Use ASCII characters instead of Unicode box characters for tree output"
        )]
        ascii: bool,
    },
    /// Rename an ID block (datablock with user-defined name) [EXPERIMENTAL]
    ///
    /// WARNING: This command modifies .blend file binary data. Use only on backup copies.
    /// Always test modified files in Blender before using them in production.
    Rename {
        file: PathBuf,
        #[arg(short, long)]
        block_index: usize,
        #[arg(short, long)]
        new_name: String,
        #[arg(long, help = "Preview changes without modifying the file")]
        dry_run: bool,
    },
    /// Enhanced mesh diff with provenance analysis [EXPERIMENTAL]
    MeshDiff {
        file1: PathBuf,
        file2: PathBuf,
        #[arg(long, help = "ME block index to analyze")]
        mesh_index: Option<usize>,
        #[arg(long, help = "Enable verbose provenance logging")]
        verbose: bool,
        #[arg(long, help = "Output detailed analysis as JSON")]
        json: bool,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let parse_options = create_parse_options(&cli);

    match cli.command {
        Commands::Info { file } => cmd_info(file, &parse_options, cli.no_auto_decompress),
        Commands::Blocks { file } => cmd_blocks(file, &parse_options, cli.no_auto_decompress),
        Commands::Dependencies {
            file,
            block_index,
            format,
            ascii,
        } => cmd_dependencies(
            file,
            block_index,
            format,
            ascii,
            &parse_options,
            cli.no_auto_decompress,
        ),
        Commands::Diff {
            file1,
            file2,
            only_modified,
            format,
            ascii,
        } => cmd_diff(
            file1,
            file2,
            only_modified,
            format,
            ascii,
            &parse_options,
            cli.no_auto_decompress,
        ),
        Commands::Rename {
            file,
            block_index,
            new_name,
            dry_run,
        } => cmd_rename(
            file,
            block_index,
            new_name,
            dry_run,
            &parse_options,
            cli.no_auto_decompress,
        ),
        Commands::MeshDiff {
            file1,
            file2,
            mesh_index,
            verbose,
            json,
        } => cmd_mesh_diff(
            file1,
            file2,
            mesh_index,
            verbose,
            json,
            &parse_options,
            cli.no_auto_decompress,
        ),
    }
}

fn create_parse_options(cli: &Cli) -> ParseOptions {
    let mut policy = DecompressionPolicy::default();

    policy.max_in_memory_bytes = cli.max_in_memory * 1024 * 1024; // Convert MB to bytes
    policy.temp_dir = cli.temp_dir.clone();

    if let Some(prefer_mmap) = cli.prefer_mmap {
        policy.prefer_mmap_temp = prefer_mmap;
    }

    ParseOptions {
        decompression_policy: policy,
    }
}

fn load_blend_file(
    file_path: &PathBuf,
    options: &ParseOptions,
    no_auto_decompress: bool,
) -> Result<BlendFile<Box<dyn dot001_parser::ReadSeekSend>>> {
    if no_auto_decompress {
        // Use old method that rejects compressed files
        let file = File::open(file_path)?;
        let reader = BufReader::new(file);
        let boxed_reader: Box<dyn dot001_parser::ReadSeekSend> = Box::new(reader);
        BlendFile::new(boxed_reader)
    } else {
        let (blend_file, _mode) = parse_from_path(file_path, Some(options))?;
        Ok(blend_file)
    }
}

fn cmd_info(file_path: PathBuf, options: &ParseOptions, no_auto_decompress: bool) -> Result<()> {
    let blend_file = load_blend_file(&file_path, options, no_auto_decompress)?;

    println!("File: {}", file_path.display());
    println!("Header:");
    println!("  Pointer size: {} bytes", blend_file.header.pointer_size);
    println!(
        "  Endianness: {}",
        if blend_file.header.is_little_endian {
            "little"
        } else {
            "big"
        }
    );
    println!("  Version: {}", blend_file.header.version);
    println!("  Total blocks: {}", blend_file.blocks.len());

    if let Some(dna) = &blend_file.dna {
        println!("  DNA structs: {}", dna.structs.len());
        println!("  DNA types: {}", dna.types.len());
    }

    Ok(())
}

fn cmd_blocks(file_path: PathBuf, options: &ParseOptions, no_auto_decompress: bool) -> Result<()> {
    let mut blend_file = load_blend_file(&file_path, options, no_auto_decompress)?;

    println!("Blocks in {}:", file_path.display());

    // Collect block info first to avoid borrowing conflicts
    let block_info: Vec<(usize, String, u32, u64)> = blend_file
        .blocks
        .iter()
        .enumerate()
        .map(|(i, block)| {
            let code_str = String::from_utf8_lossy(&block.header.code)
                .trim_end_matches('\0')
                .to_string();
            (i, code_str, block.header.size, block.header.old_address)
        })
        .collect();

    // Now process each block with name resolution
    for (i, code_str, size, address) in block_info {
        let display_name = NameResolver::get_display_name(i, &mut blend_file, &code_str);
        println!("  {i}: {display_name} (size: {size}, addr: 0x{address:x})");
    }

    Ok(())
}

fn cmd_dependencies(
    file_path: PathBuf,
    block_index: usize,
    format: OutputFormat,
    ascii: bool,
    options: &ParseOptions,
    no_auto_decompress: bool,
) -> Result<()> {
    let mut blend_file = load_blend_file(&file_path, options, no_auto_decompress)?;

    if block_index >= blend_file.blocks.len() {
        eprintln!(
            "Error: Block index {} is out of range (max: {})",
            block_index,
            blend_file.blocks.len() - 1
        );
        return Ok(());
    }

    let mut tracer = DependencyTracer::new();

    // Register all the concrete expanders
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

    let start_block = &blend_file.blocks[block_index];
    let start_code = String::from_utf8_lossy(&start_block.header.code);

    match format {
        OutputFormat::Flat => {
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
        OutputFormat::Tree => {
            println!(
                "Dependency tree for block {} ({}):",
                block_index,
                start_code.trim_end_matches('\0')
            );

            let tree = tracer.trace_dependency_tree(block_index, &mut blend_file)?;
            let tree_display = build_text_tree(&tree.root, &mut blend_file, true);

            // Use box characters for a cleaner look (ASCII option for compatibility)
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
        OutputFormat::Json => {
            let tree = tracer.trace_dependency_tree(block_index, &mut blend_file)?;
            // Note: JSON output currently doesn't include names to maintain compatibility
            // Could be extended in the future with an enriched format
            match serde_json::to_string_pretty(&tree) {
                Ok(json) => println!("{json}"),
                Err(e) => eprintln!("Error serializing to JSON: {e}"),
            }
        }
    }

    Ok(())
}

fn build_text_tree<R: std::io::Read + std::io::Seek>(
    node: &dot001_tracer::DependencyNode,
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

fn cmd_diff(
    file1_path: PathBuf,
    file2_path: PathBuf,
    only_modified: bool,
    format: OutputFormat,
    ascii: bool,
    options: &ParseOptions,
    no_auto_decompress: bool,
) -> Result<()> {
    let mut blend_file1 = load_blend_file(&file1_path, options, no_auto_decompress)?;
    let mut blend_file2 = load_blend_file(&file2_path, options, no_auto_decompress)?;

    let differ = BlendDiffer::new();
    let diff_result = differ
        .diff(&mut blend_file1, &mut blend_file2)
        .map_err(|e| std::io::Error::other(format!("Diff error: {e}")))?;

    println!(
        "Comparing {} vs {}",
        file1_path.display(),
        file2_path.display()
    );
    println!("Summary:");
    println!("  Total blocks: {}", diff_result.summary.total_blocks);
    println!("  Modified: {}", diff_result.summary.modified_blocks);
    println!("  Added: {}", diff_result.summary.added_blocks);
    println!("  Removed: {}", diff_result.summary.removed_blocks);
    println!("  Unchanged: {}", diff_result.summary.unchanged_blocks);
    println!();

    match format {
        OutputFormat::Tree => {
            diff_formatter::DiffFormatter::display_tree(
                &diff_result,
                &mut blend_file1,
                only_modified,
                ascii,
                true,
            )?;
        }
        OutputFormat::Json => match serde_json::to_string_pretty(&diff_result) {
            Ok(json) => println!("{json}"),
            Err(e) => eprintln!("Error serializing to JSON: {e}"),
        },
        OutputFormat::Flat => {
            diff_formatter::DiffFormatter::display_flat(&diff_result, only_modified);
        }
    }

    Ok(())
}

fn cmd_rename(
    file_path: PathBuf,
    block_index: usize,
    new_name: String,
    dry_run: bool,
    options: &ParseOptions,
    no_auto_decompress: bool,
) -> Result<()> {
    use dot001_editor::BlendEditor;

    let mut blend_file = load_blend_file(&file_path, options, no_auto_decompress)?;

    // Verify the block exists and get current info
    if block_index >= blend_file.blocks.len() {
        eprintln!(
            "Error: Block index {} is out of range (max: {})",
            block_index,
            blend_file.blocks.len() - 1
        );
        return Ok(());
    }

    let block_code = {
        let block = &blend_file.blocks[block_index];
        String::from_utf8_lossy(&block.header.code)
            .trim_end_matches('\0')
            .to_string()
    };

    // Try to get current name
    match NameResolver::resolve_name(block_index, &mut blend_file) {
        Some(current_name) => {
            if dry_run {
                println!("Would rename {block_code} block '{current_name}' to '{new_name}'");
            } else {
                println!("Renaming {block_code} block '{current_name}' to '{new_name}'");

                match BlendEditor::rename_id_block_and_save(&file_path, block_index, &new_name) {
                    Ok(()) => {
                        // Re-read the file to verify the change
                        let mut updated_blend_file =
                            load_blend_file(&file_path, options, no_auto_decompress)?;

                        match NameResolver::resolve_name(block_index, &mut updated_blend_file) {
                            Some(updated_name) => {
                                if updated_name == new_name {
                                    println!("Success: Block renamed to '{updated_name}'");
                                } else {
                                    eprintln!(
                                        "Warning: Name is '{updated_name}', expected '{new_name}'"
                                    );
                                }
                            }
                            None => {
                                eprintln!("Warning: Could not verify name change");
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Error: Failed to rename block: {e}");
                    }
                }
            }
        }
        None => {
            eprintln!("Error: Block {block_index} is not a named datablock");
        }
    }

    Ok(())
}

fn cmd_mesh_diff(
    file1_path: PathBuf,
    file2_path: PathBuf,
    mesh_index: Option<usize>,
    verbose: bool,
    json: bool,
    options: &ParseOptions,
    no_auto_decompress: bool,
) -> Result<()> {
    let mut blend_file1 = load_blend_file(&file1_path, options, no_auto_decompress)?;
    let mut blend_file2 = load_blend_file(&file2_path, options, no_auto_decompress)?;

    // Create a differ with provenance analysis enabled
    let differ = BlendDiffer::new()
        .with_provenance_analysis(true)
        .with_provenance_config(|analyzer| analyzer.with_verbose(verbose));

    println!("Enhanced Mesh Diff Analysis");
    println!("==========================");
    println!("File 1: {}", file1_path.display());
    println!("File 2: {}", file2_path.display());
    println!();

    if let Some(me_index) = mesh_index {
        // Analyze specific ME block
        match differ.analyze_mesh_block(me_index, &mut blend_file1, &mut blend_file2) {
            Ok(analysis) => {
                if json {
                    match serde_json::to_string_pretty(&analysis) {
                        Ok(json_str) => println!("{json_str}"),
                        Err(e) => eprintln!("Error serializing to JSON: {e}"),
                    }
                } else {
                    let me_name = NameResolver::get_display_name(me_index, &mut blend_file1, "ME");
                    println!("Analysis for ME block {me_index} ({me_name}):");
                    println!("  Classification: {:?}", analysis.overall_classification);
                    println!("  Is True Edit: {}", analysis.is_true_edit);
                    println!("  Summary: {}", analysis.summary);
                    println!();

                    if let Some(before) = &analysis.before_provenance {
                        println!(
                            "  Before: {} referenced DATA blocks",
                            before.referenced_data_blocks.len()
                        );
                    }
                    if let Some(after) = &analysis.after_provenance {
                        println!(
                            "  After: {} referenced DATA blocks",
                            after.referenced_data_blocks.len()
                        );
                    }

                    println!("  DATA Block Correlations:");
                    for (i, correlation) in analysis.data_correlations.iter().enumerate() {
                        println!(
                            "    {}: {:?} (confidence: {:.2}) - {}",
                            i + 1,
                            correlation.change_class,
                            correlation.confidence,
                            correlation.rationale
                        );
                    }
                }
            }
            Err(e) => {
                eprintln!("Error analyzing ME block {me_index}: {e}");
            }
        }
    } else {
        // Find all ME blocks and analyze them
        let me_blocks: Vec<usize> = blend_file1
            .blocks
            .iter()
            .enumerate()
            .filter_map(|(i, block)| {
                let code = String::from_utf8_lossy(&block.header.code);
                if code.trim_end_matches('\0') == "ME" {
                    Some(i)
                } else {
                    None
                }
            })
            .collect();

        println!("Found {} ME blocks to analyze", me_blocks.len());
        println!();

        let mut analyses = Vec::new();
        for &me_index in &me_blocks {
            match differ.analyze_mesh_block(me_index, &mut blend_file1, &mut blend_file2) {
                Ok(analysis) => {
                    if !json {
                        let me_name =
                            NameResolver::get_display_name(me_index, &mut blend_file1, "ME");
                        println!(
                            "ME block {} ({}): {} ({})",
                            me_index,
                            me_name,
                            if analysis.is_true_edit {
                                "TRUE EDIT"
                            } else {
                                "Layout/Noise"
                            },
                            analysis.summary
                        );
                    }
                    analyses.push(analysis);
                }
                Err(e) => {
                    eprintln!("Error analyzing ME block {me_index}: {e}");
                }
            }
        }

        if json {
            match serde_json::to_string_pretty(&analyses) {
                Ok(json_str) => println!("{json_str}"),
                Err(e) => eprintln!("Error serializing to JSON: {e}"),
            }
        } else {
            println!();
            let true_edits = analyses.iter().filter(|a| a.is_true_edit).count();
            let layout_changes = analyses.len() - true_edits;
            println!("Summary: {true_edits} true edits, {layout_changes} layout/noise changes");
        }
    }

    Ok(())
}
