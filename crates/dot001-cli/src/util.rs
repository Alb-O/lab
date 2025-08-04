// Utility functions for CLI

use dot001_parser::{DecompressionPolicy, ParseOptions};
use std::path::PathBuf;

pub fn create_parse_options(cli: &crate::Cli) -> ParseOptions {
    let mut policy = DecompressionPolicy {
        max_in_memory_bytes: cli.max_in_memory * 1024 * 1024,
        temp_dir: cli.temp_dir.clone(),
        ..Default::default()
    };
    if let Some(prefer_mmap) = cli.prefer_mmap {
        policy.prefer_mmap_temp = prefer_mmap;
    }
    ParseOptions {
        decompression_policy: policy,
    }
}

pub fn load_blend_file(
    file_path: &PathBuf,
    options: &ParseOptions,
    no_auto_decompress: bool,
) -> dot001_tracer::Result<dot001_tracer::BlendFile<Box<dyn dot001_parser::ReadSeekSend>>> {
    use std::fs::File;
    use std::io::BufReader;
    if no_auto_decompress {
        let file = File::open(file_path)?;
        let reader = BufReader::new(file);
        let boxed_reader: Box<dyn dot001_parser::ReadSeekSend> = Box::new(reader);
        dot001_tracer::BlendFile::new(boxed_reader)
    } else {
        let (blend_file, _mode) = dot001_parser::parse_from_path(file_path, Some(options))?;
        Ok(blend_file)
    }
}
