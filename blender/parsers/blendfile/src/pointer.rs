use std::fmt;

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Copy, Clone, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub enum OldPtr {
    Ptr32(u32),
    Ptr64(u64),
    Null,
}

impl OldPtr {
    pub fn is_null(&self) -> bool {
        matches!(self, OldPtr::Null)
    }
    pub fn width(&self) -> usize {
        match self {
            OldPtr::Ptr32(_) => 4,
            OldPtr::Ptr64(_) => 8,
            OldPtr::Null => 0,
        }
    }
}

impl fmt::Debug for OldPtr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OldPtr::Ptr32(v) => write!(f, "0x{v:08x}"),
            OldPtr::Ptr64(v) => write!(f, "0x{v:016x}"),
            OldPtr::Null => write!(f, "NULL"),
        }
    }
}

/// Newtype key suitable for hashing and ordering across mixed 32/64-bit pointers.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Copy, Clone, Eq, PartialEq, Hash, Ord, PartialOrd, Debug)]
pub struct OldPtrKey(u128);

impl From<OldPtr> for OldPtrKey {
    fn from(p: OldPtr) -> Self {
        match p {
            OldPtr::Ptr32(v) => Self(v as u128),
            OldPtr::Ptr64(v) => Self(v as u128),
            OldPtr::Null => Self(0),
        }
    }
}
