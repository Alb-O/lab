use blendreader::{
    AnalysisOptions, BlendFile, FileAnalysis, detailed_block_info, format, get_interesting_blocks,
};

use clap::{Parser, ValueHint};
use std::fs::File;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// A tool for analyzing Blender .blend files
#[derive(Parser)]
#[command(name = "blendreader")]
#[command(about = "Analyze and inspect Blender .blend files")]
#[command(long_about = "
BlendReader analyzes Blender .blend files and provides detailed information about their contents.

By default, it shows user data blocks (Objects, Meshes, Materials, etc.) and filters out 
system blocks (file structure, metadata). Use --include-system to see all blocks.

EXAMPLES:
    blendreader file.blend                    # Analyze a single file
    blendreader path/to/files/                # Analyze all .blend files in directory  
    blendreader -j 4 *.blend                  # Use 4 parallel threads
    blendreader --include-system file.blend   # Show system blocks too
    blendreader --max-blocks 25 file.blend    # Show more block details
")]
#[command(version)]
struct Cli {
    /// Files or directories to analyze
    #[arg(value_name = "FILES", value_hint = ValueHint::AnyPath)]
    #[arg(
        required = true,
        help = "Blender .blend files or directories to analyze"
    )]
    paths: Vec<PathBuf>,

    /// Number of parallel processing threads
    #[arg(short = 'j', long = "jobs", value_name = "N")]
    #[arg(value_parser = clap::value_parser!(u16).range(1..))]
    #[arg(help = "Number of parallel threads for processing multiple files")]
    jobs: Option<u16>,

    /// Show system blocks (GLOB, DNA1, REND, etc.) - filtered by default
    #[arg(long = "include-system")]
    #[arg(help = "Show system/metadata blocks (filtered by default)")]
    include_system: bool,

    /// Maximum number of blocks to show details for
    #[arg(long = "max-blocks", value_name = "N", default_value = "15")]
    #[arg(value_parser = clap::value_parser!(u16).range(1..))]
    #[arg(help = "Maximum number of blocks to show detailed information for")]
    max_blocks: u16,

    /// Hide warnings in analysis
    #[arg(long = "no-warnings")]
    #[arg(help = "Hide analysis warnings")]
    no_warnings: bool,

    /// Hide invalid block size warnings
    #[arg(long = "no-invalid-warnings")]
    #[arg(help = "Hide warnings about invalid block sizes")]
    no_invalid_warnings: bool,
}

fn read_all(path: &Path) -> io::Result<Vec<u8>> {
    let mut f = File::open(path)?;
    let mut buf = Vec::new();
    f.read_to_end(&mut buf)?;
    Ok(buf)
}

impl From<&Cli> for AnalysisOptions {
    fn from(cli: &Cli) -> Self {
        AnalysisOptions {
            include_system_blocks: cli.include_system,
            max_blocks_to_show: cli.max_blocks as usize,
            show_invalid_blocks: !cli.no_invalid_warnings,
            show_warnings: !cli.no_warnings,
        }
    }
}

fn discover_blend_files(inputs: Vec<PathBuf>) -> Vec<PathBuf> {
    let mut files = Vec::new();
    for p in inputs {
        if p.is_dir() {
            for entry in walkdir::WalkDir::new(&p).into_iter().filter_map(Result::ok) {
                let path = entry.path();
                if path.is_file() {
                    if let Some(ext) = path.extension() {
                        if ext.eq_ignore_ascii_case("blend") {
                            files.push(path.to_path_buf());
                        }
                    }
                }
            }
        } else if p.is_file() {
            files.push(p);
        } else {
            eprintln!("warning: path not found: {}", p.display());
        }
    }
    files
}

fn main() -> io::Result<()> {
    let cli = Cli::parse();

    let options = AnalysisOptions::from(&cli);
    let files = discover_blend_files(cli.paths);
    if files.is_empty() {
        eprintln!("No .blend files found");
        std::process::exit(1);
    }

    let process = |paths: Vec<PathBuf>, opts: AnalysisOptions| {
        use rayon::prelude::*;
        let results: Vec<(String, String)> = paths
            .into_par_iter()
            .map(|path_buf| {
                let path_str = path_buf.to_string_lossy().into_owned();
                let mut out = Vec::new();
                match read_all(&path_buf) {
                    Ok(data) => {
                        writeln!(&mut out, "-- {} --", path_buf.display()).ok();
                        let arc: Arc<[u8]> = data.into_boxed_slice().into();
                        match BlendFile::from_bytes_auto_decompress(arc) {
                            Ok(bf) => {
                                let hdr = &bf.header;
                                writeln!(
                                    &mut out,
                                    "Header: ptr_size={} endian={} file_version={} format_version={}",
                                    hdr.pointer_size, hdr.endian as u8, hdr.file_version, hdr.file_format_version
                                )
                                .ok();
                                
                                let analysis = FileAnalysis::analyze_with_options(&bf, &opts);
                                writeln!(&mut out, "\n=== File Analysis ===").ok();
                                writeln!(&mut out, "Total blocks: {}", analysis.total_blocks).ok();
                                writeln!(&mut out, "Data blocks: {} | Meta blocks: {}", analysis.data_blocks, analysis.meta_blocks).ok();
                                writeln!(&mut out, "Total size: {} bytes ({:.1} KB)", analysis.total_size, analysis.total_size as f64 / 1024.0).ok();
                                
                                if opts.show_warnings && !analysis.warnings.is_empty() {
                                    writeln!(&mut out, "\n=== Warnings ===").ok();
                                    for warning in &analysis.warnings {
                                        writeln!(&mut out, "⚠️  {warning}").ok();
                                    }
                                }
                                
                                writeln!(&mut out, "\n=== Notable Blocks ===").ok();
                                let interesting_blocks = get_interesting_blocks(&bf, &opts);
                                let mut saw_dna = false;
                                
                                for bh in interesting_blocks {
                                    if bh.code == format::codes::BLO_CODE_DNA1 {
                                        writeln!(&mut out, "{}", detailed_block_info(&bh)).ok();
                                        match bf.read_dna_block(&bh) {
                                            Ok(info) => {
                                                writeln!(
                                                    &mut out,
                                                    "  SDNA: names={} types={} structs={}",
                                                    info.names_len, info.types_len, info.structs_len
                                                )
                                                .ok();
                                            }
                                            Err(e) => {
                                                writeln!(&mut out, "  SDNA decode error: {e}").ok();
                                            }
                                        }
                                        saw_dna = true;
                                    } else {
                                        writeln!(&mut out, "{}", detailed_block_info(&bh)).ok();
                                    }
                                }
                                
                                if analysis.total_blocks > opts.max_blocks_to_show {
                                    let remaining = analysis.total_blocks - opts.max_blocks_to_show;
                                    writeln!(&mut out, "... and {remaining} more blocks (use --max-blocks to show more)").ok();
                                }
                                
                                writeln!(&mut out, "\n=== Block Type Summary ===").ok();
                                let mut types: Vec<_> = analysis.block_type_stats.iter().collect();
                                types.sort_by(|a, b| b.1.count.cmp(&a.1.count));
                                let mut system_blocks_count = 0;
                                let mut system_blocks_size = 0;
                                
                                for (block_type, stats) in types.iter().take(10) {
                                    let is_system = matches!(block_type.as_str(), 
                                        "DATA" | "GLOB" | "DNA1" | "REND" | "USER" | "ENDB" | "WindowManager" | "Screen" | "TEST");
                                    
                                    if !opts.include_system_blocks && is_system {
                                        system_blocks_count += stats.count;
                                        system_blocks_size += stats.total_size;
                                    } else {
                                        writeln!(&mut out, "{:12}: {:3} blocks, {:8} bytes total", 
                                            block_type, stats.count, stats.total_size).ok();
                                    }
                                }
                                
                                if !opts.include_system_blocks && system_blocks_count > 0 {
                                    writeln!(&mut out, "{:12}: {:3} blocks, {:8} bytes (filtered, use --include-system)", 
                                        "System", system_blocks_count, system_blocks_size).ok();
                                }
                            }
                            Err(e) => {
                                writeln!(&mut out, "Error: {e}").ok();
                            }
                        }
                    }
                    Err(e) => {
                        writeln!(&mut out, "-- {} --", path_buf.display()).ok();
                        writeln!(&mut out, "Error reading file: {e}").ok();
                    }
                }
                let s = String::from_utf8(out).unwrap_or_else(|_| String::from("<non-utf8 output>"));
                (path_str, s)
            })
            .collect();
        results
    };

    let mut results: Vec<(String, String)> = if let Some(n) = cli.jobs {
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(n as usize)
            .build()
            .unwrap();
        pool.install(|| process(files, options.clone()))
    } else {
        process(files, options)
    };

    results.sort_by(|a, b| a.0.cmp(&b.0));
    for (_path, s) in results.into_iter() {
        print!("{s}");
    }

    Ok(())
}
