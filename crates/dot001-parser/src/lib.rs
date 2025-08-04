// dot001-parser/src/lib.rs

//! # dot001-parser
//!
//! Low-level parsing engine for Blender .blend files.
//!
//! This crate provides the foundational parsing capabilities for .blend files,
//! including header parsing, block reading, DNA interpretation, and structured
//! field access.
//!
//! ## Key Features
//!
//! - **Cross-version compatibility**: Handles Blender 2.79 through 5.0+
//! - **DNA-based field reading**: Type-safe access to structured data
//! - **Efficient block access**: Direct block reading with address indexing
//! - **Memory safety**: All parsing operations are bounds-checked
//!
//! ## Architecture
//!
//! The parser follows a layered approach:
//! 1. **Header parsing**: Extracts file metadata and version information
//! 2. **Block enumeration**: Maps all data blocks and builds address index
//! 3. **DNA parsing**: Reads data structure definitions
//! 4. **Field access**: Provides structured access to block data
//!
//! This design enables sophisticated dependency analysis while maintaining
//! performance and memory safety.

pub mod block;
pub mod compression;
pub mod dna;
pub mod error;
pub mod fields;
pub mod header;

pub use block::{BlendFileBlock, BlockHeader};
pub use compression::{CompressionKind, DecompressionMode, DecompressionPolicy, ParseOptions};
pub use dna::{DnaCollection, DnaField, DnaName, DnaStruct};
pub use error::{BlendFileErrorKind, Dot001Error, Result};
pub use fields::FieldReader;
pub use header::BlendFileHeader;

use std::collections::HashMap;
use std::io::{Cursor, Read, Seek, SeekFrom};

/// A trait for readable and seekable sources that can be sent across threads
pub trait ReadSeekSend: Read + Seek + Send {}

// Blanket implementation for all types that implement the required traits
impl<T: Read + Seek + Send> ReadSeekSend for T {}

/// Main parser for .blend files
pub struct BlendFile<R: Read + Seek> {
    pub reader: R,
    pub header: BlendFileHeader,
    pub blocks: Vec<BlendFileBlock>,
    pub dna: Option<DnaCollection>,
    pub block_index: HashMap<[u8; 4], Vec<usize>>,
    pub address_index: HashMap<u64, usize>,
}

impl<R: Read + Seek> BlendFile<R> {
    pub fn new(mut reader: R) -> Result<Self> {
        // Check if file is zstd compressed by reading magic bytes
        let mut magic_bytes = [0u8; 4];
        reader.read_exact(&mut magic_bytes)?;
        reader.seek(SeekFrom::Start(0))?;

        // Zstandard magic number is 0x28B52FFD (little endian: FD 2F B5 28)
        if magic_bytes == [0x28, 0xB5, 0x2F, 0xFD] {
            return Err(Dot001Error::blend_file("Zstandard-compressed blend files require decompression first. Use 'zstd -d' to decompress the file.", BlendFileErrorKind::UnsupportedCompression));
        }

        let header = BlendFileHeader::read(&mut reader)?;

        let mut blend_file = BlendFile {
            reader,
            header,
            blocks: Vec::new(),
            dna: None,
            block_index: HashMap::new(),
            address_index: HashMap::new(),
        };

        blend_file.read_blocks()?;
        blend_file.read_dna()?;
        blend_file.build_block_index();

        Ok(blend_file)
    }

    fn read_blocks(&mut self) -> Result<()> {
        self.reader
            .seek(SeekFrom::Start(self.header.header_size() as u64))?;

        loop {
            // Record the header start offset before reading the BlockHeader
            let header_offset = self.reader.stream_position()?;
            let block_header = BlockHeader::read(&mut self.reader, &self.header)?;
            let data_offset = self.reader.stream_position()?;

            if &block_header.code == b"ENDB" {
                break;
            }

            let block_size = block_header.size;
            let block = BlendFileBlock {
                header: block_header,
                data_offset,
                header_offset,
            };
            self.blocks.push(block);

            // Seek past the block's data to the next header
            self.reader.seek(SeekFrom::Current(block_size as i64))?;
        }

        Ok(())
    }

    fn read_dna(&mut self) -> Result<()> {
        let dna_block = self
            .blocks
            .iter()
            .find(|block| &block.header.code == b"DNA1")
            .ok_or_else(|| {
                Dot001Error::blend_file("DNA block not found", BlendFileErrorKind::NoDnaFound)
            })?;

        self.reader.seek(SeekFrom::Start(dna_block.data_offset))?;
        let dna = DnaCollection::read(&mut self.reader, &self.header)?;
        self.dna = Some(dna);

        Ok(())
    }

    fn build_block_index(&mut self) {
        self.block_index.reserve(32);
        self.address_index.reserve(self.blocks.len());

        for (i, block) in self.blocks.iter().enumerate() {
            self.block_index
                .entry(block.header.code)
                .or_default()
                .push(i);
            self.address_index.insert(block.header.old_address, i);
        }
    }

    /// Compute a deterministic content hash for the given block.
    /// Hash includes sdna_index, count, size, and raw block bytes.
    /// Uses xxhash64 for speed and stability.
    pub fn block_content_hash(&mut self, block_index: usize) -> Result<u64> {
        use std::hash::Hasher;
        let block = self.blocks.get(block_index).ok_or_else(|| {
            Dot001Error::blend_file(
                format!("Invalid block index: {block_index}"),
                BlendFileErrorKind::InvalidBlockIndex,
            )
        })?;
        let mut hasher = twox_hash::XxHash64::with_seed(0);
        hasher.write(&block.header.sdna_index.to_le_bytes());
        hasher.write(&block.header.count.to_le_bytes());
        hasher.write(&block.header.size.to_le_bytes());
        // Include code for extra discrimination
        hasher.write(&block.header.code);
        let data = self.read_block_data(block_index)?;
        hasher.write(&data);
        Ok(hasher.finish())
    }

    /// Read the raw data for a specific block
    pub fn read_block_data(&mut self, block_index: usize) -> Result<Vec<u8>> {
        let block = self.blocks.get(block_index).ok_or_else(|| {
            Dot001Error::blend_file(
                format!("Invalid block index: {block_index}"),
                BlendFileErrorKind::InvalidBlockIndex,
            )
        })?;

        let mut data = vec![0u8; block.header.size as usize];
        self.reader.seek(SeekFrom::Start(block.data_offset))?;
        self.reader.read_exact(&mut data)?;
        Ok(data)
    }

    /// Get block by its memory address (pointer value)
    pub fn find_block_by_address(&self, address: u64) -> Option<usize> {
        self.address_index.get(&address).copied()
    }

    /// Get all blocks of a specific type
    pub fn blocks_by_type(&self, block_type: &[u8; 4]) -> Vec<usize> {
        self.block_index
            .get(block_type)
            .cloned()
            .unwrap_or_default()
    }

    /// Get DNA collection (required for field reading)
    pub fn dna(&self) -> Result<&DnaCollection> {
        self.dna.as_ref().ok_or_else(|| {
            Dot001Error::blend_file("DNA block not found", BlendFileErrorKind::NoDnaFound)
        })
    }

    /// Get block header by index
    pub fn get_block(&self, index: usize) -> Option<&BlendFileBlock> {
        self.blocks.get(index)
    }

    /// Create a field reader for a specific block
    pub fn create_field_reader<'a>(
        &'a self,
        block_data: &'a [u8],
    ) -> Result<crate::fields::FieldReader<'a>> {
        let dna = self.dna()?;
        let reader = crate::fields::FieldReader::new(
            block_data,
            dna,
            self.header.pointer_size as usize,
            self.header.is_little_endian,
        );
        Ok(reader)
    }
}

/// Create a BlendFile from a file path, automatically handling zstd compression
///
/// This is a backward-compatible function that uses default decompression policy.
/// For more control, use `parse_from_path` instead.
pub fn from_path<P: AsRef<std::path::Path>>(path: P) -> Result<BlendFile<Cursor<Vec<u8>>>> {
    let mut file = std::fs::File::open(path)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;

    // Check if file is zstd compressed
    if buffer.len() >= 4 && buffer[0..4] == [0x28, 0xB5, 0x2F, 0xFD] {
        // Decompress with zstd
        #[cfg(feature = "zstd")]
        {
            let decompressed = zstd::decode_all(&buffer[..])?;
            let cursor = Cursor::new(decompressed);
            BlendFile::new(cursor)
        }
        #[cfg(not(feature = "zstd"))]
        {
            Err(Dot001Error::blend_file(
                "Zstd support not compiled in",
                BlendFileErrorKind::UnsupportedCompression,
            ))
        }
    } else {
        let cursor = Cursor::new(buffer);
        BlendFile::new(cursor)
    }
}

/// Parse a blend file from a path with transparent decompression handling
pub fn parse_from_path<P: AsRef<std::path::Path>>(
    path: P,
    options: Option<&ParseOptions>,
) -> Result<(BlendFile<Box<dyn ReadSeekSend>>, DecompressionMode)> {
    use compression::{create_reader, open_source};

    let default_options = ParseOptions::default();
    let options = options.unwrap_or(&default_options);
    let blend_read = open_source(path, Some(&options.decompression_policy))?;

    // Determine the decompression mode based on what we got back
    let mode = match &blend_read {
        compression::BlendRead::Memory(_) => DecompressionMode::ZstdInMemory,
        #[cfg(feature = "mmap")]
        compression::BlendRead::TempMmap(_, _) => DecompressionMode::ZstdTempMmap,
        compression::BlendRead::TempFile(_, _) => DecompressionMode::ZstdTempFile,
        compression::BlendRead::File(_) => DecompressionMode::None,
    };

    let reader = create_reader(blend_read)?;
    let blend_file = BlendFile::new(reader)?;

    Ok((blend_file, mode))
}

/// Parse a blend file from a reader with transparent decompression handling
pub fn parse_from_reader<R: Read + Seek + Send + 'static>(
    mut reader: R,
    options: Option<&ParseOptions>,
) -> Result<(BlendFile<Box<dyn ReadSeekSend>>, DecompressionMode)> {
    use compression::{
        create_reader, detect_compression, CompressionKind, Decompressor, ZstdDecompressor,
    };

    let default_options = ParseOptions::default();
    let options = options.unwrap_or(&default_options);
    let compression = detect_compression(&mut reader)?;

    let (blend_read, mode) = match compression {
        CompressionKind::None => {
            // Box the original reader directly
            let boxed_reader: Box<dyn ReadSeekSend> = Box::new(reader);
            let blend_file = BlendFile::new(boxed_reader)?;
            return Ok((blend_file, DecompressionMode::None));
        }
        CompressionKind::Zstd => {
            let decompressor = ZstdDecompressor;
            let blend_read =
                decompressor.decompress(&mut reader, None, &options.decompression_policy)?;

            let mode = match &blend_read {
                compression::BlendRead::Memory(_) => DecompressionMode::ZstdInMemory,
                #[cfg(feature = "mmap")]
                compression::BlendRead::TempMmap(_, _) => DecompressionMode::ZstdTempMmap,
                compression::BlendRead::TempFile(_, _) => DecompressionMode::ZstdTempFile,
                compression::BlendRead::File(_) => DecompressionMode::None, // Shouldn't happen for zstd
            };

            (blend_read, mode)
        }
    };

    let reader = create_reader(blend_read)?;
    let blend_file = BlendFile::new(reader)?;

    Ok((blend_file, mode))
}
