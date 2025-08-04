mod commands;
mod diff_formatter;
mod util;

use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

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
    /// Update the filepath of a Library (LI) block
    LibPath {
        file: PathBuf,
        #[arg(short, long)]
        block_index: usize,
        #[arg(short, long)]
        new_path: String,
        #[arg(long, help = "Preview changes without modifying the file")]
        dry_run: bool,
    },
    Info {
        file: PathBuf,
    },
    Blocks {
        file: PathBuf,
    },
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
    Rename {
        file: PathBuf,
        #[arg(short, long)]
        block_index: usize,
        #[arg(short, long)]
        new_name: String,
        #[arg(long, help = "Preview changes without modifying the file")]
        dry_run: bool,
    },
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
    Filter {
        file: PathBuf,
        #[arg(short, long, help = "Filter expressions (format: [+/-][recursion]key=value_regex)", action = clap::ArgAction::Append)]
        filter: Vec<String>,
        #[arg(short, long, value_enum, default_value_t = OutputFormat::Flat)]
        format: OutputFormat,
        #[arg(long, help = "Show detailed information about each filtered block")]
        verbose: bool,
        #[arg(long, help = "Output as JSON")]
        json: bool,
    },
}

fn main() -> dot001_tracer::Result<()> {
    let cli = Cli::parse();
    let parse_options = util::create_parse_options(&cli);
    match cli.command {
        Commands::LibPath {
            file,
            block_index,
            new_path,
            dry_run,
        } => commands::cmd_libpath(file, block_index, new_path, dry_run),
        Commands::Info { file } => commands::cmd_info(file, &parse_options, cli.no_auto_decompress),
        Commands::Blocks { file } => {
            commands::cmd_blocks(file, &parse_options, cli.no_auto_decompress)
        }
        Commands::Dependencies {
            file,
            block_index,
            format,
            ascii,
        } => commands::cmd_dependencies(
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
        } => commands::cmd_diff(
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
        } => commands::cmd_rename(
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
        } => commands::cmd_mesh_diff(
            file1,
            file2,
            mesh_index,
            verbose,
            json,
            &parse_options,
            cli.no_auto_decompress,
        ),
        Commands::Filter {
            file,
            filter,
            format,
            verbose,
            json,
        } => commands::cmd_filter(
            file,
            filter,
            format,
            verbose,
            json,
            &parse_options,
            cli.no_auto_decompress,
        ),
    }
}
