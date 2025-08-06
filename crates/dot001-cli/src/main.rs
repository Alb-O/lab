mod block_display;
mod commands;
#[cfg(feature = "diff")]
mod diff_formatter;
mod util;

use clap::{Parser, Subcommand, ValueEnum};
use dot001_error::{CliErrorKind, Dot001Error};
use log::{debug, info};
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

    /// Enable verbose logging (can be used multiple times: -v, -vv, -vvv)
    #[arg(short = 'v', long = "verbose", global = true, action = clap::ArgAction::Count)]
    verbose: u8,

    /// Quiet mode: suppress explanatory output, show only raw results
    #[arg(short = 'q', long = "quiet", global = true)]
    quiet: bool,
}

#[derive(Clone, ValueEnum, Debug)]
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
    #[cfg(feature = "editor")]
    LibPath {
        #[arg(index = 1)]
        file: PathBuf,
        #[arg(
            index = 2,
            help = "Block index or datablock name (e.g., '5' or 'Cube')"
        )]
        block_index: String,
        #[arg(index = 3)]
        new_path: String,
        #[arg(long, help = "Preview changes without modifying the file")]
        dry_run: bool,
        #[arg(
            long,
            help = "Bypass validation checks (allow non-existent target and no-op)"
        )]
        no_validate: bool,
    },
    #[cfg(feature = "info")]
    Info {
        #[arg(index = 1)]
        file: PathBuf,
    },
    #[cfg(feature = "blocks")]
    Blocks {
        #[arg(index = 1)]
        file: PathBuf,
        #[arg(long, help = "Include DATA blocks in output (filtered out by default)")]
        show_data: bool,
    },
    #[cfg(feature = "trace")]
    Dependencies {
        #[arg(index = 1)]
        file: PathBuf,
        #[arg(
            index = 2,
            help = "Block index or datablock name (e.g., '5' or 'Cube')"
        )]
        block_index: String,
        #[arg(short, long, value_enum, default_value_t = OutputFormat::Flat)]
        format: OutputFormat,
        #[arg(
            long,
            help = "Use ASCII characters instead of Unicode box characters for tree output"
        )]
        ascii: bool,
    },
    #[cfg(feature = "diff")]
    Diff {
        #[arg(index = 1)]
        file1: PathBuf,
        #[arg(index = 2)]
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
    #[cfg(feature = "editor")]
    Rename {
        #[arg(index = 1)]
        file: PathBuf,
        #[arg(
            index = 2,
            help = "Block index or datablock name (e.g., '5' or 'Cube')"
        )]
        block_index: String,
        #[arg(index = 3)]
        new_name: String,
        #[arg(long, help = "Preview changes without modifying the file")]
        dry_run: bool,
    },
    #[cfg(feature = "diff")]
    MeshDiff {
        #[arg(index = 1)]
        file1: PathBuf,
        #[arg(index = 2)]
        file2: PathBuf,
        #[arg(
            long,
            help = "ME block index or mesh name to analyze (e.g., '5' or 'Cube')"
        )]
        mesh_index: Option<String>,
        #[arg(
            long = "verbose-provenance",
            help = "Enable verbose provenance logging"
        )]
        verbose_provenance: bool,
        #[arg(long, help = "Output detailed analysis as JSON")]
        json: bool,
    },
    #[cfg(feature = "trace")]
    Filter {
        #[arg(index = 1)]
        file: PathBuf,
        #[arg(index = 2, help = "Filter expressions (format: [+/-][recursion]key=value_regex or just 'name' for name matching)", action = clap::ArgAction::Append)]
        filters: Vec<String>,
        #[arg(short, long, value_enum, default_value_t = OutputFormat::Flat)]
        format: OutputFormat,
        #[arg(
            long = "verbose-details",
            help = "Show detailed information about each filtered block"
        )]
        verbose_details: bool,
        #[arg(long, help = "Output as JSON")]
        json: bool,
    },

    /// Analyze and reconstruct broken library links
    #[cfg(feature = "trace")]
    ReconstructLink {
        #[arg(index = 1)]
        file: PathBuf,
        #[arg(
            index = 2,
            help = "Block index or datablock name (e.g., '5' or 'Cube')"
        )]
        block_index: String,
        #[arg(long, help = "Preview reconstruction without modifying the file")]
        dry_run: bool,
        #[arg(long, help = "Target asset name to link to")]
        target_name: Option<String>,
    },
}

#[cfg(feature = "trace")]
fn main() -> Result<(), Dot001Error> {
    run_main()
}

#[cfg(not(feature = "trace"))]
fn main() -> Result<(), Dot001Error> {
    run_main()
}

/// Initialize logging based on verbosity level
fn init_logging(verbose: u8) {
    let log_level = match verbose {
        0 => log::LevelFilter::Warn,  // Default: only warnings and errors
        1 => log::LevelFilter::Info,  // -v: info level
        2 => log::LevelFilter::Debug, // -vv: debug level
        _ => log::LevelFilter::Trace, // -vvv+: trace level (everything)
    };

    env_logger::Builder::from_default_env()
        .filter_level(log_level)
        .format_timestamp(None)
        .format_module_path(false)
        .format_target(false)
        .format(|buf, record| {
            use std::io::Write;
            let level_style = match record.level() {
                log::Level::Error => "\x1b[31mERROR\x1b[0m", // Red
                log::Level::Warn => "\x1b[33mWARN\x1b[0m",   // Yellow
                log::Level::Info => "\x1b[32mINFO\x1b[0m",   // Green
                log::Level::Debug => "\x1b[36mDEBUG\x1b[0m", // Cyan
                log::Level::Trace => "\x1b[35mTRACE\x1b[0m", // Magenta
            };

            writeln!(buf, "[{}] {}", level_style, record.args())
        })
        .init();
}

fn run_main() -> Result<(), Dot001Error> {
    let cli = Cli::parse();

    // Initialize logging based on verbosity level
    init_logging(cli.verbose);

    info!("dot001-cli starting with verbosity level: {}", cli.verbose);
    debug!(
        "Parse options: max_in_memory={}MB, no_auto_decompress={}",
        cli.max_in_memory, cli.no_auto_decompress
    );

    let parse_options = util::create_parse_options(&cli);
    let output = util::OutputHandler::new(cli.quiet);
    let ctx = util::CommandContext::new(&parse_options, cli.no_auto_decompress, &output);

    let result = match cli.command {
        #[cfg(feature = "editor")]
        Commands::LibPath {
            file,
            block_index,
            new_path,
            dry_run,
            no_validate,
        } => commands::cmd_libpath(file, &block_index, new_path, dry_run, no_validate, &ctx),
        #[cfg(feature = "info")]
        Commands::Info { file } => commands::cmd_info(file, &ctx),
        #[cfg(feature = "blocks")]
        Commands::Blocks { file, show_data } => commands::cmd_blocks(file, show_data, &ctx),
        #[cfg(feature = "trace")]
        Commands::Dependencies {
            file,
            block_index,
            format,
            ascii,
        } => commands::cmd_dependencies(file, &block_index, format, ascii, &ctx),
        #[cfg(feature = "diff")]
        Commands::Diff {
            file1,
            file2,
            only_modified,
            format,
            ascii,
        } => commands::cmd_diff(file1, file2, only_modified, format, ascii, &ctx),
        #[cfg(feature = "editor")]
        Commands::Rename {
            file,
            block_index,
            new_name,
            dry_run,
        } => commands::cmd_rename(file, &block_index, new_name, dry_run, &ctx),
        #[cfg(feature = "diff")]
        Commands::MeshDiff {
            file1,
            file2,
            mesh_index,
            verbose_provenance,
            json,
        } => commands::cmd_mesh_diff(
            file1,
            file2,
            mesh_index.as_deref(),
            verbose_provenance,
            json,
            &ctx,
        ),
        #[cfg(feature = "trace")]
        Commands::Filter {
            file,
            filters,
            format,
            verbose_details,
            json,
        } => commands::cmd_filter(file, filters, format, verbose_details, json, &ctx),
        #[cfg(feature = "trace")]
        Commands::ReconstructLink {
            file,
            block_index,
            dry_run,
            target_name,
        } => commands::cmd_reconstruct_link(file, &block_index, dry_run, target_name, &ctx),
    };

    if let Err(e) = result {
        use log::error;
        error!("{}", e.user_message());
        std::process::exit(1);
    }

    Ok(())
}

/// Helper functions for creating unified CLI errors
pub fn create_cli_error<M: Into<String>>(message: M, kind: CliErrorKind) -> Dot001Error {
    Dot001Error::cli(message.into(), kind)
}

/// Create an invalid arguments error
pub fn invalid_arguments_error<M: Into<String>>(message: M) -> Dot001Error {
    create_cli_error(message, CliErrorKind::InvalidArguments)
}

/// Create a missing argument error
pub fn missing_argument_error<M: Into<String>>(message: M) -> Dot001Error {
    create_cli_error(message, CliErrorKind::MissingArgument)
}

/// Create an execution failed error
pub fn execution_failed_error<M: Into<String>>(message: M) -> Dot001Error {
    create_cli_error(message, CliErrorKind::ExecutionFailed)
}

/// Create an output format error
pub fn output_format_error<M: Into<String>>(message: M) -> Dot001Error {
    create_cli_error(message, CliErrorKind::OutputFormatError)
}
