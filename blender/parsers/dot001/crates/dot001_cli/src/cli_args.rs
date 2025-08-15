use clap::Args;
use std::path::PathBuf;

/// Display configuration arguments shared across commands
#[derive(Debug, Clone, Args)]
pub struct DisplayArgs {
    /// Block display template
    #[arg(short = 't', long, value_enum, default_value_t = crate::DisplayTemplate::Simple, help = "Block display template")]
    pub template: crate::DisplayTemplate,

    /// Include DATA blocks in output (filtered out by default)
    #[arg(long, help = "Include DATA blocks in output (filtered out by default)")]
    pub show_data: bool,
}

/// Output formatting arguments shared across commands  
#[derive(Debug, Clone, Args)]
pub struct OutputFormatArgs {
    /// Output format
    #[arg(short, long, value_enum, default_value_t = crate::OutputFormat::Flat)]
    pub format: crate::OutputFormat,

    /// Use ASCII characters instead of Unicode box characters for tree output
    #[arg(
        long,
        help = "Use ASCII characters instead of Unicode box characters for tree output"
    )]
    pub ascii: bool,
}

/// JSON output argument
#[derive(Debug, Clone, Args)]
pub struct JsonArgs {
    /// Output as JSON
    #[arg(long, help = "Output as JSON")]
    pub json: bool,
}

/// Execution control arguments
#[derive(Debug, Clone, Args)]
pub struct ExecutionArgs {
    /// Preview changes without modifying the file
    #[arg(long, help = "Preview changes without modifying the file")]
    pub dry_run: bool,
}

/// File input argument
#[derive(Debug, Clone, Args)]
pub struct FileArgs {
    /// Input blend file
    #[arg(index = 1)]
    pub file: PathBuf,
}

/// Two file input arguments (for diff operations)
#[derive(Debug, Clone, Args)]
pub struct TwoFileArgs {
    /// First blend file
    #[arg(index = 1)]
    pub file1: PathBuf,

    /// Second blend file  
    #[arg(index = 2)]
    pub file2: PathBuf,
}

/// Block identifier argument
#[derive(Debug, Clone, Args)]
pub struct BlockIdentifierArgs {
    /// Block index or datablock name (e.g., '5' or 'Cube')
    #[arg(
        index = 2,
        help = "Block index or datablock name (e.g., '5' or 'Cube')"
    )]
    pub block_index: String,
}
