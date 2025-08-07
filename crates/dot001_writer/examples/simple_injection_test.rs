use dot001_events::error::Result;
use dot001_writer::{BlendWriter, BlockInjection, SeedDnaProvider, WriteTemplate};

fn main() -> Result<()> {
    // Load seed file
    let mut seed = SeedDnaProvider::from_seed_path("seed_5.0.blend")?;

    // Extract just a simple material block (less likely to have complex pointer dependencies)
    let material_indices = vec![1223]; // Just the material block
    let extracted_blocks = seed.extract_blocks_by_indices(&material_indices)?;

    println!(
        "Extracted {} blocks for simple injection:",
        extracted_blocks.len()
    );
    for (index, header, data) in &extracted_blocks {
        println!(
            "  [{}] {} {} ({} bytes) old_addr: 0x{:x}",
            index,
            String::from_utf8_lossy(&header.code).trim_end_matches('\0'),
            header.sdna_index,
            data.len(),
            header.old_address
        );
    }

    // Create injection with address remapping
    let injection = BlockInjection::from_extracted_blocks(extracted_blocks);

    println!("Address mapping:");
    for (old_addr, new_addr) in &injection.address_map {
        println!("  0x{old_addr:x} -> 0x{new_addr:x}");
    }

    // Write file with injection
    let writer = BlendWriter::default();
    writer.write_with_seed_and_injection(
        "test_simple_injection.blend",
        WriteTemplate::WithInjection,
        &seed,
        Some(&injection),
    )?;

    println!("Created test_simple_injection.blend with address-remapped material");
    Ok(())
}
