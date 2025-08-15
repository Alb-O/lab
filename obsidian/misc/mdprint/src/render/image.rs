use std::path::{Path, PathBuf};

pub(super) fn resolve_image_path(raw: &str, file_path: Option<&Path>) -> PathBuf {
    let path = Path::new(raw);
    if path.is_absolute() {
        return path.to_path_buf();
    }
    if path.exists() {
        return path.to_path_buf();
    }
    if let Some(p) = file_path.and_then(|f| f.parent()) {
        let cand = p.join(path);
        if cand.exists() {
            return cand;
        }
    }
    path.to_path_buf()
}
