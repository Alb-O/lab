//! Main error types for the dot001 toolkit
//!
//! This module contains the primary `Dot001Error` enum that serves as the
//! unified error type across all dot001 crates. It provides rich contextual
//! information and domain-specific error categorization.

use crate::kinds::*;
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

    /// Configuration and settings errors
    #[error("Configuration error: {message}")]
    Config {
        message: String,
        #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
        config_path: Option<PathBuf>,
        kind: ConfigErrorKind,
    },
}

/// Convenient result type for dot001 operations
pub type Result<T> = std::result::Result<T, Dot001Error>;
