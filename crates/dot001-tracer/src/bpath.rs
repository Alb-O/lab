//! Blender path support for dot001
//! Handles blendfile-relative, POSIX, and Windows paths as bytes.

use std::fmt;
use std::ops::Div;
use std::path::{Path, PathBuf};

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BlendPath(Vec<u8>);

impl BlendPath {
    pub fn new<P: AsRef<[u8]>>(path: P) -> Self {
        let mut bytes = path.as_ref().to_vec();
        // Normalize: always use forward slashes
        for b in &mut bytes {
            if *b == b'\\' {
                *b = b'/';
            }
        }
        BlendPath(bytes)
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    pub fn is_blendfile_relative(&self) -> bool {
        self.0.starts_with(b"//")
    }

    pub fn is_absolute(&self) -> bool {
        if self.is_blendfile_relative() {
            return false;
        }
        if self.0.starts_with(b"/") {
            return true;
        }
        // Windows drive letter: C:/ or C:\
        if self.0.len() >= 3
            && self.0[0].is_ascii_alphabetic()
            && self.0[1] == b':'
            && (self.0[2] == b'/' || self.0[2] == b'\\')
        {
            return true;
        }
        false
    }

    pub fn to_pathbuf(&self) -> PathBuf {
        // Only valid for non-blendfile-relative paths
        assert!(
            !self.is_blendfile_relative(),
            "Cannot convert blendfile-relative path to PathBuf"
        );
        match std::str::from_utf8(&self.0) {
            Ok(s) => PathBuf::from(s),
            Err(_) => PathBuf::from(String::from_utf8_lossy(&self.0).into_owned()),
        }
    }

    pub fn absolute(&self, root: Option<&Path>) -> BlendPath {
        if self.is_absolute() {
            return self.clone();
        }
        let rel = if self.is_blendfile_relative() {
            &self.0[2..]
        } else {
            &self.0[..]
        };
        let rel_str = String::from_utf8_lossy(rel);
        let joined = match root {
            Some(r) => r.join(rel_str.as_ref()),
            None => std::env::current_dir().unwrap().join(rel_str.as_ref()),
        };
        BlendPath::new(joined.to_string_lossy().as_bytes())
    }

    pub fn mkrelative(asset_path: &Path, bfile_path: &Path) -> BlendPath {
        assert!(bfile_path.is_absolute());
        assert!(asset_path.is_absolute());
        if bfile_path.components().next().map(|c| c.as_os_str())
            != asset_path.components().next().map(|c| c.as_os_str())
        {
            // Different roots/drives
            return BlendPath::new(asset_path.to_string_lossy().as_bytes());
        }
        let bdir = bfile_path.parent().unwrap();
        let rel =
            pathdiff::diff_paths(asset_path, bdir).unwrap_or_else(|| asset_path.to_path_buf());
        let rel_bytes = rel.to_string_lossy().as_bytes().to_vec();
        let up = bdir.ancestors().count() - 1;
        let mut prefix = b"//".to_vec();
        for _ in 0..up {
            prefix.extend_from_slice(b"../");
        }
        prefix.extend_from_slice(&rel_bytes);
        BlendPath(prefix)
    }
}

impl fmt::Display for BlendPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", String::from_utf8_lossy(&self.0))
    }
}

impl fmt::Debug for BlendPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "BlendPath({:?})", String::from_utf8_lossy(&self.0))
    }
}

impl Div<&[u8]> for &BlendPath {
    type Output = BlendPath;
    fn div(self, rhs: &[u8]) -> BlendPath {
        let sub = BlendPath::new(rhs);
        if sub.is_absolute() {
            panic!("'a / b' only works when 'b' is a relative path");
        }
        let mut base = self.0.clone();
        while base.ends_with(b"/") {
            base.pop();
        }
        let mut out = base;
        out.push(b'/');
        out.extend_from_slice(&sub.0);
        BlendPath(out)
    }
}

// Utility: make_absolute and strip_root
use std::ffi::OsStr;

pub fn make_absolute(path: &Path) -> PathBuf {
    if path.is_absolute() {
        return path.to_path_buf();
    }
    let cwd = std::env::current_dir().unwrap();
    cwd.join(path)
}

pub fn strip_root(path: &Path) -> PathBuf {
    let comps = path.components();
    let mut parts = vec![];
    for c in comps {
        match c {
            std::path::Component::Prefix(prefix) => {
                // Windows drive letter
                if let Some(os) = prefix.as_os_str().to_str() {
                    if os.len() >= 2 && os.chars().nth(1) == Some(':') {
                        parts.push(OsStr::new(&os[0..1]).to_os_string());
                    }
                }
            }
            std::path::Component::RootDir => {}
            std::path::Component::Normal(p) => parts.push(p.to_os_string()),
            _ => {}
        }
    }
    let mut rel = PathBuf::new();
    for p in parts {
        rel.push(p);
    }
    rel
}
