use dot001_error::{Dot001Error, Result};
use dot001_parser::{BlendFile, ReadSeekSend};
use std::fs::File;
use std::io::{Cursor, Read, Seek, SeekFrom};

/// Seed DNA provider that extracts the raw DNA1 block bytes and parsed DnaCollection
/// from a user-provided 5.0-alpha .blend file.
pub struct SeedDnaProvider {
    /// Raw DNA1 block bytes as they should be written into the output file.
    raw_dna_bytes: Vec<u8>,
    /// Parsed DNA for struct lookups and sdna indices.
    dna: dot001_parser::DnaCollection,
}

impl SeedDnaProvider {
    /// Load DNA from a seed .blend path.
    /// This will parse the file header and blocks, locate the DNA1 block,
    /// capture its raw payload, and parse it into a DnaCollection.
    pub fn from_seed_path<P: AsRef<std::path::Path>>(path: P) -> Result<Self> {
        // Read entire file into memory to allow precise slicing of DNA1 payload later.
        let mut f = File::open(path)?;
        let mut buf = Vec::new();
        f.read_to_end(&mut buf)?;
        let cursor = Cursor::new(buf);

        let mut blend_file = BlendFile::new(Box::new(cursor) as Box<dyn ReadSeekSend>)?;

        // Locate DNA1 block in the parsed block list.
        let dna_block_index = blend_file
            .blocks_by_type(b"DNA1")
            .into_iter()
            .next()
            .ok_or_else(|| {
                Dot001Error::blend_file(
                    "DNA1 block not found in seed",
                    dot001_error::BlendFileErrorKind::NoDnaFound,
                )
            })?;

        // Grab block info so we can slice raw bytes
        let block = blend_file
            .get_block(dna_block_index)
            .ok_or_else(|| {
                Dot001Error::blend_file(
                    "DNA1 block index out of range",
                    dot001_error::BlendFileErrorKind::InvalidBlockIndex,
                )
            })?
            .clone();

        // Extract raw DNA payload
        let mut raw_dna = vec![0u8; block.header.size as usize];
        {
            let reader = blend_file.reader_mut();
            reader.seek(SeekFrom::Start(block.data_offset))?;
            reader.read_exact(&mut raw_dna)?;
        }

        // Parse DNA for struct metadata
        // Re-seek and re-read from the stored raw bytes using a cursor to avoid reusing the file cursor.
        let mut dna_reader = Cursor::new(raw_dna.clone());
        let dna = dot001_parser::DnaCollection::read(&mut dna_reader, blend_file.header())?;

        Ok(Self {
            raw_dna_bytes: raw_dna,
            dna,
        })
    }

    /// Raw DNA bytes to write into the output's DNA1 block.
    pub fn raw_bytes(&self) -> &[u8] {
        &self.raw_dna_bytes
    }

    /// Parsed DNA collection for sdna queries.
    pub fn dna(&self) -> &dot001_parser::DnaCollection {
        &self.dna
    }

    /// Find the SDNA struct index for a given struct type name.
    pub fn sdna_index_for_struct(&self, name: &str) -> Option<u32> {
        self.dna
            .structs
            .iter()
            .position(|s| s.type_name == name)
            .map(|i| i as u32)
    }
}
