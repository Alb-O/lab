use crate::{execution_failed_error, missing_argument_error};
use dot001_error::Dot001Error;
use dot001_writer::{BlendWriter, SeedDnaProvider, WriteTemplate};
use std::path::PathBuf;

/// Build a synthetic .blend using Blender 5.0 header format and DNA from a seed .blend.
/// For now this implements the M1 milestone: Header + DNA1 + ENDB.
pub fn cmd_build_blend(seed_path: PathBuf, out_path: PathBuf) -> Result<(), Dot001Error> {
    // Validate args
    if seed_path.as_os_str().is_empty() {
        return Err(missing_argument_error("Missing --seed path"));
    }
    if out_path.as_os_str().is_empty() {
        return Err(missing_argument_error("Missing --out path"));
    }

    // Load DNA from seed
    let provider = SeedDnaProvider::from_seed_path(&seed_path)?;

    // Write minimal file
    let writer = BlendWriter::default();
    writer
        .write_with_seed(&out_path, WriteTemplate::Minimal, &provider)
        .map_err(|e| {
            execution_failed_error(format!("Failed to write blend: {}", e.user_message()))
        })?;

    Ok(())
}
