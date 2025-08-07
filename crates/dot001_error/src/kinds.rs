//! Error kind enums for different operation domains
//!
//! This module contains all the specific error kind enums that categorize
//! errors within each domain (Parser, Editor, Diff, etc.). These provide
//! fine-grained error classification for programmatic handling.

use thiserror::Error;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

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
