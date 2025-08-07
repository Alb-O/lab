use crate::DisplayTemplate;
use crate::block_utils::BlockProcessor;
use crate::util::CommandContext;
use dot001_events::error::Error;
use std::path::PathBuf;

pub fn cmd_blocks(
    file_path: PathBuf,
    show_data: bool,
    template: DisplayTemplate,
    ctx: &CommandContext,
) -> Result<(), Error> {
    let mut blend_file = ctx.load_blend_file(&file_path)?;
    ctx.output
        .print_info_fmt(format_args!("Blocks in {}:", file_path.display()));

    let blocks = BlockProcessor::new(&mut blend_file)
        .with_data_blocks(show_data)
        .collect();

    for block in blocks {
        let display = block.create_display(&template);
        ctx.output.print_result_fmt(format_args!("  {display}"));
    }
    Ok(())
}
