use std::sync::Arc;

use crate::compress::maybe_decompress;
use crate::error::Error;
use crate::format::{self, BHead, BHeadType, Endian, Header, codes};
use crate::sdna::{SdnaInfo, decode_sdna};

// BHead now provided by format module

// Error type is provided by crate::error

pub struct BlendFile {
    data: Arc<[u8]>,
    pub header: Header,
    cursor: usize, // position just after file header
    bhead_type: BHeadType,
    endian: Endian,
}

impl BlendFile {
    pub fn from_bytes_arc(data: Arc<[u8]>) -> Result<Self, Error> {
        let (header, cursor) = decode_header(&data)?;
        Ok(Self {
            data,
            bhead_type: header.bhead_type(),
            endian: header.endian,
            header,
            cursor,
        })
    }

    pub fn from_bytes_auto_decompress(raw: Arc<[u8]>) -> Result<BlendFile, Error> {
        let data = maybe_decompress(raw)?;
        BlendFile::from_bytes_arc(data)
    }

    pub fn next_block(&mut self) -> Option<BHead> {
        if self.cursor >= self.data.len() {
            return None;
        }
        let start = self.cursor;
        let bh = format::read_bhead(&self.data, start, self.endian, self.bhead_type);
        let bh = match bh {
            Ok(v) => v,
            Err(_) => return None,
        };
        // Advance cursor past header + data payload. Do not add extra alignment padding; the next
        // header begins immediately after the payload per Blender's reader.
        let data_end = bh.data_offset + (bh.len as usize);
        self.cursor = data_end;
        // data_offset is already set correctly by format::read_bhead
        Some(bh)
    }

    pub fn read_block_payload(&self, bh: &BHead) -> Result<&[u8], Error> {
        let start = bh.data_offset;
        let end = start
            .checked_add(bh.len as usize)
            .ok_or_else(|| Error::Decode("overflow".into()))?;
        if end > self.data.len() {
            return Err(Error::Eof);
        }
        Ok(&self.data[start..end])
    }

    pub fn read_dna_block(&self, bh: &BHead) -> Result<SdnaInfo, Error> {
        if bh.code != codes::BLO_CODE_DNA1 {
            return Err(Error::Decode("not a DNA1 block".into()));
        }
        let bytes = self.read_block_payload(bh)?;
        decode_sdna(bytes, self.endian)
    }

    pub fn blocks(&self) -> BlockIter<'_> {
        BlockIter {
            data: &self.data,
            cursor: self.cursor,
            endian: self.endian,
            bhead_type: self.bhead_type,
        }
    }
}

pub struct BlockIter<'a> {
    data: &'a [u8],
    cursor: usize,
    endian: Endian,
    bhead_type: BHeadType,
}

impl<'a> Iterator for BlockIter<'a> {
    type Item = BHead;

    fn next(&mut self) -> Option<Self::Item> {
        if self.cursor >= self.data.len() {
            return None;
        }
        let start = self.cursor;
        let bh = match format::read_bhead(self.data, start, self.endian, self.bhead_type) {
            Ok(b) => b,
            Err(_) => return None,
        };
        let data_end = bh.data_offset + (bh.len as usize);
        self.cursor = data_end;
        Some(bh)
    }
}

fn decode_header(data: &[u8]) -> Result<(Header, usize), Error> {
    crate::format::decode_header(data)
}

// Low-level readers moved to format.rs

// SDNA decoding now lives in crate::sdna
