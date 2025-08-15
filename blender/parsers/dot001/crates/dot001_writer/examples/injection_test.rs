use dot001_events::error::Result;
use dot001_writer::{BlendWriter, BlockInjection, SeedDnaProvider, WriteTemplate};

fn main() -> Result<()> {
    // Load seed file
    let mut seed = SeedDnaProvider::from_seed_path("seed_5.0.blend")?;

    // Extract collection, scene, and their dependencies
    // Collection (1144) + Scene (760) + World (1206) + Object/Data dependencies
    let collection_indices = vec![760, 1206, 1144, 1150, 1154, 1148, 1173, 1171, 1156, 1223];
    let extracted_blocks = seed.extract_blocks_by_indices(&collection_indices)?;

    println!(
        "Extracted {} blocks for collection injection:",
        extracted_blocks.len()
    );
    for (index, header, data) in &extracted_blocks {
        println!(
            "  [{}] {} {} ({} bytes)",
            index,
            String::from_utf8_lossy(&header.code).trim_end_matches('\0'),
            header.sdna_index,
            data.len()
        );
    }

    // Create injection
    let injection = BlockInjection::from_extracted_blocks(extracted_blocks);

    // Write file with injection
    let writer = BlendWriter::default();
    writer.write_with_seed_and_injection(
        "test_with_collection.blend",
        WriteTemplate::WithInjection,
        &seed,
        Some(&injection),
    )?;

    println!("Created test_with_collection.blend with injected collection");
    Ok(())
}
