use dot001_parser::{BlendFile, ParseOptions};
use std::fs::{File, OpenOptions};
use std::io::{BufReader, Seek, SeekFrom, Write};
use std::path::PathBuf;

/// Reconstruct broken library links by updating ID names to match available assets
pub fn cmd_reconstruct_link(
    file_path: PathBuf,
    block_index: usize,
    dry_run: bool,
    target_name: Option<String>,
    _parse_options: &ParseOptions,
    _no_auto_decompress: bool,
) -> anyhow::Result<()> {
    let file = File::open(&file_path)?;
    let reader = BufReader::new(file);
    let mut blend_file = BlendFile::new(reader)?;

    println!("Analyzing library link for block {block_index}...");

    // Validate block exists and read data
    if block_index >= blend_file.blocks.len() {
        return Err(anyhow::anyhow!("Block index {block_index} out of range"));
    }

    let block_data = blend_file.read_block_data(block_index)?;
    let field_reader = blend_file.create_field_reader(&block_data)?;

    // Read current ID information
    let link_info = read_id_link_info(&field_reader)?;

    if link_info.lib_ptr == 0 {
        println!("This is a local asset (lib pointer is NULL), not a library link.");
        return Ok(());
    }

    // Find and read library block
    let lib_block_index = blend_file
        .find_block_by_address(link_info.lib_ptr)
        .ok_or_else(|| anyhow::anyhow!("Lib pointer does not point to a valid block"))?;

    let lib_data = blend_file.read_block_data(lib_block_index)?;
    let lib_reader = blend_file.create_field_reader(&lib_data)?;
    let lib_filepath = lib_reader
        .read_field_string("Library", "name")
        .map_err(|e| anyhow::anyhow!("Error reading library name field: {e}"))?;

    println!("Current link status:");
    println!("  ID Name: '{}'", link_info.name);
    println!("  Library Path: '{lib_filepath}'");
    println!("  Library Block Index: {lib_block_index}");

    if let Some(target) = &target_name {
        println!("  Target Name: '{target}'");
    }

    // Resolve library file path
    let lib_file_path = resolve_library_path(&lib_filepath, &file_path)?;
    println!("  Resolved Library Path: '{}'", lib_file_path.display());

    if !lib_file_path.exists() {
        return Err(anyhow::anyhow!(
            "Library file does not exist at resolved path"
        ));
    }

    // Analyze library file for available assets
    let available_collections = find_collections_in_library(&lib_file_path)?;

    println!("\nAvailable collections in library:");
    for (idx, name) in &available_collections {
        println!("  Block {idx}: '{name}'");
    }

    // Determine target collection
    let target_collection = determine_target_collection(&available_collections, target_name)?;
    println!("\nReconstructing pointer to: '{target_collection}'");

    if dry_run {
        println!(
            "[DRY RUN] Would update ID name from '{}' to '{target_collection}'",
            link_info.name
        );
        return Ok(());
    }

    // Perform the reconstruction
    reconstruct_id_name(&file_path, &mut blend_file, block_index, &target_collection)?;

    println!("SUCCESS: Pointer reconstructed! ID name updated to '{target_collection}'");
    println!("The library link should now resolve correctly.");

    Ok(())
}

/// Information about an ID block's library link
struct IdLinkInfo {
    name: String,
    lib_ptr: u64,
}

/// Read ID block information for library linking
fn read_id_link_info(field_reader: &dot001_parser::FieldReader) -> anyhow::Result<IdLinkInfo> {
    let full_name = field_reader
        .read_field_string("ID", "name")
        .map_err(|e| anyhow::anyhow!("Error reading ID name: {e}"))?;

    // ID names include the 2-byte type prefix, skip it
    let name = full_name.chars().skip(2).collect::<String>();

    let lib_ptr = field_reader
        .read_field_pointer("ID", "lib")
        .map_err(|e| anyhow::anyhow!("Error reading lib pointer: {e}"))?;

    Ok(IdLinkInfo { name, lib_ptr })
}

/// Resolve library file path from library path string and main file location
fn resolve_library_path(
    lib_filepath: &str,
    main_file_path: &std::path::Path,
) -> anyhow::Result<PathBuf> {
    if lib_filepath.starts_with("//") {
        // Blendfile-relative path
        let rel_path = lib_filepath.strip_prefix("//").unwrap();
        let main_file_dir = main_file_path
            .parent()
            .ok_or_else(|| anyhow::anyhow!("Main file has no parent directory"))?;
        Ok(main_file_dir.join(rel_path))
    } else {
        Ok(PathBuf::from(lib_filepath))
    }
}

/// Find all collections in the library file
fn find_collections_in_library(lib_file_path: &PathBuf) -> anyhow::Result<Vec<(usize, String)>> {
    let lib_file = File::open(lib_file_path)?;
    let mut lib_reader = BufReader::new(lib_file);
    let mut lib_blend_file = BlendFile::new(&mut lib_reader)?;

    let mut available_collections = Vec::new();

    // Find all GR (Collection) blocks
    let collection_indices: Vec<usize> = lib_blend_file
        .blocks
        .iter()
        .enumerate()
        .filter_map(|(index, block)| {
            let block_code = String::from_utf8_lossy(&block.header.code);
            if block_code.trim_end_matches('\0') == "GR" {
                Some(index)
            } else {
                None
            }
        })
        .collect();

    // Read collection names
    for index in collection_indices {
        if let Ok(lib_block_data) = lib_blend_file.read_block_data(index) {
            if let Ok(lib_field_reader) = lib_blend_file.create_field_reader(&lib_block_data) {
                if let Ok(name) = lib_field_reader.read_field_string("ID", "name") {
                    let clean_name = name.chars().skip(2).collect::<String>();
                    available_collections.push((index, clean_name));
                }
            }
        }
    }

    Ok(available_collections)
}

/// Determine which collection to target for reconstruction
fn determine_target_collection(
    available_collections: &[(usize, String)],
    target_name: Option<String>,
) -> anyhow::Result<String> {
    match target_name {
        Some(target) => {
            if available_collections
                .iter()
                .any(|(_, name)| name == &target)
            {
                Ok(target)
            } else {
                Err(anyhow::anyhow!(
                    "Target collection '{target}' not found in library"
                ))
            }
        }
        None => {
            if available_collections.is_empty() {
                Err(anyhow::anyhow!("No collections found in library"))
            } else if available_collections.len() == 1 {
                Ok(available_collections[0].1.clone())
            } else {
                Err(anyhow::anyhow!(
                    "Multiple collections available ({}), specify --target-name",
                    available_collections.len()
                ))
            }
        }
    }
}

/// Perform the actual ID name reconstruction
fn reconstruct_id_name(
    file_path: &PathBuf,
    blend_file: &mut BlendFile<BufReader<File>>,
    block_index: usize,
    target_name: &str,
) -> anyhow::Result<()> {
    println!("Performing pointer reconstruction...");

    // Read block data and get field offset
    let mut block_data = blend_file.read_block_data(block_index)?;
    let field_reader = blend_file.create_field_reader(&block_data)?;

    let name_offset = field_reader
        .get_field_offset("ID", "name")
        .map_err(|e| anyhow::anyhow!("Could not find name field in ID block: {e}"))?;

    // Construct new ID name with type prefix (GR for collections)
    let new_id_name = format!("GR{target_name}");
    let mut name_bytes = vec![0u8; 258]; // MAX_ID_NAME from DNA_ID.h
    let copy_len = std::cmp::min(new_id_name.len(), 257); // Leave room for null terminator
    name_bytes[..copy_len].copy_from_slice(new_id_name.as_bytes());

    // Update the name field in block data
    let name_end = name_offset + 258;
    if name_end > block_data.len() {
        return Err(anyhow::anyhow!("Name field extends beyond block data"));
    }
    block_data[name_offset..name_end].copy_from_slice(&name_bytes);

    // Write back to file
    let block = &blend_file.blocks[block_index];
    let block_data_offset = block.data_offset;

    let mut output_file = OpenOptions::new().read(true).write(true).open(file_path)?;
    output_file.seek(SeekFrom::Start(block_data_offset))?;
    output_file.write_all(&block_data)?;
    output_file.flush()?;

    Ok(())
}
