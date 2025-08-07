use crate::dna_provider::SeedDnaProvider;
use crate::header_writer::HeaderWriter;
use dot001_error::{Dot001Error, Result};
use std::fs::File;
use std::io::{BufWriter, Write};

/// Templates for initial milestones.
#[derive(Clone, Copy, Debug)]
pub enum WriteTemplate {
    /// M1: header + DNA1 + ENDB
    Minimal,
    // Future:
    // SceneOnly,
    // TriangleMesh,
}

/** Encapsulates writing a Blender 5.0-format .blend file.
Emits:
- 17-byte v1 header: "BLENDER17-01v0500"
- Minimum required ID blocks to satisfy Main:
    * "ID" (Main) placeholder with minimal header payload (zero length)
    * "GLOB" placeholder (zero length) for global state if required by reader paths
- DNA1 block with raw bytes extracted from a seed
- ENDB
Note: Some Blender codepaths assume a Main/ID list exists before DNA when opening very minimal files.
*/
#[derive(Default)]
pub struct BlendWriter {
    pub header: HeaderWriter,
}

impl BlendWriter {
    /// Write a .blend file according to a chosen template, using DNA from the provided seed.
    pub fn write_with_seed<P: AsRef<std::path::Path>>(
        &self,
        out_path: P,
        template: WriteTemplate,
        seed: &SeedDnaProvider,
    ) -> Result<()> {
        match template {
            WriteTemplate::Minimal => self.write_minimal(out_path, seed),
        }
    }

    fn write_minimal<P: AsRef<std::path::Path>>(
        &self,
        out_path: P,
        seed: &SeedDnaProvider,
    ) -> Result<()> {
        let file = File::create(out_path)?;
        let mut w = BufWriter::new(file);

        // 1) Header
        self.header.write(&mut w)?;

        // 2) REND block (render settings) - essential first block
        let rend_sdna_index = seed.sdna_index_for_struct("RenderData").unwrap_or(0u32);
        self.write_block_v1(&mut w, b"REND", rend_sdna_index, 0, seed.rend_bytes(), 1)?;

        // 3) TEST block - appears to be essential in working files
        let test_sdna_index = seed.sdna_index_for_struct("Test").unwrap_or(0u32);
        self.write_block_v1(&mut w, b"TEST", test_sdna_index, 0, seed.test_bytes(), 1)?;

        // 4) GLOB block (global settings) - use actual data from seed
        let glob_sdna_index = seed.sdna_index_for_struct("Global").unwrap_or(0u32);
        self.write_block_v1(&mut w, b"GLOB", glob_sdna_index, 0, seed.glob_bytes(), 1)?;

        // 5) DNA1 block
        self.write_block_v1(
            &mut w,
            b"DNA1",
            0u32,             // sdna_index is 0 for DNA itself
            0,                // old address not used for DNA
            seed.raw_bytes(), // payload copied from seed
            1,                // count
        )?;

        // 6) ENDB
        self.write_block_v1(&mut w, b"ENDB", 0u32, 0, &[], 0)?;

        w.flush()
            .map_err(|e| Dot001Error::io(format!("flush failed: {e}")))?;
        Ok(())
    }

    /// Write a v1 (5.0) BHead+payload block.
    /// Binary layout for v1:
    ///   code[4]
    ///   sdna_index: u32 (ASCII-less integer, little-endian)
    ///   old_address: u64
    ///   len: u64 (payload length in bytes)
    ///   count: u64 (written as u64 but should fit in u32)
    fn write_block_v1<W: Write>(
        &self,
        mut w: W,
        code: &[u8; 4],
        sdna_index: u32,
        old_address: u64,
        payload: &[u8],
        count: u32,
    ) -> Result<()> {
        if code.len() != 4 {
            return Err(Dot001Error::blend_file(
                "block code must be 4 bytes",
                dot001_error::BlendFileErrorKind::InvalidHeader,
            ));
        }

        // code
        w.write_all(code)
            .map_err(|e| Dot001Error::io(format!("write block code failed: {e}")))?;

        // sdna_index (u32 LE)
        w.write_all(&sdna_index.to_le_bytes())
            .map_err(|e| Dot001Error::io(format!("write sdna_index failed: {e}")))?;

        // old_address (u64 LE)
        w.write_all(&old_address.to_le_bytes())
            .map_err(|e| Dot001Error::io(format!("write old_address failed: {e}")))?;

        // len (u64 LE)
        let len = payload.len() as u64;
        w.write_all(&len.to_le_bytes())
            .map_err(|e| Dot001Error::io(format!("write len failed: {e}")))?;

        // count (u64 LE, but input is u32)
        w.write_all(&(count as u64).to_le_bytes())
            .map_err(|e| Dot001Error::io(format!("write count failed: {e}")))?;

        // payload
        if len > 0 {
            w.write_all(payload)
                .map_err(|e| Dot001Error::io(format!("write payload failed: {e}")))?;
        }

        Ok(())
    }
}
