use crate::{Result, validate_new_name};
use dot001_error::{Dot001Error, EditorErrorKind};
use dot001_parser::BlendFile;
#[cfg(feature = "tracer_integration")]
use dot001_tracer::NameResolver;
use log::{debug, info};
use std::fs::OpenOptions;
use std::io::{Read, Seek, SeekFrom, Write};
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
        let file = std::fs::File::open(&file_path)?;
        let mut reader = std::io::BufReader::new(file);
        let mut blend_file = BlendFile::new(&mut reader)?;
        if block_index >= blend_file.blocks_len() {
            return Err(Dot001Error::editor(
                format!("Block not found at index: {block_index}"),
                EditorErrorKind::BlockNotFound,
            ));
        }
        let block_code = {
            let Some(block) = blend_file.get_block(block_index) else {
                return Err(Dot001Error::editor(
                    format!("Block not found at index: {block_index}"),
                    EditorErrorKind::BlockNotFound,
                ));
            };
            String::from_utf8_lossy(&block.header.code)
                .trim_end_matches('\0')
                .to_string()
        };
        #[cfg(feature = "tracer_integration")]
        let _current_name =
            NameResolver::resolve_name(block_index, &mut blend_file).ok_or(Dot001Error::editor(
                "No ID structure found in block".to_string(),
                EditorErrorKind::NoIdStructure,
            ))?;

        #[cfg(not(feature = "tracer_integration"))]
        let _current_name = format!("Block{}", block_index); // Fallback without tracer
        let Some(block) = blend_file.get_block(block_index) else {
            return Err(Dot001Error::editor(
                format!("Block not found at index: {block_index}"),
                EditorErrorKind::BlockNotFound,
            ));
        };
        let block_data_offset = block.data_offset;
        let mut block_data = blend_file.read_block_data(block_index)?;
        let reader = blend_file.create_field_reader(&block_data)?;
        let name_offset = reader.get_field_offset("ID", "name").map_err(|_| {
            Dot001Error::editor(
                "No ID structure found in block".to_string(),
                EditorErrorKind::NoIdStructure,
            )
        })?;
        let prefixed_name = format!("{block_code}{new_name}");
        let mut name_bytes = [0u8; 66];
        let name_bytes_to_copy = std::cmp::min(prefixed_name.len(), 65);
        name_bytes[..name_bytes_to_copy]
            .copy_from_slice(&prefixed_name.as_bytes()[..name_bytes_to_copy]);
        let start_offset = name_offset;
        let end_offset = start_offset + 66;
        if end_offset > block_data.len() {
            return Err(Dot001Error::from(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Name field extends beyond block data",
            )));
        }
        block_data[start_offset..end_offset].copy_from_slice(&name_bytes);
        let mut file = OpenOptions::new().read(true).write(true).open(&file_path)?;
        file.seek(SeekFrom::Start(block_data_offset))?;
        file.write_all(&block_data)?;
        file.flush()?;
        Ok(())
    }

    /// Rename an ID block (in-memory only, for testing)
    pub fn rename_id_block<R: Read + Seek>(
        blend_file: &mut BlendFile<R>,
        block_index: usize,
        new_name: &str,
    ) -> Result<()> {
        validate_new_name(new_name)?;
        if block_index >= blend_file.blocks_len() {
            return Err(Dot001Error::editor(
                format!("Block not found at index: {block_index}"),
                EditorErrorKind::BlockNotFound,
            ));
        }
        let block_code = {
            let Some(block) = blend_file.get_block(block_index) else {
                return Err(Dot001Error::editor(
                    format!("Block not found at index: {block_index}"),
                    EditorErrorKind::BlockNotFound,
                ));
            };
            String::from_utf8_lossy(&block.header.code)
                .trim_end_matches('\0')
                .to_string()
        };
        #[cfg(feature = "tracer_integration")]
        let _current_name =
            NameResolver::resolve_name(block_index, blend_file).ok_or(Dot001Error::editor(
                "No ID structure found in block".to_string(),
                EditorErrorKind::NoIdStructure,
            ))?;

        #[cfg(not(feature = "tracer_integration"))]
        let _current_name = format!("Block{}", block_index); // Fallback without tracer
        let mut block_data = blend_file.read_block_data(block_index)?;
        let reader = blend_file.create_field_reader(&block_data)?;
        let name_offset = reader.get_field_offset("ID", "name").map_err(|_| {
            Dot001Error::editor(
                "No ID structure found in block".to_string(),
                EditorErrorKind::NoIdStructure,
            )
        })?;
        let prefixed_name = format!("{block_code}{new_name}");
        let mut name_bytes = [0u8; 66];
        let name_bytes_to_copy = std::cmp::min(prefixed_name.len(), 65);
        name_bytes[..name_bytes_to_copy]
            .copy_from_slice(&prefixed_name.as_bytes()[..name_bytes_to_copy]);
        let start_offset = name_offset;
        let end_offset = start_offset + 66;
        if end_offset > block_data.len() {
            return Err(Dot001Error::from(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Name field extends beyond block data",
            )));
        }
        block_data[start_offset..end_offset].copy_from_slice(&name_bytes);
        Ok(())
    }
}
