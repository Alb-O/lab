use std::path::{Path, PathBuf};
use std::time::Duration;

use crossbeam_channel::{Receiver, unbounded};
use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Result as NotifyResult, Watcher};
use thiserror::Error;

pub mod dir_emit;
pub mod normalizer;

#[cfg(feature = "async-events")]
pub mod async_watcher;

pub use dir_emit::emit_dir_child_moves;
pub use normalizer::{NormalizedEvent, Normalizer};

#[derive(Debug, Error)]
pub enum WatchError {
    #[error("notify error: {0}")]
    Notify(#[from] notify::Error),
    #[error("watcher initialization failed: {0}")]
    Init(String),
}

#[derive(Clone, Debug)]
pub struct WatchOptions {
    pub root: PathBuf,
    pub debounce_ms: u64,
    pub move_pair_window_ms: u64,
    pub follow_symlinks: bool,
}

impl Default for WatchOptions {
    fn default() -> Self {
        Self {
            root: PathBuf::from("."),
            debounce_ms: 200,
            move_pair_window_ms: 2000,
            follow_symlinks: false,
        }
    }
}

#[derive(Clone, Debug)]
pub struct PathPair {
    pub from: PathBuf,
    pub to: PathBuf,
    pub base: std::ffi::OsString,
}

#[derive(Clone, Debug)]
pub enum WatchEvent {
    BlendFileMoved(PathPair), // directory changed, base unchanged
    BlendFileRenamed {
        from: PathBuf,
        to: PathBuf,
        base_from: std::ffi::OsString,
        base_to: std::ffi::OsString,
    },
    DirRenamedOrMoved(PathPair),
    DirBlendChildMoved(PathPair), // synthetic per .blend inside moved dir
}

fn is_blend(path: &Path) -> bool {
    match path.extension().and_then(|e| e.to_str()) {
        Some(ext) => ext.eq_ignore_ascii_case("blend"),
        None => false,
    }
}

/// Start a blocking, channel-based watcher. Returns a Receiver that yields normalized WatchEvent values.
///
/// Notes:
/// - Only .blend file events and directory rename/move events are surfaced.
/// - Delete+Create within a time window with same base filename are paired as moves.
/// - On directory rename/move, synthetic events for child .blend files are emitted.
pub fn watch(
    options: WatchOptions,
) -> Result<(Receiver<WatchEvent>, RecommendedWatcher), WatchError> {
    // Output channel to the consumer
    let (out_tx, out_rx) = unbounded::<WatchEvent>();

    // Internal channel from notify to our normalizer
    let (raw_tx, raw_rx) = unbounded::<Event>();

    // Configure notify watcher
    let cfg = Config::default().with_poll_interval(Duration::from_millis(options.debounce_ms));
    // Note: notify 6.x doesn't have with_follow_symlinks method
    // Following symlinks behavior is controlled at the OS level

    let tx_clone_for_err = raw_tx.clone();
    let mut watcher: RecommendedWatcher = Watcher::new(
        move |res: NotifyResult<Event>| {
            match res {
                Ok(ev) => {
                    // Forward to our internal channel; drop on send failure
                    let _ = tx_clone_for_err.send(ev);
                }
                Err(e) => {
                    // Represent errors as a special Event? For simplicity we drop; logging is left to the app via log crate.
                    log::warn!("notify error: {e}");
                }
            }
        },
        cfg,
    )
    .map_err(|e| WatchError::Init(format!("{e}")))?;

    watcher.watch(&options.root, RecursiveMode::Recursive)?;

    // Spawn normalization worker thread
    std::thread::spawn({
        let move_pair_window = Duration::from_millis(options.move_pair_window_ms);
        let _root = options.root.clone();
        move || {
            let mut normalizer = Normalizer::new(move_pair_window);
            for ev in raw_rx.iter() {
                // Feed into normalizer; may yield zero, one, or multiple NormalizedEvent
                for ne in normalizer.ingest(ev) {
                    match ne {
                        NormalizedEvent::BlendMove { from, to } => {
                            if is_blend(&from) || is_blend(&to) {
                                let base = to.file_name().unwrap_or_default().to_os_string();
                                let pair = PathPair { from, to, base };
                                let _ = out_tx.send(WatchEvent::BlendFileMoved(pair));
                            }
                        }
                        NormalizedEvent::BlendRename {
                            from,
                            to,
                            base_from,
                            base_to,
                        } => {
                            // Only if they are .blend files
                            if is_blend(&from) || is_blend(&to) {
                                let _ = out_tx.send(WatchEvent::BlendFileRenamed {
                                    from,
                                    to,
                                    base_from,
                                    base_to,
                                });
                            }
                        }
                        NormalizedEvent::DirMove { from, to } => {
                            let base = to.file_name().unwrap_or_default().to_os_string();
                            let dir_pair = PathPair {
                                from: from.clone(),
                                to: to.clone(),
                                base,
                            };
                            let _ = out_tx.send(WatchEvent::DirRenamedOrMoved(dir_pair));
                            // Emit synthetic child .blend moves based on relative structure
                            for child_pair in emit_dir_child_moves(&from, &to) {
                                let _ = out_tx.send(WatchEvent::DirBlendChildMoved(child_pair));
                            }
                        }
                        NormalizedEvent::Ignore => {}
                    }
                }
            }
        }
    });

    Ok((out_rx, watcher))
}

#[cfg(feature = "tokio")]
pub mod async_api {
    use super::*;
    use futures::{Stream, stream};

    /// Async stream API backed by a dedicated blocking watcher thread forwarding to a tokio channel.
    pub fn watch_stream(
        options: WatchOptions,
    ) -> Result<impl Stream<Item = WatchEvent> + Send + 'static, WatchError> {
        let (cross_rx, watcher) = super::watch(options)?;

        // Bridge crossbeam Receiver -> Tokio mpsc to avoid blocking the async task on recv().
        let (tx, rx) = tokio::sync::mpsc::channel::<WatchEvent>(1024);

        // Forwarder thread: blocks on crossbeam recv and forwards into Tokio channel.
        std::thread::spawn(move || {
            while let Ok(ev) = cross_rx.recv() {
                // If the receiver side is gone, stop forwarding.
                if tx.blocking_send(ev).is_err() {
                    break;
                }
            }
            // Exiting: crossbeam channel closed or tokio receiver dropped.
        });

        // Keep the watcher alive by capturing it in the stream state alongside the Tokio receiver.
        let s = stream::unfold((rx, watcher), |(mut rx, watcher)| async move {
            match rx.recv().await {
                Some(ev) => Some((ev, (rx, watcher))),
                None => None,
            }
        });

        Ok(s)
    }
}
