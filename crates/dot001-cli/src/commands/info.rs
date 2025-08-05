use crate::util::CommandContext;
use dot001_error::Dot001Error;
use std::path::PathBuf;

pub fn cmd_info(file_path: PathBuf, ctx: &CommandContext) -> Result<(), Dot001Error> {
    let blend_file =
        crate::util::load_blend_file(&file_path, ctx.parse_options, ctx.no_auto_decompress)?;
    ctx.output
        .print_info_fmt(format_args!("File: {}", file_path.display()));
    ctx.output.print_info("Header:");
    ctx.output.print_result_fmt(format_args!(
        "  Pointer size: {} bytes",
        blend_file.header().pointer_size
    ));
    ctx.output.print_result_fmt(format_args!(
        "  Endianness: {}",
        if blend_file.header().is_little_endian {
            "little"
        } else {
            "big"
        }
    ));
    ctx.output
        .print_result_fmt(format_args!("  Version: {}", blend_file.header().version));
    ctx.output
        .print_result_fmt(format_args!("  Total blocks: {}", blend_file.blocks_len()));
    if let Ok(dna) = blend_file.dna() {
        ctx.output
            .print_result_fmt(format_args!("  DNA structs: {}", dna.structs.len()));
        ctx.output
            .print_result_fmt(format_args!("  DNA types: {}", dna.types.len()));
    }
    Ok(())
}
