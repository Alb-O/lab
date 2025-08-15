#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum Endian {
    Little,
    Big,
}

impl Endian {
    pub fn is_little(self) -> bool {
        matches!(self, Endian::Little)
    }
    pub fn is_big(self) -> bool {
        matches!(self, Endian::Big)
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum PtrWidth {
    P32,
    P64,
}

impl PtrWidth {
    pub fn bytes(self) -> usize {
        match self {
            PtrWidth::P32 => 4,
            PtrWidth::P64 => 8,
        }
    }
}
