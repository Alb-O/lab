//! # dot001_error - Unified Error Handling
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
//!
//! ## Module Organization
//!
//! - [`types`] - Main error types and Result type alias
//! - [`kinds`] - Error kind enums for fine-grained categorization
//! - [`helpers`] - Convenient functions for creating standardized errors
//! - [`conversions`] - Type conversions and contextual methods

// Re-export all public types and functions for backward compatibility
pub use kinds::*;
pub use types::*;

// Include all modules
pub mod conversions;
pub mod helpers;
pub mod kinds;
pub mod types;

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

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

    #[test]
    fn test_helper_functions() {
        let err = Dot001Error::parser_invalid_header("Bad magic bytes");
        assert!(err.is_blend_file());

        let err = Dot001Error::tracer_dependency_failed("Circular reference");
        assert!(err.is_tracer());

        let err = Dot001Error::editor_block_not_found("Block 42 missing");
        assert!(err.is_editor());
    }

    #[test]
    fn test_contextual_methods() {
        let err = Dot001Error::io("File not found").with_file_path("/test/file.blend");

        assert_eq!(err.file_path(), Some(&PathBuf::from("/test/file.blend")));

        let err = Dot001Error::blend_file("Invalid block", BlendFileErrorKind::InvalidBlockIndex)
            .with_block_index(123);

        assert_eq!(err.block_index(), Some(123));
    }

    #[test]
    fn test_type_checking() {
        let io_err = Dot001Error::io("Test");
        assert!(io_err.is_io());
        assert!(!io_err.is_editor());

        let editor_err = Dot001Error::editor("Test", EditorErrorKind::BlockNotFound);
        assert!(editor_err.is_editor());
        assert!(!editor_err.is_io());
    }
}
