use std::fmt;

#[derive(Debug)]
pub enum Error {
    InvalidHeader,
    Unsupported(&'static str),
    Eof,
    Decode(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::InvalidHeader => write!(f, "invalid .blend header"),
            Error::Unsupported(s) => write!(f, "unsupported: {s}"),
            Error::Eof => write!(f, "unexpected EOF"),
            Error::Decode(s) => write!(f, "decode error: {s}"),
        }
    }
}

impl std::error::Error for Error {}


