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
    pub pointer_size: u8,         // 4 or 8
    pub endian: Endian,           // Little or Big
    pub file_version: u32,        // e.g. 305 or 0405
    pub file_format_version: u32, // 0 or 1
}

impl Header {
    pub fn bhead_type(&self) -> BHeadType {
        if self.pointer_size == 4 {
            return BHeadType::BHead4;
        }
        if self.file_format_version == 0 {
            return BHeadType::SmallBHead8;
        }
        BHeadType::LargeBHead8
    }
}

pub mod codes {
    #[inline]
    const fn blend_make_id(a: u8, b: u8, c: u8, d: u8) -> u32 {
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
pub struct BlockCode(pub u32);

#[derive(Clone, Debug)]
pub struct BHead {
    pub code: u32,
    pub sdn_anr: i64,
    pub old_ptr: u64,
    pub len: i64,
    pub nr: i64,
    pub data_offset: usize,
}

use crate::error::Error;

pub fn decode_header(data: &[u8]) -> Result<(Header, usize), Error> {
    const MIN: usize = 12;
    if data.len() < MIN {
        return Err(Error::InvalidHeader);
    }
    if &data[0..7] != b"BLENDER" {
        return Err(Error::InvalidHeader);
    }

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
        if !v.iter().all(|c| c.is_ascii_digit()) {
            return Err(Error::InvalidHeader);
        }
        let file_version = ((v[0] - b'0') as u32) * 100
            + ((v[1] - b'0') as u32) * 10
            + ((v[2] - b'0') as u32);
        let header = Header {
            pointer_size,
            endian,
            file_version,
            file_format_version: 0,
        };
        return Ok((header, 12));
    }

    if data.len() < 17 {
        return Err(Error::InvalidHeader);
    }
    let size_digits = &data[7..9];
    if !size_digits.iter().all(|c| c.is_ascii_digit()) {
        return Err(Error::InvalidHeader);
    }
    let header_size = ((size_digits[0] - b'0') as usize) * 10 + ((size_digits[1] - b'0') as usize);
    if header_size != 17 {
        return Err(Error::InvalidHeader);
    }
    if data[9] != b'-' {
        return Err(Error::InvalidHeader);
    }
    if !data[10].is_ascii_digit() || !data[11].is_ascii_digit() {
        return Err(Error::InvalidHeader);
    }
    let fmt_ver = ((data[10] - b'0') as u32) * 10 + ((data[11] - b'0') as u32);
    if data[12] != b'v' {
        return Err(Error::InvalidHeader);
    }
    let ver_digits = &data[13..17];
    if !ver_digits.iter().all(|c| c.is_ascii_digit()) {
        return Err(Error::InvalidHeader);
    }
    let file_version = ((ver_digits[0] - b'0') as u32) * 1000
        + ((ver_digits[1] - b'0') as u32) * 100
        + ((ver_digits[2] - b'0') as u32) * 10
        + ((ver_digits[3] - b'0') as u32);
    let header = Header {
        pointer_size: 8,
        endian: Endian::Little,
        file_version,
        file_format_version: fmt_ver,
    };
    Ok((header, header_size))
}

fn read_u32(bytes: &[u8], endian: Endian) -> Result<u32, Error> {
    if bytes.len() < 4 {
        return Err(Error::Eof);
    }
    let v = u32::from_le_bytes(bytes[0..4].try_into().unwrap());
    Ok(match endian {
        Endian::Little => v,
        Endian::Big => v.swap_bytes(),
    })
}

fn read_i32(bytes: &[u8], endian: Endian) -> Result<i32, Error> {
    Ok(read_u32(bytes, endian)? as i32)
}

fn read_u64(bytes: &[u8], endian: Endian) -> Result<u64, Error> {
    if bytes.len() < 8 {
        return Err(Error::Eof);
    }
    let v = u64::from_le_bytes(bytes[0..8].try_into().unwrap());
    Ok(match endian {
        Endian::Little => v,
        Endian::Big => v.swap_bytes(),
    })
}

fn read_i64(bytes: &[u8], endian: Endian) -> Result<i64, Error> {
    Ok(read_u64(bytes, endian)? as i64)
}

pub fn read_bhead(data: &[u8], offset: usize, endian: Endian, bht: BHeadType) -> Result<BHead, Error> {
    match bht {
        BHeadType::BHead4 => read_bhead4(data, offset, endian),
        BHeadType::SmallBHead8 => read_small_bhead8(data, offset, endian),
        BHeadType::LargeBHead8 => read_large_bhead8(data, offset, endian),
    }
}

fn read_bhead4(data: &[u8], offset: usize, endian: Endian) -> Result<BHead, Error> {
    let s = &data[offset..];
    if s.len() < 4 + 4 + 4 + 4 + 4 {
        return Err(Error::Eof);
    }
    let code = read_u32(&s[0..4], endian)?;
    let len = read_i32(&s[4..8], endian)? as i64;
    let old_ptr = read_u32(&s[8..12], endian)? as u64;
    let sdn_anr = read_i32(&s[12..16], endian)? as i64;
    let nr = read_i32(&s[16..20], endian)? as i64;
    Ok(BHead { code, sdn_anr, old_ptr, len, nr, data_offset: offset + 20 })
}

fn read_small_bhead8(data: &[u8], offset: usize, endian: Endian) -> Result<BHead, Error> {
    let s = &data[offset..];
    if s.len() < 4 + 4 + 8 + 4 + 4 {
        return Err(Error::Eof);
    }
    let code = read_u32(&s[0..4], endian)?;
    let len = read_i32(&s[4..8], endian)? as i64;
    let old_ptr = read_u64(&s[8..16], endian)?;
    let sdn_anr = read_i32(&s[16..20], endian)? as i64;
    let nr = read_i32(&s[20..24], endian)? as i64;
    Ok(BHead { code, sdn_anr, old_ptr, len, nr, data_offset: offset + 24 })
}

fn read_large_bhead8(data: &[u8], offset: usize, endian: Endian) -> Result<BHead, Error> {
    let s = &data[offset..];
    if s.len() < 4 + 4 + 8 + 8 + 8 {
        return Err(Error::Eof);
    }
    let code = read_u32(&s[0..4], endian)?;
    let sdn_anr = read_i32(&s[4..8], endian)? as i64;
    let old_ptr = read_u64(&s[8..16], endian)?;
    let len = read_i64(&s[16..24], endian)?;
    let nr = read_i64(&s[24..32], endian)?;
    Ok(BHead { code, sdn_anr, old_ptr, len, nr, data_offset: offset + 32 })
}

pub fn code_to_string(code: u32) -> String {
    let a = (code & 0xFF) as u8 as char;
    let b = ((code >> 8) & 0xFF) as u8 as char;
    let c = ((code >> 16) & 0xFF) as u8 as char;
    let d = ((code >> 24) & 0xFF) as u8 as char;
    format!("{a}{b}{c}{d}")
}


