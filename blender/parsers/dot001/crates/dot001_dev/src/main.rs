use anyhow::Result;
use clap::{Parser, Subcommand};
use dot001_dev::blendfiles_downloader::{run_downloader, DownloaderConfig};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "dot001-dev")]
#[command(about = "Development utilities for the dot001 project")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    #[command(about = "Download blendfiles from the configured map")]
    Downloader {
        /// Root dir that contains the blendfiles_map.json and where files are written
        #[arg(long, value_name = "DIR", default_value = "blendfiles")]
        root: PathBuf,

        /// Path to map JSON (defaults to <root>/blendfiles_map.json)
        #[arg(long, value_name = "FILE")]
        map: Option<PathBuf>,

        /// Force re-download even if file exists
        #[arg(long, default_value_t = false)]
        force: bool,

        /// Only process a specific folder key from the JSON map
        #[arg(long, value_name = "FOLDER")]
        folder: Option<String>,

        /// Dry-run: print what would be downloaded without performing network I/O
        #[arg(long, default_value_t = false)]
        dry_run: bool,
    },
}

fn main() -> Result<()> {
    // Initialize logger for consistent log output
    env_logger::init();

    let cli = Cli::parse();

    match &cli.command {
        Commands::Downloader {
            root,
            map,
            force,
            folder,
            dry_run,
        } => {
            let config = DownloaderConfig {
                root: root.clone(),
                map_path: map.clone(),
                force: *force,
                folder: folder.clone(),
                dry_run: *dry_run,
            };
            run_downloader(config)
        }
    }
}
