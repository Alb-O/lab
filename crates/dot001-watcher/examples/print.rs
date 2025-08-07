use dot001_watcher::{WatchEvent, WatchOptions, watch};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let root = std::env::args().nth(1).unwrap_or_else(|| ".".to_string());
    let opts = WatchOptions {
        root: root.into(),
        debounce_ms: 200,
        move_pair_window_ms: 2000,
        follow_symlinks: false,
    };

    let (rx, _watcher) = watch(opts)?;

    println!("Watching .blend files under current dir. Pass a path argument to change root.");
    while let Ok(ev) = rx.recv() {
        match ev {
            WatchEvent::BlendFileMoved(pair) => {
                println!(
                    "[BlendFileMoved] {} -> {}",
                    pair.from.display(),
                    pair.to.display()
                );
            }
            WatchEvent::BlendFileRenamed {
                from,
                to,
                base_from,
                base_to,
            } => {
                println!(
                    "[BlendFileRenamed] {} (from={:?}) -> {} (to={:?})",
                    from.display(),
                    base_from,
                    to.display(),
                    base_to
                );
            }
            WatchEvent::DirRenamedOrMoved(pair) => {
                println!(
                    "[DirRenamedOrMoved] {} -> {}",
                    pair.from.display(),
                    pair.to.display()
                );
            }
            WatchEvent::DirBlendChildMoved(pair) => {
                println!(
                    "[DirBlendChildMoved] {} -> {}",
                    pair.from.display(),
                    pair.to.display()
                );
            }
        }
    }

    Ok(())
}
