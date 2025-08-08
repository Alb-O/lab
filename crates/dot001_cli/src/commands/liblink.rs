use crate::block_display::BlockInfo;
use crate::block_ops::CommandHelper;
use crate::output_utils::{CommandSummary, OutputUtils};
use crate::util::CommandContext;
use crate::{execution_failed_error, invalid_arguments_error, missing_argument_error};
use dot001_events::error::Error;
use dot001_parser::BlendFile;
use std::fs::OpenOptions;
use std::io::{Seek, SeekFrom, Write};
use std::path::PathBuf;

/// Reconstruct broken library links by updating ID names to match available assets
pub fn cmd_lib_link(
    file_path: PathBuf,
    block_identifier: &str,
    dry_run: bool,
    target_name: Option<String>,
    ctx: &CommandContext,
) -> Result<(), Error> {
    let mut blend_file = ctx.load_blend_file(&file_path)?;

    // Resolve the block identifier to a specific block index
    let block_index = {
        let mut helper = CommandHelper::new(&mut blend_file, ctx);
        let Some(index) = helper.resolve_block_or_return(block_identifier)? else {
            return Ok(());
        };
        index
    };

    ctx.output.print_info_fmt(format_args!(
        "Analyzing library link for block {block_index}..."
    ));

    let slice = blend_file.read_block_slice_for_field_view(block_index)?;
    let field_view = blend_file.create_field_view(&slice)?;

    // Read current ID information
    let link_info = read_id_link_info(&field_view)?;

    if link_info.lib_ptr == 0 {
        ctx.output
            .print_result("This is a local asset (lib pointer is NULL), not a library link.");
        return Ok(());
    }

    // Find and read library block
    let lib_block_index = blend_file
        .find_block_by_address(link_info.lib_ptr)
        .ok_or_else(|| execution_failed_error("Lib pointer does not point to a valid block"))?;

    let lib_slice = blend_file.read_block_slice_for_field_view(lib_block_index)?;
    let lib_view = blend_file.create_field_view(&lib_slice)?;
    let lib_filepath = lib_view
        .read_field_string("Library", "name")
        .map_err(|e| execution_failed_error(format!("Error reading library name field: {e}")))?;

    let mut link_summary = CommandSummary::new("Current Link Status")
        .add_item("ID Name", format!("'{}''", link_info.name))
        .add_item("Library Path", format!("'{lib_filepath}'"))
        .add_item("Library Block Index", lib_block_index.to_string());

    if let Some(target) = &target_name {
        link_summary = link_summary.add_item("Target Name", format!("'{target}'"));
    }

    // Resolve library file path
    let lib_file_path = resolve_library_path(&lib_filepath, &file_path)?;
    link_summary = link_summary.add_item(
        "Resolved Library Path",
        format!("'{}''", lib_file_path.display()),
    );

    link_summary.print(ctx);

    if !lib_file_path.exists() {
        return Err(execution_failed_error(
            "Library file does not exist at resolved path",
        ));
    }

    // Analyze library file for available assets
    let available_collections = find_collections_in_library(&lib_file_path)?;

    ctx.output.print_info("\nAvailable collections in library:");
    let collection_displays: Vec<String> = available_collections
        .iter()
        .map(|(idx, name)| {
            let block_info = BlockInfo::with_name(*idx, "GR".to_string(), name.clone());
            block_info.display().to_string()
        })
        .collect();
    OutputUtils::print_list(ctx, &collection_displays);

    // Determine target collection
    let target_collection = determine_target_collection(&available_collections, target_name)?;
    ctx.output.print_info_fmt(format_args!(
        "\nReconstructing pointer to: '{target_collection}'"
    ));

    if dry_run {
        ctx.output.print_result_fmt(format_args!(
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
        ctx.output,
    )?;

    ctx.output.print_result_fmt(format_args!(
        "SUCCESS: Pointer reconstructed! ID name updated to '{target_collection}'"
    ));
    ctx.output
        .print_result("The library link should now resolve correctly.");

    Ok(())
}

/// Information about an ID block's library link
struct IdLinkInfo {
    name: String,
    lib_ptr: u64,
}

/// Read ID block information for library linking
fn read_id_link_info(field_view: &dot001_parser::FieldView) -> Result<IdLinkInfo, Error> {
    let full_name = field_view
        .read_field_string("ID", "name")
        .map_err(|e| execution_failed_error(format!("Error reading ID name: {e}")))?;

    // ID names include the 2-byte type prefix, skip it
    let name = full_name.chars().skip(2).collect::<String>();

    let lib_ptr = field_view
        .read_field_pointer("ID", "lib")
        .map_err(|e| execution_failed_error(format!("Error reading lib pointer: {e}")))?;

    Ok(IdLinkInfo { name, lib_ptr })
}

/// Resolve library file path from library path string and main file location
fn resolve_library_path(
    lib_filepath: &str,
    main_file_path: &std::path::Path,
) -> Result<PathBuf, Error> {
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
fn find_collections_in_library(lib_file_path: &PathBuf) -> Result<Vec<(usize, String)>, Error> {
    let lib_blend_file = dot001_parser::from_path(lib_file_path)
        .map_err(|e| execution_failed_error(format!("Error opening library blend file: {e}")))?;

    let mut available_collections = Vec::new();

    // Find all GR (Collection) blocks
    let collection_indices: Vec<usize> = (0..lib_blend_file.blocks_len())
        .filter_map(|index| {
            lib_blend_file.get_block(index).and_then(|block| {
                let block_code = dot001_parser::block_code_to_string(block.header.code);
                if block_code == "GR" {
                    Some(index)
                } else {
                    None
                }
            })
        })
        .collect();

    // Read collection names
    for index in collection_indices {
        if let Ok(lib_slice) = lib_blend_file.read_block_slice_for_field_view(index) {
            if let Ok(lib_field_view) = lib_blend_file.create_field_view(&lib_slice) {
                if let Ok(name) = lib_field_view.read_field_string("ID", "name") {
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
) -> Result<String, Error> {
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
fn reconstruct_id_name(
    file_path: &PathBuf,
    blend_file: &mut BlendFile,
    block_index: usize,
    target_name: &str,
    output: &crate::util::OutputHandler,
) -> Result<(), Error> {
    output.print_info("Performing pointer reconstruction...");

    // Read block data and get field offset
    let mut block_data = blend_file.read_block_data(block_index)?;
    let dna = blend_file
        .dna()
        .map_err(|e| execution_failed_error(e.to_string()))?;
    // Find ID.name field offset and size
    let id_struct = dna
        .structs
        .iter()
        .find(|s| s.type_name == "ID")
        .ok_or_else(|| execution_failed_error("ID struct not found in DNA"))?;
    let name_field = id_struct
        .find_field("name")
        .ok_or_else(|| execution_failed_error("Field 'name' not found in ID struct"))?;
    let name_offset = name_field.offset;

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
