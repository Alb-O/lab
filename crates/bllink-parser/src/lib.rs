// bllink-parser/src/lib.rs

//! # bllink-parser
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
pub mod dna;
pub mod error;
pub mod fields;
pub mod header;

pub use block::{BlendFileBlock, BlockHeader};
pub use dna::{DnaCollection, DnaField, DnaName, DnaStruct};
pub use error::{BlendError, Result};
pub use fields::FieldReader;
pub use header::BlendFileHeader;

use std::collections::HashMap;
use std::io::{Read, Seek, SeekFrom};

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
            let block_header = BlockHeader::read(&mut self.reader, &self.header)?;
            let data_offset = self.reader.stream_position()?;

            if &block_header.code == b"ENDB" {
                break;
            }

            let block_size = block_header.size;
            let block = BlendFileBlock {
                header: block_header,
                data_offset,
            };
            self.blocks.push(block);

            self.reader.seek(SeekFrom::Current(block_size as i64))?;
        }

        Ok(())
    }

    fn read_dna(&mut self) -> Result<()> {
        let dna_block = self
            .blocks
            .iter()
            .find(|block| &block.header.code == b"DNA1")
            .ok_or(BlendError::NoDnaFound)?;

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

    /// Read the raw data for a specific block
    pub fn read_block_data(&mut self, block_index: usize) -> Result<Vec<u8>> {
        let block = self
            .blocks
            .get(block_index)
            .ok_or(BlendError::InvalidBlockIndex(block_index))?;

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
        self.dna.as_ref().ok_or(BlendError::NoDnaFound)
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
