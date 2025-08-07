//! High-performance slice-based block header scanning
//!
//! This module provides optimized block header parsing directly from memory slices,
//! avoiding the overhead of Read+Seek operations for the zero-copy fast path.

use crate::{BlendFileBlock, BlendFileErrorKind, BlendFileHeader, BlockHeader, Error, Result};

/// Calculate the size of a block header based on file format
pub fn block_header_size(header: &BlendFileHeader) -> usize {
    if header.file_format_version == 1 {
        // v1 format: code(4) + sdna_index(4) + old_address(8) + size(8) + count(8)
        4 + 4 + 8 + 8 + 8
    } else if header.pointer_size == 4 {
        // Legacy 32-bit: code(4) + size(4) + old_address(4) + sdna_index(4) + count(4)
        4 + 4 + 4 + 4 + 4
    } else {
        // Legacy 64-bit: code(4) + size(4) + old_address(8) + sdna_index(4) + count(4)
        4 + 4 + 8 + 4 + 4
    }
}

/// Parse a single block header from a byte slice at the given offset
///
/// Returns the parsed header and the size consumed (for advancing to next header)
pub fn parse_block_header_at(
    data: &[u8],
    offset: usize,
    file_header: &BlendFileHeader,
) -> Result<(BlockHeader, usize)> {
    let header_size = block_header_size(file_header);

    if offset + header_size > data.len() {
        return Err(Error::blend_file(
            format!(
                "Insufficient data for block header at offset {}: need {} bytes, have {}",
                offset,
                header_size,
                data.len() - offset
            ),
            BlendFileErrorKind::InvalidData,
        ));
    }

    let slice = &data[offset..offset + header_size];
    let mut cursor = 0;

    // Read block code (4 bytes)
    let code: [u8; 4] = slice[cursor..cursor + 4].try_into().unwrap();
    cursor += 4;

    let header = if file_header.file_format_version == 1 {
        // v1 format parsing
        let sdna_index = read_u32_at(slice, cursor, file_header.is_little_endian);
        cursor += 4;
        let old_address = read_u64_at(slice, cursor, file_header.is_little_endian);
        cursor += 8;
        let size = read_u64_at(slice, cursor, file_header.is_little_endian) as u32;
        cursor += 8;
        let count = read_u64_at(slice, cursor, file_header.is_little_endian) as u32;

        BlockHeader {
            code,
            size,
            old_address,
            sdna_index,
            count,
        }
    } else if file_header.pointer_size == 4 {
        // Legacy 32-bit format
        let size = read_u32_at(slice, cursor, file_header.is_little_endian);
        cursor += 4;
        let old_address = read_u32_at(slice, cursor, file_header.is_little_endian) as u64;
        cursor += 4;
        let sdna_index = read_u32_at(slice, cursor, file_header.is_little_endian);
        cursor += 4;
        let count = read_u32_at(slice, cursor, file_header.is_little_endian);

        BlockHeader {
            code,
            size,
            old_address,
            sdna_index,
            count,
        }
    } else {
        // Legacy 64-bit format
        let size = read_u32_at(slice, cursor, file_header.is_little_endian);
        cursor += 4;
        let old_address = read_u64_at(slice, cursor, file_header.is_little_endian);
        cursor += 8;
        let sdna_index = read_u32_at(slice, cursor, file_header.is_little_endian);
        cursor += 4;
        let count = read_u32_at(slice, cursor, file_header.is_little_endian);

        BlockHeader {
            code,
            size,
            old_address,
            sdna_index,
            count,
        }
    };

    Ok((header, header_size))
}

/// Scan all block headers from a buffer slice starting at the given offset
///
/// Returns a vector of BlendFileBlocks with correct offsets calculated.
/// This is the main zero-copy scanning function.
pub fn scan_blocks(
    data: &[u8],
    start_offset: usize,
    file_header: &BlendFileHeader,
) -> Result<Vec<BlendFileBlock>> {
    let mut blocks = Vec::new();
    let mut current_offset = start_offset;
    let header_size = block_header_size(file_header);

    loop {
        // Ensure we have enough data for at least the block header
        if current_offset + header_size > data.len() {
            return Err(Error::blend_file(
                format!("Unexpected end of data while scanning blocks at offset {current_offset}"),
                BlendFileErrorKind::InvalidData,
            ));
        }

        let header_start = current_offset;
        let (block_header, consumed) = parse_block_header_at(data, current_offset, file_header)?;
        current_offset += consumed;

        // Check for end marker
        if &block_header.code == b"ENDB" {
            break;
        }

        // Validate block size
        if block_header.size as u64 > crate::DEFAULT_MAX_BLOCK_SIZE as u64 {
            return Err(Error::blend_file(
                format!("Block size too large: {} bytes", block_header.size),
                BlendFileErrorKind::SizeLimitExceeded,
            ));
        }

        // Ensure we don't overflow when seeking past block data
        let block_end = current_offset as u64 + block_header.size as u64;
        if block_end > data.len() as u64 {
            return Err(Error::blend_file(
                format!(
                    "Block extends beyond data: block ends at {}, data length is {}",
                    block_end,
                    data.len()
                ),
                BlendFileErrorKind::InvalidData,
            ));
        }

        let block = BlendFileBlock {
            header: block_header,
            data_offset: current_offset as u64,
            header_offset: header_start as u64,
        };

        blocks.push(block);

        // Skip past the block data
        current_offset = block_end as usize;
    }

    Ok(blocks)
}

/// Read a u32 from a slice at the given offset with endianness handling
#[inline]
fn read_u32_at(data: &[u8], offset: usize, is_little_endian: bool) -> u32 {
    let bytes: [u8; 4] = data[offset..offset + 4].try_into().unwrap();
    if is_little_endian {
        u32::from_le_bytes(bytes)
    } else {
        u32::from_be_bytes(bytes)
    }
}

/// Read a u64 from a slice at the given offset with endianness handling  
#[inline]
fn read_u64_at(data: &[u8], offset: usize, is_little_endian: bool) -> u64 {
    let bytes: [u8; 8] = data[offset..offset + 8].try_into().unwrap();
    if is_little_endian {
        u64::from_le_bytes(bytes)
    } else {
        u64::from_be_bytes(bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::BlendFileHeader;

    fn create_test_header(pointer_size: u8, is_little_endian: bool) -> BlendFileHeader {
        BlendFileHeader {
            magic: *b"BLENDER",
            pointer_size,
            is_little_endian,
            file_format_version: 0,
            version: 350,
        }
    }

    #[test]
    fn test_block_header_size_calculation() {
        let header_32 = create_test_header(4, true);
        let header_64 = create_test_header(8, true);

        assert_eq!(block_header_size(&header_32), 20); // 4+4+4+4+4
        assert_eq!(block_header_size(&header_64), 24); // 4+4+8+4+4
    }

    #[test]
    fn test_parse_block_header_32bit() {
        let header = create_test_header(4, true);

        // Create test data: MESH block, size=100, address=0x1000, sdna=1, count=1
        let data = [
            // Block code "MESH"
            b'M', b'E', b'S', b'H', // Size (100, little-endian u32)
            100, 0, 0, 0, // Old address (0x1000, little-endian u32)
            0x00, 0x10, 0, 0, // SDNA index (1, little-endian u32)
            1, 0, 0, 0, // Count (1, little-endian u32)
            1, 0, 0, 0,
        ];

        let (block_header, consumed) = parse_block_header_at(&data, 0, &header).unwrap();

        assert_eq!(&block_header.code, b"MESH");
        assert_eq!(block_header.size, 100);
        assert_eq!(block_header.old_address, 0x1000);
        assert_eq!(block_header.sdna_index, 1);
        assert_eq!(block_header.count, 1);
        assert_eq!(consumed, 20);
    }

    #[test]
    fn test_insufficient_data_error() {
        let header = create_test_header(4, true);
        let data = [1, 2, 3]; // Too small for a block header

        let result = parse_block_header_at(&data, 0, &header);
        assert!(result.is_err());
    }
}
