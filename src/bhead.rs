use crate::pointer::OldPtr;

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum BHeadKind {
    BHead4,
    SmallBHead8,
    LargeBHead8,
}

/// FourCC-like block code for a BHead.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct BlockCode(pub u32);

impl BlockCode {
    pub const DATA: BlockCode = BlockCode(fourcc(*b"DATA"));
    pub const GLOB: BlockCode = BlockCode(fourcc(*b"GLOB"));
    pub const DNA1: BlockCode = BlockCode(fourcc(*b"DNA1"));
    pub const TEST: BlockCode = BlockCode(fourcc(*b"TEST"));
    pub const REND: BlockCode = BlockCode(fourcc(*b"REND"));
    pub const USER: BlockCode = BlockCode(fourcc(*b"USER"));
    pub const ENDB: BlockCode = BlockCode(fourcc(*b"ENDB"));

    pub fn as_str(&self) -> [u8; 4] {
        to_fourcc_bytes(self.0)
    }
    pub fn is_end(&self) -> bool {
        *self == Self::ENDB
    }
    pub fn is_dna(&self) -> bool {
        *self == Self::DNA1
    }
}

/// Runtime-normalized BHead representation (matches Blender's `BHead`).
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BHead {
    pub code: BlockCode,
    pub sdna_index: i64,
    pub old: OldPtr,
    pub len: i64,
    pub count: i64,
    pub kind: BHeadKind,
}

pub const fn fourcc(bytes: [u8; 4]) -> u32 {
    // We encode little-endian, stable within this model (independent from host).
    (bytes[3] as u32) << 24 | (bytes[2] as u32) << 16 | (bytes[1] as u32) << 8 | (bytes[0] as u32)
}

pub const fn to_fourcc_bytes(code: u32) -> [u8; 4] {
    [
        (code & 0xFF) as u8,
        ((code >> 8) & 0xFF) as u8,
        ((code >> 16) & 0xFF) as u8,
        ((code >> 24) & 0xFF) as u8,
    ]
}
