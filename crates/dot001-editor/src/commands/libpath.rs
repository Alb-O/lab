use crate::Result;
use dot001_error::{Dot001Error, EditorErrorKind};
use dot001_parser::BlendFile;
#[cfg(feature = "tracer_integration")]
use dot001_tracer::bpath::BlendPath;
use std::fs::OpenOptions;
use std::io::{Seek, SeekFrom, Write};
use std::path::Path;

pub struct LibPathCommand;

impl LibPathCommand {
    /// Update the filepath of a Library (LI) block and save changes to file
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
                if normalized_path.starts_with("//") {
                    // Blendfile-relative path
                    let rel_path = &normalized_path[2..];
                    let blend_file_path = file_path.as_ref();
                    let blend_dir = blend_file_path.parent().ok_or_else(|| {
                        Dot001Error::from(std::io::Error::new(
                            std::io::ErrorKind::InvalidInput,
                            "Blend file has no parent directory",
                        ))
                    })?;
                    let abs_path = blend_dir.join(rel_path);
                    if !abs_path.exists() {
                        return Err(Dot001Error::from(std::io::Error::new(
                            std::io::ErrorKind::NotFound,
                            format!("Target library file does not exist: {}", abs_path.display()),
                        )));
                    }
                } else if Path::new(&normalized_path).is_absolute() {
                    if !Path::new(&normalized_path).exists() {
                        return Err(Dot001Error::from(std::io::Error::new(
                            std::io::ErrorKind::NotFound,
                            format!("Target library file does not exist: {}", normalized_path),
                        )));
                    }
                } else {
                    return Err(Dot001Error::editor(
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
                    Dot001Error::from(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "Blend file has no parent directory",
                    ))
                })?;
                let abs_path = blend_path.absolute(Some(blend_dir));
                if !Path::new(std::str::from_utf8(abs_path.as_bytes()).unwrap_or("")).exists() {
                    return Err(Dot001Error::from(std::io::Error::new(
                        std::io::ErrorKind::NotFound,
                        format!(
                            "Target library file does not exist: {}",
                            String::from_utf8_lossy(abs_path.as_bytes())
                        ),
                    )));
                }
            } else if blend_path.is_absolute() {
                if !Path::new(std::str::from_utf8(blend_path.as_bytes()).unwrap_or("")).exists() {
                    return Err(Dot001Error::from(std::io::Error::new(
                        std::io::ErrorKind::NotFound,
                        format!(
                            "Target library file does not exist: {}",
                            String::from_utf8_lossy(blend_path.as_bytes())
                        ),
                    )));
                }
            } else {
                // Not blendfile-relative or absolute: invalid for Blender library path
                return Err(Dot001Error::editor(
                    "Library path must be absolute or blendfile-relative".to_string(),
                    EditorErrorKind::InvalidCharacters,
                ));
            }
        }

        // Open the blend file and parse
        let file = std::fs::File::open(&file_path)?;
        let mut reader = std::io::BufReader::new(file);
        let mut blend_file = BlendFile::new(&mut reader)?;

        // Check if block exists
        if block_index >= blend_file.blocks_len() {
            return Err(Dot001Error::editor(
                format!("Block not found at index: {block_index}"),
                EditorErrorKind::BlockNotFound,
            ));
        }
        let block = blend_file.get_block(block_index).unwrap();
        let block_code = String::from_utf8_lossy(&block.header.code)
            .trim_end_matches('\0')
            .to_string();
        if block_code != "LI" {
            return Err(Dot001Error::editor(
                "Block is not a Library (LI) block".to_string(),
                EditorErrorKind::InvalidCharacters,
            ));
        }
        let block_data_offset = block.data_offset;
        let mut block_data = blend_file.read_block_data(block_index)?;
        let reader = blend_file.create_field_reader(&block_data)?;
        // Find the offset and size of the name field in the LI struct
        let name_offset = reader.get_field_offset("Library", "name").map_err(|_| {
            Dot001Error::editor(
                "Could not find name field in Library block".to_string(),
                EditorErrorKind::InvalidCharacters,
            )
        })?;
        // Get the field size from the DNA struct
        let struct_def = reader
            .dna
            .structs
            .iter()
            .find(|s| s.type_name == "Library")
            .ok_or_else(|| {
                Dot001Error::editor(
                    "Struct Library not found in DNA".to_string(),
                    EditorErrorKind::InvalidCharacters,
                )
            })?;
        let field = struct_def
            .fields
            .iter()
            .find(|f| f.name.name_only == "name")
            .ok_or_else(|| {
                Dot001Error::editor(
                    "Field name not found in Library struct".to_string(),
                    EditorErrorKind::InvalidCharacters,
                )
            })?;
        let name_size = field.size;
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
                return Err(Dot001Error::editor(
                    "No change detected: new library path is identical to the current path"
                        .to_string(),
                    EditorErrorKind::InvalidCharacters,
                ));
            }
        }
        // Overwrite the name field in the block data
        let end_offset = name_offset + name_size;
        if end_offset > block_data.len() {
            return Err(Dot001Error::from(std::io::Error::new(
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
