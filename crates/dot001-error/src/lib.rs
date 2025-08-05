//! # dot001-error - Unified Error Handling
//!
//! This crate provides a unified error system for the dot001 toolkit, offering:
//! - Consistent error types across all crates
//! - Rich contextual information (file paths, block indices, operation details)
//! - User-friendly error messages for CLI and developer-friendly details for debugging
//! - Efficient error conversion between crate boundaries
//!
//! ## Design Principles
//!
//! - **Hierarchical**: Errors are organized by domain (Parser, Editor, Diff, etc.)  
//! - **Contextual**: Errors carry operation context like file paths and block indices
//! - **Convertible**: Seamless conversion between error types
//! - **User-Focused**: Clear messages for end users, detailed info for developers

use std::path::PathBuf;
use thiserror::Error;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// The main unified error type for the dot001 toolkit
#[derive(Error, Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum Dot001Error {
    /// File system and I/O related errors
    #[error("I/O error: {message}")]
    Io {
        message: String,
        #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
        file_path: Option<PathBuf>,
        // Note: We store the source error message instead of the error itself for cloneability
        #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
        source_message: Option<String>,
    },

    /// Blend file parsing and structure errors
    #[error("Blend file error: {message}")]
    BlendFile {
        message: String,
        #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
        file_path: Option<PathBuf>,
        #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
        block_index: Option<usize>,
        kind: BlendFileErrorKind,
    },

    /// Editing operation errors
    #[error("Edit operation error: {message}")]
    Editor {
        message: String,
        #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
        file_path: Option<PathBuf>,
        #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
        block_index: Option<usize>,
        #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
        operation: Option<String>,
        kind: EditorErrorKind,
    },

    /// Diff and comparison errors
    #[error("Diff operation error: {message}")]
    Diff {
        message: String,
        #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
        file1_path: Option<PathBuf>,
        #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
        file2_path: Option<PathBuf>,
        #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
        block_index: Option<usize>,
        kind: DiffErrorKind,
    },

    /// Dependency tracing errors
    #[error("Tracer error: {message}")]
    Tracer {
        message: String,
        #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
        file_path: Option<PathBuf>,
        #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
        block_index: Option<usize>,
        kind: TracerErrorKind,
    },

    /// Checkpoint management errors
    #[error("Checkpoint error: {message}")]
    Checkpoint {
        message: String,
        #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
        project_path: Option<PathBuf>,
        kind: CheckpointErrorKind,
    },

    /// CLI and user interface errors
    #[error("CLI error: {message}")]
    Cli {
        message: String,
        #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
        command: Option<String>,
        kind: CliErrorKind,
    },

    /// Configuration and setup errors
    #[error("Configuration error: {message}")]
    Config {
        message: String,
        #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
        config_path: Option<PathBuf>,
        kind: ConfigErrorKind,
    },
}

/// Specific kinds of blend file errors
#[derive(Error, Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum BlendFileErrorKind {
    #[error("Invalid file header")]
    InvalidHeader,
    #[error("Invalid magic bytes")]
    InvalidMagic,
    #[error("Unsupported file version")]
    UnsupportedVersion,
    #[error("Missing DNA block")]
    NoDnaFound,
    #[error("Invalid block index")]
    InvalidBlockIndex,
    #[error("DNA parsing error")]
    DnaError,
    #[error("Invalid data structure")]
    InvalidData,
    #[error("Field access error")]
    InvalidField,
    #[error("Compression not supported")]
    UnsupportedCompression,
    #[error("Decompression failed")]
    DecompressionFailed,
    #[error("File size limit exceeded")]
    SizeLimitExceeded,
}

/// Specific kinds of editor errors
#[derive(Error, Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum EditorErrorKind {
    #[error("Block not found")]
    BlockNotFound,
    #[error("Block has no ID structure")]
    NoIdStructure,
    #[error("Name validation failed")]
    InvalidName,
    #[error("Name too long")]
    NameTooLong,
    #[error("Invalid characters")]
    InvalidCharacters,
    #[error("Library path validation failed")]
    InvalidLibraryPath,
    #[error("Operation not permitted")]
    OperationNotPermitted,
}

/// Specific kinds of diff errors
#[derive(Error, Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum DiffErrorKind {
    #[error("Files are incompatible for comparison")]
    IncompatibleFiles,
    #[error("Analysis failed")]
    AnalysisFailed,
    #[error("Insufficient data for comparison")]
    InsufficientData,
    #[error("Mesh comparison failed")]
    MeshComparisonFailed,
}

/// Specific kinds of tracer errors  
#[derive(Error, Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum TracerErrorKind {
    #[error("Dependency resolution failed")]
    DependencyResolutionFailed,
    #[error("Name resolution failed")]
    NameResolutionFailed,
    #[error("Block expansion failed")]
    BlockExpansionFailed,
    #[error("Circular dependency detected")]
    CircularDependency,
}

/// Specific kinds of checkpoint errors
#[derive(Error, Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum CheckpointErrorKind {
    #[error("Invalid project path")]
    InvalidProjectPath,
    #[error("Checkpoint creation failed")]
    CreationFailed,
    #[error("Checkpoint restoration failed")]
    RestorationFailed,
    #[error("Storage backend error")]
    StorageError,
}

/// Specific kinds of CLI errors
#[derive(Error, Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum CliErrorKind {
    #[error("Invalid command arguments")]
    InvalidArguments,
    #[error("Missing required argument")]
    MissingArgument,
    #[error("Command execution failed")]
    ExecutionFailed,
    #[error("Output formatting failed")]
    OutputFormatError,
}

/// Specific kinds of configuration errors
#[derive(Error, Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum ConfigErrorKind {
    #[error("Configuration file not found")]
    NotFound,
    #[error("Invalid configuration format")]
    InvalidFormat,
    #[error("Missing required configuration")]
    MissingRequired,
    #[error("Invalid configuration value")]
    InvalidValue,
}

/// Convenient result type for dot001 operations
pub type Result<T> = std::result::Result<T, Dot001Error>;

impl Dot001Error {
    /// Create a new I/O error with optional context
    pub fn io<M: Into<String>>(message: M) -> Self {
        Self::Io {
            message: message.into(),
            file_path: None,
            source_message: None,
        }
    }

    /// Create a new I/O error with file path context
    pub fn io_with_path<M: Into<String>, P: Into<PathBuf>>(message: M, path: P) -> Self {
        Self::Io {
            message: message.into(),
            file_path: Some(path.into()),
            source_message: None,
        }
    }

    /// Create a new blend file error
    pub fn blend_file<M: Into<String>>(message: M, kind: BlendFileErrorKind) -> Self {
        Self::BlendFile {
            message: message.into(),
            file_path: None,
            block_index: None,
            kind,
        }
    }

    /// Create a new blend file error with context
    pub fn blend_file_with_context<M: Into<String>, P: Into<PathBuf>>(
        message: M,
        kind: BlendFileErrorKind,
        file_path: Option<P>,
        block_index: Option<usize>,
    ) -> Self {
        Self::BlendFile {
            message: message.into(),
            file_path: file_path.map(|p| p.into()),
            block_index,
            kind,
        }
    }

    /// Create a new editor error
    pub fn editor<M: Into<String>>(message: M, kind: EditorErrorKind) -> Self {
        Self::Editor {
            message: message.into(),
            file_path: None,
            block_index: None,
            operation: None,
            kind,
        }
    }

    /// Create a new editor error with context
    pub fn editor_with_context<M: Into<String>, P: Into<PathBuf>, O: Into<String>>(
        message: M,
        kind: EditorErrorKind,
        file_path: Option<P>,
        block_index: Option<usize>,
        operation: Option<O>,
    ) -> Self {
        Self::Editor {
            message: message.into(),
            file_path: file_path.map(|p| p.into()),
            block_index,
            operation: operation.map(|o| o.into()),
            kind,
        }
    }

    /// Create a new diff error
    pub fn diff<M: Into<String>>(message: M, kind: DiffErrorKind) -> Self {
        Self::Diff {
            message: message.into(),
            file1_path: None,
            file2_path: None,
            block_index: None,
            kind,
        }
    }

    /// Create a new tracer error
    pub fn tracer<M: Into<String>>(message: M, kind: TracerErrorKind) -> Self {
        Self::Tracer {
            message: message.into(),
            file_path: None,
            block_index: None,
            kind,
        }
    }

    /// Create a new CLI error
    pub fn cli<M: Into<String>>(message: M, kind: CliErrorKind) -> Self {
        Self::Cli {
            message: message.into(),
            command: None,
            kind,
        }
    }

    /// Add file path context to an existing error
    pub fn with_file_path<P: Into<PathBuf>>(mut self, path: P) -> Self {
        let path = path.into();
        match &mut self {
            Self::Io { file_path, .. } => *file_path = Some(path),
            Self::BlendFile { file_path, .. } => *file_path = Some(path),
            Self::Editor { file_path, .. } => *file_path = Some(path),
            Self::Diff { file1_path, .. } => *file1_path = Some(path),
            Self::Tracer { file_path, .. } => *file_path = Some(path),
            Self::Config { config_path, .. } => *config_path = Some(path),
            _ => {} // Other variants don't have file paths
        }
        self
    }

    /// Add block index context to an existing error
    pub fn with_block_index(mut self, index: usize) -> Self {
        match &mut self {
            Self::BlendFile { block_index, .. } => *block_index = Some(index),
            Self::Editor { block_index, .. } => *block_index = Some(index),
            Self::Diff { block_index, .. } => *block_index = Some(index),
            Self::Tracer { block_index, .. } => *block_index = Some(index),
            _ => {} // Other variants don't have block indices
        }
        self
    }

    /// Add operation context to editor errors
    pub fn with_operation<O: Into<String>>(mut self, operation: O) -> Self {
        if let Self::Editor { operation: op, .. } = &mut self {
            *op = Some(operation.into());
        }
        self
    }

    /// Check if this error is of a specific kind
    pub fn is_io(&self) -> bool {
        matches!(self, Self::Io { .. })
    }

    /// Check if this error is a blend file error
    pub fn is_blend_file(&self) -> bool {
        matches!(self, Self::BlendFile { .. })
    }

    /// Check if this error is an editor error
    pub fn is_editor(&self) -> bool {
        matches!(self, Self::Editor { .. })
    }

    /// Get the file path associated with this error, if any
    pub fn file_path(&self) -> Option<&PathBuf> {
        match self {
            Self::Io { file_path, .. } => file_path.as_ref(),
            Self::BlendFile { file_path, .. } => file_path.as_ref(),
            Self::Editor { file_path, .. } => file_path.as_ref(),
            Self::Diff { file1_path, .. } => file1_path.as_ref(),
            Self::Tracer { file_path, .. } => file_path.as_ref(),
            Self::Config { config_path, .. } => config_path.as_ref(),
            _ => None,
        }
    }

    /// Get the block index associated with this error, if any
    pub fn block_index(&self) -> Option<usize> {
        match self {
            Self::BlendFile { block_index, .. } => *block_index,
            Self::Editor { block_index, .. } => *block_index,
            Self::Diff { block_index, .. } => *block_index,
            Self::Tracer { block_index, .. } => *block_index,
            _ => None,
        }
    }

    /// Get a user-friendly error message suitable for CLI display
    pub fn user_message(&self) -> String {
        match self {
            Self::Io {
                message, file_path, ..
            } => {
                if let Some(path) = file_path {
                    format!("File error in '{}': {}", path.display(), message)
                } else {
                    message.clone()
                }
            }
            Self::BlendFile {
                message,
                file_path,
                block_index,
                kind: _,
            } => {
                let mut msg = format!("Blend file error: {message}");
                if let Some(path) = file_path {
                    msg.push_str(&format!(" (file: {})", path.display()));
                }
                if let Some(index) = block_index {
                    msg.push_str(&format!(" (block: {index})"));
                }
                msg
            }
            Self::Editor {
                message,
                file_path,
                block_index,
                operation,
                ..
            } => {
                let mut msg = format!("Edit failed: {message}");
                if let Some(op) = operation {
                    msg.push_str(&format!(" (operation: {op})"));
                }
                if let Some(path) = file_path {
                    msg.push_str(&format!(" (file: {})", path.display()));
                }
                if let Some(index) = block_index {
                    msg.push_str(&format!(" (block: {index})"));
                }
                msg
            }
            Self::Diff {
                message,
                file1_path,
                file2_path,
                block_index,
                ..
            } => {
                let mut msg = format!("Diff failed: {message}");
                if let Some(f1) = file1_path {
                    if let Some(f2) = file2_path {
                        msg.push_str(&format!(
                            " (comparing '{}' and '{}')",
                            f1.display(),
                            f2.display()
                        ));
                    } else {
                        msg.push_str(&format!(" (file1: {})", f1.display()));
                    }
                } else if let Some(f2) = file2_path {
                    msg.push_str(&format!(" (file2: {})", f2.display()));
                }
                if let Some(index) = block_index {
                    msg.push_str(&format!(" (block: {index})"));
                }
                msg
            }
            Self::Tracer {
                message,
                file_path,
                block_index,
                ..
            } => {
                let mut msg = format!("Dependency tracing failed: {message}");
                if let Some(path) = file_path {
                    msg.push_str(&format!(" (file: {})", path.display()));
                }
                if let Some(index) = block_index {
                    msg.push_str(&format!(" (block: {index})"));
                }
                msg
            }
            Self::Cli {
                message, command, ..
            } => {
                if let Some(cmd) = command {
                    format!("Command '{cmd}' failed: {message}")
                } else {
                    format!("CLI error: {message}")
                }
            }
            Self::Config {
                message,
                config_path,
                ..
            } => {
                if let Some(path) = config_path {
                    format!("Configuration error in '{}': {}", path.display(), message)
                } else {
                    format!("Configuration error: {message}")
                }
            }
            Self::Checkpoint {
                message,
                project_path,
                ..
            } => {
                if let Some(path) = project_path {
                    format!("Checkpoint error in '{}': {}", path.display(), message)
                } else {
                    format!("Checkpoint error: {message}")
                }
            }
        }
    }

    /// Get a detailed error message with full context for debugging
    pub fn debug_message(&self) -> String {
        let mut msg = self.user_message();

        // Add error kind information
        match self {
            Self::BlendFile { kind, .. } => {
                msg.push_str(&format!(" [Kind: {kind}]"));
            }
            Self::Editor { kind, .. } => {
                msg.push_str(&format!(" [Kind: {kind}]"));
            }
            Self::Diff { kind, .. } => {
                msg.push_str(&format!(" [Kind: {kind}]"));
            }
            Self::Tracer { kind, .. } => {
                msg.push_str(&format!(" [Kind: {kind}]"));
            }
            Self::Cli { kind, .. } => {
                msg.push_str(&format!(" [Kind: {kind}]"));
            }
            Self::Config { kind, .. } => {
                msg.push_str(&format!(" [Kind: {kind}]"));
            }
            Self::Checkpoint { kind, .. } => {
                msg.push_str(&format!(" [Kind: {kind}]"));
            }
            _ => {}
        }

        msg
    }

    /// Get a short error summary without context details
    pub fn summary(&self) -> String {
        match self {
            Self::Io { message, .. } => format!("I/O: {message}"),
            Self::BlendFile { kind, .. } => format!("Blend: {kind}"),
            Self::Editor { kind, .. } => format!("Edit: {kind}"),
            Self::Diff { kind, .. } => format!("Diff: {kind}"),
            Self::Tracer { kind, .. } => format!("Trace: {kind}"),
            Self::Cli { kind, .. } => format!("CLI: {kind}"),
            Self::Config { kind, .. } => format!("Config: {kind}"),
            Self::Checkpoint { kind, .. } => format!("Checkpoint: {kind}"),
        }
    }
}

/// Standardized error helper functions for consistent error creation across crates.
/// These functions replace ad hoc error creation patterns and ensure consistent
/// user/debug messages across the dot001 toolkit.
impl Dot001Error {
    // === Parser Domain Helpers ===

    /// Create a parser error for invalid file headers
    pub fn parser_invalid_header<M: Into<String>>(message: M) -> Self {
        Self::blend_file(message, BlendFileErrorKind::InvalidHeader)
    }

    /// Create a parser error for missing DNA blocks
    pub fn parser_no_dna<M: Into<String>>(message: M) -> Self {
        Self::blend_file(message, BlendFileErrorKind::NoDnaFound)
    }

    /// Create a parser error for invalid block indices
    pub fn parser_invalid_block<M: Into<String>>(message: M) -> Self {
        Self::blend_file(message, BlendFileErrorKind::InvalidBlockIndex)
    }

    /// Create a parser error for DNA parsing failures
    pub fn parser_dna_error<M: Into<String>>(message: M) -> Self {
        Self::blend_file(message, BlendFileErrorKind::DnaError)
    }

    /// Create a parser error for field access failures
    pub fn parser_invalid_field<M: Into<String>>(message: M) -> Self {
        Self::blend_file(message, BlendFileErrorKind::InvalidField)
    }

    // === Tracer Domain Helpers ===

    /// Create a tracer error for dependency resolution failures
    pub fn tracer_dependency_failed<M: Into<String>>(message: M) -> Self {
        Self::tracer(message, TracerErrorKind::DependencyResolutionFailed)
    }

    /// Create a tracer error for name resolution failures
    pub fn tracer_name_resolution_failed<M: Into<String>>(message: M) -> Self {
        Self::tracer(message, TracerErrorKind::NameResolutionFailed)
    }

    /// Create a tracer error for block expansion failures
    pub fn tracer_block_expansion_failed<M: Into<String>>(message: M) -> Self {
        Self::tracer(message, TracerErrorKind::BlockExpansionFailed)
    }

    /// Create a tracer error for circular dependency detection
    pub fn tracer_circular_dependency<M: Into<String>>(message: M) -> Self {
        Self::tracer(message, TracerErrorKind::CircularDependency)
    }

    // === Diff Domain Helpers ===

    /// Create a diff error for incompatible files
    pub fn diff_incompatible_files<M: Into<String>>(message: M) -> Self {
        Self::diff(message, DiffErrorKind::IncompatibleFiles)
    }

    /// Create a diff error for analysis failures
    pub fn diff_analysis_failed<M: Into<String>>(message: M) -> Self {
        Self::diff(message, DiffErrorKind::AnalysisFailed)
    }

    /// Create a diff error for insufficient data
    pub fn diff_insufficient_data<M: Into<String>>(message: M) -> Self {
        Self::diff(message, DiffErrorKind::InsufficientData)
    }

    /// Create a diff error for mesh comparison failures
    pub fn diff_mesh_comparison_failed<M: Into<String>>(message: M) -> Self {
        Self::diff(message, DiffErrorKind::MeshComparisonFailed)
    }

    // === Editor Domain Helpers ===

    /// Create an editor error for missing blocks
    pub fn editor_block_not_found<M: Into<String>>(message: M) -> Self {
        Self::editor(message, EditorErrorKind::BlockNotFound)
    }

    /// Create an editor error for invalid names
    pub fn editor_invalid_name<M: Into<String>>(message: M) -> Self {
        Self::editor(message, EditorErrorKind::InvalidName)
    }

    /// Create an editor error for missing ID structures
    pub fn editor_no_id_structure<M: Into<String>>(message: M) -> Self {
        Self::editor(message, EditorErrorKind::NoIdStructure)
    }

    // === CLI Domain Helpers ===

    /// Create a CLI error for invalid arguments
    pub fn cli_invalid_arguments<M: Into<String>>(message: M) -> Self {
        Self::cli(message, CliErrorKind::InvalidArguments)
    }

    /// Create a CLI error for missing required arguments
    pub fn cli_missing_argument<M: Into<String>>(message: M) -> Self {
        Self::cli(message, CliErrorKind::MissingArgument)
    }

    /// Create a CLI error for execution failures
    pub fn cli_execution_failed<M: Into<String>>(message: M) -> Self {
        Self::cli(message, CliErrorKind::ExecutionFailed)
    }
}

/// Convert from std::io::Error
impl From<std::io::Error> for Dot001Error {
    fn from(err: std::io::Error) -> Self {
        Self::Io {
            message: err.to_string(),
            file_path: None,
            source_message: Some(format!("IO Error: {err}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_creation() {
        let err = Dot001Error::blend_file("Invalid header", BlendFileErrorKind::InvalidHeader);
        assert!(err.is_blend_file());
        assert_eq!(err.block_index(), None);
    }

    #[test]
    fn test_error_context() {
        let err = Dot001Error::editor("Block not found", EditorErrorKind::BlockNotFound)
            .with_file_path("/path/to/file.blend")
            .with_block_index(42)
            .with_operation("rename");

        assert_eq!(err.file_path(), Some(&PathBuf::from("/path/to/file.blend")));
        assert_eq!(err.block_index(), Some(42));
    }

    #[test]
    fn test_user_message() {
        let err = Dot001Error::editor("Block not found", EditorErrorKind::BlockNotFound)
            .with_file_path("/path/to/file.blend")
            .with_operation("rename");

        let msg = err.user_message();
        assert!(msg.contains("Edit failed"));
        assert!(msg.contains("rename"));
        assert!(msg.contains("file.blend"));
    }

    #[test]
    fn test_io_conversion() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "File not found");
        let err: Dot001Error = io_err.into();
        assert!(err.is_io());
    }
}
