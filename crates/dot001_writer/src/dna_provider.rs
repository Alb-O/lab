use dot001_error::{Dot001Error, Result};
use dot001_parser::{BlendFile, ReadSeekSend};
use std::fs::File;
use std::io::{Cursor, Read, Seek, SeekFrom};

/// Seed provider that extracts essential blocks (REND, TEST, GLOB, DNA1) and parsed DnaCollection
/// from a user-provided 5.0-alpha .blend file.
pub struct SeedDnaProvider {
    /// Raw DNA1 block bytes as they should be written into the output file.
    raw_dna_bytes: Vec<u8>,
    /// Raw REND block bytes (render settings).
    raw_rend_bytes: Vec<u8>,
    /// Raw TEST block bytes.
    raw_test_bytes: Vec<u8>,
    /// Raw GLOB block bytes (global settings).
    raw_glob_bytes: Vec<u8>,
    /// Parsed DNA for struct lookups and sdna indices.
    dna: dot001_parser::DnaCollection,
    /// Source file path for re-reading blocks.
    source_path: std::path::PathBuf,
}

impl SeedDnaProvider {
    /// Load essential blocks from a seed .blend path.
    /// This will parse the file header and blocks, locate REND, TEST, GLOB, and DNA1 blocks,
    /// capture their raw payloads, and parse DNA into a DnaCollection.
    pub fn from_seed_path<P: AsRef<std::path::Path>>(path: P) -> Result<Self> {
        let source_path = path.as_ref().to_path_buf();

        // Read entire file into memory to allow precise slicing of block payloads later.
        let mut f = File::open(&path)?;
        let mut buf = Vec::new();
        f.read_to_end(&mut buf)?;
        let cursor = Cursor::new(buf);

        let mut blend_file = BlendFile::new(Box::new(cursor) as Box<dyn ReadSeekSend>)?;

        // Helper function to extract block bytes
        let mut extract_block = |block_type: &[u8; 4]| -> Result<Vec<u8>> {
            let block_index = blend_file
                .blocks_by_type(block_type)
                .into_iter()
                .next()
                .ok_or_else(|| {
                    Dot001Error::blend_file(
                        format!(
                            "{} block not found in seed",
                            String::from_utf8_lossy(block_type)
                        ),
                        dot001_error::BlendFileErrorKind::NoDnaFound,
                    )
                })?;

            let block = blend_file
                .get_block(block_index)
                .ok_or_else(|| {
                    Dot001Error::blend_file(
                        format!(
                            "{} block index out of range",
                            String::from_utf8_lossy(block_type)
                        ),
                        dot001_error::BlendFileErrorKind::InvalidBlockIndex,
                    )
                })?
                .clone();

            let mut block_bytes = vec![0u8; block.header.size as usize];
            {
                let reader = blend_file.reader_mut();
                reader.seek(SeekFrom::Start(block.data_offset))?;
                reader.read_exact(&mut block_bytes)?;
            }
            Ok(block_bytes)
        };

        // Extract all essential blocks
        let raw_rend_bytes = extract_block(b"REND")?;
        let raw_test_bytes = extract_block(b"TEST")?;
        let raw_glob_bytes = extract_block(b"GLOB")?;
        let raw_dna_bytes = extract_block(b"DNA1")?;

        // Parse DNA for struct metadata
        let mut dna_reader = Cursor::new(raw_dna_bytes.clone());
        let dna = dot001_parser::DnaCollection::read(&mut dna_reader, blend_file.header())?;

        Ok(Self {
            raw_dna_bytes,
            raw_rend_bytes,
            raw_test_bytes,
            raw_glob_bytes,
            dna,
            source_path,
        })
    }

    /// Raw DNA bytes to write into the output's DNA1 block.
    pub fn raw_bytes(&self) -> &[u8] {
        &self.raw_dna_bytes
    }

    /// Raw REND block bytes (render settings).
    pub fn rend_bytes(&self) -> &[u8] {
        &self.raw_rend_bytes
    }

    /// Raw TEST block bytes.
    pub fn test_bytes(&self) -> &[u8] {
        &self.raw_test_bytes
    }

    /// Raw GLOB block bytes (global settings).
    pub fn glob_bytes(&self) -> &[u8] {
        &self.raw_glob_bytes
    }

    /// Parsed DNA collection for sdna queries.
    pub fn dna(&self) -> &dot001_parser::DnaCollection {
        &self.dna
    }

    /// Get the source file path for re-reading blocks.
    pub fn source_path(&self) -> &std::path::Path {
        &self.source_path
    }

    /// Find the SDNA struct index for a given struct type name.
    pub fn sdna_index_for_struct(&self, name: &str) -> Option<u32> {
        self.dna
            .structs
            .iter()
            .position(|s| s.type_name == name)
            .map(|i| i as u32)
    }

    /// Extract specific blocks by their indices from the source file.
    /// Returns a vector of (block_index, block_header, block_data) tuples.
    pub fn extract_blocks_by_indices(
        &mut self,
        indices: &[usize],
    ) -> Result<Vec<(usize, dot001_parser::BlockHeader, Vec<u8>)>> {
        // Re-read the file to access block data
        let mut f = File::open(&self.source_path)?;
        let mut buf = Vec::new();
        f.read_to_end(&mut buf)?;
        let cursor = Cursor::new(buf);

        let mut blend_file = BlendFile::new(Box::new(cursor) as Box<dyn ReadSeekSend>)?;

        let mut results = Vec::new();
        for &index in indices {
            let block = blend_file
                .get_block(index)
                .ok_or_else(|| {
                    Dot001Error::blend_file(
                        format!("Block index {index} out of range"),
                        dot001_error::BlendFileErrorKind::InvalidBlockIndex,
                    )
                })?
                .clone();

            let mut block_data = vec![0u8; block.header.size as usize];
            {
                let reader = blend_file.reader_mut();
                reader.seek(SeekFrom::Start(block.data_offset))?;
                reader.read_exact(&mut block_data)?;
            }

            results.push((index, block.header.clone(), block_data));
        }

        Ok(results)
    }
}
