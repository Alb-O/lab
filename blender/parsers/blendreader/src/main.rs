use blendreader::{
    AnalysisOptions, BlendFile, FileAnalysis, get_interesting_blocks, serialize_blend_file,
    serialize_multiple_files, serialize_to_json, serialize_to_json_compact, table_format,
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

Output is formatted in beautiful tables for easy reading, or can be exported as JSON for
programmatic processing.

EXAMPLES:
    blendreader file.blend                    # Analyze a single file (table format)
    blendreader path/to/files/                # Analyze all .blend files in directory  
    blendreader -j 4 *.blend                  # Use 4 parallel threads
    blendreader --include-system file.blend   # Show system blocks too
    blendreader --max-blocks 25 file.blend    # Show more block details
    blendreader --format json file.blend      # Export as JSON
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

    /// Output format
    #[arg(long = "format", value_name = "FORMAT", default_value = "text")]
    #[arg(help = "Output format: text, json, json-compact")]
    #[arg(value_parser = ["text", "json", "json-compact"])]
    output_format: String,
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

    let process = |paths: Vec<PathBuf>, opts: AnalysisOptions, format: &str| {
        use rayon::prelude::*;

        match format {
            "json" | "json-compact" => {
                let paths_clone = paths.clone();
                let results: Vec<Result<blendreader::BlendFileData, String>> = paths
                    .into_par_iter()
                    .map(|path_buf| {
                        let path_str = path_buf.to_string_lossy().into_owned();
                        match read_all(&path_buf) {
                            Ok(data) => {
                                let arc: Arc<[u8]> = data.into_boxed_slice().into();
                                match BlendFile::from_bytes_auto_decompress(arc) {
                                    Ok(bf) => match serialize_blend_file(&path_str, &bf, &opts) {
                                        Ok(data) => Ok(data),
                                        Err(e) => Err(format!("Serialization error: {e}")),
                                    },
                                    Err(e) => Err(format!("Parse error: {e}")),
                                }
                            }
                            Err(e) => Err(format!("Read error: {e}")),
                        }
                    })
                    .collect();

                let mut success_files = Vec::new();
                let mut errors = Vec::new();

                for (i, result) in results.into_iter().enumerate() {
                    match result {
                        Ok(data) => success_files.push(data),
                        Err(e) => errors.push((paths_clone[i].to_string_lossy().to_string(), e)),
                    }
                }

                if success_files.len() == 1 {
                    let output = if format == "json-compact" {
                        serialize_to_json_compact(&success_files[0])
                    } else {
                        serialize_to_json(&success_files[0])
                    };

                    match output {
                        Ok(json) => {
                            println!("{json}");
                            if !errors.is_empty() {
                                eprintln!("\nErrors:");
                                for (path, error) in errors {
                                    eprintln!("{path}: {error}");
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!("JSON serialization error: {e}");
                        }
                    }
                } else {
                    let multi_data = serialize_multiple_files(success_files);
                    let output = if format == "json-compact" {
                        serde_json::to_string(&multi_data)
                    } else {
                        serde_json::to_string_pretty(&multi_data)
                    };

                    match output {
                        Ok(json) => {
                            println!("{json}");
                            if !errors.is_empty() {
                                eprintln!("\nErrors:");
                                for (path, error) in errors {
                                    eprintln!("{path}: {error}");
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!("JSON serialization error: {e}");
                        }
                    }
                }
            }
            _ => {
                let mut results: Vec<(String, String)> = paths
                    .into_par_iter()
                    .map(|path_buf| {
                        let path_str = path_buf.to_string_lossy().into_owned();
                        let mut out = Vec::new();
                        match read_all(&path_buf) {
                            Ok(data) => {
                                let arc: Arc<[u8]> = data.into_boxed_slice().into();
                                match BlendFile::from_bytes_auto_decompress(arc) {
                                    Ok(bf) => {
                                        let path_str = path_buf.display().to_string();
                                        writeln!(&mut out, "{}", table_format::format_file_header(&path_str, &bf.header)).ok();

                                        let analysis = FileAnalysis::analyze_with_options(&bf, &opts);
                                        writeln!(&mut out, "\n{}", table_format::format_file_analysis(&analysis)).ok();

                                        if opts.show_warnings && !analysis.warnings.is_empty() {
                                            writeln!(&mut out, "\n{}", table_format::format_warnings(&analysis.warnings)).ok();
                                        }

                                        let interesting_blocks = get_interesting_blocks(&bf, &opts);
                                        if !interesting_blocks.is_empty() {
                                            writeln!(&mut out, "\n{}", table_format::format_section_header("Notable Blocks")).ok();
                                            writeln!(&mut out, "{}", table_format::format_blocks_table(&interesting_blocks, &bf, &opts)).ok();
                                        }

                                        if analysis.total_blocks > opts.max_blocks_to_show {
                                            let remaining = analysis.total_blocks - opts.max_blocks_to_show;
                                            writeln!(&mut out, "\nNote: {remaining} additional blocks not shown (use --max-blocks to show more)").ok();
                                        }

                                        writeln!(&mut out, "\n{}", table_format::format_section_header("Block Type Summary")).ok();
                                        writeln!(&mut out, "{}", table_format::format_block_type_summary(&analysis.block_type_stats, &opts)).ok();
                                    }
                                    Err(e) => {
                                        let path_str = path_buf.display().to_string();
                                        writeln!(&mut out, "{}", table_format::format_section_header(&format!("Error: {path_str}"))).ok();
                                        writeln!(&mut out, "ERROR: {e}").ok();
                                    }
                                }
                            }
                            Err(e) => {
                                let path_str = path_buf.display().to_string();
                                writeln!(&mut out, "{}", table_format::format_section_header(&format!("Error: {path_str}"))).ok();
                                writeln!(&mut out, "ERROR: Error reading file: {e}").ok();
                            }
                        }
                        let s = String::from_utf8(out).unwrap_or_else(|_| String::from("<non-utf8 output>"));
                        (path_str, s)
                    })
                    .collect();

                results.sort_by(|a, b| a.0.cmp(&b.0));
                for (_path, s) in results.into_iter() {
                    print!("{s}");
                }
            }
        }
    };

    if let Some(n) = cli.jobs {
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(n as usize)
            .build()
            .unwrap();
        pool.install(|| process(files, options.clone(), &cli.output_format))
    } else {
        process(files, options, &cli.output_format)
    }

    Ok(())
}
