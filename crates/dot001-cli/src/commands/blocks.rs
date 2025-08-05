#[cfg(feature = "trace")]
use crate::commands::NameResolver;
use crate::util::CommandContext;
use dot001_error::Dot001Error;
use std::path::PathBuf;

pub fn cmd_blocks(file_path: PathBuf, ctx: &CommandContext) -> Result<(), Dot001Error> {
    let mut blend_file =
        crate::util::load_blend_file(&file_path, ctx.parse_options, ctx.no_auto_decompress)?;
    ctx.output
        .print_info_fmt(format_args!("Blocks in {}:", file_path.display()));
    let block_info: Vec<(usize, String, u32, u64)> = (0..blend_file.blocks_len())
        .filter_map(|i| {
            blend_file.get_block(i).map(|block| {
                let code_str = String::from_utf8_lossy(&block.header.code)
                    .trim_end_matches('\0')
                    .to_string();
                (i, code_str, block.header.size, block.header.old_address)
            })
        })
        .collect();
    for (i, code_str, size, address) in block_info {
        #[cfg(feature = "trace")]
        let display_name = NameResolver::get_display_name(i, &mut blend_file, &code_str);
        #[cfg(not(feature = "trace"))]
        let display_name = format!("{code_str}");

        ctx.output.print_result_fmt(format_args!(
            "  {i}: {display_name} (size: {size}, addr: 0x{address:x})"
        ));
    }
    Ok(())
}
