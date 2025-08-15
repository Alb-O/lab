use dot001_events::error::Result;
/// Experimental exhaustive block injection
///
/// This attempts to trace complete pointer dependency trees and reconstruct
/// linked list structures. It is highly unstable and frequently crashes.
///
/// This represents research into Blender's internal data structures but
/// is not functional software.
use dot001_writer::{BlendWriter, ExhaustivePointerTracer, SeedDnaProvider, WriteTemplate};

fn main() -> Result<()> {
    println!("Experimental exhaustive block injection");
    println!("Warning: This frequently crashes and is for research purposes only\n");

    // Load seed file and get DNA
    let mut seed = SeedDnaProvider::from_seed_path("seed_5.0.blend")?;

    // Test 1: Exhaustive tracing for a single Material
    println!("████████████████████████████████████████████████████████████████");
    println!("█ Test 1: Exhaustive Dependency Tracing for Material Block 1223 █");
    println!("████████████████████████████████████████████████████████████████");

    let material_injection = ExhaustivePointerTracer::trace_complete_dependencies(
        &mut seed,
        &[1223], // Single Material block
    )?;

    println!(
        "\n=== Creating Injection with {} blocks ===",
        material_injection.blocks.len()
    );
    println!("Address mappings:");
    for (old_addr, new_addr) in &material_injection.address_map {
        println!("  0x{old_addr:x} → 0x{new_addr:x}");
    }

    let writer = BlendWriter::default();
    writer.write_with_seed_and_injection(
        "test_exhaustive_material.blend",
        WriteTemplate::WithInjection,
        &seed,
        Some(&material_injection),
    )?;

    println!("✅ Created test_exhaustive_material.blend");

    // Test 2: Exhaustive tracing for Object + Mesh
    println!("\n████████████████████████████████████████████████████████████████");
    println!("█ Test 2: Exhaustive Dependency Tracing for Object + Mesh      █");
    println!("████████████████████████████████████████████████████████████████");

    let object_mesh_injection = ExhaustivePointerTracer::trace_complete_dependencies(
        &mut seed,
        &[1150, 1173], // Object + Mesh
    )?;

    println!(
        "\n=== Creating Injection with {} blocks ===",
        object_mesh_injection.blocks.len()
    );

    writer.write_with_seed_and_injection(
        "test_exhaustive_object_mesh.blend",
        WriteTemplate::WithInjection,
        &seed,
        Some(&object_mesh_injection),
    )?;

    println!("✅ Created test_exhaustive_object_mesh.blend");

    // Test 3: Just test one more complex case - a Collection
    println!("\n████████████████████████████████████████████████████████████████");
    println!("█ Test 3: Exhaustive Dependency Tracing for Collection 1144    █");
    println!("████████████████████████████████████████████████████████████████");

    let collection_injection = ExhaustivePointerTracer::trace_complete_dependencies(
        &mut seed,
        &[1144], // Collection block
    )?;

    writer.write_with_seed_and_injection(
        "test_exhaustive_collection.blend",
        WriteTemplate::WithInjection,
        &seed,
        Some(&collection_injection),
    )?;

    println!("✅ Created test_exhaustive_collection.blend");

    // Now test all the files
    println!("\n████████████████████████████████████████████████████████████████");
    println!("█ Testing All Files with Blender                               █");
    println!("████████████████████████████████████████████████████████████████");

    test_file_with_blender(
        "test_exhaustive_material.blend",
        "Material with complete dependencies",
    )?;
    test_file_with_blender(
        "test_exhaustive_object_mesh.blend",
        "Object+Mesh with complete dependencies",
    )?;
    test_file_with_blender(
        "test_exhaustive_collection.blend",
        "Collection with complete dependencies",
    )?;

    Ok(())
}

fn test_file_with_blender(filename: &str, description: &str) -> Result<()> {
    use std::process::Command;

    println!("\n--- Testing {filename} ---");

    let result = Command::new("A:\\bin\\blender-5.0.0-alpha\\blender.exe")
        .args([
            filename,
            "--background",
            "--python-exit-code",
            "1",
            "--python-expr",
            &format!(
                r#"
import bpy
print('🔍 ANALYZING: {description}')
print(f'Objects: {{len(bpy.data.objects)}}')
print(f'Materials: {{len(bpy.data.materials)}}')
print(f'Meshes: {{len(bpy.data.meshes)}}')
print(f'Collections: {{len(bpy.data.collections)}}')
print(f'Node Groups: {{len(bpy.data.node_groups)}}')
print('📊 SUMMARY: File loaded successfully with exhaustive dependencies')
"#
            ),
        ])
        .current_dir("A:\\repos\\dot001")
        .output();

    match result {
        Ok(output) => {
            if output.status.success() {
                println!("✅ SUCCESS: {description}");
                let stdout = String::from_utf8_lossy(&output.stdout);
                for line in stdout.lines() {
                    if line.contains("🔍")
                        || line.contains("Objects:")
                        || line.contains("Materials:")
                        || line.contains("📊")
                    {
                        println!("  {line}");
                    }
                }
            } else {
                println!("❌ FAILED: {description}");
                let stderr = String::from_utf8_lossy(&output.stderr);
                if stderr.contains("EXCEPTION_ACCESS_VIOLATION") {
                    println!(
                        "  💥 CRASH: Access violation - exhaustive tracing didn't prevent crash"
                    );
                } else if !stderr.trim().is_empty() {
                    let error_line = stderr.lines().next().unwrap_or("Unknown error");
                    println!("  ⚠️ ERROR: {error_line}");
                }
            }
        }
        Err(e) => {
            println!("❌ EXECUTION FAILED: {e}");
        }
    }

    Ok(())
}
