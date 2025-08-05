//! Helper functions for creating standardized errors
//!
//! This module provides convenient helper functions for creating errors with
//! consistent messaging and context across the dot001 toolkit. These replace
//! ad hoc error creation patterns and ensure uniformity.

use crate::kinds::*;
use crate::types::Dot001Error;
use std::path::PathBuf;

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

    /// Create a new checkpoint error
    pub fn checkpoint<M: Into<String>>(message: M, kind: CheckpointErrorKind) -> Self {
        Self::Checkpoint {
            message: message.into(),
            project_path: None,
            kind,
        }
    }

    /// Create a new config error  
    pub fn config<M: Into<String>>(message: M, kind: ConfigErrorKind) -> Self {
        Self::Config {
            message: message.into(),
            config_path: None,
            kind,
        }
    }

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

    // === Editor Domain Helpers ===

    /// Create an editor error for blocks not found
    pub fn editor_block_not_found<M: Into<String>>(message: M) -> Self {
        Self::editor(message, EditorErrorKind::BlockNotFound)
    }

    /// Create an editor error for blocks without ID structures
    pub fn editor_no_id_structure<M: Into<String>>(message: M) -> Self {
        Self::editor(message, EditorErrorKind::NoIdStructure)
    }

    /// Create an editor error for invalid names
    pub fn editor_invalid_name<M: Into<String>>(message: M) -> Self {
        Self::editor(message, EditorErrorKind::InvalidName)
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

    // === CLI Domain Helpers ===

    /// Create a CLI error for invalid arguments
    pub fn cli_invalid_arguments<M: Into<String>>(message: M) -> Self {
        Self::cli(message, CliErrorKind::InvalidArguments)
    }

    /// Create a CLI error for missing arguments
    pub fn cli_missing_argument<M: Into<String>>(message: M) -> Self {
        Self::cli(message, CliErrorKind::MissingArgument)
    }

    // === Checkpoint Domain Helpers ===

    /// Create a checkpoint error for invalid project paths
    pub fn checkpoint_invalid_path<M: Into<String>>(message: M) -> Self {
        Self::checkpoint(message, CheckpointErrorKind::InvalidProjectPath)
    }

    /// Create a checkpoint error for creation failures
    pub fn checkpoint_creation_failed<M: Into<String>>(message: M) -> Self {
        Self::checkpoint(message, CheckpointErrorKind::CreationFailed)
    }

    // === Config Domain Helpers ===

    /// Create a config error for missing configuration files
    pub fn config_not_found<M: Into<String>>(message: M) -> Self {
        Self::config(message, ConfigErrorKind::NotFound)
    }

    /// Create a config error for invalid format
    pub fn config_invalid_format<M: Into<String>>(message: M) -> Self {
        Self::config(message, ConfigErrorKind::InvalidFormat)
    }
}
