use dot001_error::Dot001Error;
use dot001_parser::ParseOptions;
use log::info;
use std::path::PathBuf;

pub fn cmd_rename(
    file_path: PathBuf,
    block_index: usize,
    new_name: String,
    dry_run: bool,
    options: &ParseOptions,
    no_auto_decompress: bool,
) -> Result<(), Dot001Error> {
    use dot001_editor::BlendEditor;
    let mut blend_file = crate::util::load_blend_file(&file_path, options, no_auto_decompress)?;
    if block_index >= blend_file.blocks_len() {
        eprintln!(
            "Error: Block index {} is out of range (max: {})",
            block_index,
            blend_file.blocks_len() - 1
        );
        return Ok(());
    }
    let block_code = {
        let Some(block) = blend_file.get_block(block_index) else {
            eprintln!("Error: Block index {block_index} is out of range");
            return Ok(());
        };
        String::from_utf8_lossy(&block.header.code)
            .trim_end_matches('\0')
            .to_string()
    };
    #[cfg(feature = "trace")]
    let current_name_opt =
        { dot001_tracer::NameResolver::resolve_name(block_index, &mut blend_file) };

    #[cfg(not(feature = "trace"))]
    let current_name_opt = Some(format!("Block{}", block_index));

    match current_name_opt {
        Some(current_name) => {
            if dry_run {
                info!("Would rename {block_code} block '{current_name}' to '{new_name}'");
            } else {
                info!("Renaming {block_code} block '{current_name}' to '{new_name}'");
                match BlendEditor::rename_id_block_and_save(&file_path, block_index, &new_name) {
                    Ok(()) => {
                        #[cfg(feature = "trace")]
                        {
                            let mut updated_blend_file = crate::util::load_blend_file(
                                &file_path,
                                options,
                                no_auto_decompress,
                            )?;
                            match dot001_tracer::NameResolver::resolve_name(
                                block_index,
                                &mut updated_blend_file,
                            ) {
                                Some(updated_name) => {
                                    if updated_name == new_name {
                                        println!("Success: Block renamed to '{updated_name}'");
                                    } else {
                                        eprintln!(
                                            "Warning: Name is '{updated_name}', expected '{new_name}'"
                                        );
                                    }
                                }
                                None => {
                                    eprintln!("Warning: Could not verify name change");
                                }
                            }
                        }
                        #[cfg(not(feature = "trace"))]
                        {
                            println!(
                                "Success: Block renamed (verification unavailable without trace feature)"
                            );
                        }
                    }
                    Err(e) => {
                        eprintln!("Error: Failed to rename block: {e}");
                    }
                }
            }
        }
        None => {
            eprintln!("Error: Block {block_index} is not a named datablock");
        }
    }
    Ok(())
}
