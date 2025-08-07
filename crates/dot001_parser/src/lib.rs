// dot001_parser/src/lib.rs

//! # dot001_parser
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
//! 2. **Block enumeration**: Maps all data-blocks and builds address index
//! 3. **DNA parsing**: Reads data structure definitions
//! 4. **Field access**: Provides structured access to block data
//!
//! This design enables sophisticated dependency analysis while maintaining
//! performance and memory safety.

pub mod block;
pub mod buf;
pub mod compact_dna;
pub mod compression;
pub mod dna;
pub mod error;
pub mod fields;
pub mod fieldview;
pub mod header;
pub mod index;
pub mod interner;
pub mod name_resolver;
pub mod policy;
pub mod reflect;
pub mod scan;

pub use block::{BlendFileBlock, BlockHeader, block_code_to_string};
pub use buf::{BlendBuf, BlendSlice, BlendSource};
pub use compact_dna::{CompactDnaCollection, CompactDnaField, CompactDnaStruct};
pub use compression::{CompressionKind, DecompressionMode, DecompressionPolicy, ParseOptions};
pub use dna::{DnaCollection, DnaField, DnaName, DnaStruct};
pub use error::{BlendFileErrorKind, Error, Result};
pub use fields::FieldReader;
pub use fieldview::{FieldView, FieldViewExt};
pub use header::BlendFileHeader;
pub use name_resolver::NameResolver;
pub use policy::{
    DataBlockCompareMode, DataBlockVisibility, is_block_visible, is_data_block_code,
    is_data_size_change_significant,
};
pub use reflect::PointerTraversal;

use dot001_events::{
    event::{Event, ParserEvent},
    prelude::*,
};
use log::{debug, trace, warn};
use std::collections::HashMap;
use std::io::{Cursor, Read, Seek, SeekFrom};

use crate::{
    index::{AddressIndex, AddressIndexExt, BlockIndex, BlockIndexExt, build_indices},
    scan::scan_blocks,
};

/// A trait for readable and seekable sources that can be sent across threads
pub trait ReadSeekSend: Read + Seek + Send {}

// Blanket implementation for all types that implement the required traits
impl<T: Read + Seek + Send> ReadSeekSend for T {}

/// Maximum block size allowed for memory allocation safety (100MB default)
const DEFAULT_MAX_BLOCK_SIZE: u32 = 100_000_000;

/// Main parser for .blend files (streaming, legacy path)
///
/// This is the original Read+Seek based parser that works with any reader.
/// For better performance, use BlendFileBuf when working with files that can be mapped.
pub struct BlendFile<R: Read + Seek> {
    reader: R,
    header: BlendFileHeader,
    blocks: Vec<BlendFileBlock>,
    dna: Option<DnaCollection>,
    block_index: HashMap<[u8; 4], Vec<usize>>,
    address_index: HashMap<u64, usize>,
}

impl<R: Read + Seek> BlendFile<R> {
    /// Get the current maximum allowed block size
    pub fn max_block_size(&self) -> u32 {
        // Currently we store no per-instance override; return default for API stability.
        // This is kept to provide a stable surface while we plumb ParseOptions-based override below.
        DEFAULT_MAX_BLOCK_SIZE
    }
    /// Access to the header information
    pub fn header(&self) -> &BlendFileHeader {
        &self.header
    }

    /// Get the number of blocks in the file
    pub fn blocks_len(&self) -> usize {
        self.blocks.len()
    }

    /// Get an iterator over block types of a specific kind
    pub fn blocks_by_type_iter(&self, code: &[u8; 4]) -> impl Iterator<Item = usize> + '_ {
        self.block_index.get(code).into_iter().flatten().copied()
    }

    /// Get mutable access to the reader (needed for certain operations)
    pub fn reader_mut(&mut self) -> &mut R {
        &mut self.reader
    }

    pub fn new(mut reader: R) -> Result<Self> {
        trace!("Starting BlendFile parsing");
        // Check if file is zstd compressed by reading magic bytes
        let mut magic_bytes = [0u8; 4];
        reader.read_exact(&mut magic_bytes)?;
        reader.seek(SeekFrom::Start(0))?;

        // Zstandard magic number is 0x28B52FFD (little endian: FD 2F B5 28)
        if magic_bytes == [0x28, 0xB5, 0x2F, 0xFD] {
            warn!("Attempted to parse zstd-compressed file without decompression");
            return Err(Error::blend_file(
                "Zstandard-compressed blend files require decompression first. Use 'zstd -d' to decompress the file.",
                BlendFileErrorKind::UnsupportedCompression,
            ));
        }

        debug!("Reading blend file header");
        let header = BlendFileHeader::read(&mut reader)?;
        trace!(
            "Header parsed successfully: version={}, pointer_size={}",
            header.version, header.pointer_size
        );

        // Emit header parsed event
        let endianness = if header.is_little_endian {
            "little"
        } else {
            "big"
        };
        emit_global_sync!(Event::Parser(ParserEvent::HeaderParsed {
            version: header.version.to_string(),
            endianness: endianness.to_string(),
            pointer_size: header.pointer_size,
        }));

        let mut blend_file = BlendFile {
            reader,
            header,
            blocks: Vec::new(),
            dna: None,
            block_index: HashMap::new(),
            address_index: HashMap::new(),
        };

        debug!("Reading file blocks");
        blend_file.read_blocks()?;
        debug!("Reading DNA structures");
        blend_file.read_dna()?;
        debug!("Building block indices");
        blend_file.build_block_index();

        debug!(
            "BlendFile parsing completed: {} blocks, {} expanders indexed",
            blend_file.blocks.len(),
            blend_file.block_index.len()
        );
        Ok(blend_file)
    }

    fn read_blocks(&mut self) -> Result<()> {
        self.reader
            .seek(SeekFrom::Start(self.header.header_size() as u64))?;

        let mut block_count = 0;
        loop {
            // Record the header start offset before reading the BlockHeader
            let header_offset = self.reader.stream_position()?;
            let block_header = BlockHeader::read(&mut self.reader, &self.header)?;
            let data_offset = self.reader.stream_position()?;

            if &block_header.code == b"ENDB" {
                trace!("Found ENDB marker, finished reading {block_count} blocks");
                break;
            }

            let block_size = block_header.size;
            let block = BlendFileBlock {
                header: block_header,
                data_offset,
                header_offset,
            };

            // Emit block parsed event (at trace level)
            let block_type = String::from_utf8_lossy(&block.header.code).to_string();
            emit_global_sync!(
                Event::Parser(ParserEvent::BlockParsed {
                    index: block_count,
                    block_type,
                    size: block.header.size as usize,
                }),
                Severity::Trace
            );

            self.blocks.push(block);
            block_count += 1;

            if block_count % 1000 == 0 {
                trace!("Read {block_count} blocks so far");
            }

            // Seek past the block's data to the next header
            self.reader.seek(SeekFrom::Current(block_size as i64))?;
        }

        debug!("Read {block_count} total blocks");
        Ok(())
    }

    fn read_dna(&mut self) -> Result<()> {
        let dna_block = self
            .blocks
            .iter()
            .find(|block| &block.header.code == b"DNA1")
            .ok_or_else(|| {
                Error::blend_file("DNA block not found", BlendFileErrorKind::NoDnaFound)
            })?;

        self.reader.seek(SeekFrom::Start(dna_block.data_offset))?;
        let dna = DnaCollection::read(&mut self.reader, &self.header)?;

        // Emit DNA parsed event
        emit_global_sync!(Event::Parser(ParserEvent::DnaParsed {
            struct_count: dna.structs.len(),
            name_count: dna.names.len(),
        }));

        self.dna = Some(dna);

        Ok(())
    }

    fn build_block_index(&mut self) {
        const INITIAL_BLOCK_INDEX_CAPACITY: usize = 32;
        self.block_index.reserve(INITIAL_BLOCK_INDEX_CAPACITY);
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
            Error::blend_file(
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
            Error::blend_file(
                format!("Invalid block index: {block_index}"),
                BlendFileErrorKind::InvalidBlockIndex,
            )
        })?;

        // Validate block size to prevent excessive memory allocation
        let limit = self.max_block_size();
        if block.header.size > limit {
            return Err(Error::blend_file(
                format!(
                    "Block size too large: {} bytes (limit {})",
                    block.header.size, limit
                ),
                BlendFileErrorKind::SizeLimitExceeded,
            ));
        }
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
        self.dna
            .as_ref()
            .ok_or_else(|| Error::blend_file("DNA block not found", BlendFileErrorKind::NoDnaFound))
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
            Err(Error::blend_file(
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

    let path = path.as_ref();

    // Emit parsing started event
    let file_size = std::fs::metadata(path).ok().map(|m| m.len());
    emit_global_sync!(Event::Parser(ParserEvent::Started {
        input: path.to_path_buf(),
        file_size,
    }));

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
    let start_time = std::time::Instant::now();
    let blend_file = BlendFile::new(reader)?;

    // Emit completion event
    let duration_ms = start_time.elapsed().as_millis() as u64;
    emit_global_sync!(Event::Parser(ParserEvent::Finished {
        total_blocks: blend_file.blocks.len(),
        total_size: std::fs::metadata(path).ok().map(|m| m.len()).unwrap_or(0),
        duration_ms,
    }));

    Ok((blend_file, mode))
}

/// Parse a blend file from a reader with transparent decompression handling
pub fn parse_from_reader<R: Read + Seek + Send + 'static>(
    mut reader: R,
    options: Option<&ParseOptions>,
) -> Result<(BlendFile<Box<dyn ReadSeekSend>>, DecompressionMode)> {
    use compression::{
        CompressionKind, Decompressor, ZstdDecompressor, create_reader, detect_compression,
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

/// Parse a blend file from a path using the zero-copy BlendFileBuf (fast path)
///
/// This function automatically handles compression and chooses the most efficient
/// buffer strategy based on the file and platform capabilities.
pub fn parse_from_path_buf<P: AsRef<std::path::Path>>(
    path: P,
    options: Option<&ParseOptions>,
) -> Result<(BlendFileBuf, DecompressionMode)> {
    use compression::{create_buffer_from_source, open_source};

    let path = path.as_ref();

    // Emit parsing started event
    let file_size = std::fs::metadata(path).ok().map(|m| m.len());
    emit_global_sync!(Event::Parser(ParserEvent::Started {
        input: path.to_path_buf(),
        file_size,
    }));

    let default_options = ParseOptions::default();
    let options = options.unwrap_or(&default_options);
    let blend_read = open_source(path, Some(&options.decompression_policy))?;

    // Determine the decompression mode
    let mode = match &blend_read {
        compression::BlendRead::Memory(_) => DecompressionMode::ZstdInMemory,
        #[cfg(feature = "mmap")]
        compression::BlendRead::TempMmap(_, _) => DecompressionMode::ZstdTempMmap,
        compression::BlendRead::TempFile(_, _) => DecompressionMode::ZstdTempFile,
        compression::BlendRead::File(_) => DecompressionMode::None,
    };

    // Create buffer from the source
    let buf = create_buffer_from_source(blend_read)?;

    let start_time = std::time::Instant::now();
    let blend_file = BlendFileBuf::new(buf)?;

    // Emit completion event
    let duration_ms = start_time.elapsed().as_millis() as u64;
    emit_global_sync!(Event::Parser(ParserEvent::Finished {
        total_blocks: blend_file.blocks.len(),
        total_size: std::fs::metadata(path).ok().map(|m| m.len()).unwrap_or(0),
        duration_ms,
    }));

    Ok((blend_file, mode))
}

/// Create a BlendFileBuf directly from a Vec<u8>
pub fn from_bytes_buf(data: Vec<u8>) -> Result<BlendFileBuf> {
    let buf = BlendBuf::from_vec(data);
    BlendFileBuf::new(buf)
}

/// Create a BlendFileBuf from Bytes
pub fn from_bytes_slice_buf(bytes: bytes::Bytes) -> Result<BlendFileBuf> {
    let buf = BlendBuf::from_bytes(bytes);
    BlendFileBuf::new(buf)
}

/// Zero-copy, buffer-backed blend file parser (fast path)
///
/// BlendFileBuf is the high-performance variant that operates on memory buffers
/// for zero-copy block access. It uses memory mapping when possible and provides
/// significantly better performance than the streaming BlendFile variant.
pub struct BlendFileBuf {
    buf: BlendBuf,
    header: BlendFileHeader,
    blocks: Vec<BlendFileBlock>,
    dna: Option<DnaCollection>,
    /// Optimized compact DNA for better performance (optional)
    compact_dna: Option<CompactDnaCollection>,
    block_index: BlockIndex,
    address_index: AddressIndex,
}

impl BlendFileBuf {
    /// Create a new BlendFileBuf from a buffer
    pub fn new(buf: BlendBuf) -> Result<Self> {
        trace!("Starting BlendFileBuf parsing (zero-copy path)");

        let data = buf.as_slice();

        // Check for zstd compression magic bytes
        if data.len() >= 4 && data[0..4] == [0x28, 0xB5, 0x2F, 0xFD] {
            warn!("Attempted to parse zstd-compressed data without decompression");
            return Err(Error::blend_file(
                "Zstandard-compressed blend data requires decompression first.",
                BlendFileErrorKind::UnsupportedCompression,
            ));
        }

        debug!("Reading blend file header from buffer");

        // Parse header from buffer slice
        if data.len() < 12 {
            return Err(Error::blend_file(
                "File too small to contain header",
                BlendFileErrorKind::InvalidHeader,
            ));
        }

        let header = BlendFileHeader::read_from_slice(data)?;
        trace!(
            "Header parsed successfully: version={}, pointer_size={}",
            header.version, header.pointer_size
        );

        // Emit header parsed event
        let endianness = if header.is_little_endian {
            "little"
        } else {
            "big"
        };
        emit_global_sync!(Event::Parser(ParserEvent::HeaderParsed {
            version: header.version.to_string(),
            endianness: endianness.to_string(),
            pointer_size: header.pointer_size,
        }));

        debug!("Scanning file blocks from buffer");
        let blocks = scan_blocks(data, header.header_size(), &header)?;

        debug!("Reading DNA structures from buffer");
        let dna = Self::read_dna_from_buffer(&buf, &blocks, &header)?;

        // Emit DNA parsed event
        emit_global_sync!(Event::Parser(ParserEvent::DnaParsed {
            struct_count: dna.structs.len(),
            name_count: dna.names.len(),
        }));

        debug!("Building block indices");
        let (block_index, address_index) = build_indices(&blocks);

        debug!(
            "BlendFileBuf parsing completed: {} blocks, {} block types indexed",
            blocks.len(),
            block_index.len()
        );

        Ok(BlendFileBuf {
            buf,
            header,
            blocks,
            dna: Some(dna),
            compact_dna: None, // Will be created on-demand
            block_index,
            address_index,
        })
    }

    /// Read DNA structures from the buffer
    fn read_dna_from_buffer(
        buf: &BlendBuf,
        blocks: &[BlendFileBlock],
        header: &BlendFileHeader,
    ) -> Result<DnaCollection> {
        let dna_block = blocks
            .iter()
            .find(|block| &block.header.code == b"DNA1")
            .ok_or_else(|| {
                Error::blend_file("DNA block not found", BlendFileErrorKind::NoDnaFound)
            })?;

        let dna_data = buf.slice(
            dna_block.data_offset as usize
                ..(dna_block.data_offset + dna_block.header.size as u64) as usize,
        )?;

        let mut cursor = Cursor::new(dna_data.as_ref());
        DnaCollection::read(&mut cursor, header)
    }

    /// Access to the header information
    pub fn header(&self) -> &BlendFileHeader {
        &self.header
    }

    /// Get the number of blocks in the file
    pub fn blocks_len(&self) -> usize {
        self.blocks.len()
    }

    /// Get all blocks of a specific type
    pub fn blocks_by_type(&self, block_type: &[u8; 4]) -> Vec<usize> {
        self.block_index.blocks_by_type(block_type)
    }

    /// Get an iterator over block indices of a specific type
    pub fn blocks_by_type_iter(&self, code: &[u8; 4]) -> impl Iterator<Item = usize> + '_ {
        self.blocks_by_type(code).into_iter()
    }

    /// Get block by index
    pub fn get_block(&self, index: usize) -> Option<&BlendFileBlock> {
        self.blocks.get(index)
    }

    /// Get DNA collection (required for field reading)
    pub fn dna(&self) -> Result<&DnaCollection> {
        self.dna
            .as_ref()
            .ok_or_else(|| Error::blend_file("DNA block not found", BlendFileErrorKind::NoDnaFound))
    }

    /// Get block by its memory address (pointer value)
    pub fn find_block_by_address(&self, address: u64) -> Option<usize> {
        self.address_index.find_block_by_address(address)
    }

    /// Read block data as a zero-copy Bytes slice
    ///
    /// This is the primary method for accessing block data in the zero-copy path.
    /// Returns a Bytes slice that shares the underlying buffer data.
    pub fn read_block_slice(&self, block_index: usize) -> Result<BlendSlice> {
        let block = self.blocks.get(block_index).ok_or_else(|| {
            Error::blend_file(
                format!("Invalid block index: {block_index}"),
                BlendFileErrorKind::InvalidBlockIndex,
            )
        })?;

        let start = block.data_offset as usize;
        let end = start + block.header.size as usize;

        self.buf.slice(start..end)
    }

    /// Read block data as a Vec<u8> (compatibility method)
    ///
    /// This method copies the data and should be used when Vec<u8> is specifically needed.
    /// For zero-copy access, prefer read_block_slice.
    pub fn read_block_data(&self, block_index: usize) -> Result<Vec<u8>> {
        let slice = self.read_block_slice(block_index)?;
        Ok(slice.to_vec())
    }

    /// Compute a deterministic content hash for the given block using zero-copy access
    pub fn block_content_hash(&self, block_index: usize) -> Result<u64> {
        use std::hash::Hasher;
        let block = self.blocks.get(block_index).ok_or_else(|| {
            Error::blend_file(
                format!("Invalid block index: {block_index}"),
                BlendFileErrorKind::InvalidBlockIndex,
            )
        })?;

        let mut hasher = twox_hash::XxHash64::with_seed(0);
        hasher.write(&block.header.sdna_index.to_le_bytes());
        hasher.write(&block.header.count.to_le_bytes());
        hasher.write(&block.header.size.to_le_bytes());
        hasher.write(&block.header.code);

        let data = self.read_block_slice(block_index)?;
        hasher.write(data.as_ref());
        Ok(hasher.finish())
    }

    /// Create a field reader for a specific block using zero-copy access
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

    /// Create a field reader for a block by index using zero-copy access
    ///
    /// Note: This method requires the caller to manage the lifetime of the block data.
    /// For convenience, use read_block_slice() first, then create_field_reader() with the slice.
    pub fn create_field_reader_for_block<'a>(
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

    /// Create a high-performance FieldView for zero-copy block access
    ///
    /// This is the preferred method for accessing block data in performance-critical code.
    /// It provides faster field access than the compatibility FieldReader.
    pub fn create_field_view<'a>(&'a self, block_data: &'a BlendSlice) -> Result<FieldView<'a>> {
        let dna = self.dna()?;
        Ok(FieldView::from_bytes(
            block_data,
            dna,
            self.header.pointer_size as usize,
            self.header.is_little_endian,
        ))
    }

    /// Read block data as a slice for creating field views
    ///
    /// This is a convenience method that returns the block data slice
    /// that can then be used with create_field_view.
    ///
    /// Due to Rust lifetime constraints, the slice and FieldView must be
    /// created separately. Use this pattern:
    /// ```ignore
    /// let slice = blend_file.read_block_slice_for_field_view(block_index)?;
    /// let view = blend_file.create_field_view(&slice)?;
    /// ```
    pub fn read_block_slice_for_field_view(&self, block_index: usize) -> Result<BlendSlice> {
        self.read_block_slice(block_index)
    }

    /// Get or create the compact DNA collection
    ///
    /// This creates an optimized DNA representation on first access and caches it.
    /// The compact DNA uses string interning and provides faster lookups.
    pub fn compact_dna(&mut self) -> Result<&CompactDnaCollection> {
        if self.compact_dna.is_none() {
            let original_dna = self.dna()?;
            let compact = CompactDnaCollection::from_original(original_dna);

            debug!("Created compact DNA: {}", compact.memory_stats());
            self.compact_dna = Some(compact);
        }

        Ok(self.compact_dna.as_ref().unwrap())
    }

    /// Create a FieldView using compact DNA (most optimized path)
    ///
    /// This uses the compact DNA representation for the fastest possible field access.
    /// The compact DNA must be created first via compact_dna().
    ///
    /// TODO: Currently falls back to regular DNA. Future enhancement will implement
    /// native CompactFieldView for even better performance.
    pub fn create_field_view_compact<'a>(
        &'a mut self,
        block_data: &'a BlendSlice,
    ) -> Result<FieldView<'a>> {
        let _compact_dna = self.compact_dna()?;
        // Convert compact DNA back to regular DNA for FieldView
        // TODO: Create a native CompactFieldView for even better performance
        let dna = self.dna()?;
        Ok(FieldView::from_bytes(
            block_data,
            dna,
            self.header.pointer_size as usize,
            self.header.is_little_endian,
        ))
    }

    /// Convert an address to block index for dependency resolution
    ///
    /// This looks up an address in the address index to find the corresponding block.
    /// Returns None if the address is not found.
    pub fn address_to_block_index(&self, address: u64) -> Option<usize> {
        self.address_index.get(&address).copied()
    }
}
