// bllink-cli/src/main.rs

use bllink_tracer::{BlendFile, Result};
use bllink_tracer::{
    CollectionExpander, DependencyTracer, MeshExpander, ObjectExpander, SceneExpander,
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
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Info { file } => cmd_info(file),
        Commands::Blocks { file } => cmd_blocks(file),
        Commands::Dependencies {
            file,
            block_index,
            format,
            ascii,
        } => cmd_dependencies(file, block_index, format, ascii),
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

fn cmd_blocks(file_path: PathBuf) -> Result<()> {
    let file = File::open(&file_path)?;
    let mut reader = BufReader::new(file);
    let blend_file = BlendFile::new(&mut reader)?;

    println!("Blocks in {}:", file_path.display());
    for (i, block) in blend_file.blocks.iter().enumerate() {
        let code_str = String::from_utf8_lossy(&block.header.code);
        println!(
            "  {}: {} (size: {}, addr: 0x{:x})",
            i,
            code_str.trim_end_matches('\0'),
            block.header.size,
            block.header.old_address
        );
    }

    Ok(())
}

fn cmd_dependencies(
    file_path: PathBuf,
    block_index: usize,
    format: OutputFormat,
    ascii: bool,
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
                        let code_str = String::from_utf8_lossy(&block.header.code);
                        println!(
                            "    {}: Block {} ({})",
                            i + 1,
                            dep_index,
                            code_str.trim_end_matches('\0')
                        );
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
            let tree_display = build_text_tree(&tree.root);

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
            match serde_json::to_string_pretty(&tree) {
                Ok(json) => println!("{json}"),
                Err(e) => eprintln!("Error serializing to JSON: {e}"),
            }
        }
    }

    Ok(())
}

fn build_text_tree(node: &bllink_tracer::DependencyNode) -> StringTreeNode {
    let label = format!(
        "Block {} ({}) - size: {}, addr: 0x{:x}",
        node.block_index, node.block_code, node.block_size, node.block_address
    );

    if node.children.is_empty() {
        StringTreeNode::new(label)
    } else {
        let child_nodes: Vec<StringTreeNode> = node.children.iter().map(build_text_tree).collect();

        StringTreeNode::with_child_nodes(label, child_nodes.into_iter())
    }
}
