use std::fmt;

#[derive(Debug)]
pub enum BlendError {
    Io(std::io::Error),
    InvalidHeader,
    InvalidMagic(Vec<u8>),
    UnsupportedHeader(String),
    UnsupportedVersion(u32),
    NoDnaFound,
    InvalidBlockIndex(usize),
    DnaError(String),
    InvalidDna(String),
    InvalidData(String),
    InvalidField(String),
}

pub type Result<T> = std::result::Result<T, BlendError>;

impl fmt::Display for BlendError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BlendError::Io(err) => write!(f, "I/O error: {err}"),
            BlendError::InvalidHeader => write!(f, "Invalid header"),
            BlendError::InvalidMagic(magic) => write!(f, "Invalid magic bytes: {magic:?}"),
            BlendError::UnsupportedHeader(msg) => write!(f, "Unsupported header: {msg}"),
            BlendError::UnsupportedVersion(version) => write!(f, "Unsupported version: {version}"),
            BlendError::NoDnaFound => write!(f, "DNA block not found"),
            BlendError::InvalidBlockIndex(index) => {
                write!(f, "Invalid block index: {index}")
            }
            BlendError::DnaError(msg) => {
                write!(f, "DNA parsing error: {msg}")
            }
            BlendError::InvalidDna(msg) => {
                write!(f, "Invalid DNA: {msg}")
            }
            BlendError::InvalidData(msg) => {
                write!(f, "Invalid data: {msg}")
            }
            BlendError::InvalidField(field) => {
                write!(f, "Invalid field access: {field}")
            }
        }
    }
}

impl std::error::Error for BlendError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            BlendError::Io(err) => Some(err),
            _ => None,
        }
    }
}

impl From<std::io::Error> for BlendError {
    fn from(err: std::io::Error) -> Self {
        BlendError::Io(err)
    }
}
