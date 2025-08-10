use crate::{endian::Endian, endian::PtrWidth};

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BlenderHeader {
    pub ptr_width: PtrWidth,
    pub endian: Endian,
    pub file_version: u16,       // e.g. 405 for 4.05, or 500 for 5.00
    pub file_format_version: u8, // low-level format 0 or 1 (see Blender sources)
}

impl BlenderHeader {
    pub fn bhead_large(&self) -> bool {
        self.file_format_version >= 1
    }
}
