//! Integration tests for BlendPath and Blender path utilities

use dot001_tracer::bpath::BlendPath;
use std::path::Path;

#[test]
fn test_blendfile_relative() {
    let bp = BlendPath::new(b"//textures/tex.png");
    assert!(bp.is_blendfile_relative());
    assert!(!bp.is_absolute());
}

#[test]
fn test_absolute_posix() {
    let bp = BlendPath::new(b"/usr/share/blender/asset.png");
    assert!(bp.is_absolute());
    assert!(!bp.is_blendfile_relative());
}

#[test]
fn test_absolute_windows() {
    let bp = BlendPath::new(b"C:/blender/asset.png");
    assert!(bp.is_absolute());
    assert!(!bp.is_blendfile_relative());
}

#[test]
fn test_to_pathbuf() {
    let bp = BlendPath::new(b"/tmp/foo/bar.png");
    let pb = bp.to_pathbuf();
    assert_eq!(pb, Path::new("/tmp/foo/bar.png"));
}

#[test]
fn test_absolute_conversion() {
    let rel = BlendPath::new(b"//textures/tex.png");
    let abs = rel.absolute(Some(Path::new("/blend/project.blend")));
    assert!(abs.is_absolute());
}

#[test]
fn test_mkrelative_same_drive() {
    // Use valid absolute paths for this system
    #[cfg(windows)]
    let (asset, blend) = (
        Path::new("A:/blend/assets/tex.png"),
        Path::new("A:/blend/project.blend"),
    );
    #[cfg(unix)]
    let (asset, blend) = (
        Path::new("/home/user/blend/assets/tex.png"),
        Path::new("/home/user/blend/project.blend"),
    );

    let rel = BlendPath::mkrelative(asset, blend);
    assert!(rel.is_blendfile_relative());
}

#[test]
#[cfg(windows)]
fn test_mkrelative_different_drive() {
    // Test different drives (Windows only - Unix doesn't have drive letters)
    let asset = Path::new("D:/assets/tex.png");
    let blend = Path::new("C:/blend/project.blend");

    let rel = BlendPath::mkrelative(asset, blend);
    assert!(!rel.is_blendfile_relative());
    assert!(rel.is_absolute());
}

#[test]
#[cfg(unix)]
fn test_mkrelative_unix_absolute() {
    // On Unix, test that distant absolute paths still work correctly
    let asset = Path::new("/usr/share/assets/tex.png");
    let blend = Path::new("/home/user/blend/project.blend");

    let rel = BlendPath::mkrelative(asset, blend);
    // On Unix, these will be made relative since they share the same root "/"
    // This is the expected behavior on Unix systems
    assert!(rel.is_blendfile_relative());
}
