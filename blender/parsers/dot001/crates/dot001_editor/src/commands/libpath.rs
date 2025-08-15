use crate::Result;
use dot001_events::error::{EditorErrorKind, Error};
use dot001_events::{
    event::{EditorEvent, Event},
    prelude::*,
};
#[cfg(feature = "tracer_integration")]
use dot001_tracer::bpath::BlendPath;
use std::fs::OpenOptions;
use std::io::{Seek, SeekFrom, Write};
use std::path::Path;

pub struct LibPathCommand;

impl LibPathCommand {
    /// Update the filepath of a linked library file (LI block) and save changes to file
    pub fn update_libpath_and_save<P: AsRef<Path>>(
        file_path: P,
        block_index: usize,
        new_path: &str,
        no_validate: bool,
    ) -> Result<()> {
        // If the new_path is a simple filename (no slashes), treat it as blendfile-relative
        let mut normalized_path = new_path.to_string();
        if !normalized_path.starts_with("//")
            && !normalized_path.contains('/')
            && !normalized_path.contains('\\')
        {
            normalized_path = format!("//{normalized_path}");
        }
        // Validate and normalize the new path
        #[cfg(feature = "tracer_integration")]
        let blend_path = BlendPath::new(normalized_path.as_bytes());

        #[cfg(not(feature = "tracer_integration"))]
        let blend_path = {
            // Simple validation without tracer
            if !no_validate {
                if let Some(rel_path) = normalized_path.strip_prefix("//") {
                    // Blendfile-relative path
                    let blend_file_path = file_path.as_ref();
                    let blend_dir = blend_file_path.parent().ok_or_else(|| {
                        Error::from(std::io::Error::new(
                            std::io::ErrorKind::InvalidInput,
                            "Blend file has no parent directory",
                        ))
                    })?;
                    let abs_path = blend_dir.join(rel_path);
                    if !abs_path.exists() {
                        return Err(Error::from(std::io::Error::new(
                            std::io::ErrorKind::NotFound,
                            format!("Target library file does not exist: {}", abs_path.display()),
                        )));
                    }
                } else if Path::new(&normalized_path).is_absolute() {
                    if !Path::new(&normalized_path).exists() {
                        return Err(Error::from(std::io::Error::new(
                            std::io::ErrorKind::NotFound,
                            format!("Target library file does not exist: {normalized_path}"),
                        )));
                    }
                } else {
                    return Err(Error::editor(
                        "Library path must be absolute or blendfile-relative".to_string(),
                        EditorErrorKind::InvalidCharacters,
                    ));
                }
            }
            normalized_path.as_bytes()
        };

        #[cfg(feature = "tracer_integration")]
        if !no_validate {
            if blend_path.is_blendfile_relative() {
                // Resolve relative to blend file location
                let blend_file_path = file_path.as_ref();
                let blend_dir = blend_file_path.parent().ok_or_else(|| {
                    Error::from(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "Blend file has no parent directory",
                    ))
                })?;
                let abs_path = blend_path.absolute(Some(blend_dir));
                if !Path::new(std::str::from_utf8(abs_path.as_bytes()).unwrap_or("")).exists() {
                    return Err(Error::from(std::io::Error::new(
                        std::io::ErrorKind::NotFound,
                        format!(
                            "Target library file does not exist: {}",
                            String::from_utf8_lossy(abs_path.as_bytes())
                        ),
                    )));
                }
            } else if blend_path.is_absolute() {
                if !Path::new(std::str::from_utf8(blend_path.as_bytes()).unwrap_or("")).exists() {
                    return Err(Error::from(std::io::Error::new(
                        std::io::ErrorKind::NotFound,
                        format!(
                            "Target library file does not exist: {}",
                            String::from_utf8_lossy(blend_path.as_bytes())
                        ),
                    )));
                }
            } else {
                // Not blendfile-relative or absolute: invalid for Blender library path
                return Err(Error::editor(
                    "Library path must be absolute or blendfile-relative".to_string(),
                    EditorErrorKind::InvalidCharacters,
                ));
            }
        }

        // Open and parse the blend file using zero-copy buffer API
        let blend_file = dot001_parser::from_path(&file_path)?;

        // Check if block exists
        if block_index >= blend_file.blocks_len() {
            return Err(Error::editor(
                format!("Block not found at index: {block_index}"),
                EditorErrorKind::BlockNotFound,
            ));
        }
        let block = blend_file.get_block(block_index).unwrap();
        let block_code = dot001_parser::block_code_to_string(block.header.code);
        if block_code != "LI" {
            return Err(Error::editor(
                "Block is not a linked library file (LI block)".to_string(),
                EditorErrorKind::InvalidCharacters,
            ));
        }
        let block_data_offset = block.data_offset;
        let mut block_data = blend_file.read_block_data(block_index)?;
        // Use DNA to locate the name field offset/size in Library struct
        let dna = blend_file.dna()?;
        // Find the offset and size of the name field in the LI struct
        let name_offset = {
            let struct_def = dna
                .structs
                .iter()
                .find(|s| s.type_name == "Library")
                .ok_or_else(|| {
                    Error::editor(
                        "Struct Library not found in DNA".to_string(),
                        EditorErrorKind::InvalidCharacters,
                    )
                })?;
            let field = struct_def
                .fields
                .iter()
                .find(|f| f.name.name_only == "name")
                .ok_or_else(|| {
                    Error::editor(
                        "Field name not found in Library struct".to_string(),
                        EditorErrorKind::InvalidCharacters,
                    )
                })?;
            field.offset
        };
        // Get the field size from the DNA struct
        let name_size = {
            let struct_def = dna
                .structs
                .iter()
                .find(|s| s.type_name == "Library")
                .unwrap();
            let field = struct_def
                .fields
                .iter()
                .find(|f| f.name.name_only == "name")
                .unwrap();
            field.size
        };
        // Prepare the new path bytes, null-terminated and sized for Blender
        let mut path_bytes = vec![0u8; name_size];
        #[cfg(feature = "tracer_integration")]
        let path_slice = blend_path.as_bytes();
        #[cfg(not(feature = "tracer_integration"))]
        let path_slice = blend_path;

        let copy_len = std::cmp::min(path_slice.len(), name_size.saturating_sub(1));
        path_bytes[..copy_len].copy_from_slice(&path_slice[..copy_len]);
        // Check if the new path is the same as the current one
        if !no_validate {
            let current_name = &block_data[name_offset..(name_offset + name_size)];
            if &path_bytes[..] == current_name {
                return Err(Error::editor(
                    "No change detected: new library path is identical to the current path"
                        .to_string(),
                    EditorErrorKind::InvalidCharacters,
                ));
            }
        }

        // Emit field modification event
        let current_name = &block_data[name_offset..(name_offset + name_size)];
        let old_path = String::from_utf8_lossy(current_name)
            .trim_end_matches('\0')
            .to_string();
        let new_path = String::from_utf8_lossy(path_slice).to_string();

        emit_global_sync!(Event::Editor(EditorEvent::BlockEdited {
            operation: "update_libpath".to_string(),
            block_index,
            block_type: "LI".to_string(),
            old_value: Some(old_path),
            new_value: new_path,
        }));

        // Overwrite the name field in the block data
        let end_offset = name_offset + name_size;
        if end_offset > block_data.len() {
            return Err(Error::from(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Name field extends beyond block data",
            )));
        }
        block_data[name_offset..end_offset].copy_from_slice(&path_bytes);
        // Write the modified block data back to the file
        let mut file = OpenOptions::new().read(true).write(true).open(&file_path)?;
        file.seek(SeekFrom::Start(block_data_offset))?;
        file.write_all(&block_data)?;
        file.flush()?;
        Ok(())
    }
}
