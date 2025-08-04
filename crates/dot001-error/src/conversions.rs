//! Conversion utilities for migrating from existing error types
//!
//! This module provides `From` implementations to convert from the existing
//! crate-specific error types to the unified `Dot001Error` type.

use crate::{BlendFileErrorKind, Dot001Error};

// We'll add these implementations once we update the crates, but define the structure now

/// Marker trait for errors that can provide contextual information
pub trait ErrorContext {
    /// Get the file path associated with this error, if any
    fn file_path(&self) -> Option<std::path::PathBuf> {
        None
    }

    /// Get the block index associated with this error, if any
    fn block_index(&self) -> Option<usize> {
        None
    }

    /// Get additional context information
    fn context(&self) -> Option<String> {
        None
    }
}

/// Helper macro for creating error conversions with context preservation
#[allow(unused_macros)] // Will be used when we implement actual conversions
macro_rules! impl_error_conversion {
    ($from_type:ty, $variant:ident, $kind_fn:expr) => {
        impl From<$from_type> for Dot001Error {
            fn from(err: $from_type) -> Self {
                let message = err.to_string();
                let file_path = if let Some(ctx) =
                    (&err as &dyn std::any::Any).downcast_ref::<dyn ErrorContext>()
                {
                    ctx.file_path()
                } else {
                    None
                };
                let block_index = if let Some(ctx) =
                    (&err as &dyn std::any::Any).downcast_ref::<dyn ErrorContext>()
                {
                    ctx.block_index()
                } else {
                    None
                };

                Dot001Error::$variant {
                    message,
                    file_path,
                    block_index,
                    kind: $kind_fn(&err),
                }
            }
        }
    };
}

// Actual conversions for existing error types

/// Convert the old parser BlendError to the unified error type
/// We'll implement this once we add the dependency
pub fn convert_parser_error<M: Into<String>>(message: M, old_error_kind: &str) -> Dot001Error {
    let kind = match old_error_kind {
        "InvalidHeader" => BlendFileErrorKind::InvalidHeader,
        "InvalidMagic" => BlendFileErrorKind::InvalidMagic,
        "UnsupportedVersion" => BlendFileErrorKind::UnsupportedVersion,
        "NoDnaFound" => BlendFileErrorKind::NoDnaFound,
        "InvalidBlockIndex" => BlendFileErrorKind::InvalidBlockIndex,
        "DnaError" => BlendFileErrorKind::DnaError,
        "InvalidData" => BlendFileErrorKind::InvalidData,
        "InvalidField" => BlendFileErrorKind::InvalidField,
        "UnsupportedCompression" => BlendFileErrorKind::UnsupportedCompression,
        "DecompressionFailed" => BlendFileErrorKind::DecompressionFailed,
        "SizeLimitExceeded" => BlendFileErrorKind::SizeLimitExceeded,
        _ => BlendFileErrorKind::InvalidData,
    };

    Dot001Error::blend_file(message.into(), kind)
}

/// Convert a generic error message to a Dot001Error
pub fn generic_error<S: Into<String>>(message: S) -> Dot001Error {
    Dot001Error::io(message.into())
}

/// Convert an error with file context
pub fn error_with_file<S: Into<String>, P: Into<std::path::PathBuf>>(
    message: S,
    file_path: P,
) -> Dot001Error {
    Dot001Error::io_with_path(message.into(), file_path.into())
}

/// Convert an error with block context
pub fn error_with_block<S: Into<String>, P: Into<std::path::PathBuf>>(
    message: S,
    file_path: P,
    block_index: usize,
) -> Dot001Error {
    Dot001Error::blend_file_with_context(
        message.into(),
        BlendFileErrorKind::InvalidData,
        Some(file_path.into()),
        Some(block_index),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generic_error() {
        let err = generic_error("Test error");
        assert!(err.is_io());
    }

    #[test]
    fn test_error_with_file() {
        let err = error_with_file("Test error", "/path/to/file.blend");
        assert!(err.is_io());
        assert_eq!(
            err.file_path(),
            Some(&std::path::PathBuf::from("/path/to/file.blend"))
        );
    }

    #[test]
    fn test_error_with_block() {
        let err = error_with_block("Test error", "/path/to/file.blend", 42);
        assert!(err.is_blend_file());
        assert_eq!(err.block_index(), Some(42));
    }
}
