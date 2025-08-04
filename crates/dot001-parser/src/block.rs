use crate::error::{Dot001Error, Result};
use crate::header::BlendFileHeader;
use std::io::{Read, Seek};

#[derive(Debug, Clone)]
pub struct BlockHeader {
    pub code: [u8; 4],
    pub size: u32,
    pub old_address: u64,
    pub sdna_index: u32,
    pub count: u32,
}

#[derive(Debug, Clone)]
pub struct BlendFileBlock {
    /// Parsed header fields
    pub header: BlockHeader,
    /// Offset in the file where the block's raw data starts
    pub data_offset: u64,
    /// Offset in the file where this block header starts (for diagnostics/debugging)
    pub header_offset: u64,
}

impl BlockHeader {
    pub fn read<R: Read + Seek>(reader: &mut R, file_header: &BlendFileHeader) -> Result<Self> {
        let mut code = [0u8; 4];
        reader.read_exact(&mut code)?;

        if file_header.file_format_version == 1 {
            let sdna_index = read_u32(reader, file_header.is_little_endian)?;
            let old_address = read_u64(reader, file_header.is_little_endian)?;
            let size = read_u64(reader, file_header.is_little_endian)? as u32;
            let count = read_u64(reader, file_header.is_little_endian)? as u32;

            Ok(BlockHeader {
                code,
                size,
                old_address,
                sdna_index,
                count,
            })
        } else if file_header.pointer_size == 4 {
            let size = read_u32(reader, file_header.is_little_endian)?;
            let old_address = read_u32(reader, file_header.is_little_endian)? as u64;
            let sdna_index = read_u32(reader, file_header.is_little_endian)?;
            let count = read_u32(reader, file_header.is_little_endian)?;

            Ok(BlockHeader {
                code,
                size,
                old_address,
                sdna_index,
                count,
            })
        } else {
            let size = read_u32(reader, file_header.is_little_endian)?;
            let old_address = read_u64(reader, file_header.is_little_endian)?;
            let sdna_index = read_u32(reader, file_header.is_little_endian)?;
            let count = read_u32(reader, file_header.is_little_endian)?;

            Ok(BlockHeader {
                code,
                size,
                old_address,
                sdna_index,
                count,
            })
        }
    }

    pub fn code_str(&self) -> Result<&str> {
        std::str::from_utf8(&self.code).map_err(|_| {
            Dot001Error::blend_file(
                "Block code is not valid UTF-8",
                crate::error::BlendFileErrorKind::InvalidData,
            )
        })
    }

    pub fn is_end_block(&self) -> bool {
        &self.code == b"ENDB"
    }

    pub fn is_dna_block(&self) -> bool {
        &self.code == b"DNA1"
    }
}

fn read_u32<R: Read>(reader: &mut R, is_little_endian: bool) -> Result<u32> {
    let mut buf = [0u8; 4];
    reader.read_exact(&mut buf)?;
    Ok(if is_little_endian {
        u32::from_le_bytes(buf)
    } else {
        u32::from_be_bytes(buf)
    })
}

fn read_u64<R: Read>(reader: &mut R, is_little_endian: bool) -> Result<u64> {
    let mut buf = [0u8; 8];
    reader.read_exact(&mut buf)?;
    Ok(if is_little_endian {
        u64::from_le_bytes(buf)
    } else {
        u64::from_be_bytes(buf)
    })
}
