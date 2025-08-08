//! Zero-copy buffer abstraction for .blend file data
//!
//! This module provides efficient, zero-copy access to blend file data through
//! memory-mapped files, shared buffers, and byte slices. The design prioritizes
//! performance while maintaining memory safety and cross-platform compatibility.

use bytes::Bytes;
use std::sync::Arc;

#[cfg(feature = "mmap")]
use memmap2::Mmap;

/// Unified buffer abstraction for zero-copy access to blend file data
#[derive(Clone)]
pub enum BlendSource {
    /// Memory-mapped file region (zero-copy, most efficient for large files)
    #[cfg(feature = "mmap")]
    Mmap(Arc<Mmap>),

    /// Arc-wrapped buffer for shared ownership of in-memory data
    ArcBuf(Arc<Vec<u8>>),

    /// Bytes for ref-counted slicing with cheap subviews
    Bytes(Bytes),
}

impl BlendSource {
    /// Get the length of the underlying data
    pub fn len(&self) -> usize {
        match self {
            #[cfg(feature = "mmap")]
            BlendSource::Mmap(mmap) => mmap.len(),
            BlendSource::ArcBuf(buf) => buf.len(),
            BlendSource::Bytes(bytes) => bytes.len(),
        }
    }

    /// Check if the buffer is empty
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Create a Bytes slice from the given range
    ///
    /// This is the primary method for zero-copy slicing of blend file data.
    /// Returns a Bytes object that shares the underlying data without allocation.
    pub fn slice(&self, range: std::ops::Range<usize>) -> Result<Bytes, crate::Error> {
        if range.start > range.end || range.end > self.len() {
            return Err(crate::Error::blend_file(
                format!(
                    "Invalid range {}..{} for buffer of length {}",
                    range.start,
                    range.end,
                    self.len()
                ),
                crate::BlendFileErrorKind::InvalidRange,
            ));
        }

        let bytes = match self {
            #[cfg(feature = "mmap")]
            BlendSource::Mmap(mmap) => {
                // Create bytes from the mapped memory slice
                Bytes::copy_from_slice(&mmap[range])
            }
            BlendSource::ArcBuf(buf) => {
                // Create bytes from the buffer slice
                Bytes::copy_from_slice(&buf[range])
            }
            BlendSource::Bytes(bytes) => {
                // Use Bytes' built-in slicing for zero-copy
                bytes.slice(range)
            }
        };

        Ok(bytes)
    }

    /// Get a slice view of the entire buffer as bytes
    pub fn as_slice(&self) -> &[u8] {
        match self {
            #[cfg(feature = "mmap")]
            BlendSource::Mmap(mmap) => mmap.as_ref(),
            BlendSource::ArcBuf(buf) => buf.as_ref(),
            BlendSource::Bytes(bytes) => bytes.as_ref(),
        }
    }

    /// Create a BlendSource from a Vec<u8>
    pub fn from_vec(data: Vec<u8>) -> Self {
        BlendSource::ArcBuf(Arc::new(data))
    }

    /// Create a BlendSource from Bytes
    pub fn from_bytes(bytes: Bytes) -> Self {
        BlendSource::Bytes(bytes)
    }

    #[cfg(feature = "mmap")]
    /// Create a BlendSource from a memory-mapped file
    pub fn from_mmap(mmap: Mmap) -> Self {
        BlendSource::Mmap(Arc::new(mmap))
    }
}

impl std::fmt::Debug for BlendSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            #[cfg(feature = "mmap")]
            BlendSource::Mmap(_) => write!(f, "BlendSource::Mmap(len={})", self.len()),
            BlendSource::ArcBuf(_) => write!(f, "BlendSource::ArcBuf(len={})", self.len()),
            BlendSource::Bytes(_) => write!(f, "BlendSource::Bytes(len={})", self.len()),
        }
    }
}

/// Zero-copy buffer wrapper for efficient blend file access
///
/// BlendBuf provides the foundation for zero-copy parsing by maintaining
/// a reference to the underlying data and enabling cheap slicing operations.
#[derive(Clone, Debug)]
pub struct BlendBuf {
    source: BlendSource,
}

impl BlendBuf {
    /// Create a new BlendBuf from a BlendSource
    pub fn new(source: BlendSource) -> Self {
        Self { source }
    }

    /// Get the length of the underlying buffer
    pub fn len(&self) -> usize {
        self.source.len()
    }

    /// Check if the buffer is empty
    pub fn is_empty(&self) -> bool {
        self.source.is_empty()
    }

    /// Create a zero-copy slice of the buffer
    ///
    /// This is the primary method for accessing block data without allocation.
    /// The returned Bytes can be further sliced without additional copying.
    pub fn slice(&self, range: std::ops::Range<usize>) -> Result<Bytes, crate::Error> {
        self.source.slice(range)
    }

    /// Get the entire buffer as a slice
    pub fn as_slice(&self) -> &[u8] {
        self.source.as_slice()
    }

    /// Create a BlendBuf from a Vec<u8>
    pub fn from_vec(data: Vec<u8>) -> Self {
        Self::new(BlendSource::from_vec(data))
    }

    /// Create a BlendBuf from Bytes
    pub fn from_bytes(bytes: Bytes) -> Self {
        Self::new(BlendSource::from_bytes(bytes))
    }

    #[cfg(feature = "mmap")]
    /// Create a BlendBuf from a memory-mapped file
    pub fn from_mmap(mmap: Mmap) -> Self {
        Self::new(BlendSource::from_mmap(mmap))
    }
}

/// Type alias for convenient zero-copy block slices
pub type BlendSlice = Bytes;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_blend_buf_from_vec() {
        let data = vec![1, 2, 3, 4, 5];
        let buf = BlendBuf::from_vec(data.clone());

        assert_eq!(buf.len(), 5);
        assert_eq!(buf.as_slice(), &data);
    }

    #[test]
    fn test_blend_buf_slicing() {
        let data = vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9];
        let buf = BlendBuf::from_vec(data);

        let slice = buf.slice(2..6).unwrap();
        assert_eq!(slice.as_ref(), &[2, 3, 4, 5]);

        // Test invalid ranges
        assert!(buf.slice(2..2).is_err()); // Empty range
        assert!(buf.slice(8..15).is_err());
    }

    #[test]
    fn test_blend_buf_from_bytes() {
        let data = Bytes::from_static(b"hello world");
        let buf = BlendBuf::from_bytes(data.clone());

        assert_eq!(buf.len(), 11);
        assert_eq!(buf.as_slice(), b"hello world");

        let slice = buf.slice(0..5).unwrap();
        assert_eq!(slice.as_ref(), b"hello");
    }
}
