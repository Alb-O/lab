use std::collections::VecDeque;
use std::ffi::OsString;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use notify::{Event, EventKind};

/// Higher-level events produced by the normalizer before crate-level WatchEvent mapping.
#[derive(Clone, Debug)]
pub enum NormalizedEvent {
    /// File moved (directory path changed), base filename assumed unchanged
    BlendMove { from: PathBuf, to: PathBuf },
    /// File renamed (base filename changed); directory may or may not change
    BlendRename {
        from: PathBuf,
        to: PathBuf,
        base_from: OsString,
        base_to: OsString,
    },
    /// Directory moved/renamed
    DirMove { from: PathBuf, to: PathBuf },
    /// Nothing to emit
    Ignore,
}

/// Pending op kinds we try to pair (delete & create)
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
enum PendingKind {
    Create,
    Delete,
    RenameFrom,
    RenameTo,
}

#[derive(Clone, Debug)]
struct Pending {
    when: Instant,
    path: PathBuf,
    kind: PendingKind,
}

/// Normalizer pairs delete+create into moves when base filename matches within a time window.
/// It also maps native rename events for files and directories.
pub struct Normalizer {
    window: Duration,
    /// Queue of pending singletons; we scan from the back to find best matches and pop stale from front.
    pending: VecDeque<Pending>,
}

impl Normalizer {
    pub fn new(window: Duration) -> Self {
        Self {
            window,
            pending: VecDeque::new(),
        }
    }

    /// Ingest a raw notify::Event, possibly yielding zero, one or multiple NormalizedEvent values.
    pub fn ingest(&mut self, ev: Event) -> Vec<NormalizedEvent> {
        use std::time::Instant;
        let now = Instant::now();
        self.gc(now);

        match ev.kind {
            EventKind::Remove(_) => self.handle_singletons(now, ev.paths, PendingKind::Delete),
            EventKind::Create(_) => self.handle_singletons(now, ev.paths, PendingKind::Create),
            EventKind::Modify(notify::event::ModifyKind::Name(rename_mode)) => {
                // In notify 8.x, renames generate separate From/To events with single paths
                if ev.paths.len() == 1 {
                    return self.handle_rename_events(
                        now,
                        ev.paths,
                        match rename_mode {
                            notify::event::RenameMode::From => PendingKind::RenameFrom,
                            notify::event::RenameMode::To => PendingKind::RenameTo,
                            _ => return vec![NormalizedEvent::Ignore],
                        },
                    );
                }
                vec![NormalizedEvent::Ignore]
            }
            EventKind::Modify(_) => {
                // Other modify events we ignore
                vec![NormalizedEvent::Ignore]
            }
            EventKind::Any => vec![NormalizedEvent::Ignore],
            EventKind::Access(_) => vec![NormalizedEvent::Ignore],
            EventKind::Other => vec![NormalizedEvent::Ignore],
        }
    }

    fn handle_rename_events(
        &mut self,
        now: Instant,
        paths: Vec<PathBuf>,
        kind: PendingKind,
    ) -> Vec<NormalizedEvent> {
        let mut out = Vec::new();

        for p in paths {
            // Try to find a matching rename event within the time window
            let target_kind = match kind {
                PendingKind::RenameFrom => PendingKind::RenameTo,
                PendingKind::RenameTo => PendingKind::RenameFrom,
                _ => unreachable!(),
            };

            if let Some((idx, matched)) = self.find_rename_match(now, target_kind) {
                // Build rename event: compose from/to in correct order
                let (from, to) = match kind {
                    PendingKind::RenameTo => (matched.path.clone(), p.clone()),
                    PendingKind::RenameFrom => (p.clone(), matched.path.clone()),
                    _ => unreachable!(),
                };
                // Remove the matched pending
                self.pending.remove(idx);

                // Check if this is a directory rename/move
                if from.is_dir() || to.is_dir() {
                    out.push(NormalizedEvent::DirMove { from, to });
                } else {
                    let base_from = from.file_name().unwrap_or_default().to_os_string();
                    let base_to = to.file_name().unwrap_or_default().to_os_string();

                    if base_from == base_to {
                        out.push(NormalizedEvent::BlendMove { from, to });
                    } else {
                        out.push(NormalizedEvent::BlendRename {
                            from,
                            to,
                            base_from,
                            base_to,
                        });
                    }
                }
            } else {
                // No match; enqueue as pending
                self.pending.push_back(Pending {
                    when: now,
                    path: p,
                    kind,
                });
            }
        }

        out
    }

    fn handle_singletons(
        &mut self,
        now: Instant,
        paths: Vec<PathBuf>,
        kind: PendingKind,
    ) -> Vec<NormalizedEvent> {
        let mut out = Vec::new();

        for p in paths {
            // Directories: we do not attempt to pair singletons for them here.
            if p.is_dir() {
                // Directory create/remove without explicit rename is too ambiguous; ignore here.
                continue;
            }

            // Try to match with an opposite kind by same base filename within the window.
            let base = match p.file_name() {
                Some(b) => b.to_os_string(),
                None => {
                    self.pending.push_back(Pending {
                        when: now,
                        path: p,
                        kind,
                    });
                    continue;
                }
            };

            if let Some((idx, matched)) = self.find_match(now, &base, kind) {
                // Build event: same base implies move (directory changed). Compose from/to in correct order.
                let (from, to) = match kind {
                    PendingKind::Create => (matched.path.clone(), p.clone()),
                    PendingKind::Delete => (p.clone(), matched.path.clone()),
                    // Rename events should not reach this point
                    PendingKind::RenameFrom | PendingKind::RenameTo => unreachable!(),
                };
                // Remove the matched pending
                self.pending.remove(idx);

                // Determine if base actually changed (shouldn't, since we matched by base, but be safe)
                let base_from = from.file_name().unwrap_or_default().to_os_string();
                let base_to = to.file_name().unwrap_or_default().to_os_string();

                if base_from == base_to {
                    out.push(NormalizedEvent::BlendMove { from, to });
                } else {
                    out.push(NormalizedEvent::BlendRename {
                        from,
                        to,
                        base_from,
                        base_to,
                    });
                }
            } else {
                // No match; enqueue as pending and may be paired by future counterpart.
                self.pending.push_back(Pending {
                    when: now,
                    path: p,
                    kind,
                });
            }
        }

        out
    }

    fn find_rename_match(
        &self,
        now: Instant,
        target_kind: PendingKind,
    ) -> Option<(usize, Pending)> {
        // Search from the back (recent) for the most recent matching rename event within window
        for (idx, pending) in self.pending.iter().enumerate().rev() {
            if pending.kind != target_kind {
                continue;
            }
            if now.duration_since(pending.when) > self.window {
                // Too old
                continue;
            }
            return Some((idx, pending.clone()));
        }
        None
    }

    fn find_match(
        &self,
        now: Instant,
        base: &OsString,
        incoming_kind: PendingKind,
    ) -> Option<(usize, Pending)> {
        let target_kind = match incoming_kind {
            PendingKind::Create => PendingKind::Delete,
            PendingKind::Delete => PendingKind::Create,
            // Rename events should use find_rename_match instead
            PendingKind::RenameFrom | PendingKind::RenameTo => return None,
        };

        // Search from the back (recent) for best candidate by same base, within window.
        for (idx, pending) in self.pending.iter().enumerate().rev() {
            if pending.kind != target_kind {
                continue;
            }
            if now.duration_since(pending.when) > self.window {
                // Too old
                continue;
            }
            let pbase = pending.path.file_name().unwrap_or_default().to_os_string();
            if eq_base_cross_platform(&pbase, base) {
                return Some((idx, pending.clone()));
            }
        }
        None
    }

    fn gc(&mut self, now: Instant) {
        while let Some(front) = self.pending.front() {
            if now.duration_since(front.when) > self.window {
                self.pending.pop_front();
            } else {
                break;
            }
        }
    }
}

#[inline]
fn eq_base_cross_platform(a: &OsString, b: &OsString) -> bool {
    // Windows is case-insensitive; Unix is case-sensitive. We choose a conservative cross-platform approach:
    // Case-insensitive compare on Windows; exact on others.
    #[cfg(windows)]
    {
        // Convert to lowercase String lossily
        let al = a.to_string_lossy().to_lowercase();
        let bl = b.to_string_lossy().to_lowercase();
        al == bl
    }
    #[cfg(not(windows))]
    {
        a == b
    }
}
