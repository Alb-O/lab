use std::fmt;
use std::sync::Arc;

// Core constants mirrored from Blender's BLO_core_bhead.hh
pub mod codes {
    #[inline]
    const fn blend_make_id(a: u8, b: u8, c: u8, d: u8) -> u32 {
        // Little-endian encoding per Blender
        ((d as u32) << 24) | ((c as u32) << 16) | ((b as u32) << 8) | (a as u32)
    }

    pub const BLO_CODE_DATA: u32 = blend_make_id(b'D', b'A', b'T', b'A');
    pub const BLO_CODE_GLOB: u32 = blend_make_id(b'G', b'L', b'O', b'B');
    pub const BLO_CODE_DNA1: u32 = blend_make_id(b'D', b'N', b'A', b'1');
    pub const BLO_CODE_TEST: u32 = blend_make_id(b'T', b'E', b'S', b'T');
    pub const BLO_CODE_REND: u32 = blend_make_id(b'R', b'E', b'N', b'D');
    pub const BLO_CODE_USER: u32 = blend_make_id(b'U', b'S', b'E', b'R');
    pub const BLO_CODE_ENDB: u32 = blend_make_id(b'E', b'N', b'D', b'B');
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Endian {
    Little,
    Big,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum BHeadType {
    BHead4,
    SmallBHead8,
    LargeBHead8,
}

#[derive(Clone, Debug)]
pub struct Header {
    pub pointer_size: u8,      // 4 or 8
    pub endian: Endian,        // Little or Big
    pub file_version: u32,     // e.g. 305 or 0405
    pub file_format_version: u32, // 0 or 1
}

impl Header {
    fn bhead_type(&self) -> BHeadType {
        if self.pointer_size == 4 {
            return BHeadType::BHead4;
        }
        if self.file_format_version == 0 {
            return BHeadType::SmallBHead8;
        }
        BHeadType::LargeBHead8
    }
}

#[derive(Clone, Debug)]
pub struct BHead {
    pub code: u32,
    pub sdn_anr: i64,
    pub old_ptr: u64,
    pub len: i64,
    pub nr: i64,
    // offset to data start (after header)
    pub data_offset: usize,
}

#[derive(Debug)]
pub enum Error {
    InvalidHeader,
    Unsupported(String),
    Eof,
    Decode(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::InvalidHeader => write!(f, "invalid .blend header"),
            Error::Unsupported(s) => write!(f, "unsupported: {}", s),
            Error::Eof => write!(f, "unexpected EOF"),
            Error::Decode(s) => write!(f, "decode error: {}", s),
        }
    }
}

impl std::error::Error for Error {}

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
        // Zstd magic: 0x28 B5 2F FD
        let looks_like_zstd = raw.len() >= 4 && raw[0..4] == [0x28, 0xB5, 0x2F, 0xFD];
        if looks_like_zstd {
            let mut decoder = zstd::stream::read::Decoder::new(&*raw)
                .map_err(|e| Error::Decode(format!("zstd init: {}", e)))?;
            let mut buf = Vec::new();
            use std::io::Read;
            decoder
                .read_to_end(&mut buf)
                .map_err(|e| Error::Decode(format!("zstd decode: {}", e)))?;
            let arc: Arc<[u8]> = buf.into_boxed_slice().into();
            return BlendFile::from_bytes_arc(arc);
        }
        BlendFile::from_bytes_arc(raw)
    }

    pub fn next_block(&mut self) -> Option<BHead> {
        if self.cursor >= self.data.len() {
            return None;
        }
        let start = self.cursor;
        let bh = match self.bhead_type {
            BHeadType::BHead4 => read_bhead4(&self.data, start, self.endian),
            BHeadType::SmallBHead8 => read_small_bhead8(&self.data, start, self.endian),
            BHeadType::LargeBHead8 => read_large_bhead8(&self.data, start, self.endian),
        };
        let mut bh = match bh {
            Ok(v) => v,
            Err(_) => return None,
        };
        // Advance cursor past header + data payload. Do not add extra alignment padding; the next
        // header begins immediately after the payload per Blender's reader.
        let data_end = bh.data_offset + (bh.len as usize);
        self.cursor = data_end;
        // Mark data_offset at payload start
        bh.data_offset = bh.data_offset;
        Some(bh)
    }

    pub fn read_block_payload(&self, bh: &BHead) -> Result<&[u8], Error> {
        let start = bh.data_offset;
        let end = start.checked_add(bh.len as usize).ok_or_else(|| Error::Decode("overflow".into()))?;
        if end > self.data.len() { return Err(Error::Eof); }
        Ok(&self.data[start..end])
    }

    pub fn read_dna_block(&self, bh: &BHead) -> Result<SdnaInfo, Error> {
        if bh.code != codes::BLO_CODE_DNA1 {
            return Err(Error::Decode("not a DNA1 block".into()));
        }
        let bytes = self.read_block_payload(bh)?;
        decode_sdna(bytes, self.endian)
    }
}

fn decode_header(data: &[u8]) -> Result<(Header, usize), Error> {
    const MIN: usize = 12; // minimal header
    if data.len() < MIN { return Err(Error::InvalidHeader); }
    if &data[0..7] != b"BLENDER" { return Err(Error::InvalidHeader); }

    let b7 = data[7];
    let is_legacy = b7 == b'_' || b7 == b'-';
    if is_legacy {
        let pointer_size = match data[7] {
            b'_' => 4,
            b'-' => 8,
            _ => return Err(Error::InvalidHeader),
        } as u8;
        let endian = match data[8] {
            b'v' => Endian::Little,
            b'V' => Endian::Big,
            _ => return Err(Error::InvalidHeader),
        };
        let v = &data[9..12];
        if !v.iter().all(|c| c.is_ascii_digit()) { return Err(Error::InvalidHeader); }
        let file_version = ((v[0]-b'0') as u32) * 100 + ((v[1]-b'0') as u32) * 10 + ((v[2]-b'0') as u32);
        let header = Header { pointer_size, endian, file_version, file_format_version: 0 };
        return Ok((header, 12));
    }

    // New header (version 1): 17 bytes total
    if data.len() < 17 { return Err(Error::InvalidHeader); }
    let size_digits = &data[7..9];
    if !size_digits.iter().all(|c| c.is_ascii_digit()) { return Err(Error::InvalidHeader); }
    let header_size = ((size_digits[0]-b'0') as usize) * 10 + ((size_digits[1]-b'0') as usize);
    if header_size != 17 { return Err(Error::InvalidHeader); }
    if data[9] != b'-' { return Err(Error::InvalidHeader); }
    if !data[10].is_ascii_digit() || !data[11].is_ascii_digit() { return Err(Error::InvalidHeader); }
    let fmt_ver = ((data[10]-b'0') as u32) * 10 + ((data[11]-b'0') as u32);
    if data[12] != b'v' { return Err(Error::InvalidHeader); }
    let ver_digits = &data[13..17];
    if !ver_digits.iter().all(|c| c.is_ascii_digit()) { return Err(Error::InvalidHeader); }
    let file_version = ((ver_digits[0]-b'0') as u32) * 1000
        + ((ver_digits[1]-b'0') as u32) * 100
        + ((ver_digits[2]-b'0') as u32) * 10
        + ((ver_digits[3]-b'0') as u32);
    let header = Header { pointer_size: 8, endian: Endian::Little, file_version, file_format_version: fmt_ver };
    Ok((header, header_size))
}

fn read_u32(bytes: &[u8], endian: Endian) -> Result<u32, Error> {
    if bytes.len() < 4 { return Err(Error::Eof); }
    let v = u32::from_le_bytes(bytes[0..4].try_into().unwrap());
    Ok(match endian { Endian::Little => v, Endian::Big => v.swap_bytes() })
}

fn read_i32(bytes: &[u8], endian: Endian) -> Result<i32, Error> { Ok(read_u32(bytes, endian)? as i32) }

fn read_u64(bytes: &[u8], endian: Endian) -> Result<u64, Error> {
    if bytes.len() < 8 { return Err(Error::Eof); }
    let v = u64::from_le_bytes(bytes[0..8].try_into().unwrap());
    Ok(match endian { Endian::Little => v, Endian::Big => v.swap_bytes() })
}

fn read_i64(bytes: &[u8], endian: Endian) -> Result<i64, Error> { Ok(read_u64(bytes, endian)? as i64) }

fn read_bhead4(data: &[u8], offset: usize, endian: Endian) -> Result<BHead, Error> {
    let s = &data[offset..];
    if s.len() < 4 + 4 + 4 + 4 + 4 { return Err(Error::Eof); }
    let code = read_u32(&s[0..4], endian)?;
    let len = read_i32(&s[4..8], endian)? as i64;
    let old_ptr = read_u32(&s[8..12], endian)? as u64;
    let sdn_anr = read_i32(&s[12..16], endian)? as i64;
    let nr = read_i32(&s[16..20], endian)? as i64;
    Ok(BHead { code, sdn_anr, old_ptr, len, nr, data_offset: offset + 20 })
}

fn read_small_bhead8(data: &[u8], offset: usize, endian: Endian) -> Result<BHead, Error> {
    let s = &data[offset..];
    if s.len() < 4 + 4 + 8 + 4 + 4 { return Err(Error::Eof); }
    let code = read_u32(&s[0..4], endian)?;
    let len = read_i32(&s[4..8], endian)? as i64;
    let old_ptr = read_u64(&s[8..16], endian)?;
    let sdn_anr = read_i32(&s[16..20], endian)? as i64;
    let nr = read_i32(&s[20..24], endian)? as i64;
    Ok(BHead { code, sdn_anr, old_ptr, len, nr, data_offset: offset + 24 })
}

fn read_large_bhead8(data: &[u8], offset: usize, endian: Endian) -> Result<BHead, Error> {
    let s = &data[offset..];
    if s.len() < 4 + 4 + 8 + 8 + 8 { return Err(Error::Eof); }
    let code = read_u32(&s[0..4], endian)?;
    let sdn_anr = read_i32(&s[4..8], endian)? as i64;
    let old_ptr = read_u64(&s[8..16], endian)?;
    let len = read_i64(&s[16..24], endian)?;
    let nr = read_i64(&s[24..32], endian)?;
    Ok(BHead { code, sdn_anr, old_ptr, len, nr, data_offset: offset + 32 })
}

// Minimal SDNA info summary for CLI
pub struct SdnaInfo {
    pub names_len: usize,
    pub types_len: usize,
    pub structs_len: usize,
}

fn align4(idx: usize) -> usize { (idx + 3) & !3 }

fn expect_tag(bytes: &[u8], idx: usize, tag: &[u8;4]) -> Result<(), Error> {
    if bytes.len() < idx + 4 { return Err(Error::Eof); }
    if &bytes[idx..idx+4] != tag { return Err(Error::Decode(format!("expected {:?} tag", std::str::from_utf8(tag).unwrap()))); }
    Ok(())
}

fn read_u32_le(bytes: &[u8], idx: usize, endian: Endian) -> Result<(u32, usize), Error> {
    if bytes.len() < idx + 4 { return Err(Error::Eof); }
    let mut val = u32::from_le_bytes(bytes[idx..idx+4].try_into().unwrap());
    if let Endian::Big = endian { val = val.swap_bytes(); }
    Ok((val, idx + 4))
}

pub fn decode_sdna(bytes: &[u8], endian: Endian) -> Result<SdnaInfo, Error> {
    let mut i = 0usize;
    expect_tag(bytes, i, b"SDNA")?; i += 4;
    expect_tag(bytes, i, b"NAME")?; i += 4;
    let (names_count, mut i2) = read_u32_le(bytes, i, endian)?; i = i2;
    // Walk null-terminated strings
    for _ in 0..names_count {
        let mut j = i;
        while j < bytes.len() && bytes[j] != 0 { j += 1; }
        if j >= bytes.len() { return Err(Error::Eof); }
        i = j + 1;
    }
    i = align4(i);

    expect_tag(bytes, i, b"TYPE")?; i += 4;
    let (types_count, i3) = read_u32_le(bytes, i, endian)?; i = i3;
    for _ in 0..types_count {
        let mut j = i;
        while j < bytes.len() && bytes[j] != 0 { j += 1; }
        if j >= bytes.len() { return Err(Error::Eof); }
        i = j + 1;
    }
    i = align4(i);

    expect_tag(bytes, i, b"TLEN")?; i += 4;
    // `types_count` u16 lengths, then align to 4
    let need = (types_count as usize) * 2;
    if bytes.len() < i + need { return Err(Error::Eof); }
    i += need;
    if (types_count & 1) != 0 { i += 2; } // prevent BUS error, per Blender

    expect_tag(bytes, i, b"STRC")?; i += 4;
    let (structs_count, mut i4) = read_u32_le(bytes, i, endian)?; i = i4;
    // We won't fully walk members; just sanity-skip
    for _ in 0..structs_count {
        // Each struct begins with: short type_index; short members_num
        if bytes.len() < i + 4 { return Err(Error::Eof); }
        // skip header
        i += 4;
        // For members we need to read back the members_num we just skipped; re-read for safety
        // But since this is a fast path summary, conservatively skip by at least one member record size.
        // We cannot decode without endianness-conscious shorts; do minimal parsing:
        // Read members_num as little-endian u16 (swap if big endian)
        let members_num_raw = u16::from_le_bytes(bytes[i-2..i].try_into().unwrap());
        let members_num = match endian { Endian::Little => members_num_raw, Endian::Big => members_num_raw.swap_bytes() } as usize;
        let member_rec_sz: usize = 4; // two shorts per member
        let bytes_to_advance = member_rec_sz.checked_mul(members_num).ok_or_else(|| Error::Decode("overflow".into()))?;
        if bytes.len() < i + bytes_to_advance { return Err(Error::Eof); }
        i += bytes_to_advance;
    }

    Ok(SdnaInfo { names_len: names_count as usize, types_len: types_count as usize, structs_len: structs_count as usize })
}


