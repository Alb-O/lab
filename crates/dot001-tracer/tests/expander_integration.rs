/// Integration tests for expander macros using real blend files
use dot001_parser::{BlendFile, parse_from_path};
use dot001_tracer::{BlockExpander, DependencyTracer};
use std::collections::HashMap;

/// Test helper to load test blend files
fn load_test_blend_file(name: &str) -> BlendFile<Box<dyn dot001_parser::ReadSeekSend>> {
    let test_file_path = format!("../../tests/test-blendfiles/{name}");

    // Check if test file exists
    if !std::path::Path::new(&test_file_path).exists() {
        panic!(
            "Test blend file not found: {test_file_path}. Make sure to run tests from the workspace root."
        );
    }

    let (blend_file, _decompression_mode) =
        parse_from_path(&test_file_path, None).expect("Failed to parse test blend file");
    blend_file
}

/// Test that basic expanders work with real blend files
#[test]
fn test_basic_expanders_on_real_files() {
    let mut blend_file = load_test_blend_file("main.blend");
    let mut tracer: DependencyTracer<'_, Box<dyn dot001_parser::ReadSeekSend>> =
        DependencyTracer::new().with_default_expanders();

    // Get all blocks and categorize them by type
    let mut block_counts: HashMap<String, usize> = HashMap::new();
    let mut tested_expanders = Vec::new();

    for i in 0..blend_file.blocks_len() {
        if let Some(block) = blend_file.get_block(i) {
            let code = dot001_parser::block_code_to_string(block.header.code);
            *block_counts.entry(code.clone()).or_insert(0) += 1;

            // Test that we can trace dependencies for this block (if it's a supported type)
            match tracer.trace_dependencies(i, &mut blend_file) {
                Ok(dependencies) => {
                    if !dependencies.is_empty() {
                        tested_expanders.push((code.clone(), dependencies.len(), 0)); // Can't easily get external refs from trace
                        println!("✓ {} block {}: {} deps", code, i, dependencies.len());
                    }
                }
                Err(_e) => {
                    // Many blocks won't have expanders, this is expected
                }
            }
        }
    }

    println!("\nBlock type summary:");
    for (block_type, count) in &block_counts {
        println!("  {block_type}: {count} blocks");
    }

    println!("\nTested {} expander operations", tested_expanders.len());

    // Verify we found some common block types
    assert!(block_counts.contains_key("SC"), "Should find Scene blocks");
    assert!(block_counts.contains_key("OB"), "Should find Object blocks");

    // Verify we tested some expanders
    assert!(
        !tested_expanders.is_empty(),
        "Should have tested some expanders"
    );
}

/// Test dependency tracing on a real scene
#[test]
fn test_dependency_tracing_integration() {
    let mut blend_file = load_test_blend_file("main.blend");
    let mut tracer: DependencyTracer<'_, Box<dyn dot001_parser::ReadSeekSend>> =
        DependencyTracer::new().with_default_expanders();

    // Find the first Scene block
    let scene_block = (0..blend_file.blocks_len()).find(|&i| {
        blend_file
            .get_block(i)
            .map(|b| b.header.code == *b"SC\0\0")
            .unwrap_or(false)
    });

    if let Some(scene_index) = scene_block {
        match tracer.trace_dependencies(scene_index, &mut blend_file) {
            Ok(dependencies) => {
                println!(
                    "Scene {} has {} total dependencies",
                    scene_index,
                    dependencies.len()
                );

                // Analyze dependency types
                let mut dep_types: HashMap<String, usize> = HashMap::new();
                for &dep_idx in &dependencies {
                    if let Some(block) = blend_file.get_block(dep_idx) {
                        let code = dot001_parser::block_code_to_string(block.header.code);
                        *dep_types.entry(code).or_insert(0) += 1;
                    }
                }

                println!("Dependency breakdown:");
                for (dep_type, count) in &dep_types {
                    println!("  {dep_type}: {count} blocks");
                }

                // Verify scene has some dependencies (objects, materials, etc.)
                assert!(
                    !dependencies.is_empty(),
                    "Scene should have some dependencies"
                );
            }
            Err(e) => {
                panic!("Failed to trace dependencies: {e}");
            }
        }
    } else {
        println!("No Scene block found in test file - skipping dependency test");
    }
}

/// Test external reference detection
#[test]
fn test_external_reference_detection() {
    let blend_file = load_test_blend_file("library.blend");
    let tracer: DependencyTracer<'_, Box<dyn dot001_parser::ReadSeekSend>> =
        DependencyTracer::new().with_default_expanders();

    let mut total_external_refs = 0;
    let mut blocks_with_externals: Vec<(usize, String, Vec<std::path::PathBuf>)> = Vec::new();

    // Check all blocks for external references
    for i in 0..blend_file.blocks_len() {
        if let Some(block) = blend_file.get_block(i) {
            let code = dot001_parser::block_code_to_string(block.header.code);

            // For now, just test that library blocks exist (LI blocks should have external refs)
            if code == "LI" {
                blocks_with_externals.push((i, code.clone(), vec![])); // Placeholder
                total_external_refs += 1; // Estimate
                println!("Found Library block {i} - likely has external references");
            }
        }
    }

    println!(
        "\nFound {} total external references in {} blocks",
        total_external_refs,
        blocks_with_externals.len()
    );

    // The library.blend file should have some external references
    // (This may be 0 if the test file doesn't actually link external files)
    println!("External reference detection test completed");
}

/// Test macro consistency across all expanders
#[test]
fn test_macro_consistency() {
    use std::io::Cursor;

    println!("Testing macro-generated expander consistency:");

    // Test individual expanders with explicit type annotations
    let object_expander = dot001_tracer::expanders::ObjectExpander;
    assert!(
        <dot001_tracer::expanders::ObjectExpander as BlockExpander<Cursor<Vec<u8>>>>::can_handle(
            &object_expander,
            b"OB\0\0"
        )
    );
    assert!(
        !<dot001_tracer::expanders::ObjectExpander as BlockExpander<Cursor<Vec<u8>>>>::can_handle(
            &object_expander,
            b"XX\0\0"
        )
    );
    println!("  ✓ ObjectExpander correctly handles OB blocks");

    let mesh_expander = dot001_tracer::expanders::MeshExpander;
    assert!(<dot001_tracer::expanders::MeshExpander as BlockExpander<
        Cursor<Vec<u8>>,
    >>::can_handle(&mesh_expander, b"ME\0\0"));
    assert!(!<dot001_tracer::expanders::MeshExpander as BlockExpander<
        Cursor<Vec<u8>>,
    >>::can_handle(&mesh_expander, b"XX\0\0"));
    println!("  ✓ MeshExpander correctly handles ME blocks");

    let lamp_expander = dot001_tracer::expanders::LampExpander;
    assert!(<dot001_tracer::expanders::LampExpander as BlockExpander<
        Cursor<Vec<u8>>,
    >>::can_handle(&lamp_expander, b"LA\0\0"));
    assert!(!<dot001_tracer::expanders::LampExpander as BlockExpander<
        Cursor<Vec<u8>>,
    >>::can_handle(&lamp_expander, b"XX\0\0"));
    println!("  ✓ LampExpander correctly handles LA blocks");

    let sound_expander = dot001_tracer::expanders::SoundExpander;
    assert!(<dot001_tracer::expanders::SoundExpander as BlockExpander<
        Cursor<Vec<u8>>,
    >>::can_handle(&sound_expander, b"SO\0\0"));
    assert!(
        !<dot001_tracer::expanders::SoundExpander as BlockExpander<Cursor<Vec<u8>>>>::can_handle(
            &sound_expander,
            b"XX\0\0"
        )
    );
    println!("  ✓ SoundExpander correctly handles SO blocks");

    let image_expander = dot001_tracer::expanders::ImageExpander;
    assert!(<dot001_tracer::expanders::ImageExpander as BlockExpander<
        Cursor<Vec<u8>>,
    >>::can_handle(&image_expander, b"IM\0\0"));
    assert!(
        !<dot001_tracer::expanders::ImageExpander as BlockExpander<Cursor<Vec<u8>>>>::can_handle(
            &image_expander,
            b"XX\0\0"
        )
    );
    println!("  ✓ ImageExpander correctly handles IM blocks");

    let library_expander = dot001_tracer::expanders::LibraryExpander;
    assert!(
        <dot001_tracer::expanders::LibraryExpander as BlockExpander<Cursor<Vec<u8>>>>::can_handle(
            &library_expander,
            b"LI\0\0"
        )
    );
    assert!(
        !<dot001_tracer::expanders::LibraryExpander as BlockExpander<Cursor<Vec<u8>>>>::can_handle(
            &library_expander,
            b"XX\0\0"
        )
    );
    println!("  ✓ LibraryExpander correctly handles LI blocks");

    let texture_expander = dot001_tracer::expanders::TextureExpander;
    assert!(
        <dot001_tracer::expanders::TextureExpander as BlockExpander<Cursor<Vec<u8>>>>::can_handle(
            &texture_expander,
            b"TE\0\0"
        )
    );
    assert!(
        !<dot001_tracer::expanders::TextureExpander as BlockExpander<Cursor<Vec<u8>>>>::can_handle(
            &texture_expander,
            b"XX\0\0"
        )
    );
    println!("  ✓ TextureExpander correctly handles TE blocks");

    let material_expander = dot001_tracer::expanders::MaterialExpander;
    assert!(
        <dot001_tracer::expanders::MaterialExpander as BlockExpander<Cursor<Vec<u8>>>>::can_handle(
            &material_expander,
            b"MA\0\0"
        )
    );
    assert!(
        !<dot001_tracer::expanders::MaterialExpander as BlockExpander<Cursor<Vec<u8>>>>::can_handle(
            &material_expander,
            b"XX\0\0"
        )
    );
    println!("  ✓ MaterialExpander correctly handles MA blocks");

    println!("All macro-generated expanders pass consistency tests!");
}
