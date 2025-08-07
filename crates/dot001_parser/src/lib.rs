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
pub mod fieldview;
pub mod header;
pub mod index;
pub mod interner;
pub mod name_resolver;
pub mod policy;
pub mod reflect;
pub mod scan;

// CORE TYPES - New Architecture
pub use block::{BlendFileBlock, BlockHeader, block_code_to_string};
pub use buf::{BlendBuf, BlendSlice, BlendSource};
pub use compact_dna::{CompactDnaCollection, CompactDnaField, CompactDnaStruct};
pub use compression::{CompressionKind, DecompressionMode, DecompressionPolicy, ParseOptions};
pub use error::{BlendFileErrorKind, Error, Result};
pub use fieldview::{FieldView, FieldViewExt};
pub use header::BlendFileHeader;
pub use name_resolver::NameResolver;
pub use policy::{
    DataBlockCompareMode, DataBlockVisibility, is_block_visible, is_data_block_code,
    is_data_size_change_significant,
};
pub use reflect::PointerTraversal;

// REMOVED: Legacy APIs have been completely removed
// - DnaCollection -> Use CompactDnaCollection with zero-copy access
// - FieldReader -> Use FieldView for zero-copy field access

// RE-EXPORT FOR PRIMARY API
pub use BlendFileBuf as BlendFile;

use dot001_events::{
    event::{Event, ParserEvent},
    prelude::*,
};
use log::{debug, trace, warn};
use std::io::Cursor;

use crate::{
    dna::DnaCollection,
    index::{AddressIndex, AddressIndexExt, BlockIndex, BlockIndexExt, build_indices},
    scan::scan_blocks,
};

/// Maximum block size allowed for memory allocation safety (100MB default)
pub const DEFAULT_MAX_BLOCK_SIZE: u32 = 100_000_000;

// REMOVED: Legacy streaming parser and functions
// All streaming-based parsing has been removed in favor of zero-copy BlendFileBuf.
// See `from_path()` and `parse_from_path_buf()` for the new buffer-based API.

/// Parse a blend file from a path using the zero-copy BlendFileBuf
///
/// This is the primary parsing function that automatically handles compression
/// and chooses the most efficient buffer strategy based on the file and platform capabilities.
/// Provides zero-copy access with memory mapping when possible.
pub fn from_path<P: AsRef<std::path::Path>>(path: P) -> Result<BlendFileBuf> {
    let (blend_file, _mode) = parse_from_path_buf(path, None)?;
    Ok(blend_file)
}

/// Parse a blend file from a path with full options control  
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

    // REMOVED: create_field_reader methods
    // FieldReader has been completely removed in favor of FieldView for zero-copy access.
    // Use create_field_view() instead.

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
