#![allow(dead_code)] // Allow unused public API functions

mod block_display;
mod block_ops;
mod block_utils;
mod cli_args;
mod commands;
#[cfg(feature = "diff")]
mod diff_formatter;
mod output_utils;
mod util;

use clap::{Parser, Subcommand, ValueEnum};
use dot001_error::{CliErrorKind, Dot001Error};
use dot001_events::{
    bus::{EventFilter, Subscriber, init_global_bus},
    bus_impl::{TokioEventBus, spawn_subscriber_task},
    event::{CliEvent, Event},
    format::{Formatter, PlainFormatter, PrettyFormatter},
    prelude::*,
};

#[cfg(feature = "json")]
use dot001_events::format::JsonFormatter;
use log::{debug, info};
use std::path::PathBuf;
use std::sync::Arc;

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

    /// Event output format
    #[arg(long = "event-format", global = true, value_enum, default_value_t = EventFormat::Pretty)]
    event_format: EventFormat,

    /// Event domains to show (comma-separated, e.g. "core,parser")
    #[arg(long = "event-domains", global = true, value_delimiter = ',')]
    event_domains: Option<Vec<String>>,
}

#[derive(Clone, ValueEnum, Debug)]
enum EventFormat {
    /// Colorized, human-friendly output (default)
    Pretty,
    /// Single-line, no color output for piping
    Plain,
    /// JSON output for machine consumption
    Json,
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

#[derive(Clone, ValueEnum, Debug)]
enum DisplayTemplate {
    /// Simple template with basic information
    Simple,
    /// Detailed template with size and address information
    Detailed,
    /// Compact template without block indices
    Compact,
}

#[derive(Subcommand)]
enum Commands {
    /// Build a synthetic .blend file using a seed DNA (5.0 header + DNA1 + ENDB)
    BuildBlend {
        /// Path to a 5.0-alpha seed .blend file to extract DNA1 from
        #[arg(long = "seed")]
        seed: PathBuf,
        /// Output .blend path to write
        #[arg(long = "out")]
        out: PathBuf,
    },

    /// Update the filepath of a linked library file (LI block)
    #[cfg(feature = "editor")]
    LibPath {
        #[command(flatten)]
        file: cli_args::FileArgs,
        #[command(flatten)]
        block_id: cli_args::BlockIdentifierArgs,
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
    /// Display blend file header information and statistics
    #[cfg(feature = "info")]
    Info {
        #[arg(index = 1)]
        file: PathBuf,
    },
    /// List all blocks in a blend file with their details
    #[cfg(feature = "blocks")]
    Blocks {
        #[command(flatten)]
        file: cli_args::FileArgs,
        #[command(flatten)]
        display: cli_args::DisplayArgs,
    },
    /// Trace and display dependencies for a specific block
    #[cfg(feature = "trace")]
    Dependencies {
        #[command(flatten)]
        file: cli_args::FileArgs,
        #[command(flatten)]
        block_id: cli_args::BlockIdentifierArgs,
        #[command(flatten)]
        format_args: cli_args::OutputFormatArgs,
        #[command(flatten)]
        display: cli_args::DisplayArgs,
    },
    /// Compare two blend files and show differences
    #[cfg(feature = "diff")]
    Diff {
        #[command(flatten)]
        files: cli_args::TwoFileArgs,
        #[arg(long, help = "Show only modified blocks, not all differences")]
        only_modified: bool,
        #[command(flatten)]
        format_args: cli_args::OutputFormatArgs,
        /// Block display template
        #[arg(short = 't', long, value_enum, default_value_t = DisplayTemplate::Compact, help = "Block display template")]
        template: DisplayTemplate,
    },
    /// Rename a datablock in a blend file
    #[cfg(feature = "editor")]
    Rename {
        #[command(flatten)]
        file: cli_args::FileArgs,
        #[command(flatten)]
        block_id: cli_args::BlockIdentifierArgs,
        #[arg(index = 3)]
        new_name: String,
        #[command(flatten)]
        display: cli_args::DisplayArgs,
        #[arg(long, help = "Preview changes without modifying the file")]
        dry_run: bool,
    },
    /// Perform enhanced mesh comparison between two blend files
    #[cfg(feature = "diff")]
    MeshDiff {
        #[command(flatten)]
        files: cli_args::TwoFileArgs,
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
        #[command(flatten)]
        display: cli_args::DisplayArgs,
        #[command(flatten)]
        json: cli_args::JsonArgs,
    },
    /// Filter and search blocks based on various criteria
    #[cfg(feature = "trace")]
    Filter {
        #[command(flatten)]
        file: cli_args::FileArgs,
        #[arg(index = 2, help = "Filter expressions (format: [+/-][recursion]key=value_regex or just 'name' for name matching)", action = clap::ArgAction::Append)]
        filters: Vec<String>,
        #[command(flatten)]
        format_args: cli_args::OutputFormatArgs,
        #[command(flatten)]
        display: cli_args::DisplayArgs,
        #[command(flatten)]
        json: cli_args::JsonArgs,
    },

    /// Analyze and reconstruct broken library linked data-blocks
    #[cfg(feature = "trace")]
    LibLink {
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

    /// Watch directory for .blend file changes and movements
    #[cfg(feature = "watch")]
    Watch {
        /// Directory to watch for .blend file changes
        #[arg(default_value = ".")]
        path: PathBuf,
        /// Debounce delay in milliseconds for filesystem events
        #[arg(long, default_value = "200")]
        debounce_ms: u64,
        /// Time window in milliseconds to pair move events (delete+create)
        #[arg(long, default_value = "2000")]
        move_pair_window_ms: u64,
        /// Follow symbolic links
        #[arg(long)]
        follow_symlinks: bool,
        /// Print verbose event information
        #[arg(long = "verbose-events")]
        verbose_events: bool,
    },
}

/// CLI subscriber that formats and prints events
struct CliSubscriber {
    formatter: Box<dyn Formatter + Send + Sync>,
    quiet: bool,
}

impl CliSubscriber {
    fn new(format: EventFormat, quiet: bool) -> Self {
        let formatter: Box<dyn Formatter + Send + Sync> = match format {
            EventFormat::Pretty => Box::new(PrettyFormatter::new()),
            EventFormat::Plain => Box::new(PlainFormatter::new()),
            #[cfg(feature = "json")]
            EventFormat::Json => Box::new(JsonFormatter::new()),
            #[cfg(not(feature = "json"))]
            EventFormat::Json => {
                eprintln!("Warning: JSON format requested but not enabled. Using plain format.");
                Box::new(PlainFormatter::new())
            }
        };

        Self { formatter, quiet }
    }
}

#[async_trait::async_trait]
impl Subscriber for CliSubscriber {
    async fn on_event(&self, event: &dot001_events::event::EventWithMetadata) {
        // Skip events in quiet mode except for errors
        if self.quiet && !matches!(event.metadata.severity, Severity::Error) {
            return;
        }

        let formatted = self.formatter.format(event);

        // Print to stderr for events, keeping stdout clean for actual output
        match event.metadata.severity {
            Severity::Error => eprintln!("{formatted}"),
            _ => eprintln!("{formatted}"),
        }
    }
}

#[tokio::main]
async fn main() {
    if let Err(e) = run_main().await {
        use log::error;
        error!("{}", e.user_message());
        std::process::exit(1);
    }
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

async fn run_main() -> std::result::Result<(), Dot001Error> {
    let cli = Cli::parse();

    // Initialize logging based on verbosity level
    init_logging(cli.verbose);

    // Create and initialize the event bus
    let bus: Arc<dyn EventBus> = Arc::new(TokioEventBus::with_default_capacity());

    // Initialize global bus for ergonomic access
    if let Err(_) = init_global_bus(bus.clone()) {
        return Err(execution_failed_error(
            "Failed to initialize global event bus",
        ));
    }

    // Create event filter based on verbosity and domain settings
    let min_severity = match (cli.quiet, cli.verbose) {
        (true, _) => Severity::Error,  // -q: only errors
        (false, 0) => Severity::Info,  // default: info, warn, error
        (false, 1) => Severity::Debug, // -v: debug, info, warn, error
        (false, _) => Severity::Trace, // -vv: all events
    };

    let mut event_filter = EventFilter::new().min_severity(min_severity);
    if let Some(domains) = &cli.event_domains {
        event_filter = event_filter.domains(domains.clone());
    }

    // Create and spawn CLI subscriber
    let cli_subscriber = Arc::new(CliSubscriber::new(cli.event_format.clone(), cli.quiet));
    let subscription = bus.subscribe(event_filter);
    let _subscriber_task = spawn_subscriber_task(subscription, cli_subscriber);

    // Emit command started event
    emit!(
        bus,
        Event::Cli(CliEvent::CommandStarted {
            command: std::env::args()
                .next()
                .unwrap_or_else(|| "dot001".to_string()),
            args: std::env::args().skip(1).collect(),
        })
    );

    info!("dot001_cli starting with verbosity level: {}", cli.verbose);
    debug!(
        "Parse options: max_in_memory={}MB, no_auto_decompress={}",
        cli.max_in_memory, cli.no_auto_decompress
    );

    let parse_options = util::create_parse_options(&cli);
    let output = util::OutputHandler::new(cli.quiet);
    let ctx = util::CommandContext::new(&parse_options, cli.no_auto_decompress, &output);

    // Capture start time for command execution timing
    let start_time = std::time::Instant::now();

    // Propagate the result; main() maps error to exit code and user message
    let result = match cli.command {
        Commands::BuildBlend { seed, out } => commands::cmd_build_blend(seed, out),
        #[cfg(feature = "editor")]
        Commands::LibPath {
            file,
            block_id,
            new_path,
            dry_run,
            no_validate,
        } => commands::cmd_libpath(
            file.file,
            &block_id.block_index,
            new_path,
            dry_run,
            no_validate,
            &ctx,
        ),
        #[cfg(feature = "info")]
        Commands::Info { file } => commands::cmd_info(file, &ctx),
        #[cfg(feature = "blocks")]
        Commands::Blocks { file, display } => {
            commands::cmd_blocks(file.file, display.show_data, display.template, &ctx)
        }
        #[cfg(feature = "trace")]
        Commands::Dependencies {
            file,
            block_id,
            format_args,
            display,
        } => commands::cmd_dependencies(
            file.file,
            &block_id.block_index,
            format_args.format,
            format_args.ascii,
            display.show_data,
            display.template,
            &ctx,
        ),
        #[cfg(feature = "diff")]
        Commands::Diff {
            files,
            only_modified,
            format_args,
            template,
        } => commands::cmd_diff(
            files.file1,
            files.file2,
            only_modified,
            format_args.format,
            template,
            format_args.ascii,
            &ctx,
        ),
        #[cfg(feature = "editor")]
        Commands::Rename {
            file,
            block_id,
            new_name,
            display,
            dry_run,
        } => commands::cmd_rename(
            file.file,
            &block_id.block_index,
            new_name,
            display.template,
            dry_run,
            &ctx,
        ),
        #[cfg(feature = "diff")]
        Commands::MeshDiff {
            files,
            mesh_index,
            verbose_provenance,
            display,
            json,
        } => commands::cmd_mesh_diff(
            files.file1,
            files.file2,
            mesh_index.as_deref(),
            verbose_provenance,
            display.template,
            json.json,
            &ctx,
        ),
        #[cfg(feature = "trace")]
        Commands::Filter {
            file,
            filters,
            format_args,
            display,
            json,
        } => commands::cmd_filter(
            file.file,
            filters,
            format_args.format,
            display.template,
            display.show_data,
            json.json,
            &ctx,
        ),
        #[cfg(feature = "trace")]
        Commands::LibLink {
            file,
            block_index,
            dry_run,
            target_name,
        } => commands::cmd_lib_link(file, &block_index, dry_run, target_name, &ctx),
        #[cfg(feature = "watch")]
        Commands::Watch {
            path,
            debounce_ms,
            move_pair_window_ms,
            follow_symlinks,
            verbose_events,
        } => {
            let args = commands::watch::WatchArgs {
                path,
                debounce_ms,
                move_pair_window_ms,
                follow_symlinks,
                verbose: verbose_events,
            };
            commands::cmd_watch(args).map_err(|e| execution_failed_error(e.to_string()))
        }
    };

    // Calculate execution time and emit completion event
    let duration_ms = start_time.elapsed().as_millis() as u64;
    let exit_code = if result.is_ok() { 0 } else { 1 };
    let command_name = std::env::args()
        .next()
        .unwrap_or_else(|| "dot001".to_string());

    emit!(
        bus,
        Event::Cli(CliEvent::CommandCompleted {
            command: command_name,
            exit_code,
            duration_ms,
        })
    );

    // Give events a moment to be processed before exiting
    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

    result
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
