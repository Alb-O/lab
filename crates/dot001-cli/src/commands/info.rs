use dot001_parser::ParseOptions;
use std::path::PathBuf;

pub fn cmd_info(
    file_path: PathBuf,
    options: &ParseOptions,
    no_auto_decompress: bool,
) -> anyhow::Result<()> {
    let blend_file = crate::util::load_blend_file(&file_path, options, no_auto_decompress)?;
    println!("File: {}", file_path.display());
    println!("Header:");
    println!("  Pointer size: {} bytes", blend_file.header.pointer_size);
    println!(
        "  Endianness: {}",
        if blend_file.header.is_little_endian {
            "little"
        } else {
            "big"
        }
    );
    println!("  Version: {}", blend_file.header.version);
    println!("  Total blocks: {}", blend_file.blocks.len());
    if let Some(dna) = &blend_file.dna {
        println!("  DNA structs: {}", dna.structs.len());
        println!("  DNA types: {}", dna.types.len());
    }
    Ok(())
}
