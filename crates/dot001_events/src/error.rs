//! Error types and Result alias for the dot001 toolkit
//!
//! This module contains the unified error system ported from dot001_error,
//! providing hierarchical error types organized by domain with rich contextual
//! information.

use std::path::PathBuf;
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
    #[error("Invalid range for buffer access")]
    InvalidRange,
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

/// Additional error kinds for writer domain
#[derive(Error, Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum WriterErrorKind {
    #[error("Write operation failed")]
    WriteFailed,
    #[error("Header generation failed")]
    HeaderGenerationFailed,
    #[error("Block injection failed")]
    BlockInjectionFailed,
    #[error("DNA provider error")]
    DnaProviderError,
}

/// Additional error kinds for watcher domain
#[derive(Error, Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum WatcherErrorKind {
    #[error("Watch setup failed")]
    WatchSetupFailed,
    #[error("Event processing failed")]
    EventProcessingFailed,
    #[error("Path normalization failed")]
    PathNormalizationFailed,
}

/// The main unified error type for the dot001 toolkit
#[derive(Error, Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum Error {
    /// File system and I/O related errors
    #[error("I/O error: {message}")]
    Io {
        message: String,
        #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
        file_path: Option<PathBuf>,
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

    /// Writer operation errors
    #[error("Writer error: {message}")]
    Writer {
        message: String,
        #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
        file_path: Option<PathBuf>,
        kind: WriterErrorKind,
    },

    /// Watcher operation errors
    #[error("Watcher error: {message}")]
    Watcher {
        message: String,
        #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
        file_path: Option<PathBuf>,
        kind: WatcherErrorKind,
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

/// Alias for ErrorKind enum for backwards compatibility
pub use Error as ErrorKind;

/// Convenient result type for dot001 operations
pub type Result<T> = std::result::Result<T, Error>;

/// Extension trait for adding context to Results
pub trait ContextExt<T> {
    /// Add context message to an error
    fn context<C: Into<String>>(self, ctx: C) -> Result<T>;

    /// Add context message via closure (lazy evaluation)
    fn with_context<C, F>(self, f: F) -> Result<T>
    where
        C: Into<String>,
        F: FnOnce() -> C;
}

impl<T, E> ContextExt<T> for std::result::Result<T, E>
where
    E: Into<Error>,
{
    fn context<C: Into<String>>(self, ctx: C) -> Result<T> {
        self.map_err(|e| {
            let mut error = e.into();
            // Try to add context to the error message
            match error {
                Error::Io {
                    ref mut message, ..
                } => {
                    *message = format!("{}: {}", ctx.into(), message);
                }
                Error::BlendFile {
                    ref mut message, ..
                } => {
                    *message = format!("{}: {}", ctx.into(), message);
                }
                Error::Editor {
                    ref mut message, ..
                } => {
                    *message = format!("{}: {}", ctx.into(), message);
                }
                Error::Diff {
                    ref mut message, ..
                } => {
                    *message = format!("{}: {}", ctx.into(), message);
                }
                Error::Tracer {
                    ref mut message, ..
                } => {
                    *message = format!("{}: {}", ctx.into(), message);
                }
                Error::Writer {
                    ref mut message, ..
                } => {
                    *message = format!("{}: {}", ctx.into(), message);
                }
                Error::Watcher {
                    ref mut message, ..
                } => {
                    *message = format!("{}: {}", ctx.into(), message);
                }
                Error::Checkpoint {
                    ref mut message, ..
                } => {
                    *message = format!("{}: {}", ctx.into(), message);
                }
                Error::Cli {
                    ref mut message, ..
                } => {
                    *message = format!("{}: {}", ctx.into(), message);
                }
                Error::Config {
                    ref mut message, ..
                } => {
                    *message = format!("{}: {}", ctx.into(), message);
                }
            }
            error
        })
    }

    fn with_context<C, F>(self, f: F) -> Result<T>
    where
        C: Into<String>,
        F: FnOnce() -> C,
    {
        self.context(f())
    }
}

// === From implementations for standard library types ===

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Self::Io {
            message: err.to_string(),
            file_path: None,
            source_message: Some(format!("IO Error: {err}")),
        }
    }
}

// === Helper constructors ===

impl Error {
    /// Create a new I/O error with optional context
    pub fn io<M: Into<String>>(message: M) -> Self {
        Self::Io {
            message: message.into(),
            file_path: None,
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

    /// Create a new writer error
    pub fn writer<M: Into<String>>(message: M, kind: WriterErrorKind) -> Self {
        Self::Writer {
            message: message.into(),
            file_path: None,
            kind,
        }
    }

    /// Create a new watcher error
    pub fn watcher<M: Into<String>>(message: M, kind: WatcherErrorKind) -> Self {
        Self::Watcher {
            message: message.into(),
            file_path: None,
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
}

// === Contextual builder methods ===

impl Error {
    /// Add file path context to any error type
    pub fn with_file_path<P: Into<PathBuf>>(mut self, path: P) -> Self {
        match &mut self {
            Self::Io { file_path, .. } => *file_path = Some(path.into()),
            Self::BlendFile { file_path, .. } => *file_path = Some(path.into()),
            Self::Editor { file_path, .. } => *file_path = Some(path.into()),
            Self::Tracer { file_path, .. } => *file_path = Some(path.into()),
            Self::Writer { file_path, .. } => *file_path = Some(path.into()),
            Self::Watcher { file_path, .. } => *file_path = Some(path.into()),
            Self::Diff { file1_path, .. } => *file1_path = Some(path.into()),
            Self::Checkpoint { project_path, .. } => *project_path = Some(path.into()),
            Self::Config { config_path, .. } => *config_path = Some(path.into()),
            Self::Cli { .. } => {} // CLI errors don't have file paths
        }
        self
    }

    /// Add block index context to supported error types
    pub fn with_block_index(mut self, index: usize) -> Self {
        match &mut self {
            Self::BlendFile { block_index, .. } => *block_index = Some(index),
            Self::Editor { block_index, .. } => *block_index = Some(index),
            Self::Tracer { block_index, .. } => *block_index = Some(index),
            Self::Diff { block_index, .. } => *block_index = Some(index),
            _ => {} // Other error types don't have block indices
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

    /// Add command context to CLI errors
    pub fn with_command<C: Into<String>>(mut self, command: C) -> Self {
        if let Self::Cli { command: cmd, .. } = &mut self {
            *cmd = Some(command.into());
        }
        self
    }
}

// === Type checking methods ===

impl Error {
    /// Check if this error is an I/O error
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

    /// Check if this error is a diff error
    pub fn is_diff(&self) -> bool {
        matches!(self, Self::Diff { .. })
    }

    /// Check if this error is a tracer error
    pub fn is_tracer(&self) -> bool {
        matches!(self, Self::Tracer { .. })
    }

    /// Check if this error is a writer error
    pub fn is_writer(&self) -> bool {
        matches!(self, Self::Writer { .. })
    }

    /// Check if this error is a watcher error
    pub fn is_watcher(&self) -> bool {
        matches!(self, Self::Watcher { .. })
    }

    /// Check if this error is a checkpoint error
    pub fn is_checkpoint(&self) -> bool {
        matches!(self, Self::Checkpoint { .. })
    }

    /// Check if this error is a CLI error
    pub fn is_cli(&self) -> bool {
        matches!(self, Self::Cli { .. })
    }

    /// Check if this error is a config error
    pub fn is_config(&self) -> bool {
        matches!(self, Self::Config { .. })
    }
}

// === Context accessor methods ===

impl Error {
    /// Get the file path associated with this error, if any
    pub fn file_path(&self) -> Option<&PathBuf> {
        match self {
            Self::Io { file_path, .. } => file_path.as_ref(),
            Self::BlendFile { file_path, .. } => file_path.as_ref(),
            Self::Editor { file_path, .. } => file_path.as_ref(),
            Self::Tracer { file_path, .. } => file_path.as_ref(),
            Self::Writer { file_path, .. } => file_path.as_ref(),
            Self::Watcher { file_path, .. } => file_path.as_ref(),
            Self::Diff { file1_path, .. } => file1_path.as_ref(),
            Self::Checkpoint { project_path, .. } => project_path.as_ref(),
            Self::Config { config_path, .. } => config_path.as_ref(),
            Self::Cli { .. } => None,
        }
    }

    /// Get the block index associated with this error, if any
    pub fn block_index(&self) -> Option<usize> {
        match self {
            Self::BlendFile { block_index, .. } => *block_index,
            Self::Editor { block_index, .. } => *block_index,
            Self::Tracer { block_index, .. } => *block_index,
            Self::Diff { block_index, .. } => *block_index,
            _ => None,
        }
    }

    /// Get the operation context for editor errors, if any
    pub fn operation(&self) -> Option<&str> {
        if let Self::Editor { operation, .. } = self {
            operation.as_deref()
        } else {
            None
        }
    }

    /// Get the command context for CLI errors, if any
    pub fn command(&self) -> Option<&str> {
        if let Self::Cli { command, .. } = self {
            command.as_deref()
        } else {
            None
        }
    }
}

// === User-friendly message generation ===

impl Error {
    /// Generate a user-friendly error message with context
    pub fn user_message(&self) -> String {
        match self {
            Self::Io {
                message, file_path, ..
            } => {
                if let Some(path) = file_path {
                    format!("File operation failed on '{}': {}", path.display(), message)
                } else {
                    format!("File operation failed: {message}")
                }
            }
            Self::BlendFile {
                message,
                file_path,
                block_index,
                ..
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
                operation,
                ..
            } => {
                let mut msg = if let Some(op) = operation {
                    format!("Edit failed during {op}: {message}")
                } else {
                    format!("Edit failed: {message}")
                };
                if let Some(path) = file_path {
                    msg.push_str(&format!(" (file: {})", path.display()));
                }
                msg
            }
            Self::Diff {
                message,
                file1_path,
                file2_path,
                ..
            } => {
                let mut msg = format!("Diff operation failed: {message}");
                match (file1_path, file2_path) {
                    (Some(f1), Some(f2)) => {
                        msg.push_str(&format!(
                            " (comparing {} vs {})",
                            f1.display(),
                            f2.display()
                        ));
                    }
                    (Some(f1), None) => {
                        msg.push_str(&format!(" (file: {})", f1.display()));
                    }
                    _ => {}
                }
                msg
            }
            Self::Tracer {
                message, file_path, ..
            } => {
                let mut msg = format!("Dependency analysis failed: {message}");
                if let Some(path) = file_path {
                    msg.push_str(&format!(" (file: {})", path.display()));
                }
                msg
            }
            Self::Writer {
                message, file_path, ..
            } => {
                let mut msg = format!("Write operation failed: {message}");
                if let Some(path) = file_path {
                    msg.push_str(&format!(" (file: {})", path.display()));
                }
                msg
            }
            Self::Watcher {
                message, file_path, ..
            } => {
                let mut msg = format!("Watcher operation failed: {message}");
                if let Some(path) = file_path {
                    msg.push_str(&format!(" (file: {})", path.display()));
                }
                msg
            }
            Self::Checkpoint {
                message,
                project_path,
                ..
            } => {
                let mut msg = format!("Checkpoint operation failed: {message}");
                if let Some(path) = project_path {
                    msg.push_str(&format!(" (project: {})", path.display()));
                }
                msg
            }
            Self::Cli {
                message, command, ..
            } => {
                if let Some(cmd) = command {
                    format!("Command '{cmd}' failed: {message}")
                } else {
                    format!("Command failed: {message}")
                }
            }
            Self::Config {
                message,
                config_path,
                ..
            } => {
                let mut msg = format!("Configuration error: {message}");
                if let Some(path) = config_path {
                    msg.push_str(&format!(" (config: {})", path.display()));
                }
                msg
            }
        }
    }
}
