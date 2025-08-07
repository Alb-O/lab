use crate::error::{Error, Result};
use std::io::{Read, Seek, SeekFrom};

#[derive(Debug, Clone)]
pub struct BlendFileHeader {
    pub magic: [u8; 7],
    pub file_format_version: u32,
    pub pointer_size: u8,
    pub is_little_endian: bool,
    pub version: u32,
}

impl BlendFileHeader {
    pub fn read<R: Read + Seek>(reader: &mut R) -> Result<Self> {
        reader.seek(SeekFrom::Start(0))?;

        let mut magic = [0u8; 7];
        reader.read_exact(&mut magic)?;
        if &magic != b"BLENDER" {
            return Err(Error::blend_file(
                format!("Invalid magic bytes: {magic:?}"),
                crate::error::BlendFileErrorKind::InvalidMagic,
            ));
        }

        let mut byte_7 = [0u8; 1];
        reader.read_exact(&mut byte_7)?;

        let is_legacy_header = byte_7[0] == b'_' || byte_7[0] == b'-';

        if is_legacy_header {
            let file_format_version = 0;
            let pointer_size = if byte_7[0] == b'_' { 4 } else { 8 };

            let mut byte_8 = [0u8; 1];
            reader.read_exact(&mut byte_8)?;
            let is_little_endian = match byte_8[0] {
                b'v' => true,
                b'V' => false,
                _ => {
                    return Err(Error::blend_file(
                        format!("Invalid endian indicator: {}", byte_8[0] as char),
                        crate::error::BlendFileErrorKind::InvalidHeader,
                    ));
                }
            };

            let mut version_bytes = [0u8; 3];
            reader.read_exact(&mut version_bytes)?;
            let version = std::str::from_utf8(&version_bytes)
                .map_err(|_| {
                    Error::blend_file(
                        "Invalid version format",
                        crate::error::BlendFileErrorKind::InvalidHeader,
                    )
                })?
                .parse::<u32>()
                .map_err(|_| {
                    Error::blend_file(
                        "Invalid version number",
                        crate::error::BlendFileErrorKind::InvalidHeader,
                    )
                })?;

            Ok(BlendFileHeader {
                magic,
                file_format_version,
                pointer_size,
                is_little_endian,
                version,
            })
        } else {
            let mut byte_8 = [0u8; 1];
            reader.read_exact(&mut byte_8)?;

            let header_size = (byte_7[0] - b'0') * 10 + (byte_8[0] - b'0');
            if header_size != 17 {
                return Err(Error::blend_file(
                    format!("Unknown header size: {header_size}"),
                    crate::error::BlendFileErrorKind::InvalidHeader,
                ));
            }

            let mut separator = [0u8; 1];
            reader.read_exact(&mut separator)?;
            if separator[0] != b'-' {
                return Err(Error::blend_file(
                    "Expected '-' separator",
                    crate::error::BlendFileErrorKind::InvalidHeader,
                ));
            }

            let pointer_size = 8;

            let mut version_bytes = [0u8; 2];
            reader.read_exact(&mut version_bytes)?;
            let file_format_version = std::str::from_utf8(&version_bytes)
                .map_err(|_| {
                    Error::blend_file(
                        "Invalid file format version",
                        crate::error::BlendFileErrorKind::InvalidHeader,
                    )
                })?
                .parse::<u32>()
                .map_err(|_| {
                    Error::blend_file(
                        "Invalid file format version number",
                        crate::error::BlendFileErrorKind::InvalidHeader,
                    )
                })?;

            if file_format_version != 1 {
                return Err(Error::blend_file(
                    format!("Unsupported version: {file_format_version}"),
                    crate::error::BlendFileErrorKind::UnsupportedVersion,
                ));
            }

            let mut endian_indicator = [0u8; 1];
            reader.read_exact(&mut endian_indicator)?;
            if endian_indicator[0] != b'v' {
                return Err(Error::blend_file(
                    "Expected 'v' endian indicator",
                    crate::error::BlendFileErrorKind::InvalidHeader,
                ));
            }
            let is_little_endian = true;

            let mut version_bytes = [0u8; 4];
            reader.read_exact(&mut version_bytes)?;
            let version = std::str::from_utf8(&version_bytes)
                .map_err(|_| {
                    Error::blend_file(
                        "Invalid version format",
                        crate::error::BlendFileErrorKind::InvalidHeader,
                    )
                })?
                .parse::<u32>()
                .map_err(|_| {
                    Error::blend_file(
                        "Invalid version number",
                        crate::error::BlendFileErrorKind::InvalidHeader,
                    )
                })?;

            Ok(BlendFileHeader {
                magic,
                file_format_version,
                pointer_size,
                is_little_endian,
                version,
            })
        }
    }

    pub fn header_size(&self) -> usize {
        if self.file_format_version == 0 {
            12
        } else {
            17
        }
    }

    pub fn is_legacy(&self) -> bool {
        self.file_format_version == 0
    }
}
