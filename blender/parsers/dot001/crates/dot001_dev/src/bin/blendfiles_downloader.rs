use anyhow::Result;
use clap::Parser;
use dot001_dev::blendfiles_downloader::{run_downloader, DownloaderConfig};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(author, version, about = "Download blendfiles listed in blendfiles/blendfiles_map.json", long_about = None)]
struct Args {
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
}

fn main() -> Result<()> {
    let args = Args::parse();

    let config = DownloaderConfig {
        root: args.root,
        map_path: args.map,
        force: args.force,
        folder: args.folder,
        dry_run: args.dry_run,
    };

    run_downloader(config)
}
