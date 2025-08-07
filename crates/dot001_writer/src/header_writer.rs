use dot001_error::{Dot001Error, Result};
use std::io::Write;

/// Writer for Blender 5.0 low-level header (file_format_version = 1).
///
/// Header layout (17 bytes total):
///   "BLENDER" (7)
///   header_size_ascii (2)  -> "17"
///   '-' (1)
///   format_version (2)     -> "01"
///   'v' (1)                -> little-endian marker (kept for readability)
///   blender_version (4)    -> e.g. "0500"
///
/// Note: For v1 header we fix pointer size to 8 and little-endian.
pub struct HeaderWriter {
    /// 4-digit Blender version, e.g. 500 for 5.0.0 encoded as "0500".
    pub blender_version: u16,
    /// Low-level file format version; currently fixed to 1.
    pub file_format_version: u8,
}

impl Default for HeaderWriter {
    fn default() -> Self {
        Self {
            blender_version: 500, // "0500"
            file_format_version: 1,
        }
    }
}

impl HeaderWriter {
    pub fn write<W: Write>(&self, mut w: W) -> Result<()> {
        // Magic
        w.write_all(b"BLENDER")
            .map_err(|e| Dot001Error::io(format!("write magic failed: {e}")))?;

        // Header size: currently fixed to 17
        w.write_all(b"17")
            .map_err(|e| Dot001Error::io(format!("write header size failed: {e}")))?;

        // Separator '-'
        w.write_all(b"-")
            .map_err(|e| Dot001Error::io(format!("write separator failed: {e}")))?;

        // File format version 2 ASCII digits, currently "01"
        let ver = format!("{:02}", self.file_format_version);
        w.write_all(ver.as_bytes())
            .map_err(|e| Dot001Error::io(format!("write file format version failed: {e}")))?;

        // Endian marker (kept for readability)
        w.write_all(b"v")
            .map_err(|e| Dot001Error::io(format!("write endian marker failed: {e}")))?;

        // Blender version: 4 ASCII digits
        let bl_ver = format!("{:04}", self.blender_version);
        w.write_all(bl_ver.as_bytes())
            .map_err(|e| Dot001Error::io(format!("write blender version failed: {e}")))?;

        Ok(())
    }
}
