// bllink-cli/src/main.rs

use bllink_tracer::{BlendFile, Result};
use bllink_tracer::{DependencyTracer, MeshExpander, ObjectExpander, SceneExpander};
use clap::{Parser, Subcommand};
use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
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
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Info { file } => cmd_info(file),
        Commands::Blocks { file } => cmd_blocks(file),
        Commands::Dependencies { file, block_index } => cmd_dependencies(file, block_index),
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

fn cmd_dependencies(file_path: PathBuf, block_index: usize) -> Result<()> {
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

    let start_block = &blend_file.blocks[block_index];
    let start_code = String::from_utf8_lossy(&start_block.header.code);
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

    Ok(())
}
