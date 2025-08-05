use crate::util::OutputHandler;
use dot001_error::Dot001Error;
use dot001_parser::ParseOptions;
use std::path::PathBuf;

pub fn cmd_info(
    file_path: PathBuf,
    options: &ParseOptions,
    no_auto_decompress: bool,
    output: &OutputHandler,
) -> Result<(), Dot001Error> {
    let blend_file = crate::util::load_blend_file(&file_path, options, no_auto_decompress)?;
    output.print_info_fmt(format_args!("File: {}", file_path.display()));
    output.print_info("Header:");
    output.print_result_fmt(format_args!(
        "  Pointer size: {} bytes",
        blend_file.header().pointer_size
    ));
    output.print_result_fmt(format_args!(
        "  Endianness: {}",
        if blend_file.header().is_little_endian {
            "little"
        } else {
            "big"
        }
    ));
    output.print_result_fmt(format_args!("  Version: {}", blend_file.header().version));
    output.print_result_fmt(format_args!("  Total blocks: {}", blend_file.blocks_len()));
    if let Ok(dna) = blend_file.dna() {
        output.print_result_fmt(format_args!("  DNA structs: {}", dna.structs.len()));
        output.print_result_fmt(format_args!("  DNA types: {}", dna.types.len()));
    }
    Ok(())
}
