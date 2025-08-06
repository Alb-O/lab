use crate::block_display::{BlockDisplay, BlockInfo, DetailedFormatter};
use crate::util::CommandContext;
use dot001_error::Dot001Error;
use std::path::PathBuf;

pub fn cmd_blocks(
    file_path: PathBuf,
    show_data: bool,
    ctx: &CommandContext,
) -> Result<(), Dot001Error> {
    let mut blend_file = ctx.load_blend_file(&file_path)?;
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
        .filter(|(_, code_str, _, _)| {
            // Filter out DATA blocks by default unless show_data is true
            show_data || code_str != "DATA"
        })
        .collect();
    for (i, _code_str, size, address) in block_info {
        let block_info = BlockInfo::from_blend_file(i, &mut blend_file)
            .unwrap_or_else(|_| BlockInfo::new(i, "????".to_string()));

        let detailed_formatter = DetailedFormatter::new()
            .with_size(size as u64)
            .with_address(address);
        let display = BlockDisplay::new(block_info).with_formatter(detailed_formatter);
        ctx.output.print_result_fmt(format_args!("  {display}"));
    }
    Ok(())
}
