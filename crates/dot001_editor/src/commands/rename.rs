use crate::{Result, validate_new_name};
use dot001_events::error::{EditorErrorKind, Error};
use dot001_events::{
    event::{EditorEvent, Event},
    prelude::*,
};
use dot001_parser::BlendFile;
#[cfg(feature = "tracer_integration")]
use dot001_tracer::NameResolver;
use log::{debug, info};
use std::fs::OpenOptions;
use std::io::{Seek, SeekFrom, Write};
use std::path::Path;

pub struct RenameCommand;

impl RenameCommand {
    /// Rename an ID block and save changes to file
    pub fn rename_id_block_and_save<P: AsRef<Path>>(
        file_path: P,
        block_index: usize,
        new_name: &str,
    ) -> Result<()> {
        info!(
            "Starting ID block rename: file={}, block={}, new_name='{}'",
            file_path.as_ref().display(),
            block_index,
            new_name
        );
        validate_new_name(new_name)?;
        debug!("Name validation passed for '{new_name}'");
        let blend_file = dot001_parser::from_path(&file_path)?;
        if block_index >= blend_file.blocks_len() {
            return Err(Error::editor(
                format!("Block not found at index: {block_index}"),
                EditorErrorKind::BlockNotFound,
            ));
        }
        let block_code = {
            let Some(block) = blend_file.get_block(block_index) else {
                return Err(Error::editor(
                    format!("Block not found at index: {block_index}"),
                    EditorErrorKind::BlockNotFound,
                ));
            };
            dot001_parser::block_code_to_string(block.header.code)
        };
        #[cfg(feature = "tracer_integration")]
        let _current_name =
            NameResolver::resolve_name(block_index, &mut blend_file).ok_or(Error::editor(
                "No ID structure found in block".to_string(),
                EditorErrorKind::NoIdStructure,
            ))?;

        #[cfg(not(feature = "tracer_integration"))]
        let _current_name = format!("Block{block_index}"); // Fallback without tracer
        let Some(block) = blend_file.get_block(block_index) else {
            return Err(Error::editor(
                format!("Block not found at index: {block_index}"),
                EditorErrorKind::BlockNotFound,
            ));
        };
        let block_data_offset = block.data_offset;
        let mut block_data = blend_file.read_block_data(block_index)?;
        let dna = blend_file.dna()?;
        let name_offset = {
            let struct_def = dna
                .structs
                .iter()
                .find(|s| s.type_name == "ID")
                .ok_or_else(|| {
                    Error::editor(
                        "No ID structure found in block".to_string(),
                        EditorErrorKind::NoIdStructure,
                    )
                })?;
            let field = struct_def
                .fields
                .iter()
                .find(|f| f.name.name_only == "name")
                .ok_or_else(|| {
                    Error::editor(
                        "No ID.name field found".to_string(),
                        EditorErrorKind::NoIdStructure,
                    )
                })?;
            field.offset
        };
        let prefixed_name = format!("{block_code}{new_name}");
        let mut name_bytes = [0u8; 66];
        let name_bytes_to_copy = std::cmp::min(prefixed_name.len(), 65);
        name_bytes[..name_bytes_to_copy]
            .copy_from_slice(&prefixed_name.as_bytes()[..name_bytes_to_copy]);
        let start_offset = name_offset;
        let end_offset = start_offset + 66;
        if end_offset > block_data.len() {
            return Err(Error::from(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Name field extends beyond block data",
            )));
        }

        // Emit field modification event
        #[cfg(feature = "tracer_integration")]
        let old_name = _current_name;
        #[cfg(not(feature = "tracer_integration"))]
        let old_name = _current_name;

        emit_global_sync!(Event::Editor(EditorEvent::BlockEdited {
            operation: "rename_id".to_string(),
            block_index,
            block_type: block_code.clone(),
            old_value: Some(old_name),
            new_value: prefixed_name.clone(),
        }));

        block_data[start_offset..end_offset].copy_from_slice(&name_bytes);
        let mut file = OpenOptions::new().read(true).write(true).open(&file_path)?;
        file.seek(SeekFrom::Start(block_data_offset))?;
        file.write_all(&block_data)?;
        file.flush()?;
        Ok(())
    }

    /// Rename an ID block (in-memory only, for testing)
    pub fn rename_id_block(
        blend_file: &mut BlendFile,
        block_index: usize,
        new_name: &str,
    ) -> Result<()> {
        validate_new_name(new_name)?;
        if block_index >= blend_file.blocks_len() {
            return Err(Error::editor(
                format!("Block not found at index: {block_index}"),
                EditorErrorKind::BlockNotFound,
            ));
        }
        let block_code = {
            let Some(block) = blend_file.get_block(block_index) else {
                return Err(Error::editor(
                    format!("Block not found at index: {block_index}"),
                    EditorErrorKind::BlockNotFound,
                ));
            };
            dot001_parser::block_code_to_string(block.header.code)
        };
        #[cfg(feature = "tracer_integration")]
        let _current_name =
            NameResolver::resolve_name(block_index, blend_file).ok_or(Error::editor(
                "No ID structure found in block".to_string(),
                EditorErrorKind::NoIdStructure,
            ))?;

        #[cfg(not(feature = "tracer_integration"))]
        let _current_name = format!("Block{block_index}"); // Fallback without tracer
        let mut block_data = blend_file.read_block_data(block_index)?;
        let dna = blend_file.dna()?;
        let name_offset = {
            let struct_def = dna
                .structs
                .iter()
                .find(|s| s.type_name == "ID")
                .ok_or_else(|| {
                    Error::editor(
                        "No ID structure found in block".to_string(),
                        EditorErrorKind::NoIdStructure,
                    )
                })?;
            let field = struct_def
                .fields
                .iter()
                .find(|f| f.name.name_only == "name")
                .ok_or_else(|| {
                    Error::editor(
                        "No ID.name field found".to_string(),
                        EditorErrorKind::NoIdStructure,
                    )
                })?;
            field.offset
        };
        let prefixed_name = format!("{block_code}{new_name}");
        let mut name_bytes = [0u8; 66];
        let name_bytes_to_copy = std::cmp::min(prefixed_name.len(), 65);
        name_bytes[..name_bytes_to_copy]
            .copy_from_slice(&prefixed_name.as_bytes()[..name_bytes_to_copy]);
        let start_offset = name_offset;
        let end_offset = start_offset + 66;
        if end_offset > block_data.len() {
            return Err(Error::from(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Name field extends beyond block data",
            )));
        }
        block_data[start_offset..end_offset].copy_from_slice(&name_bytes);
        Ok(())
    }
}
