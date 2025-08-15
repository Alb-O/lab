//! Extra integration tests for BlendPath, make_absolute, and strip_root
use dot001_tracer::bpath::{BlendPath, strip_root};
use std::path::{Path, PathBuf};

#[test]
fn test_string_path() {
    let p = BlendPath::new(b"//some/file.blend");
    assert_eq!(b"//some/file.blend", p.as_bytes());
    let p = BlendPath::new(b"C:/some/file.blend");
    assert_eq!(b"C:/some/file.blend", p.as_bytes());
}

#[test]
fn test_debug_repr() {
    let p = BlendPath::new(b"//some/file.blend");
    assert_eq!(format!("{p:?}"), "BlendPath(\"//some/file.blend\")");
}

#[test]
fn test_to_path() {
    let p = BlendPath::new(b"/some/file.blend");
    assert_eq!(p.to_pathbuf(), Path::new("/some/file.blend"));
    let p = BlendPath::new(b"C:/some/file.blend");
    assert_eq!(p.to_pathbuf(), Path::new("C:/some/file.blend"));
    let p = BlendPath::new(b"C:\\some\\file.blend");
    assert_eq!(p.to_pathbuf(), Path::new("C:/some/file.blend"));
}

#[test]
fn test_is_absolute() {
    assert!(!BlendPath::new(b"//some/file.blend").is_absolute());
    assert!(BlendPath::new(b"/some/file.blend").is_absolute());
    assert!(BlendPath::new(b"C:/some/file.blend").is_absolute());
    assert!(BlendPath::new(b"C:\\some\\file.blend").is_absolute());
    assert!(!BlendPath::new(b"some/file.blend").is_absolute());
}

#[test]
fn test_is_blendfile_relative() {
    assert!(BlendPath::new(b"//some/file.blend").is_blendfile_relative());
    assert!(!BlendPath::new(b"/some/file.blend").is_blendfile_relative());
    assert!(!BlendPath::new(b"C:/some/file.blend").is_blendfile_relative());
    assert!(!BlendPath::new(b"some/file.blend").is_blendfile_relative());
}

#[test]
fn test_make_absolute() {
    let root = Path::new("/root/to");
    let bp = BlendPath::new(b"//some/file.blend");
    assert_eq!(
        bp.absolute(Some(root)).as_bytes(),
        b"/root/to/some/file.blend"
    );
    let bp = BlendPath::new(b"some/file.blend");
    assert_eq!(
        bp.absolute(Some(root)).as_bytes(),
        b"/root/to/some/file.blend"
    );
    let bp = BlendPath::new(b"../some/file.blend");
    assert_eq!(
        bp.absolute(Some(root)).as_bytes(),
        b"/root/to/../some/file.blend"
    );
    let bp = BlendPath::new(b"/shared/some/file.blend");
    assert_eq!(
        bp.absolute(Some(root)).as_bytes(),
        b"/shared/some/file.blend"
    );
}

#[test]
fn test_slash() {
    let a = BlendPath::new(b"/root/and");
    let b = BlendPath::new(b"parent.blend");
    assert_eq!((&a / b.as_bytes()).as_bytes(), b"/root/and/parent.blend");
}

#[test]
#[should_panic]
fn test_slash_absolute_rhs() {
    let a = BlendPath::new(b"/root/and");
    let b = BlendPath::new(b"/parent.blend");
    let _ = &a / b.as_bytes();
}

#[test]
#[cfg(windows)]
fn test_strip_root_windows() {
    let p = Path::new("C:/Program Files/Blender");
    let stripped = strip_root(p);
    assert_eq!(stripped, PathBuf::from("C/Program Files/Blender"));
    let p = Path::new("C:\\Program Files\\Blender");
    let stripped = strip_root(p);
    assert_eq!(stripped, PathBuf::from("C/Program Files/Blender"));
}

#[test]
#[cfg(unix)]
fn test_strip_root_unix() {
    // On Unix, test stripping the root directory "/"
    let p = Path::new("/usr/share/blender");
    let stripped = strip_root(p);
    assert_eq!(stripped, PathBuf::from("usr/share/blender"));
}

#[test]
fn test_strip_root_posix() {
    let p = Path::new("/C/path/to/blender");
    let stripped = strip_root(p);
    assert_eq!(stripped, PathBuf::from("C/path/to/blender"));
    let p = Path::new("C/path/to/blender");
    let stripped = strip_root(p);
    assert_eq!(stripped, PathBuf::from("C/path/to/blender"));
}
