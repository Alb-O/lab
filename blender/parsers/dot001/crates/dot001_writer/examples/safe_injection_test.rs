use dot001_events::error::Result;
/// Safe block injection with pointer sanitization
///
/// This example demonstrates block injection that sanitizes dangerous pointers
/// to reduce crashes, though functionality may be limited.
///
/// Expected results:
/// - Material blocks: Basic functionality, may have empty NodeTrees
/// - Object/Mesh blocks: Frequently crash despite sanitization
/// - Collection blocks: Limited testing
///
/// This is experimental software and not suitable for production use.
use dot001_writer::{BlendWriter, SafeBlockInjection, SeedDnaProvider, WriteTemplate};

fn main() -> Result<()> {
    println!("Safe block injection test (experimental)");
    println!("Warning: May still crash with complex structures\n");

    // Load seed file and get DNA
    let mut seed = SeedDnaProvider::from_seed_path("seed_5.0.blend")?;

    // Test 1: Material with NodeTree using safe injection
    println!("=== Test 1: Material + NodeTree with Safe Injection ===");
    let material_nodetree_indices = vec![1223, 1225]; // Material + its NodeTree

    let injection = SafeBlockInjection::from_block_indices_with_safe_handling(
        &mut seed,
        &material_nodetree_indices,
    )?;

    let writer = BlendWriter::default();
    writer.write_with_seed_and_injection(
        "test_safe_material_nodetree.blend",
        WriteTemplate::WithInjection,
        &seed,
        Some(&injection),
    )?;

    println!("Created test_safe_material_nodetree.blend");

    // Test 2: Try a simple Object block with safe injection
    println!("\n=== Test 2: Object with Safe Injection ===");
    let object_indices = vec![1150]; // Just the Object block

    let object_injection =
        SafeBlockInjection::from_block_indices_with_safe_handling(&mut seed, &object_indices)?;

    writer.write_with_seed_and_injection(
        "test_safe_object.blend",
        WriteTemplate::WithInjection,
        &seed,
        Some(&object_injection),
    )?;

    println!("Created test_safe_object.blend");

    // Test 3: Object + Mesh with safe injection
    println!("\n=== Test 3: Object + Mesh with Safe Injection ===");
    let object_mesh_indices = vec![1150, 1173]; // Object + Mesh

    let object_mesh_injection =
        SafeBlockInjection::from_block_indices_with_safe_handling(&mut seed, &object_mesh_indices)?;

    writer.write_with_seed_and_injection(
        "test_safe_object_mesh.blend",
        WriteTemplate::WithInjection,
        &seed,
        Some(&object_mesh_injection),
    )?;

    println!("Created test_safe_object_mesh.blend");

    println!("\n=== Testing Files with Blender ===");

    // Test each file
    test_file_with_blender(
        "test_safe_material_nodetree.blend",
        "Material + NodeTree (safe)",
    )?;
    test_file_with_blender("test_safe_object.blend", "Object only (safe)")?;
    test_file_with_blender("test_safe_object_mesh.blend", "Object + Mesh (safe)")?;

    // Compare with known working simple injection
    println!("\n=== Baseline Comparison ===");
    test_file_with_blender(
        "test_material_injection.blend",
        "Simple Material (baseline)",
    )?;

    Ok(())
}

fn test_file_with_blender(filename: &str, description: &str) -> Result<()> {
    use std::process::Command;

    let result = Command::new("A:\\bin\\blender-5.0.0-alpha\\blender.exe")
        .args([
            filename,
            "--background", 
            "--python-exit-code", "1", 
            "--python-expr",
            &format!("import bpy; print('SUCCESS: {description} loaded'); print('Objects:', len(bpy.data.objects)); print('Materials:', len(bpy.data.materials)); print('Meshes:', len(bpy.data.meshes)); print('NodeTrees:', len([nt for nt in bpy.data.node_groups if nt]))")
        ])
        .current_dir("A:\\repos\\dot001")
        .output();

    match result {
        Ok(output) => {
            if output.status.success() {
                println!("✅ {filename} - {description}");
                // Show the success line
                let stdout = String::from_utf8_lossy(&output.stdout);
                for line in stdout.lines() {
                    if line.contains("SUCCESS:")
                        || line.contains("Objects:")
                        || line.contains("Materials:")
                        || line.contains("NodeTrees:")
                    {
                        println!("  {line}");
                    }
                }
            } else {
                println!("❌ {filename} - Failed");
                let stderr = String::from_utf8_lossy(&output.stderr);
                if stderr.contains("EXCEPTION_ACCESS_VIOLATION") {
                    println!("  CRASH: Access violation");
                } else if !stderr.trim().is_empty() {
                    println!(
                        "  Error: {}",
                        stderr.lines().next().unwrap_or("Unknown error")
                    );
                }
            }
        }
        Err(e) => {
            println!("❌ {filename} - Test execution failed: {e}");
        }
    }

    Ok(())
}
