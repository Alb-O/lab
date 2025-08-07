use std::path::PathBuf;

use clap::Args;
use dot001_watcher::{WatchEvent, WatchOptions, watch};
use log::info;

#[derive(Args)]
pub struct WatchArgs {
    /// Directory to watch for .blend file changes
    #[arg(default_value = ".")]
    pub path: PathBuf,

    /// Debounce delay in milliseconds for filesystem events
    #[arg(long, default_value = "200")]
    pub debounce_ms: u64,

    /// Time window in milliseconds to pair move events (delete+create)
    #[arg(long, default_value = "2000")]
    pub move_pair_window_ms: u64,

    /// Follow symbolic links
    #[arg(long)]
    pub follow_symlinks: bool,

    /// Print verbose event information
    #[arg(short, long)]
    pub verbose: bool,
}

pub fn cmd_watch(args: WatchArgs) -> Result<(), Box<dyn std::error::Error>> {
    let opts = WatchOptions {
        root: args.path.clone(),
        debounce_ms: args.debounce_ms,
        move_pair_window_ms: args.move_pair_window_ms,
        follow_symlinks: args.follow_symlinks,
    };

    let (rx, _watcher) = watch(opts)?;

    info!(
        "Watching for .blend file changes under {}",
        args.path.display()
    );
    println!(
        "Watching for .blend file changes under {}...",
        args.path.display()
    );
    println!("Press Ctrl+C to stop.");
    println!();

    loop {
        match rx.recv() {
            Ok(event) => match event {
                WatchEvent::BlendFileMoved(pair) => {
                    println!(
                        "🔄 Blend file moved: {} → {}",
                        pair.from.display(),
                        pair.to.display()
                    );
                    if args.verbose {
                        println!("   Base filename: {:?}", pair.base);
                    }
                }
                WatchEvent::BlendFileRenamed {
                    from,
                    to,
                    base_from,
                    base_to,
                } => {
                    println!(
                        "📝 Blend file renamed: {} → {}",
                        from.display(),
                        to.display()
                    );
                    if args.verbose {
                        println!(
                            "   {} → {}",
                            base_from.to_string_lossy(),
                            base_to.to_string_lossy()
                        );
                    }
                }
                WatchEvent::DirRenamedOrMoved(pair) => {
                    println!(
                        "📁 Directory moved: {} → {}",
                        pair.from.display(),
                        pair.to.display()
                    );
                    if args.verbose {
                        println!("   Directory: {:?}", pair.base);
                    }
                }
                WatchEvent::DirBlendChildMoved(pair) => {
                    println!("🔗 Child blend file affected by directory move:");
                    println!("   {} → {}", pair.from.display(), pair.to.display());
                    if args.verbose {
                        println!("   Base filename: {:?}", pair.base);
                    }
                }
            },
            Err(_) => {
                println!("Watcher channel closed. Exiting.");
                break;
            }
        }
    }

    Ok(())
}
