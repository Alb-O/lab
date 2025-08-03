// bllink-cli/src/main.rs

use bllink_diff::BlendDiffer;
use bllink_tracer::{BlendFile, Result};

mod diff_formatter;
use bllink_tracer::{
    CacheFileExpander, CollectionExpander, DataBlockExpander, DependencyTracer, ImageExpander,
    LampExpander, LibraryExpander, MaterialExpander, MeshExpander, NameResolver, NodeTreeExpander,
    ObjectExpander, SceneExpander, SoundExpander, TextureExpander,
};
use clap::{Parser, Subcommand, ValueEnum};
use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;
use text_trees::{FormatCharacters, StringTreeNode, TreeFormatting};

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
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
    Blocks {
        file: PathBuf,
        #[arg(
            long,
            help = "Show user-defined names for datablocks (e.g., 'Cube', 'Material.001')"
        )]
        show_names: bool,
    },
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
        #[arg(
            long,
            help = "Show user-defined names for datablocks (e.g., 'Cube', 'Material.001')"
        )]
        show_names: bool,
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
        #[arg(
            long,
            help = "Show user-defined names for datablocks (e.g., 'Cube', 'Material.001')"
        )]
        show_names: bool,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Info { file } => cmd_info(file),
        Commands::Blocks { file, show_names } => cmd_blocks(file, show_names),
        Commands::Dependencies {
            file,
            block_index,
            format,
            ascii,
            show_names,
        } => cmd_dependencies(file, block_index, format, ascii, show_names),
        Commands::Diff {
            file1,
            file2,
            only_modified,
            format,
            ascii,
            show_names,
        } => cmd_diff(file1, file2, only_modified, format, ascii, show_names),
    }
}

fn cmd_info(file_path: PathBuf) -> Result<()> {
    let file = File::open(&file_path)?;
    let mut reader = BufReader::new(file);
    let blend_file = BlendFile::new(&mut reader)?;

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

fn cmd_blocks(file_path: PathBuf, show_names: bool) -> Result<()> {
    let file = File::open(&file_path)?;
    let mut reader = BufReader::new(file);
    let mut blend_file = BlendFile::new(&mut reader)?;

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
        let display_name = if show_names {
            NameResolver::get_display_name(i, &mut blend_file, &code_str)
        } else {
            code_str
        };

        println!("  {i}: {display_name} (size: {size}, addr: 0x{address:x})");
    }

    Ok(())
}

fn cmd_dependencies(
    file_path: PathBuf,
    block_index: usize,
    format: OutputFormat,
    ascii: bool,
    show_names: bool,
) -> Result<()> {
    let file = File::open(&file_path)?;
    let mut reader = BufReader::new(file);
    let mut blend_file = BlendFile::new(&mut reader)?;

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

                        let display_name = if show_names {
                            NameResolver::get_display_name(dep_index, &mut blend_file, &code_str)
                        } else {
                            code_str
                        };

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
            let tree_display = build_text_tree(&tree.root, &mut blend_file, show_names);

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
    node: &bllink_tracer::DependencyNode,
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
    show_names: bool,
) -> Result<()> {
    let file1 = File::open(&file1_path)?;
    let mut reader1 = BufReader::new(file1);
    let mut blend_file1 = BlendFile::new(&mut reader1)?;

    let file2 = File::open(&file2_path)?;
    let mut reader2 = BufReader::new(file2);
    let mut blend_file2 = BlendFile::new(&mut reader2)?;

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
                show_names,
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
