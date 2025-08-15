use crate::endian::{Endian, PtrWidth};
use crate::error::{BlendModelError, Result};
use crate::header::BlenderHeader;

fn is_digit(b: u8) -> bool {
    b.is_ascii_digit()
}

/// Decode a Blender header from a byte slice.
/// Accepts both legacy (12-byte) and new (17-byte) headers.
pub fn decode_header_bytes(bytes: &[u8]) -> Result<BlenderHeader> {
    if bytes.len() < 12 {
        return Err(BlendModelError::InvalidHeader);
    }
    if &bytes[..7] != b"BLENDER" {
        return Err(BlendModelError::InvalidHeader);
    }
    let tag = bytes[7];
    if tag == b'_' || tag == b'-' {
        // Legacy header: ["BLENDER"][ptr]['v'|'V'][ver3]
        let ptr_width = match tag {
            b'_' => PtrWidth::P32,
            b'-' => PtrWidth::P64,
            _ => return Err(BlendModelError::UnknownHeader),
        };
        let endian = match bytes[8] {
            b'v' => Endian::Little,
            b'V' => Endian::Big,
            _ => return Err(BlendModelError::UnknownHeader),
        };
        if !is_digit(bytes[9]) || !is_digit(bytes[10]) || !is_digit(bytes[11]) {
            return Err(BlendModelError::UnknownHeader);
        }
        let ver = (bytes[9] - b'0') as u16 * 100
            + (bytes[10] - b'0') as u16 * 10
            + (bytes[11] - b'0') as u16;
        Ok(BlenderHeader {
            ptr_width,
            endian,
            file_version: ver,
            file_format_version: 0,
        })
    } else {
        // New header: ["BLENDER"][header_size2]['-'][fmt2]['v'][ver4]
        if !is_digit(bytes[7]) || !is_digit(bytes[8]) {
            return Err(BlendModelError::UnknownHeader);
        }
        let header_size = ((bytes[7] - b'0') as usize) * 10 + (bytes[8] - b'0') as usize;
        if bytes.len() < header_size {
            return Err(BlendModelError::InvalidHeader);
        }
        if bytes[9] != b'-' {
            return Err(BlendModelError::UnknownHeader);
        }
        // Currently always 64-bit pointers.
        let ptr_width = PtrWidth::P64;
        if !is_digit(bytes[10]) || !is_digit(bytes[11]) {
            return Err(BlendModelError::UnknownHeader);
        }
        let fmt = (bytes[10] - b'0') * 10 + (bytes[11] - b'0');
        if fmt != 1 {
            return Err(BlendModelError::UnknownHeader);
        }
        if bytes[12] != b'v' {
            return Err(BlendModelError::UnknownHeader);
        }
        let endian = Endian::Little;
        if !is_digit(bytes[13])
            || !is_digit(bytes[14])
            || !is_digit(bytes[15])
            || !is_digit(bytes[16])
        {
            return Err(BlendModelError::UnknownHeader);
        }
        let ver = (bytes[13] - b'0') as u16 * 1000
            + (bytes[14] - b'0') as u16 * 100
            + (bytes[15] - b'0') as u16 * 10
            + (bytes[16] - b'0') as u16;
        Ok(BlenderHeader {
            ptr_width,
            endian,
            file_version: ver,
            file_format_version: 1,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_legacy_header_le_64() {
        let mut h = *b"BLENDER-v405";
        let hdr = decode_header_bytes(&h).unwrap();
        assert_eq!(hdr.ptr_width, PtrWidth::P64);
        assert_eq!(hdr.endian, Endian::Little);
        assert_eq!(hdr.file_version, 405);
        assert_eq!(hdr.file_format_version, 0);

        h = *b"BLENDER_v280"; // 32-bit little endian 2.80
        let hdr = decode_header_bytes(&h).unwrap();
        assert_eq!(hdr.ptr_width, PtrWidth::P32);
        assert_eq!(hdr.endian, Endian::Little);
        assert_eq!(hdr.file_version, 280);
        assert_eq!(hdr.file_format_version, 0);
    }

    #[test]
    fn decode_new_header() {
        // New header is 17 bytes, e.g., BLENDER17-01v4050
        let h = *b"BLENDER17-01v4050";
        let hdr = decode_header_bytes(&h).unwrap();
        assert_eq!(hdr.ptr_width, PtrWidth::P64);
        assert_eq!(hdr.endian, Endian::Little);
        assert_eq!(hdr.file_version, 4050);
        assert_eq!(hdr.file_format_version, 1);
    }
}
