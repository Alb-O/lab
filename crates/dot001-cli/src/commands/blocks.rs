#[cfg(feature = "trace")]
use crate::commands::NameResolver;
use dot001_error::Dot001Error;
use dot001_parser::ParseOptions;
use std::path::PathBuf;

pub fn cmd_blocks(
    file_path: PathBuf,
    options: &ParseOptions,
    no_auto_decompress: bool,
) -> Result<(), Dot001Error> {
    let mut blend_file = crate::util::load_blend_file(&file_path, options, no_auto_decompress)?;
    println!("Blocks in {}:", file_path.display());
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

        println!("  {i}: {display_name} (size: {size}, addr: 0x{address:x})");
    }
    Ok(())
}
