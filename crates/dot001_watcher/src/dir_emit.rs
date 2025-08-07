use std::fs;
use std::path::{Path, PathBuf};

use crate::PathPair;

/// Recursively enumerate .blend files under a directory, returning paths relative to the directory root.
fn collect_blend_rel_paths(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    if !root.exists() {
        return out;
    }
    let mut stack = vec![PathBuf::from(root)];

    while let Some(dir) = stack.pop() {
        let read_dir = match fs::read_dir(&dir) {
            Ok(rd) => rd,
            Err(_) => continue,
        };
        for entry in read_dir.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if is_blend(&path) {
                if let Ok(rel) = path.strip_prefix(root) {
                    out.push(rel.to_path_buf());
                }
            }
        }
    }
    out
}

/// Emits synthetic .blend child move events when a directory is moved/renamed from `from_dir` to `to_dir`.
/// We assume relative structure is preserved; for each .blend file present in either side, we map by relative path.
/// - If rel exists on both sides, emit from=from_dir/rel, to=to_dir/rel
/// - If exists only on one side, still emit a best-effort pair using the same relative path (consumer can decide to ignore)
pub fn emit_dir_child_moves(from_dir: &Path, to_dir: &Path) -> Vec<PathPair> {
    let from_list = collect_blend_rel_paths(from_dir);
    let to_list = collect_blend_rel_paths(to_dir);

    // Use sets of rel paths to deduplicate
    use std::collections::BTreeSet;
    let from_set: BTreeSet<PathBuf> = from_list.into_iter().collect();
    let to_set: BTreeSet<PathBuf> = to_list.into_iter().collect();

    let rel_union: BTreeSet<PathBuf> = from_set.union(&to_set).cloned().collect();

    let mut out = Vec::new();
    for rel in rel_union {
        let from_abs = from_dir.join(&rel);
        let to_abs = to_dir.join(&rel);

        // Only emit if at least one side exists now or likely existed before.
        if from_abs.exists() || to_abs.exists() {
            let base = to_abs.file_name().unwrap_or_default().to_os_string();
            out.push(PathPair {
                from: from_abs,
                to: to_abs,
                base,
            });
        }
    }
    out
}

#[inline]
fn is_blend(path: &Path) -> bool {
    match path.extension().and_then(|e| e.to_str()) {
        Some(ext) => ext.eq_ignore_ascii_case("blend"),
        None => false,
    }
}
