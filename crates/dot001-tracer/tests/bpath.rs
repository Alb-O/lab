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
    // Use valid absolute paths for this system (A: drive)
    let asset = Path::new("A:/blend/assets/tex.png");
    let blend = Path::new("A:/blend/project.blend");
    let rel = BlendPath::mkrelative(asset, blend);
    assert!(rel.is_blendfile_relative());
}

#[test]
fn test_mkrelative_different_drive() {
    let asset = Path::new("D:/assets/tex.png");
    let blend = Path::new("C:/blend/project.blend");
    let rel = BlendPath::mkrelative(asset, blend);
    assert!(!rel.is_blendfile_relative());
    assert!(rel.is_absolute());
}
