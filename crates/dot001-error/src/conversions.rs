//! Error conversion implementations and contextual methods
//!
//! This module provides conversion implementations from standard library errors
//! to Dot001Error, as well as utility methods for adding context and checking
//! error types.

use crate::types::Dot001Error;
use std::path::PathBuf;

// === From implementations for standard library types ===

impl From<std::io::Error> for Dot001Error {
    fn from(err: std::io::Error) -> Self {
        Self::Io {
            message: err.to_string(),
            file_path: None,
            source_message: Some(format!("IO Error: {err}")),
        }
    }
}

// === Contextual builder methods ===

impl Dot001Error {
    /// Add file path context to any error type
    pub fn with_file_path<P: Into<PathBuf>>(mut self, path: P) -> Self {
        match &mut self {
            Self::Io { file_path, .. } => *file_path = Some(path.into()),
            Self::BlendFile { file_path, .. } => *file_path = Some(path.into()),
            Self::Editor { file_path, .. } => *file_path = Some(path.into()),
            Self::Tracer { file_path, .. } => *file_path = Some(path.into()),
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

impl Dot001Error {
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

impl Dot001Error {
    /// Get the file path associated with this error, if any
    pub fn file_path(&self) -> Option<&PathBuf> {
        match self {
            Self::Io { file_path, .. } => file_path.as_ref(),
            Self::BlendFile { file_path, .. } => file_path.as_ref(),
            Self::Editor { file_path, .. } => file_path.as_ref(),
            Self::Tracer { file_path, .. } => file_path.as_ref(),
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

impl Dot001Error {
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
