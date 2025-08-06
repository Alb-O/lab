use crate::output_utils::CommandSummary;
use crate::util::CommandContext;
use dot001_error::Dot001Error;
use std::path::PathBuf;

pub fn cmd_info(file_path: PathBuf, ctx: &CommandContext) -> Result<(), Dot001Error> {
    let blend_file = ctx.load_blend_file(&file_path)?;

    ctx.output
        .print_info_fmt(format_args!("File: {}", file_path.display()));

    let mut summary = CommandSummary::new("Header")
        .add_item(
            "Pointer size",
            format!("{} bytes", blend_file.header().pointer_size),
        )
        .add_item(
            "Endianness",
            if blend_file.header().is_little_endian {
                "little".to_string()
            } else {
                "big".to_string()
            },
        )
        .add_item("Version", blend_file.header().version.to_string())
        .add_count("Total blocks", blend_file.blocks_len());

    if let Ok(dna) = blend_file.dna() {
        summary = summary
            .add_count("DNA structs", dna.structs.len())
            .add_count("DNA types", dna.types.len());
    }

    summary.print(ctx);
    Ok(())
}
