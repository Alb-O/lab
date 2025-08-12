use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Endian {
    Little,
    Big,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum BHeadType {
    BHead4,
    SmallBHead8,
    LargeBHead8,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
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

    pub const BLO_CODE_WM: u32 = blend_make_id(b'W', b'M', b'\0', b'\0');
    pub const BLO_CODE_SN: u32 = blend_make_id(b'S', b'N', b'\0', b'\0');
    pub const BLO_CODE_SC: u32 = blend_make_id(b'S', b'C', b'\0', b'\0');
    pub const BLO_CODE_OB: u32 = blend_make_id(b'O', b'B', b'\0', b'\0');
    pub const BLO_CODE_ME: u32 = blend_make_id(b'M', b'E', b'\0', b'\0');
    pub const BLO_CODE_CU: u32 = blend_make_id(b'C', b'U', b'\0', b'\0');
    pub const BLO_CODE_MB: u32 = blend_make_id(b'M', b'B', b'\0', b'\0');
    pub const BLO_CODE_MA: u32 = blend_make_id(b'M', b'A', b'\0', b'\0');
    pub const BLO_CODE_TE: u32 = blend_make_id(b'T', b'E', b'\0', b'\0');
    pub const BLO_CODE_IM: u32 = blend_make_id(b'I', b'M', b'\0', b'\0');
    pub const BLO_CODE_LT: u32 = blend_make_id(b'L', b'T', b'\0', b'\0');
    pub const BLO_CODE_LA: u32 = blend_make_id(b'L', b'A', b'\0', b'\0');
    pub const BLO_CODE_CA: u32 = blend_make_id(b'C', b'A', b'\0', b'\0');
    pub const BLO_CODE_IP: u32 = blend_make_id(b'I', b'P', b'\0', b'\0');
    pub const BLO_CODE_KE: u32 = blend_make_id(b'K', b'E', b'\0', b'\0');
    pub const BLO_CODE_WO: u32 = blend_make_id(b'W', b'O', b'\0', b'\0');
    pub const BLO_CODE_AC: u32 = blend_make_id(b'A', b'C', b'\0', b'\0');
    pub const BLO_CODE_TX: u32 = blend_make_id(b'T', b'X', b'\0', b'\0');
    pub const BLO_CODE_VF: u32 = blend_make_id(b'V', b'F', b'\0', b'\0');
    pub const BLO_CODE_SO: u32 = blend_make_id(b'S', b'O', b'\0', b'\0');
    pub const BLO_CODE_GR: u32 = blend_make_id(b'G', b'R', b'\0', b'\0');
    pub const BLO_CODE_AR: u32 = blend_make_id(b'A', b'R', b'\0', b'\0');
    pub const BLO_CODE_BR: u32 = blend_make_id(b'B', b'R', b'\0', b'\0');
    pub const BLO_CODE_PA: u32 = blend_make_id(b'P', b'A', b'\0', b'\0');
    pub const BLO_CODE_LI: u32 = blend_make_id(b'L', b'I', b'\0', b'\0');
    pub const BLO_CODE_NT: u32 = blend_make_id(b'N', b'T', b'\0', b'\0');
    pub const BLO_CODE_LS: u32 = blend_make_id(b'L', b'S', b'\0', b'\0');
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BlockInfo {
    pub name: &'static str,
    pub description: &'static str,
    pub is_data_block: bool,
    pub is_system_block: bool,
    pub typical_size_range: Option<(usize, usize)>,
}

impl BlockInfo {
    pub fn for_code(code: u32) -> BlockInfo {
        match code {
            codes::BLO_CODE_DATA => BlockInfo {
                name: "DATA",
                description: "File data block containing blend file structures",
                is_data_block: true,
                is_system_block: true,
                typical_size_range: Some((1, 1024 * 1024)),
            },
            codes::BLO_CODE_GLOB => BlockInfo {
                name: "GLOB",
                description: "Global settings and file information",
                is_data_block: false,
                is_system_block: true,
                typical_size_range: Some((200, 2000)),
            },
            codes::BLO_CODE_DNA1 => BlockInfo {
                name: "DNA1",
                description: "Structure DNA - type definitions for file format",
                is_data_block: false,
                is_system_block: true,
                typical_size_range: Some((1000, 100000)),
            },
            codes::BLO_CODE_TEST => BlockInfo {
                name: "TEST",
                description: "Test block for validation",
                is_data_block: false,
                is_system_block: true,
                typical_size_range: Some((4, 1024)),
            },
            codes::BLO_CODE_REND => BlockInfo {
                name: "REND",
                description: "Render settings",
                is_data_block: false,
                is_system_block: true,
                typical_size_range: Some((100, 5000)),
            },
            codes::BLO_CODE_USER => BlockInfo {
                name: "USER",
                description: "User preferences",
                is_data_block: false,
                is_system_block: true,
                typical_size_range: Some((100, 10000)),
            },
            codes::BLO_CODE_ENDB => BlockInfo {
                name: "ENDB",
                description: "End of file marker",
                is_data_block: false,
                is_system_block: true,
                typical_size_range: Some((0, 0)),
            },
            codes::BLO_CODE_WM => BlockInfo {
                name: "WindowManager",
                description: "Window manager data",
                is_data_block: true,
                is_system_block: true,
                typical_size_range: None,
            },
            codes::BLO_CODE_SN => BlockInfo {
                name: "Screen",
                description: "Screen layout data",
                is_data_block: true,
                is_system_block: true,
                typical_size_range: None,
            },
            codes::BLO_CODE_SC => BlockInfo {
                name: "Scene",
                description: "Scene data",
                is_data_block: true,
                is_system_block: false,
                typical_size_range: None,
            },
            codes::BLO_CODE_OB => BlockInfo {
                name: "Object",
                description: "Object data",
                is_data_block: true,
                is_system_block: false,
                typical_size_range: None,
            },
            codes::BLO_CODE_ME => BlockInfo {
                name: "Mesh",
                description: "Mesh geometry data",
                is_data_block: true,
                is_system_block: false,
                typical_size_range: None,
            },
            codes::BLO_CODE_CU => BlockInfo {
                name: "Curve",
                description: "Curve geometry data",
                is_data_block: true,
                is_system_block: false,
                typical_size_range: None,
            },
            codes::BLO_CODE_MB => BlockInfo {
                name: "MetaBall",
                description: "MetaBall geometry data",
                is_data_block: true,
                is_system_block: false,
                typical_size_range: None,
            },
            codes::BLO_CODE_MA => BlockInfo {
                name: "Material",
                description: "Material data",
                is_data_block: true,
                is_system_block: false,
                typical_size_range: None,
            },
            codes::BLO_CODE_TE => BlockInfo {
                name: "Texture",
                description: "Texture data",
                is_data_block: true,
                is_system_block: false,
                typical_size_range: None,
            },
            codes::BLO_CODE_IM => BlockInfo {
                name: "Image",
                description: "Image data",
                is_data_block: true,
                is_system_block: false,
                typical_size_range: None,
            },
            codes::BLO_CODE_LT => BlockInfo {
                name: "Lattice",
                description: "Lattice deformation data",
                is_data_block: true,
                is_system_block: false,
                typical_size_range: None,
            },
            codes::BLO_CODE_LA => BlockInfo {
                name: "Lamp",
                description: "Light/lamp data",
                is_data_block: true,
                is_system_block: false,
                typical_size_range: None,
            },
            codes::BLO_CODE_CA => BlockInfo {
                name: "Camera",
                description: "Camera data",
                is_data_block: true,
                is_system_block: false,
                typical_size_range: None,
            },
            codes::BLO_CODE_IP => BlockInfo {
                name: "Ipo",
                description: "Animation curve data (legacy)",
                is_data_block: true,
                is_system_block: false,
                typical_size_range: None,
            },
            codes::BLO_CODE_KE => BlockInfo {
                name: "Key",
                description: "Shape key data",
                is_data_block: true,
                is_system_block: false,
                typical_size_range: None,
            },
            codes::BLO_CODE_WO => BlockInfo {
                name: "World",
                description: "World/environment data",
                is_data_block: true,
                is_system_block: false,
                typical_size_range: None,
            },
            codes::BLO_CODE_AC => BlockInfo {
                name: "Action",
                description: "Animation action data",
                is_data_block: true,
                is_system_block: false,
                typical_size_range: None,
            },
            codes::BLO_CODE_TX => BlockInfo {
                name: "Text",
                description: "Text object data",
                is_data_block: true,
                is_system_block: false,
                typical_size_range: None,
            },
            codes::BLO_CODE_VF => BlockInfo {
                name: "VFont",
                description: "Vector font data",
                is_data_block: true,
                is_system_block: false,
                typical_size_range: None,
            },
            codes::BLO_CODE_SO => BlockInfo {
                name: "Sound",
                description: "Sound data",
                is_data_block: true,
                is_system_block: false,
                typical_size_range: None,
            },
            codes::BLO_CODE_GR => BlockInfo {
                name: "Collection",
                description: "Collection data",
                is_data_block: true,
                is_system_block: false,
                typical_size_range: None,
            },
            codes::BLO_CODE_AR => BlockInfo {
                name: "Armature",
                description: "Armature/skeleton data",
                is_data_block: true,
                is_system_block: false,
                typical_size_range: None,
            },
            codes::BLO_CODE_BR => BlockInfo {
                name: "Brush",
                description: "Brush data",
                is_data_block: true,
                is_system_block: false,
                typical_size_range: None,
            },
            codes::BLO_CODE_PA => BlockInfo {
                name: "ParticleSettings",
                description: "Particle system settings",
                is_data_block: true,
                is_system_block: false,
                typical_size_range: None,
            },
            codes::BLO_CODE_LI => BlockInfo {
                name: "Library",
                description: "Library link data",
                is_data_block: true,
                is_system_block: false,
                typical_size_range: None,
            },
            codes::BLO_CODE_NT => BlockInfo {
                name: "NodeTree",
                description: "Node tree data",
                is_data_block: true,
                is_system_block: false,
                typical_size_range: None,
            },
            codes::BLO_CODE_LS => BlockInfo {
                name: "LineStyle",
                description: "Freestyle line style data",
                is_data_block: true,
                is_system_block: false,
                typical_size_range: None,
            },
            _ => BlockInfo {
                name: "Unknown",
                description: "Unknown or custom block type",
                is_data_block: false,
                is_system_block: false,
                typical_size_range: None,
            },
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockCode(pub u32);

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BHead {
    pub code: u32,
    pub sdn_anr: i64,
    pub old_ptr: u64,
    pub len: i64,
    pub nr: i64,
    pub data_offset: usize,
}

impl BHead {
    pub fn block_info(&self) -> BlockInfo {
        BlockInfo::for_code(self.code)
    }

    pub fn block_type_name(&self) -> &'static str {
        self.block_info().name
    }

    pub fn block_description(&self) -> &'static str {
        self.block_info().description
    }

    pub fn is_data_block(&self) -> bool {
        self.block_info().is_data_block
    }

    pub fn is_system_block(&self) -> bool {
        self.block_info().is_system_block
    }

    pub fn code_string(&self) -> String {
        code_to_string(self.code)
    }

    pub fn is_valid_size(&self) -> bool {
        if self.len < 0 {
            return false;
        }

        if let Some((min_size, max_size)) = self.block_info().typical_size_range {
            let size = self.len as usize;
            size >= min_size && size <= max_size
        } else {
            true
        }
    }

    pub fn estimated_structure_count(&self, structure_size: usize) -> Option<usize> {
        if structure_size == 0 || self.len <= 0 {
            return None;
        }
        Some(self.len as usize / structure_size)
    }

    pub fn size_category(&self) -> &'static str {
        let size = self.len as usize;
        match size {
            0 => "Empty",
            1..=256 => "Small",
            257..=4096 => "Medium",
            4097..=65536 => "Large",
            65537..=1048576 => "Very Large",
            _ => "Huge",
        }
    }
}

use crate::error::Error;
use std::convert::TryInto;

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
        let file_version =
            ((v[0] - b'0') as u32) * 100 + ((v[1] - b'0') as u32) * 10 + ((v[2] - b'0') as u32);
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

pub fn read_bhead(
    data: &[u8],
    offset: usize,
    endian: Endian,
    bht: BHeadType,
) -> Result<BHead, Error> {
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
    Ok(BHead {
        code,
        sdn_anr,
        old_ptr,
        len,
        nr,
        data_offset: offset + 20,
    })
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
    Ok(BHead {
        code,
        sdn_anr,
        old_ptr,
        len,
        nr,
        data_offset: offset + 24,
    })
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
    Ok(BHead {
        code,
        sdn_anr,
        old_ptr,
        len,
        nr,
        data_offset: offset + 32,
    })
}

pub fn code_to_string(code: u32) -> String {
    let a = (code & 0xFF) as u8 as char;
    let b = ((code >> 8) & 0xFF) as u8 as char;
    let c = ((code >> 16) & 0xFF) as u8 as char;
    let d = ((code >> 24) & 0xFF) as u8 as char;
    format!("{a}{b}{c}{d}")
}
