use crate::util::OutputHandler;
use crate::{execution_failed_error, invalid_arguments_error, missing_argument_error};
use dot001_error::Dot001Error;
use dot001_parser::{BlendFile, ParseOptions};
use std::fs::{File, OpenOptions};
use std::io::{BufReader, Seek, SeekFrom, Write};
use std::path::PathBuf;

/// Reconstruct broken library links by updating ID names to match available assets
pub fn cmd_reconstruct_link(
    file_path: PathBuf,
    block_identifier: &str,
    dry_run: bool,
    target_name: Option<String>,
    parse_options: &ParseOptions,
    no_auto_decompress: bool,
    output: &OutputHandler,
) -> Result<(), Dot001Error> {
    let mut blend_file =
        crate::util::load_blend_file(&file_path, parse_options, no_auto_decompress)?;

    // Resolve the block identifier to a specific block index
    let Some(block_index) = crate::util::resolve_block_or_exit(block_identifier, &mut blend_file)
    else {
        return Ok(());
    };

    output.print_info_fmt(format_args!(
        "Analyzing library link for block {block_index}..."
    ));

    let block_data = blend_file.read_block_data(block_index)?;
    let field_reader = blend_file.create_field_reader(&block_data)?;

    // Read current ID information
    let link_info = read_id_link_info(&field_reader)?;

    if link_info.lib_ptr == 0 {
        output.print_result("This is a local asset (lib pointer is NULL), not a library link.");
        return Ok(());
    }

    // Find and read library block
    let lib_block_index = blend_file
        .find_block_by_address(link_info.lib_ptr)
        .ok_or_else(|| execution_failed_error("Lib pointer does not point to a valid block"))?;

    let lib_data = blend_file.read_block_data(lib_block_index)?;
    let lib_reader = blend_file.create_field_reader(&lib_data)?;
    let lib_filepath = lib_reader
        .read_field_string("Library", "name")
        .map_err(|e| execution_failed_error(format!("Error reading library name field: {e}")))?;

    output.print_info("Current link status:");
    output.print_result_fmt(format_args!("  ID Name: '{}'", link_info.name));
    output.print_result_fmt(format_args!("  Library Path: '{lib_filepath}'"));
    output.print_result_fmt(format_args!("  Library Block Index: {lib_block_index}"));

    if let Some(target) = &target_name {
        output.print_info_fmt(format_args!("  Target Name: '{target}'"));
    }

    // Resolve library file path
    let lib_file_path = resolve_library_path(&lib_filepath, &file_path)?;
    output.print_result_fmt(format_args!(
        "  Resolved Library Path: '{}'",
        lib_file_path.display()
    ));

    if !lib_file_path.exists() {
        return Err(execution_failed_error(
            "Library file does not exist at resolved path",
        ));
    }

    // Analyze library file for available assets
    let available_collections = find_collections_in_library(&lib_file_path)?;

    output.print_info("\nAvailable collections in library:");
    for (idx, name) in &available_collections {
        output.print_result_fmt(format_args!("  Block {idx}: '{name}'"));
    }

    // Determine target collection
    let target_collection = determine_target_collection(&available_collections, target_name)?;
    output.print_info_fmt(format_args!(
        "\nReconstructing pointer to: '{target_collection}'"
    ));

    if dry_run {
        output.print_result_fmt(format_args!(
            "[DRY RUN] Would update ID name from '{}' to '{target_collection}'",
            link_info.name
        ));
        return Ok(());
    }

    // Perform the reconstruction
    reconstruct_id_name(
        &file_path,
        &mut blend_file,
        block_index,
        &target_collection,
        output,
    )?;

    output.print_result_fmt(format_args!(
        "SUCCESS: Pointer reconstructed! ID name updated to '{target_collection}'"
    ));
    output.print_result("The library link should now resolve correctly.");

    Ok(())
}

/// Information about an ID block's library link
struct IdLinkInfo {
    name: String,
    lib_ptr: u64,
}

/// Read ID block information for library linking
fn read_id_link_info(field_reader: &dot001_parser::FieldReader) -> Result<IdLinkInfo, Dot001Error> {
    let full_name = field_reader
        .read_field_string("ID", "name")
        .map_err(|e| execution_failed_error(format!("Error reading ID name: {e}")))?;

    // ID names include the 2-byte type prefix, skip it
    let name = full_name.chars().skip(2).collect::<String>();

    let lib_ptr = field_reader
        .read_field_pointer("ID", "lib")
        .map_err(|e| execution_failed_error(format!("Error reading lib pointer: {e}")))?;

    Ok(IdLinkInfo { name, lib_ptr })
}

/// Resolve library file path from library path string and main file location
fn resolve_library_path(
    lib_filepath: &str,
    main_file_path: &std::path::Path,
) -> Result<PathBuf, Dot001Error> {
    if lib_filepath.starts_with("//") {
        // Blendfile-relative path
        let rel_path = lib_filepath.strip_prefix("//").unwrap();
        let main_file_dir = main_file_path
            .parent()
            .ok_or_else(|| execution_failed_error("Main file has no parent directory"))?;
        Ok(main_file_dir.join(rel_path))
    } else {
        Ok(PathBuf::from(lib_filepath))
    }
}

/// Find all collections in the library file
fn find_collections_in_library(
    lib_file_path: &PathBuf,
) -> Result<Vec<(usize, String)>, Dot001Error> {
    let lib_file = File::open(lib_file_path)?;
    let mut lib_reader = BufReader::new(lib_file);
    let mut lib_blend_file = BlendFile::new(&mut lib_reader)
        .map_err(|e| execution_failed_error(format!("Error opening library blend file: {e}")))?;

    let mut available_collections = Vec::new();

    // Find all GR (Collection) blocks
    let collection_indices: Vec<usize> = (0..lib_blend_file.blocks_len())
        .filter_map(|index| {
            lib_blend_file.get_block(index).and_then(|block| {
                let block_code = String::from_utf8_lossy(&block.header.code);
                if block_code.trim_end_matches('\0') == "GR" {
                    Some(index)
                } else {
                    None
                }
            })
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
) -> Result<String, Dot001Error> {
    match target_name {
        Some(target) => {
            if available_collections
                .iter()
                .any(|(_, name)| name == &target)
            {
                Ok(target)
            } else {
                Err(invalid_arguments_error(format!(
                    "Target collection '{target}' not found in library"
                )))
            }
        }
        None => {
            if available_collections.is_empty() {
                Err(execution_failed_error("No collections found in library"))
            } else if available_collections.len() == 1 {
                Ok(available_collections[0].1.clone())
            } else {
                Err(missing_argument_error(format!(
                    "Multiple collections available ({}), specify --target-name",
                    available_collections.len()
                )))
            }
        }
    }
}

/// Perform the actual ID name reconstruction
fn reconstruct_id_name<R: std::io::Read + std::io::Seek>(
    file_path: &PathBuf,
    blend_file: &mut BlendFile<R>,
    block_index: usize,
    target_name: &str,
    output: &OutputHandler,
) -> Result<(), Dot001Error> {
    output.print_info("Performing pointer reconstruction...");

    // Read block data and get field offset
    let mut block_data = blend_file.read_block_data(block_index)?;
    let field_reader = blend_file.create_field_reader(&block_data)?;

    let name_offset = field_reader.get_field_offset("ID", "name").map_err(|e| {
        execution_failed_error(format!("Could not find name field in ID block: {e}"))
    })?;

    // Construct new ID name with type prefix (GR for collections)
    let new_id_name = format!("GR{target_name}");
    let mut name_bytes = vec![0u8; 258]; // MAX_ID_NAME from DNA_ID.h
    let copy_len = std::cmp::min(new_id_name.len(), 257); // Leave room for null terminator
    name_bytes[..copy_len].copy_from_slice(new_id_name.as_bytes());

    // Update the name field in block data
    let name_end = name_offset + 258;
    if name_end > block_data.len() {
        return Err(execution_failed_error(
            "Name field extends beyond block data",
        ));
    }
    block_data[name_offset..name_end].copy_from_slice(&name_bytes);

    // Write back to file
    let block = blend_file.get_block(block_index).unwrap();
    let block_data_offset = block.data_offset;

    let mut output_file = OpenOptions::new().read(true).write(true).open(file_path)?;
    output_file.seek(SeekFrom::Start(block_data_offset))?;
    output_file.write_all(&block_data)?;
    output_file.flush()?;

    Ok(())
}
