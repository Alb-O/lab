use thiserror::Error;

pub type Result<T, E = BlendModelError> = std::result::Result<T, E>;

#[derive(Debug, Error)]
pub enum BlendModelError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("invalid member name: {0}")]
    InvalidMemberName(String),
    #[error("invalid .blend header")]
    InvalidHeader,
    #[error("unknown .blend header format")]
    UnknownHeader,
    #[error("unknown SDNA struct index: {0}")]
    UnknownStructIndex(u32),
    #[error("unknown SDNA type index: {0}")]
    UnknownTypeIndex(u32),
    #[error("unknown SDNA member index: {0}")]
    UnknownMemberIndex(u32),
}
